#![warn(clippy::all)]

use serde::Deserialize;
use serde::Serialize;

pub mod process_manager;
pub mod unit;

#[derive(Serialize, Deserialize)]
pub enum SocketMessage {
    Start(String),
    Stop(String),
    Status(String),
    Reload,
}
