use thiserror::Error;

use super::read;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("Dongle not ok: {}", .0)]
    Dongle(String),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error("Failed to read from serial: {}", .0)]
    Comm(#[from] read::Error),
}
