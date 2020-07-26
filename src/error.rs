use crate::protocol::ProtocolError;
use serde::export::Formatter;
use thiserror::Error;
use tokio::io;

pub type Result<T> = std::result::Result<T, TaskError>;

#[derive(Debug, Error)]
pub enum TaskError {
    ProtocolError(ProtocolError),
    IoError(io::Error),
    ControllerError,
}

impl From<std::io::Error> for TaskError {
    fn from(e: io::Error) -> Self {
        TaskError::IoError(e)
    }
}

impl std::fmt::Display for TaskError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "TaskError")
    }
}
