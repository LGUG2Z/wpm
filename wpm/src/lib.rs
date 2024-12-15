#![warn(clippy::all)]

use serde::Deserialize;
use serde::Serialize;

pub mod communication;
pub mod process_manager;
pub mod unit;

#[derive(Debug, Serialize, Deserialize)]
pub enum SocketMessage {
    Start(String),
    Stop(String),
    Status(String),
    State,
    Reload,
}
