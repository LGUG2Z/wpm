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
use wpm::unit::WpmUnit;
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
                /// Target unit
                unit: String,
            }
        )+
    };
}

gen_unit_subcommands! {
    Start,
    Stop,
    Status,
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
    /// Start a unit
    Start(Start),
    /// Stop a unit
    Stop(Stop),
    /// Show the state of the process manager
    State,
    /// Show status of a unit
    Status(Status),
    /// Reload all unit definitions
    Reload,
    /// Tail the logs of a unit or of the process manager
    Log(Log),
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
            println!("{}", WpmUnit::schemagen());
        }
        SubCommand::Examplegen => {
            WpmUnit::examplegen();
        }
        SubCommand::Start(args) => {
            send_message("wpmd.sock", SocketMessage::Start(args.unit))?;
        }
        SubCommand::Stop(args) => {
            send_message("wpmd.sock", SocketMessage::Stop(args.unit))?;
        }
        SubCommand::Status(args) => {
            send_message("wpmd.sock", SocketMessage::Status(args.unit.clone()))?;
            let response = listen_for_response()?;
            if !response.contains("Unregistered") {
                let home = dirs::home_dir().expect("could not find home dir");
                let dir = home.join(".config").join("wpm").join("logs");

                if !dir.is_dir() {
                    std::fs::create_dir_all(&dir).expect("could not create ~/.config/wpm/logs");
                }

                let log = dir.join(format!("{}.log", args.unit));
                let log_contents = std::fs::read_to_string(log)?;
                let lines = log_contents
                    .lines()
                    .filter(|line| !line.is_empty())
                    .collect::<Vec<_>>();
                let last_ten_lines = lines.iter().rev().take(10).rev().collect::<Vec<_>>();

                println!("{}", response);

                if !last_ten_lines.is_empty() {
                    println!("\nLogs:");
                    for line in last_ten_lines {
                        println!("{line}");
                    }
                }
            } else {
                println!("{}", response);
            }
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
                let home = dirs::home_dir().expect("could not find home dir");
                let dir = home.join(".config").join("wpm").join("logs");

                if !dir.is_dir() {
                    std::fs::create_dir_all(&dir).expect("could not create ~/.config/wpm/logs");
                }

                let file = File::open(dir.join(format!("{}.log", unit))).unwrap();

                let file = TailedFile::new(file);
                let locked = file.lock();
                #[allow(clippy::significant_drop_in_scrutinee, clippy::lines_filter_map_ok)]
                for line in locked.lines().flatten() {
                    println!("{line}");
                }
            }
        },
    }

    Ok(())
}
