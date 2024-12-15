#![warn(clippy::all)]

use clap::Parser;
use fs_tail::TailedFile;
use interprocess::local_socket::traits::Stream as StreamExt;
use interprocess::local_socket::GenericNamespaced;
use interprocess::local_socket::Stream;
use interprocess::local_socket::ToNsName;
use std::fs::File;
use std::io::BufRead;
use std::io::Write;
use wpm::SocketMessage;

#[derive(Parser)]
#[clap(author, about, version)]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Parser)]
struct Start {
    unit: String,
}

#[derive(Parser)]
struct Stop {
    unit: String,
}

#[derive(Parser)]
struct Status {
    unit: String,
}

#[derive(Parser)]
struct Log {
    unit: String,
}

#[derive(Parser)]
enum SubCommand {
    /// Start a unit
    Start(Start),
    /// Stop a unit
    Stop(Stop),
    /// Show status of a unit
    Status(Status),
    /// Reload all unit definitions
    Reload,
    /// Tail the logs of a unit
    Log(Log),
}

fn send_message(message: SocketMessage) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string(&message)?;
    let name = "wpmd.sock".to_ns_name::<GenericNamespaced>()?;
    let connection = Stream::connect(name)?;
    let (_, mut sender) = connection.split();
    sender.write_all(json.as_bytes())?;

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts: Opts = Opts::parse();
    match opts.subcmd {
        SubCommand::Start(args) => {
            send_message(SocketMessage::Start(args.unit))?;
        }
        SubCommand::Stop(args) => {
            send_message(SocketMessage::Stop(args.unit))?;
        }
        SubCommand::Status(_args) => {}
        SubCommand::Reload => {
            send_message(SocketMessage::Reload)?;
        }
        SubCommand::Log(args) => {
            let home = dirs::home_dir().expect("could not find home dir");
            let dir = home.join(".config").join("wpm").join("logs");

            if !dir.is_dir() {
                std::fs::create_dir_all(&dir).expect("could not create ~/.config/wpm/logs");
            }

            let file = File::open(dir.join(format!("{}.log", args.unit))).unwrap();

            let file = TailedFile::new(file);
            let locked = file.lock();
            #[allow(clippy::significant_drop_in_scrutinee, clippy::lines_filter_map_ok)]
            for line in locked.lines().flatten() {
                println!("{line}");
            }
        }
    }

    Ok(())
}
