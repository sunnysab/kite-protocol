use crate::host::HostError;
use crate::protocol::ProtocolError;
use serde::export::Formatter;
use thiserror::Error;
use tokio::io;
use tokio::task::JoinError;

pub type Result<T> = std::result::Result<T, TaskError>;

#[derive(Debug, Error)]
pub enum TaskError {
    Protocol(ProtocolError),
    Io(io::Error),
    Controller(HostError),
    Runtime(JoinError),
}

impl From<std::io::Error> for TaskError {
    fn from(e: io::Error) -> Self {
        TaskError::Io(e)
    }
}

impl From<JoinError> for TaskError {
    fn from(e: JoinError) -> Self {
        TaskError::Runtime(e)
    }
}

impl std::fmt::Display for TaskError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "TaskError")
    }
}
