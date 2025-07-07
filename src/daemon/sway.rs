use std::{
    io,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::Sender,
        Arc,
    },
    thread,
    time::Duration,
};
use swayipc::{Connection, Error as SwayIPCError};

use crate::daemon::{state::StateUpdate, DaemonEvent};

pub struct SwaySubscription {
    con: Connection,
    running: Arc<AtomicBool>,
    _thread: thread::JoinHandle<()>,
}

impl SwaySubscription {
    pub fn init<T: std::convert::From<swayipc::Event> + Send + std::fmt::Debug + 'static>(
        con_factory: fn() -> Result<Connection, SwayIPCError>,
        tx: Sender<T>,
        delay: u64,
    ) -> Result<Self, io::Error> {
        let running = Arc::new(AtomicBool::new(true));
        let r = running.clone();
        let sub_con = con_factory().map_err(|e| {
            eprintln!("Failed to create sway connection: {}", e);
            io::Error::new(io::ErrorKind::Other, e)
        })?;
        let tick_con = con_factory().map_err(|e| {
            eprintln!("Failed to create sway connection: {}", e);
            io::Error::new(io::ErrorKind::Other, e)
        })?;

        let _thread = thread::spawn(move || {
            let subs = [
                swayipc::EventType::Window,
                swayipc::EventType::Shutdown,
                swayipc::EventType::Workspace,
                swayipc::EventType::Output,
                swayipc::EventType::Tick,
            ];

            let stream = sub_con
                .subscribe(subs)
                .expect("Failed to subscribe to events");
            for event in stream {
                // eprintln!("Received event: {:?}", event.as_ref());
                if !r.load(Ordering::SeqCst) {
                    eprintln!("Sway listener is shutting down...");
                    break;
                }

                match event {
                    Ok(event) => {
                        match &event {
                            swayipc::Event::Workspace(event) => match event.change {
                                swayipc::WorkspaceChange::Reload => {}
                                _ => continue,
                            },
                            _ => continue,
                        }

                        // HACK: Let sway settle for a moment.
                        // Without this, the bar or other things may end up moving things around and throwing off
                        // the math. I would expect that to trigger a window or workspace event, but it doesn't
                        // appear to do so in my testing environment.
                        thread::sleep(Duration::from_millis(delay));

                        let _ = tx.send(event.into());
                    }
                    Err(_) => {
                        break;
                    }
                }
            }

            eprintln!("Sway subscription was closed.");
        });

        Ok(Self {
            con: tick_con,
            running,
            _thread,
        })
    }

    pub fn shutdown(self) {}
}

impl Drop for SwaySubscription {
    fn drop(&mut self) {
        self.running.store(false, Ordering::SeqCst);

        // ensure the thread has an event to process, which triggers the running check
        let _ = self.con.send_tick("");
    }
}

impl From<swayipc::Event> for DaemonEvent {
    fn from(_: swayipc::Event) -> Self {
        DaemonEvent::Update(StateUpdate::default())
    }
}
