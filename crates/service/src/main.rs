use std::path::PathBuf;

use clap::{Parser, Subcommand};
use tunelock_protocol::{CONTROL_SOCKET, EXTENSION_PORT};

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
    Run {
        /// Where to keep the service's data.
        #[arg(long, env = "TUNELOCK_DATA_DIR", default_value_os_t = default_data_dir())]
        data_dir: PathBuf,
        /// Name of the control socket.
        #[arg(long, default_value = CONTROL_SOCKET)]
        socket: String,
        /// Loopback port for the Spotify extension.
        #[arg(long, default_value_t = EXTENSION_PORT)]
        port: u16,
    },
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    match Cli::parse().command {
        Command::Run {
            data_dir,
            socket,
            port,
        } => tunelock_service::run(&data_dir, &socket, port).await,
    }
}
