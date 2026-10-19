//! Integration with the Windows Service Control Manager: running as a
//! service, installing and uninstalling.

use std::ffi::OsString;
use std::path::Path;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use anyhow::{Context, bail};
use chrono::Utc;
use tokio::sync::watch;
use windows_service::service::{
    PowerEventParam, ServiceAccess, ServiceAction, ServiceActionType, ServiceControl,
    ServiceControlAccept, ServiceErrorControl, ServiceExitCode, ServiceFailureActions,
    ServiceFailureResetPeriod, ServiceInfo, ServiceStartType, ServiceState, ServiceStatus,
    ServiceType,
};
use windows_service::service_control_handler::{
    self, ServiceControlHandlerResult, ServiceStatusHandle,
};
use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};
use windows_service::{define_windows_service, service_dispatcher};
use windows_sys::Win32::Foundation::{ERROR_INVALID_SERVICE_CONTROL, ERROR_SERVICE_DOES_NOT_EXIST};

use crate::app::App;
use crate::engine::Engine;
use crate::store::{JsonFileStore, MemoryStore, Store};
use crate::{Options, Service};

pub const SERVICE_NAME: &str = "FocusLock";
const DISPLAY_NAME: &str = "Focus Lock";
const DESCRIPTION: &str = "Keeps focus sessions running: limits Spotify to the allowed playlists.";
const SERVICE_TYPE: ServiceType = ServiceType::OWN_PROCESS;

/// How often the accepted controls are brought in line with the session.
const STATUS_PERIOD: Duration = Duration::from_secs(1);
/// How long `uninstall` waits for the service to stop.
const STOP_TIMEOUT: Duration = Duration::from_secs(30);

static OPTIONS: OnceLock<Options> = OnceLock::new();

define_windows_service!(ffi_service_main, service_main);

/// Hands the process to the Service Control Manager. Returns when the
/// service stops.
///
/// # Errors
/// If the process was not started by the Service Control Manager.
pub fn dispatch(options: Options) -> anyhow::Result<()> {
    let _ = OPTIONS.set(options);
    service_dispatcher::start(SERVICE_NAME, ffi_service_main)
        .context("not started by the Service Control Manager; use `run` in a console")
}

fn service_main(_arguments: Vec<OsString>) {
    if let Err(error) = run_service() {
        tracing::error!("service failed: {error:#}");
    }
}

