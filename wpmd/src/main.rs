#![warn(clippy::all)]

use clap::Parser;
use interprocess::local_socket::traits::Listener;
use interprocess::local_socket::GenericNamespaced;
use interprocess::local_socket::ListenerOptions;
use interprocess::local_socket::Stream;
use interprocess::local_socket::ToNsName;
use parking_lot::Mutex;
use std::io::BufRead;
use std::io::BufReader;
use std::process::exit;
use std::sync::mpsc;
use std::sync::Arc;
use sysinfo::Process;
use sysinfo::ProcessesToUpdate;
use sysinfo::System;
use thiserror::Error;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::EnvFilter;
use wpm::communication::send_str;
use wpm::process_manager::ProcessManager;
use wpm::process_manager::ProcessManagerError;
use wpm::SocketMessage;

shadow_rs::shadow!(build);

static SOCKET_NAME: &str = "wpmd.sock";

#[derive(Parser)]
#[clap(author, about, version = build::CLAP_LONG_VERSION)]
struct Opts;

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
    let _opts: Opts = Opts::parse();

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

    {
        let mut system = System::new_all();
        system.refresh_processes(ProcessesToUpdate::All, true);
        let matched_procs: Vec<&Process> = system.processes_by_name("wpmd.exe".as_ref()).collect();
        if matched_procs.len() > 1 {
            let mut len = matched_procs.len();
            for proc in matched_procs {
                if let Some(executable_path) = proc.exe() {
                    if executable_path.to_string_lossy().contains("shims") {
                        len -= 1;
                    }
                }
            }

            if len > 1 {
                tracing::error!("wpmd.exe is already running, please exit the existing process before starting a new one");
                exit(1);
            }
        }
    }

    let process_manager = ProcessManager::init()?;

    let process_manager_arc = Arc::new(Mutex::new(process_manager));
    let loop_arc = process_manager_arc.clone();
    let ctrlc_arc = process_manager_arc.clone();

    let name = SOCKET_NAME.to_ns_name::<GenericNamespaced>()?;
    let opts = ListenerOptions::new().name(name.clone());

    let (tx, rx) = mpsc::channel::<SocketMessage>();

    let listener = match opts.create_sync() {
        Err(error) if error.kind() == std::io::ErrorKind::AddrInUse => {
            tracing::error!("failed to create listener: {error}");
            return Err(error.into());
        }
        x => x?,
    };

    tracing::info!("listening on {SOCKET_NAME}");

    std::thread::spawn(move || loop {
        let conn = match listener.accept() {
            Ok(connection) => connection,
            Err(error) => {
                tracing::error!("failed to accept incoming socket connection: {error}");
                continue;
            }
        };

        if let Ok(socket_message) = extract_socket_message(conn) {
            match tx.send(socket_message) {
                Ok(_) => {
                    tracing::info!("successfully queued socket message");
                }
                Err(_) => {
                    tracing::warn!("failed to queue socket message");
                }
            }
        }
    });

    std::thread::spawn(move || {
        while let Ok(message) = rx.recv() {
            let pm = loop_arc.clone();
            if let Err(error) = handle_socket_message(pm, message) {
                tracing::error!("{error}")
            }
        }
    });

    let (ctrlc_sender, ctrlc_receiver) = mpsc::channel();
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

fn extract_socket_message(conn: Stream) -> Result<SocketMessage, WpmdError> {
    let mut conn = BufReader::new(&conn);
    let mut buf = String::new();
    conn.read_line(&mut buf)?;

    match serde_json::from_str::<SocketMessage>(&buf) {
        Err(error) => {
            tracing::error!("failed to deserialize socket message: {error}");
            Err(WpmdError::SerdeJson(error))
        }
        Ok(socket_message) => {
            tracing::info!("received socket message: {socket_message:?}");
            Ok(socket_message)
        }
    }
}

fn handle_socket_message(
    pm: Arc<Mutex<ProcessManager>>,
    socket_message: SocketMessage,
) -> Result<(), WpmdError> {
    let mut pm = pm.lock();

    match socket_message {
        SocketMessage::Start(arg) => {
            for name in arg {
                pm.start(&name)?;
            }
        }
        SocketMessage::Stop(arg) => {
            for name in arg {
                pm.stop(&name)?;
            }
        }
        SocketMessage::Restart(arg) => {
            for name in arg {
                pm.stop(&name)?;
                pm.start(&name)?;
            }
        }
        SocketMessage::Status(arg) => {
            let status_message = pm.state().unit_status(&arg)?;
            send_str("wpmctl.sock", &status_message)?;
        }
        SocketMessage::State => {
            let table = format!("{}\n", pm.state().as_table());
            send_str("wpmctl.sock", &table)?;
        }
        SocketMessage::Reload => {
            pm.load_units()?;
        }
        SocketMessage::Reset(arg) => {
            for name in arg {
                pm.reset(&name);
            }
        }
    }

    Ok(())
}
