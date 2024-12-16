#![warn(clippy::all)]

use interprocess::local_socket::traits::Listener;
use interprocess::local_socket::GenericNamespaced;
use interprocess::local_socket::ListenerOptions;
use interprocess::local_socket::Stream;
use interprocess::local_socket::ToNsName;
use parking_lot::Mutex;
use std::io::BufRead;
use std::io::BufReader;
use std::sync::Arc;
use thiserror::Error;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::EnvFilter;
use wpm::communication::send_str;
use wpm::process_manager::ProcessManager;
use wpm::process_manager::ProcessManagerError;
use wpm::SocketMessage;

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("RUST_LIB_BACKTRACE").is_err() {
        std::env::set_var("RUST_LIB_BACKTRACE", "1");
    }

    color_eyre::install()?;

    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }

    let appender = tracing_appender::rolling::daily(std::env::temp_dir(), "wpmd_plaintext.log");
    let color_appender = tracing_appender::rolling::daily(std::env::temp_dir(), "wpmd.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(appender);
    let (color_non_blocking, _color_guard) = tracing_appender::non_blocking(color_appender);

    tracing::subscriber::set_global_default(
        tracing_subscriber::fmt::Subscriber::builder()
            .with_env_filter(EnvFilter::from_default_env())
            .finish()
            .with(
                tracing_subscriber::fmt::Layer::default()
                    .with_writer(non_blocking)
                    .with_ansi(false),
            )
            .with(
                tracing_subscriber::fmt::Layer::default()
                    .with_writer(color_non_blocking)
                    .with_ansi(true),
            ),
    )?;

    let process_manager = ProcessManager::init()?;

    let process_manager_arc = Arc::new(Mutex::new(process_manager));
    let loop_arc = process_manager_arc.clone();
    let ctrlc_arc = process_manager_arc.clone();

    let name = SOCKET_NAME.to_ns_name::<GenericNamespaced>()?;
    let opts = ListenerOptions::new().name(name);

    let listener = match opts.create_sync() {
        Err(error) if error.kind() == std::io::ErrorKind::AddrInUse => {
            tracing::error!("{error}");
            return Err(error.into());
        }
        x => x?,
    };

    tracing::info!("listening on {SOCKET_NAME}");

    std::thread::spawn(move || loop {
        let conn = match listener.accept() {
            Ok(connection) => connection,
            Err(error) => {
                tracing::error!("{error}");
                continue;
            }
        };

        let pm = loop_arc.clone();

        if let Err(error) = handle_connection(pm, conn) {
            tracing::error!("{error}");
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

    ctrlc_arc.lock().shutdown()?;

    Ok(())
}

fn handle_connection(pm: Arc<Mutex<ProcessManager>>, conn: Stream) -> Result<(), WpmdError> {
    let mut conn = BufReader::new(&conn);
    let mut buf = String::new();
    conn.read_line(&mut buf)?;
    match serde_json::from_str::<SocketMessage>(&buf) {
        Err(error) => {
            tracing::error!("{error}");
        }
        Ok(socket_message) => {
            tracing::info!("received socket message: {socket_message:?}");

            let mut pm = pm.lock();

            match socket_message {
                SocketMessage::Start(arg) => {
                    pm.start(&arg)?;
                }
                SocketMessage::Stop(arg) => {
                    pm.stop(&arg)?;
                }
                SocketMessage::Status(arg) => {
                    if let Some(status) = pm.state().unit(&arg) {
                        send_str("wpmctl.sock", &status.state.to_string())?;
                    } else {
                        send_str("wpmctl.sock", &format!("Unregistered unit: {arg}"))?;
                    }
                }
                SocketMessage::State => {
                    let table = format!("{}\n", pm.state().as_table());
                    send_str("wpmctl.sock", &table)?;
                }
                SocketMessage::Reload => {
                    pm.load_units()?;
                }
                SocketMessage::Reset(arg) => {
                    pm.reset(&arg);
                }
            }
        }
    }

    Ok(())
}
