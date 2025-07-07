use std::{env, path::PathBuf, sync::LazyLock};

use clap::Parser;

use crate::daemon::state::{Horizontal, Unit, Vertical};

/// Automatically position and resize a floating window in Sway.
///
/// When run as a daemon, this will listen for events from Sway and a standalone socket, and
/// automatically position and resize windows in reaction to those events. While running, the
/// daemon will preserve the most recent calculated state, and automatically update the window
/// whenever necessary.
///
/// When run as a client, this will send a message to the daemon to position and resize the window
/// Events do not need included every possible property allowed, only the ones that need to be
/// changed.
///
/// `width` and `height` can both be provided as a percentage of the available area, or as a pixel
/// value. When both `width` and `height` is provided, the window will be resized to exactly that
/// size. When only one dimension is provided, the other will be automatically calculated to
/// maintain the aspect ratio of the window.
#[derive(Debug, Parser)]
#[command(version, about)]
pub struct Args {
    /// The vertical third of the screen to place the window in
    pub vertical: Option<Vertical>,
    /// The horizontal third of the screen to place the window in
    pub horizontal: Option<Horizontal>,

    /// The amount of padding to add around moved window
    #[arg(short, long)]
    pub padding: Option<u32>,

    /// Resize the window to this width
    #[arg(long, value_enum, allow_hyphen_values = true)]
    pub width: Option<Unit>,

    /// Resize the window to this height
    #[arg(long, value_enum, allow_hyphen_values = true)]
    pub height: Option<Unit>,

    /// Attempt to resize the window to its natural aspect ratio
    #[arg(long)]
    pub natural: Option<bool>,

    /// Run as a daemon, and wait for events via IPC
    #[arg(short, long)]
    pub daemon: bool,

    /// The path to use for the socket to listen on
    #[arg(short, long, default_value = DEFAULT_SOCKET.as_str())]
    pub socket: PathBuf,

    /// Delay (in milliseconds) to wait before processing events from the sway IPC
    ///
    /// This is mainly for allowing sway to settle after a reload or other event.
    #[arg(long, default_value_t = 200)]
    pub sway_event_delay: u64,

    /// Instruct the running daemon to shutdown
    #[arg(long)]
    pub shutdown: bool,
}

static DEFAULT_SOCKET: LazyLock<String> = LazyLock::new(|| {
    format!(
        "{}/sway-gravity/{}.sock",
        env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "./".to_string()),
        env::var("WAYLAND_DISPLAY").unwrap_or_else(|_| "sway".to_string())
    )
});
