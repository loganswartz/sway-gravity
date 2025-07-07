use serde::de::DeserializeOwned;
use std::{
    io,
    os::{fd::AsRawFd, unix::net::UnixListener},
    path::PathBuf,
    sync::mpsc::Sender,
    thread,
    time::Duration,
};

use crate::{
    client::send_message,
    daemon::{DaemonError, DaemonEvent},
};

pub struct IpcSocket {
    fd: i32,
    path: PathBuf,
    _thread: thread::JoinHandle<()>,
}

impl IpcSocket {
    pub fn init<T: DeserializeOwned + Send + std::fmt::Debug + 'static>(
        path: PathBuf,
        tx: Sender<T>,
    ) -> Result<Self, io::Error> {
        let socket = UnixListener::bind(&path)?;
        let fd = socket.as_raw_fd();

        let _thread = thread::spawn(move || {
            for event in socket.incoming() {
                match event {
                    Ok(stream) => {
                        let msg = serde_json::from_reader(stream)
                            .expect("message should be serializable");
                        eprintln!("Received message: {:?}", msg);

                        tx.send(msg).expect("failed to send message");
                    }
                    Err(_) => {
                        break;
                    }
                }
            }

            eprintln!("Socket listener was closed.");
        });

        Ok(Self { fd, path, _thread })
    }

    pub fn init_or_replace(
        socket_path: &PathBuf,
        tx: Sender<DaemonEvent>,
    ) -> Result<Self, DaemonError> {
        match std::fs::exists(socket_path) {
            Ok(true) => {
                eprintln!("Socket already exists, shutting down existing daemon...");
                send_message(socket_path, DaemonEvent::Shutdown)?;

                while let Ok(true) = std::fs::exists(socket_path) {
                    thread::sleep(Duration::from_millis(100));
                }
            }
            _ => eprintln!("Socket does not exist, creating it..."),
        }

        let Some(_) = socket_path.parent().map(std::fs::create_dir_all) else {
            eprintln!("No parent directory found for socket");
            return Err(DaemonError::IoError(io::Error::new(
                io::ErrorKind::NotFound,
                "No parent directory found for socket",
            )));
        };

        let socket = Self::init(socket_path.clone(), tx)?;
        eprintln!("Listening on {}", socket_path.display());

        Ok(socket)
    }

    pub fn shutdown(self) {}
}

impl Drop for IpcSocket {
    fn drop(&mut self) {
        unsafe {
            libc::shutdown(self.fd, libc::SHUT_RDWR);
        }

        let _ = std::fs::remove_file(&self.path);
    }
}
