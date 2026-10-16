use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use tunelock_protocol::{CONTROL_SOCKET, EXTENSION_PORT};
use tunelock_service::Options;

#[derive(Parser)]
#[command(
    name = "tunelock-service",
    version,
    about = "Tunelock background service"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run in the foreground (development and debugging).
    Run(RunArgs),
    /// Entry point used by the Windows Service Control Manager.
    #[cfg(windows)]
    #[command(hide = true)]
    Service,
    /// Register and start the Windows service (as administrator).
    #[cfg(windows)]
    Install,
    /// Stop and remove the Windows service (as administrator). Refused
    /// while a locked session runs.
    #[cfg(windows)]
    Uninstall,
}

#[derive(Args)]
struct RunArgs {
    /// Where to keep the service's data.
    #[arg(long, env = "TUNELOCK_DATA_DIR", default_value_os_t = default_data_dir())]
    data_dir: PathBuf,
    /// Name of the control socket.
    #[arg(long, default_value = CONTROL_SOCKET)]
    socket: String,
    /// Loopback port for the Spotify extension.
    #[arg(long, default_value_t = EXTENSION_PORT)]
    port: u16,
}

fn default_data_dir() -> PathBuf {
    if cfg!(windows) {
        std::env::var_os("ProgramData")
            .map_or_else(|| PathBuf::from(r"C:\ProgramData"), PathBuf::from)
            .join("Tunelock")
    } else {
        std::env::var_os("HOME")
            .map_or_else(|| PathBuf::from("."), PathBuf::from)
            .join(".local/state/tunelock")
    }
}

fn env_filter() -> tracing_subscriber::EnvFilter {
    tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into())
}

fn main() -> anyhow::Result<()> {
    match Cli::parse().command {
        Command::Run(args) => {
            tracing_subscriber::fmt()
                .with_env_filter(env_filter())
                .init();
            let options = Options {
                data_dir: args.data_dir,
                socket: args.socket,
                port: args.port,
                system: false,
            };
            tokio::runtime::Runtime::new()?.block_on(tunelock_service::run_console(&options))
        }
        #[cfg(windows)]
        Command::Service => {
            let options = Options {
                data_dir: default_data_dir(),
                socket: CONTROL_SOCKET.to_owned(),
                port: EXTENSION_PORT,
                system: true,
            };
            log_to_file(&options.data_dir)?;
            tunelock_service::windows::scm::dispatch(options)
        }
        #[cfg(windows)]
        Command::Install => tunelock_service::windows::scm::install(),
        #[cfg(windows)]
        Command::Uninstall => tunelock_service::windows::scm::uninstall(&default_data_dir()),
    }
}

/// A service has no console, so it logs to `service.log` in the data
/// directory. The log starts over once it grows past 1 MiB.
#[cfg(windows)]
fn log_to_file(data_dir: &std::path::Path) -> anyhow::Result<()> {
    const MAX_LOG_LEN: u64 = 1024 * 1024;
    std::fs::create_dir_all(data_dir)?;
    let path = data_dir.join("service.log");
    let too_big = std::fs::metadata(&path).is_ok_and(|m| m.len() > MAX_LOG_LEN);
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(!too_big)
        .write(true)
        .truncate(too_big)
        .open(path)?;
    tracing_subscriber::fmt()
        .with_env_filter(env_filter())
        .with_ansi(false)
        .with_writer(std::sync::Mutex::new(file))
        .init();
    Ok(())
}
