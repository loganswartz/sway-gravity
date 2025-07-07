use std::{
    error::Error,
    fmt::Display,
    io::{self, Write},
    os::unix::net::UnixStream,
    path::PathBuf,
};

use crate::daemon::DaemonEvent;

#[derive(Debug)]
pub enum ClientError {
    IoError(io::Error),
    InvalidMessage(serde_json::Error),
}

impl Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientError::IoError(err) => write!(f, "IO error: {}", err),
            ClientError::InvalidMessage(err) => write!(f, "Message encoding error: {}", err),
        }
    }
}

impl Error for ClientError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ClientError::IoError(err) => Some(err),
            ClientError::InvalidMessage(err) => Some(err),
        }
    }
}

impl From<io::Error> for ClientError {
    fn from(value: io::Error) -> Self {
        Self::IoError(value)
    }
}

impl From<serde_json::Error> for ClientError {
    fn from(value: serde_json::Error) -> Self {
        Self::InvalidMessage(value)
    }
}

pub fn send_message(socket: &PathBuf, event: DaemonEvent) -> Result<(), ClientError> {
    eprintln!("Sending message to {}", socket.display());
    let mut socket = UnixStream::connect(socket)?;

    let message = serde_json::to_string(&event).expect("message should be serializable");
    Ok(socket.write_all(message.as_bytes())?)
}