fn run_service() -> anyhow::Result<()> {
    let options = OPTIONS.get().context("options not set")?;
    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
    let app_slot: Arc<OnceLock<App>> = Arc::default();

    let handler_app = app_slot.clone();
    let handler = move |control| -> ServiceControlHandlerResult {
        let app = handler_app.get();
        match control {
            ServiceControl::Stop => {
                // STOP is not accepted while locked; this catches a session
                // that became locked after the status was last updated.
                if app.is_some_and(App::is_locked) {
                    return ServiceControlHandlerResult::Other(ERROR_INVALID_SERVICE_CONTROL);
                }
                shutdown_tx.send_replace(true);
                ServiceControlHandlerResult::NoError
            }
            // The machine is shutting down. The state is already saved.
            ServiceControl::Shutdown => {
                shutdown_tx.send_replace(true);
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::PowerEvent(
                PowerEventParam::ResumeAutomatic | PowerEventParam::ResumeSuspend,
            ) => {
                if let Some(app) = app {
                    app.resume();
                }
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate | ServiceControl::PowerEvent(_) => {
                ServiceControlHandlerResult::NoError
            }
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };
    let status = service_control_handler::register(SERVICE_NAME, handler)?;
    set_status(
        status,
        ServiceState::StartPending,
        ServiceControlAccept::empty(),
    )?;

    let runtime = tokio::runtime::Runtime::new()?;
    let result = runtime.block_on(async {
        let service = Service::bind(options).await?;
        let app = service.app();
        let _ = app_slot.set(app.clone());
        let mut accepted = accepted_controls(&app);
        set_status(status, ServiceState::Running, accepted)?;

        let shutdown = async {
            let mut interval = tokio::time::interval(STATUS_PERIOD);
            loop {
                tokio::select! {
                    _ = shutdown_rx.wait_for(|stop| *stop) => return,
                    _ = interval.tick() => {}
                }
                let now = accepted_controls(&app);
                if now != accepted {
                    match set_status(status, ServiceState::Running, now) {
                        Ok(()) => accepted = now,
                        Err(error) => tracing::warn!("cannot update service status: {error}"),
                    }
                }
            }
        };
        service.run(shutdown).await
    });

    let exit = u32::from(result.is_err());
    status.set_service_status(ServiceStatus {
        service_type: SERVICE_TYPE,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::ServiceSpecific(exit),
        checkpoint: 0,
        wait_hint: Duration::ZERO,
        process_id: None,
    })?;
    result
}

/// A locked session must not be stoppable, so STOP is only accepted
/// outside one.
fn accepted_controls(app: &App) -> ServiceControlAccept {
    let always = ServiceControlAccept::SHUTDOWN | ServiceControlAccept::POWER_EVENT;
    if app.is_locked() {
        always
    } else {
        always | ServiceControlAccept::STOP
    }
}

fn set_status(
    handle: ServiceStatusHandle,
    state: ServiceState,
    accepted: ServiceControlAccept,
) -> windows_service::Result<()> {
    handle.set_service_status(ServiceStatus {
        service_type: SERVICE_TYPE,
        current_state: state,
        controls_accepted: accepted,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: if state == ServiceState::StartPending {
            Duration::from_secs(10)
        } else {
            Duration::ZERO
        },
        process_id: None,
    })
}

/// Registers the running executable as an auto-start service that runs as
/// `LocalSystem` and restarts on failure, then starts it.
///
/// # Errors
/// If not run as an administrator or the service already exists.
pub fn install() -> anyhow::Result<()> {
    let executable = std::env::current_exe()?;
    let manager = ServiceManager::local_computer(
        None::<&str>,
        ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE,
    )
    .context("cannot open the Service Control Manager (run as administrator)")?;
    let info = ServiceInfo {
        name: SERVICE_NAME.into(),
        display_name: DISPLAY_NAME.into(),
        service_type: SERVICE_TYPE,
        start_type: ServiceStartType::AutoStart,
        error_control: ServiceErrorControl::Normal,
        executable_path: executable.clone(),
        launch_arguments: vec!["service".into()],
        dependencies: vec![],
        account_name: None, // LocalSystem
        account_password: None,
    };
    let service = manager
        .create_service(
            &info,
            ServiceAccess::CHANGE_CONFIG | ServiceAccess::START | ServiceAccess::QUERY_STATUS,
        )
        .context("cannot create the service (is it already installed?)")?;
    service.set_description(DESCRIPTION)?;

    let restart = ServiceAction {
        action_type: ServiceActionType::Restart,
        delay: Duration::from_secs(2),
    };
    service.update_failure_actions(ServiceFailureActions {
        reset_period: ServiceFailureResetPeriod::After(Duration::from_hours(24)),
        reboot_msg: None,
        command: None,
        actions: Some(vec![restart; 3]),
    })?;
    service.set_failure_actions_on_non_crash_failures(true)?;
    service.start::<&str>(&[])?;
    println!(
        "Installed and started {SERVICE_NAME} ({})",
        executable.display()
    );
    Ok(())
}

/// Stops and removes the service. Refused while a locked session runs.
/// Succeeds without changes when the service is not installed, and waits
/// until it has stopped so the installer can replace its files.
///
/// # Errors
/// If not run as an administrator, a locked session is running or the
/// service doesn't stop in time.
pub fn uninstall(data_dir: &Path) -> anyhow::Result<()> {
    if is_locked(data_dir)? {
        bail!("a locked session is running; the service can be removed after it ends");
    }
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
        .context("cannot open the Service Control Manager (run as administrator)")?;
    let service = match manager.open_service(
        SERVICE_NAME,
        ServiceAccess::QUERY_STATUS | ServiceAccess::STOP | ServiceAccess::DELETE,
    ) {
        Ok(service) => service,
        Err(windows_service::Error::Winapi(error))
            if error.raw_os_error() == Some(ERROR_SERVICE_DOES_NOT_EXIST.cast_signed()) =>
        {
            println!("{SERVICE_NAME} is not installed");
            return Ok(());
        }
        Err(error) => return Err(error).context("cannot open the service"),
    };
    if service.query_status()?.current_state != ServiceState::Stopped {
        service.stop().context("the service refused to stop")?;
        let deadline = std::time::Instant::now() + STOP_TIMEOUT;
        while service.query_status()?.current_state != ServiceState::Stopped {
            if std::time::Instant::now() > deadline {
                bail!("the service didn't stop within {}s", STOP_TIMEOUT.as_secs());
            }
            std::thread::sleep(Duration::from_millis(200));
        }
    }
    service.delete()?;
    println!("Removed {SERVICE_NAME}");
    Ok(())
}

/// Reads the saved state, crediting the time since its last checkpoint,
/// without changing it.
fn is_locked(data_dir: &Path) -> anyhow::Result<bool> {
    let saved = JsonFileStore::open(data_dir)?
        .load()
        .with_context(|| format!("reading the state in {}", data_dir.display()))?;
    let engine = Engine::load(Box::new(MemoryStore(saved)), Utc::now());
    Ok(engine.is_locked())
}
