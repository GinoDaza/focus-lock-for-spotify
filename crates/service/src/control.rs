//! Control channel: line-delimited JSON over a local socket.

use std::io;

use chrono::Local;
use interprocess::local_socket::tokio::{Listener, Stream, prelude::*};
use interprocess::local_socket::{GenericNamespaced, ListenerOptions};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tunelock_protocol::{
    ErrorBody, ErrorCode, MAX_LINE_LEN, Request, Response, decode_line, encode_line,
};

use crate::app::App;

/// Who may open the control pipe when running as the Windows service:
/// SYSTEM, Administrators and interactively logged-on users. Remote clients
/// are rejected by the pipe itself. What each client may do is decided by
/// the engine, never by who is asking.
#[cfg(windows)]
const PIPE_SDDL: &str = "D:(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;IU)";

/// Binds the control socket. With `system`, the Windows named pipe gets a
/// security descriptor that lets interactive users connect to a service
/// running as SYSTEM.
///
/// # Errors
/// If the name is invalid or already in use.
pub fn bind(name: &str, system: bool) -> io::Result<Listener> {
    let name = name.to_ns_name::<GenericNamespaced>()?;
    let options = ListenerOptions::new().name(name);
    #[cfg(windows)]
    let options = if system {
        use interprocess::os::windows::local_socket::ListenerOptionsExt;
        use interprocess::os::windows::security_descriptor::SecurityDescriptor;
        let sddl = widestring::U16CString::from_str(PIPE_SDDL)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        options.security_descriptor(SecurityDescriptor::deserialize(&sddl)?)
    } else {
        options
    };
    #[cfg(not(windows))]
    let _ = system;
    options.create_tokio()
}

/// Accepts clients forever, one task per connection.
pub async fn serve(listener: Listener, app: App) {
    loop {
        match listener.accept().await {
            Ok(stream) => {
                let app = app.clone();
                tokio::spawn(async move {
                    if let Err(error) = handle_connection(stream, app).await {
                        tracing::debug!("control connection closed: {error}");
                    }
                });
            }
            Err(error) => tracing::warn!("control accept failed: {error}"),
        }
    }
}

async fn handle_connection(stream: Stream, app: App) -> io::Result<()> {
    let mut reader = BufReader::new(&stream);
    let mut writer = &stream;
    let mut line = String::new();
    loop {
        line.clear();
        let limit = u64::try_from(MAX_LINE_LEN).unwrap_or(u64::MAX) + 1;
        let read = (&mut reader).take(limit).read_line(&mut line).await?;
        if read == 0 {
            return Ok(());
        }
        if line.len() > MAX_LINE_LEN {
            let response = error(ErrorCode::InvalidInput, "message too long".into());
            writer.write_all(encode_line(&response)?.as_bytes()).await?;
            return Ok(());
        }

        let response = match decode_line::<Request>(&line) {
            Ok(request) => {
                tracing::debug!(?request, "control request");
                app.engine().handle(request, Local::now().naive_local())
            }
            Err(e) => error(ErrorCode::InvalidInput, format!("invalid request: {e}")),
        };
        writer.write_all(encode_line(&response)?.as_bytes()).await?;
    }
}

fn error(code: ErrorCode, message: String) -> Response {
    Response::Error(ErrorBody { code, message })
}
