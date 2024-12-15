#![warn(clippy::all)]

use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize)]
pub enum SocketMessage {
    Start(String),
    Stop(String),
    Status(String),
    Reload,
}
