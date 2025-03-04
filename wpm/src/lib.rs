#![warn(clippy::all)]

use serde::Deserialize;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::OnceLock;

pub mod communication;
mod generators;
pub mod process_manager;
mod process_manager_status;
pub mod unit;
mod unit_status;

static DATA_DIR: OnceLock<PathBuf> = OnceLock::new();
static REQWEST_CLIENT: OnceLock<reqwest::blocking::Client> = OnceLock::new();

pub fn reqwest_client() -> reqwest::blocking::Client {
    REQWEST_CLIENT
        .get_or_init(|| {
            let builder = reqwest::blocking::Client::builder();
            builder.user_agent("wpm").build().unwrap()
        })
        .clone()
}

pub fn wpm_data_dir() -> PathBuf {
    DATA_DIR
        .get_or_init(|| {
            let wpm_dir = dirs::data_local_dir()
                .expect("could not find the system's local data dir")
                .join("wpm");

            std::fs::create_dir_all(&wpm_dir)
                .expect("could not ensure creation of the wpm local data dir");

            let log_dir = wpm_dir.join("logs");

            std::fs::create_dir_all(&log_dir)
                .expect("could not ensure creation of the wpm logs local data dir");

            let store_dir = wpm_dir.join("store");

            std::fs::create_dir_all(&store_dir)
                .expect("could not ensure creation of the wpm store local data dir");

            wpm_dir
        })
        .clone()
}

pub fn wpm_store_dir() -> PathBuf {
    wpm_data_dir().join("store")
}

pub fn wpm_log_dir() -> PathBuf {
    wpm_data_dir().join("logs")
}

pub fn wpm_units_dir() -> PathBuf {
    dirs::home_dir()
        .expect("could not find home dir")
        .join(".config")
        .join("wpm")
}

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
