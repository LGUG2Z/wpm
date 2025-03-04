#![warn(clippy::all)]

use chrono::Utc;
use clap::Parser;
use fs_tail::TailedFile;
use interprocess::local_socket::traits::Listener;
use interprocess::local_socket::GenericNamespaced;
use interprocess::local_socket::ListenerOptions;
use interprocess::local_socket::ToNsName;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use wpm::communication::send_message;
use wpm::process_manager::ProcessManager;
use wpm::unit::Definition;
use wpm::unit::Executable;
use wpm::unit::ScoopExecutable;
use wpm::wpm_data_dir;
use wpm::wpm_units_dir;
use wpm::SocketMessage;

#[derive(Parser)]
#[clap(author, about, version)]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

macro_rules! gen_unit_subcommands {
    // SubCommand Pattern
    ( $( $name:ident ),+ $(,)? ) => {
        $(
            #[derive(clap::Parser)]
            pub struct $name {
                /// Target units
                units: Vec<String>,
            }
        )+
    };
}

gen_unit_subcommands! {
    Start,
    Stop,
    Reset,
    Restart
}

#[derive(Parser)]
struct Status {
    /// Target unit
    unit: String,
}

#[derive(Parser)]
struct Log {
    /// Target unit
    unit: Option<String>,
}

#[derive(Parser)]
enum SubCommand {
    /// Generate a JSON schema for wpm units
    #[clap(hide = true)]
    Schemagen,
    /// Generate some example wpm units
    #[clap(hide = true)]
    Examplegen,
    /// Start units
    #[clap(arg_required_else_help = true)]
    Start(Start),
    /// Stop units
    #[clap(arg_required_else_help = true)]
    Stop(Stop),
    /// Restart units
    #[clap(arg_required_else_help = true)]
    Restart(Restart),
    /// Reset units
    #[clap(arg_required_else_help = true)]
    Reset(Reset),
    /// Show the state of the process manager
    State,
    /// Show status of a unit
    #[clap(arg_required_else_help = true)]
    Status(Status),
    /// Reload all unit definitions
    Reload,
    /// Tail the logs of a unit or of the process manager
    Log(Log),
    /// Ensure all remote dependencies are downloaded and built
    Rebuild,
    /// Print the path to the wpm global unit definition directory
    Units,
}

fn listen_for_response() -> Result<String, Box<dyn std::error::Error>> {
    let name = "wpmctl.sock".to_ns_name::<GenericNamespaced>()?;
    let opts = ListenerOptions::new().name(name);

    let listener = match opts.create_sync() {
        Err(error) if error.kind() == std::io::ErrorKind::AddrInUse => {
            println!("{error}");
            return Err(error.into());
        }
        x => x?,
    };

    let mut buf = String::new();
    let stream = match listener.accept() {
        Ok(connection) => connection,
        Err(error) => {
            println!("{error}");
            return Err(error.into());
        }
    };

    let mut receiver = BufReader::new(&stream);

    receiver.read_to_string(&mut buf)?;

    Ok(buf)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts: Opts = Opts::parse();
    match opts.subcmd {
        SubCommand::Schemagen => {
            println!("{}", Definition::schemagen());
        }
        SubCommand::Examplegen => {
            Definition::examplegen();
        }
        SubCommand::Start(args) => {
            send_message("wpmd.sock", SocketMessage::Start(args.units))?;
        }
        SubCommand::Stop(args) => {
            send_message("wpmd.sock", SocketMessage::Stop(args.units))?;
        }
        SubCommand::Restart(args) => {
            send_message("wpmd.sock", SocketMessage::Restart(args.units))?;
        }
        SubCommand::Reset(args) => {
            send_message("wpmd.sock", SocketMessage::Reset(args.units))?;
        }
        SubCommand::Status(args) => {
            send_message("wpmd.sock", SocketMessage::Status(args.unit.clone()))?;
            let response = listen_for_response()?;
            println!("{}", response);
        }
        SubCommand::State => {
            send_message("wpmd.sock", SocketMessage::State)?;
            println!("{}", listen_for_response()?);
        }
        SubCommand::Reload => {
            send_message("wpmd.sock", SocketMessage::Reload)?;
        }
        SubCommand::Log(args) => match args.unit {
            None => {
                let timestamp = Utc::now().format("%Y-%m-%d").to_string();
                let color_log = std::env::temp_dir().join(format!("wpmd.log.{timestamp}"));
                let file = TailedFile::new(File::open(color_log)?);
                let locked = file.lock();
                #[allow(clippy::significant_drop_in_scrutinee, clippy::lines_filter_map_ok)]
                for line in locked.lines().flatten() {
                    println!("{line}");
                }
            }
            Some(unit) => {
                let dir = wpm_data_dir().join("logs");
                let file = File::open(dir.join(format!("{}.log", unit))).unwrap();

                let file = TailedFile::new(file);
                let locked = file.lock();
                #[allow(clippy::significant_drop_in_scrutinee, clippy::lines_filter_map_ok)]
                for line in locked.lines().flatten() {
                    println!("{line}");
                }
            }
        },
        SubCommand::Rebuild => {
            let units = ProcessManager::retrieve_units()?;
            for unit in units {
                let name = unit.unit.name;
                let executable = unit.service.exec_start.executable;
                let url = match &executable {
                    Executable::Remote(remote) => remote.url.to_string(),
                    Executable::Scoop(scoop) => match scoop {
                        ScoopExecutable::Package(_) => continue,
                        ScoopExecutable::Manifest(manifest) => manifest.manifest.to_string(),
                    },
                    _ => continue,
                };

                let path = executable.cached_executable_path()?;
                if !path.is_file() {
                    println!("[{name}]: Downloading from {url}");
                    executable.download_remote_executable()?;
                } else {
                    println!("[{name}]: Already exists at {}", path.display());
                }
            }
        }
        SubCommand::Units => {
            println!("{}", wpm_units_dir().display());
        }
    }

    Ok(())
}
