#![warn(clippy::all)]

use serde::Deserialize;
use serde::Serialize;

pub mod communication;
mod generators;
pub mod process_manager;
mod process_manager_status;
pub mod unit;
mod unit_status;

#[derive(Debug, Serialize, Deserialize)]
pub enum SocketMessage {
    Start(Vec<String>),
    Stop(Vec<String>),
    Status(String),
    State,
    Reload,
    Reset(Vec<String>),
    Restart(Vec<String>),
}
