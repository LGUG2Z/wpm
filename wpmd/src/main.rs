#![warn(clippy::all)]

use crate::process_manager::ProcessManager;
use crate::process_manager::ProcessManagerError;
use interprocess::local_socket::tokio::Stream;
use interprocess::local_socket::traits::tokio::Listener;
use interprocess::local_socket::GenericNamespaced;
use interprocess::local_socket::ListenerOptions;
use interprocess::local_socket::ToNsName;
use std::sync::Arc;
use thiserror::Error;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::sync::Mutex;
use tracing_subscriber::EnvFilter;
use wpmc::SocketMessage;

mod process_manager;
mod unit;

static SOCKET_NAME: &str = "wpmd.sock";

#[derive(Error, Debug)]
pub enum WpmdError {
    #[error(transparent)]
    ProcessManager(#[from] ProcessManagerError),
    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("RUST_LIB_BACKTRACE").is_err() {
        std::env::set_var("RUST_LIB_BACKTRACE", "1");
    }

    color_eyre::install()?;

    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }

    tracing::subscriber::set_global_default(
        tracing_subscriber::fmt::Subscriber::builder()
            .with_env_filter(EnvFilter::from_default_env())
            .finish(),
    )?;

    let process_manager = ProcessManager::init()?;

    let process_manager_arc = Arc::new(Mutex::new(process_manager));
    let loop_arc = process_manager_arc.clone();
    let ctrlc_arc = process_manager_arc.clone();

    let name = SOCKET_NAME.to_ns_name::<GenericNamespaced>()?;
    let opts = ListenerOptions::new().name(name);

    let listener = match opts.create_tokio() {
        Err(error) if error.kind() == std::io::ErrorKind::AddrInUse => {
            tracing::error!("{error}");
            return Err(error.into());
        }
        x => x?,
    };

    tracing::info!("listening on {SOCKET_NAME}");

    tokio::spawn(async move {
        loop {
            let conn = match listener.accept().await {
                Ok(connection) => connection,
                Err(error) => {
                    tracing::error!("{error}");
                    continue;
                }
            };

            let pm = loop_arc.clone();

            tokio::spawn(async move {
                if let Err(error) = handle_connection(pm, conn).await {
                    tracing::error!("{error}");
                }
            });
        }
    });

    let (ctrlc_sender, ctrlc_receiver) = std::sync::mpsc::channel();
    ctrlc::set_handler(move || {
        ctrlc_sender
            .send(())
            .expect("could not send signal on ctrl-c channel");
    })?;

    ctrlc_receiver
        .recv()
        .expect("could not receive signal on ctrl-c channel");

    ctrlc_arc.lock().await.shutdown().await?;

    Ok(())
}

async fn handle_connection(pm: Arc<Mutex<ProcessManager>>, conn: Stream) -> Result<(), WpmdError> {
    let mut receiver = BufReader::new(&conn);
    let mut buf = String::new();
    receiver.read_line(&mut buf).await?;

    let socket_message: SocketMessage = serde_json::from_str(&buf)?;
    match socket_message {
        SocketMessage::Start(arg) => {
            pm.lock().await.start(&arg).await?;
        }
        SocketMessage::Stop(arg) => {
            pm.lock().await.stop(&arg).await?;
        }
        SocketMessage::Status(_arg) => {}
        SocketMessage::Reload => {
            pm.lock().await.load_units()?;
        }
    }

    Ok(())
}
