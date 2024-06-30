use std::io;
use tokio::{net::{TcpListener, TcpStream}, sync::broadcast::{self, Sender}};
use tracing::{error, info};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{EnvFilter, fmt};
use tracing_subscriber::fmt::writer::MakeWriterExt;
use tracing_subscriber::layer::SubscriberExt;

use chat_server::{handle, lib::Names,lib::Rooms};

const LOGS_DIR: &str = "logs";
pub fn stdout_logging() {
    let subscriber = tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(
            fmt::Layer::new()
            .without_time()
            .compact()
            .with_ansi(true)
            .with_writer(io::stdout)
        );
    tracing::subscriber::set_global_default(subscriber)
        .expect("Unable to set a global subscriber");
}

pub fn file_logging(rotation: Rotation, log_file: &str) -> WorkerGuard {
    let _ = std::fs::create_dir(LOGS_DIR);
    let file_appender = RollingFileAppender::new(rotation, LOGS_DIR, log_file);
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    let subscriber = tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(fmt::Layer::new().compact().with_ansi(false).with_writer(non_blocking));
    tracing::subscriber::set_global_default(subscriber)
        .expect("Unable to set a global subscriber");
    guard
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _guard = if cfg!(debug_assertions) {
        stdout_logging();
        tracing::info!("Running debug build");
        None
    } else {
        let guard = file_logging(Rotation::DAILY, "chat-server.log");
        tracing::info!("Running release build");
        Some(guard)
    };

    error!("start chat server");

    // 创建一个 Tcp Listener，监听 127.0.0.1:42069
    let server = TcpListener::bind("127.0.0.1:42069").await?;
    let names =Names::new();
    let rooms = Rooms::new();
    loop {
        let (tcp, _) = server.accept().await?;
        tokio::spawn(handle(tcp, rooms.clone(),names.clone()));
    }
}
