use std::{error::Error, fmt::Display, io, path::PathBuf, sync::mpsc::channel};

use crate::{
    cli::Args,
    client::ClientError,
    daemon::{
        ipc::IpcSocket,
        state::{PositionUpdate, StateUpdate, StateUpdateError},
        sway::SwaySubscription,
    },
    find_target_node, move_window,
    sway::SwayConnection,
    State,
};
use serde::{Deserialize, Serialize};
use swayipc::Connection;

pub mod ipc;
pub mod state;
pub mod sway;
pub mod unit;

pub fn run_daemon(
    socket_path: PathBuf,
    initial_state: State,
    sway_delay: u64,
) -> Result<(), DaemonError> {
    let mut state = initial_state;
    let mut con = SwayConnection::new()?;

    let (tx, rx) = channel::<DaemonEvent>();
    let sway_tx = tx.clone();
    let ctrlc_tx = tx.clone();

    let socket = IpcSocket::init_or_replace(&socket_path, tx)?;
    let sway_sub = SwaySubscription::init(Connection::new, sway_tx, sway_delay)?;

    ctrlc::set_handler(move || {
        ctrlc_tx
            .send(DaemonEvent::Shutdown)
            .expect("Failed to send shutdown event");
    })
    .expect("Error setting Ctrl-C handler");

    for event in rx.iter() {
        match event {
            DaemonEvent::Shutdown => {
                eprintln!("Shutdown requested.");
                break;
            }
            DaemonEvent::Update(update) => {
                let window = find_target_node(&mut con)?;

                match move_window(&mut con, window, state.clone(), update) {
                    Ok(updated) => {
                        state = updated;
                        eprintln!("Window moved successfully: {:?}", state);
                    }
                    Err(e) => {
                        eprintln!("Failed to move window: {}", e);
                    }
                };
            }
        }
    }

    socket.shutdown();
    sway_sub.shutdown();

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DaemonEvent {
    Shutdown,
    Update(StateUpdate),
}

impl From<Args> for DaemonEvent {
    fn from(args: Args) -> Self {
        if args.shutdown {
            Self::Shutdown
        } else {
            Self::Update(StateUpdate::from(args))
        }
    }
}

impl From<Args> for StateUpdate {
    fn from(args: Args) -> Self {
        Self {
            position: PositionUpdate(args.vertical, args.horizontal),
            padding: args.padding,
            width: args.width,
            height: args.height,
            natural: args.natural,
        }
    }
}

#[derive(Debug)]
pub enum DaemonError {
    IoError(io::Error),
    InvalidMessage(serde_json::Error),
    InvalidInitialState(String),
    StateUpdateFailed(StateUpdateError),
}

impl Display for DaemonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DaemonError::IoError(err) => write!(f, "IO error: {}", err),
            DaemonError::InvalidMessage(err) => write!(f, "Message decoding error: {}", err),
            DaemonError::InvalidInitialState(err) => write!(f, "Invalid initial state: {}", err),
            DaemonError::StateUpdateFailed(err) => write!(f, "State update error: {}", err),
        }
    }
}

impl Error for DaemonError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            DaemonError::IoError(err) => Some(err),
            DaemonError::InvalidMessage(err) => Some(err),
            DaemonError::InvalidInitialState(_) => None,
            DaemonError::StateUpdateFailed(err) => Some(err),
        }
    }
}

impl From<ClientError> for DaemonError {
    fn from(value: ClientError) -> Self {
        match value {
            ClientError::IoError(err) => Self::IoError(err),
            ClientError::InvalidMessage(err) => Self::InvalidMessage(err),
        }
    }
}

impl From<io::Error> for DaemonError {
    fn from(value: io::Error) -> Self {
        Self::IoError(value)
    }
}

impl From<serde_json::Error> for DaemonError {
    fn from(value: serde_json::Error) -> Self {
        Self::InvalidMessage(value)
    }
}

impl From<StateUpdateError> for DaemonError {
    fn from(value: StateUpdateError) -> Self {
        Self::StateUpdateFailed(value)
    }
}

impl From<swayipc::Error> for DaemonError {
    fn from(value: swayipc::Error) -> Self {
        Self::StateUpdateFailed(StateUpdateError::SwayIPC(value))
    }
}

impl From<std::sync::mpsc::SendError<DaemonEvent>> for DaemonError {
    fn from(value: std::sync::mpsc::SendError<DaemonEvent>) -> Self {
        Self::IoError(io::Error::new(io::ErrorKind::Other, value))
    }
}
