#![warn(clippy::all)]

use serde::Deserialize;
use serde::Serialize;

pub mod communication;
pub mod process_manager;
pub mod unit;

#[derive(Debug, Serialize, Deserialize)]
pub enum SocketMessage {
    Start(Vec<String>),
    Stop(Vec<String>),
    Status(String),
    State,
    Reload,
    Reset(String),
}
