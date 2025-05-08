use std::{env, fmt::Display, io::{self, Read, Write}, num::ParseIntError, os::unix::net::{UnixListener, UnixStream}, path::PathBuf, sync::LazyLock};

use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};
use sway::SwayConnection;
use swayipc::{Error as SwayIPCError, NodeType};

mod sway;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
enum Vertical {
    Top,
    Middle,
    Bottom,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
enum Horizontal {
    Left,
    Middle,
    Right,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Position(Vertical, Horizontal);

#[derive(Debug, Clone, Serialize, Deserialize)]
enum Unit {
    Pixels(u32),
    Percentage(f32),
}

#[derive(Debug, Clone)]
enum ParseUnitError {
    ParseIntError(ParseIntError),
    ParseFloatError(std::num::ParseFloatError),
}

impl From<ParseUnitError> for Box<dyn std::error::Error + Send + Sync> {
    fn from(err: ParseUnitError) -> Self {
        match err {
            ParseUnitError::ParseIntError(err) => Box::new(err),
            ParseUnitError::ParseFloatError(err) => Box::new(err),
        }
    }
}

impl From<ParseIntError> for ParseUnitError {
    fn from(err: ParseIntError) -> Self {
        Self::ParseIntError(err)
    }
}

impl From<std::num::ParseFloatError> for ParseUnitError {
    fn from(err: std::num::ParseFloatError) -> Self {
        Self::ParseFloatError(err)
    }
}

impl std::str::FromStr for Unit {
    type Err = ParseUnitError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(value) = s.strip_suffix("%") {
            Ok(Self::Percentage(value.parse()?))
        } else {
            // Default to pixels if no suffix is provided
            let _ = s.strip_suffix("px");
            Ok(Self::Pixels(s.parse()?))
        }
    }
}

impl std::fmt::Display for Unit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pixels(value) => write!(f, "{} px", value),
            Self::Percentage(value) => write!(f, "{} ppt", value),
        }
    }
}

static DEFAULT_SOCKET: LazyLock<String> = LazyLock::new(|| format!("{}/sway-gravity/{}.sock", env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "./".to_string()), env::var("WAYLAND_DISPLAY").unwrap_or_else(|_| "sway".to_string())));

/// Move a floating window to the specified position
#[derive(Debug, Parser)]
#[command(version, about)]
struct Args {
    vertical: Vertical,
    horizontal: Horizontal,

    #[arg(short, long)]
    padding: Option<u32>,

    #[arg(long)]
    width: Option<Unit>,

    #[arg(long)]
    height: Option<Unit>,

    #[arg(short, long, default_value = DEFAULT_SOCKET.as_str())]
    socket: PathBuf,

    /// Run as a daemon, and wait for events via IPC
    #[arg(short, long)]
    daemon: bool,
}

#[derive(Debug, Clone)]
struct State {
    position: Position,
    padding: u32,
    width: Option<Unit>,
    height: Option<Unit>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            position: Position(Vertical::Bottom, Horizontal::Right),
            padding: 0,
            width: None,
            height: None,
        }
    }
}

impl State {
    fn update(&mut self, update: StateUpdate) {
        if let Some(position) = update.position {
            self.position = position;
        }
        if let Some(padding) = update.padding {
            self.padding = padding;
        }
        if let Some(width) = update.width {
            self.width = width;
        }
        if let Some(height) = update.height {
            self.height = height;
        }
    }

    fn with(self, update: StateUpdate) -> Self {
        let mut new_state = self.clone();
        new_state.update(update);
        new_state
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct StateUpdate {
    position: Option<Position>,
    padding: Option<u32>,
    width: Option<Option<Unit>>,
    height: Option<Option<Unit>>,
}

impl From<Args> for StateUpdate {
    fn from(args: Args) -> Self {
        Self {
            position: Some(Position(args.vertical, args.horizontal)),
            padding: args.padding,
            width: Some(args.width),
            height: Some(args.height),
        }
    }
}

impl From<State> for StateUpdate {
    fn from(state: State) -> Self {
        Self {
            position: Some(state.position),
            padding: Some(state.padding),
            width: Some(state.width),
            height: Some(state.height),
        }
    }
}

#[derive(Debug)]
enum StateUpdateError {
    SwayIPC(swayipc::Error),
    NoApplicableNode,
    MultipleApplicableNodes,
}

impl std::fmt::Display for StateUpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StateUpdateError::SwayIPC(err) => write!(f, "SwayIPC error: {}", err),
            StateUpdateError::NoApplicableNode => write!(f, "No applicable node found"),
            StateUpdateError::MultipleApplicableNodes => write!(f, "Multiple applicable nodes found"),
        }
    }
}

impl From<SwayIPCError> for StateUpdateError {
    fn from(err: SwayIPCError) -> Self {
        Self::SwayIPC(err)
    }
}

fn main() {
    let args = Args::parse();

    if let Err(e) = submain(args) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

#[derive(Debug)]
enum DaemonError {
    IoError(io::Error),
    InvalidMessage(serde_json::Error),
    StateUpdateFailed(StateUpdateError),
}

impl Display for DaemonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DaemonError::IoError(err) => write!(f, "IO error: {}", err),
            DaemonError::InvalidMessage(err) => write!(f, "Message decoding error: {}", err),
            DaemonError::StateUpdateFailed(err) => write!(f, "State update error: {}", err),
        }
    }
}

#[derive(Debug)]
enum ClientError {
    IoError(io::Error),
    InvalidMessage(serde_json::Error),
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

impl Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientError::IoError(err) => write!(f, "IO error: {}", err),
            ClientError::InvalidMessage(err) => write!(f, "Message encoding error: {}", err),
        }
    }
}

enum ApplicationError {
    Daemon(DaemonError),
    Client(ClientError),
}

impl Display for ApplicationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApplicationError::Daemon(err) => write!(f, "Daemon error: {}", err),
            ApplicationError::Client(err) => write!(f, "Client error: {}", err),
        }
    }
}

impl From<DaemonError> for ApplicationError {
    fn from(value: DaemonError) -> Self {
        Self::Daemon(value)
    }
}

impl From<ClientError> for ApplicationError {
    fn from(value: ClientError) -> Self {
        Self::Client(value)
    }
}

impl From<StateUpdateError> for ApplicationError {
    fn from(value: StateUpdateError) -> Self {
        Self::Daemon(DaemonError::StateUpdateFailed(value))
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

fn run_daemon(socket_path: &PathBuf, initial_state: State) -> Result<(), DaemonError> {
    let Some(_) = socket_path.parent().map(std::fs::create_dir_all) else {
        eprintln!("No parent directory found for socket");
        return Err(DaemonError::IoError(io::Error::new(
            io::ErrorKind::NotFound,
            "No parent directory found for socket",
        )))
    };
    if let Err(e) = std::fs::remove_file(socket_path) {
        eprintln!("Error removing socket: {}", e);
    }

    let socket = UnixListener::bind(socket_path)?;
    let mut state = initial_state;
    let mut con = SwayConnection::new()?;

    eprintln!("Listening on {}", socket_path.display());
    for stream in socket.incoming() {
        match stream {
            Ok(mut stream) => {
                if let Err(err) = handle_socket(&mut con, &mut stream, &mut state) {
                    eprintln!("Error handling socket: {}", err);
                }
            }
            Err(e) => eprintln!("Error: {}", e),
        }
    }

    Ok(())
}

fn handle_socket(con: &mut SwayConnection, stream: &mut UnixStream, state: &mut State) -> Result<StateUpdate, DaemonError> {
    let mut buffer = Vec::new();
    stream.read_to_end(&mut buffer)?;

    let update: StateUpdate = serde_json::from_slice(&buffer)?;
    eprintln!("Received message: {:?}", update);

    state.update(update.clone());
    move_window(con, state.clone())?;

    Ok(update)
}

fn send_message(socket: &PathBuf, state: StateUpdate) -> Result<(), ClientError> {
    eprintln!("Sending message to {}", socket.display());
    let mut socket = UnixStream::connect(socket)?;

    let message = serde_json::to_string(&state).expect("message should be serializable");
    Ok(socket.write_all(message.as_bytes())?)
}

fn submain(args: Args) -> Result<(), ApplicationError> {
    let Ok(_) = env::var("WAYLAND_DISPLAY") else {
        eprintln!("No WAYLAND_DISPLAY environment variable found");
        return Ok(());
    };
    let socket = args.socket.clone();

    if args.daemon {
        Ok(run_daemon(&socket, State::default().with(args.into()))?)
    } else {
        Ok(send_message(&socket, args.into())?)
    }
}

fn move_window(con: &mut SwayConnection, state: State) -> Result<(), StateUpdateError> {
    let tree = con.get_tree()?;

    let floating_nodes: Vec<_> = tree.iter().filter(|node| node.node_type == NodeType::FloatingCon).collect();
    let target_node = match floating_nodes.len() {
        1 => {
            println!("Only one floating node found, using it.");
            floating_nodes[0]
        },
        0 => return Err(StateUpdateError::NoApplicableNode),
        _ => tree.find_focused_as_ref(|node| node.focused && node.node_type == NodeType::FloatingCon).ok_or(StateUpdateError::MultipleApplicableNodes)?,
    };

    let working_area = con.find_working_area_for(target_node.id)?.map(Rect::from).ok_or(StateUpdateError::NoApplicableNode)?;
    let proper_area = working_area.with_padding(state.padding as i32);

    let mut rect: Rect = target_node.rect.into();
    // TODO: do this properly
    rect.height += target_node.deco_rect.height;

    let (width, height) = match (state.width, state.height) {
        (Some(width), Some(height)) => {
            let rect = &rect.scale(width, height, &proper_area);
            (rect.width, rect.height)
        },
        (Some(width), None) => {
            let rect = &rect.scale_to_match_width(width, &proper_area);
            (rect.width, rect.height)
        },
        (None, Some(height)) => {
            let rect = &rect.scale_to_match_height(height, &proper_area);
            (rect.width, rect.height)
        },
        _ => (target_node.rect.width, target_node.rect.height),
    };
    con.resize_node(target_node.id, Unit::Pixels(width as u32), Unit::Pixels(height as u32))?;

    rect.height = height;
    rect.width = width;

    let rect = proper_area.get_pos_for_rect_of_size(state.position, &rect);
    // we added a padding to our working area, but the center of the new area is not the same as the
    // center of the old area, so we need to adjust the position of the window
    let final_pos = rect.translate(state.padding as i32, state.padding as i32);

    con.move_node_to_position(target_node.id, final_pos.x, final_pos.y)?;

    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct Rect {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}


impl Rect {
    fn _new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self { x, y, width, height }
    }

    fn with_padding(&self, padding: i32) -> Self {
        let mut rect = *self;

        rect.x += padding;
        rect.y += padding;
        rect.width -= padding * 2;
        rect.height -= padding * 2;

        rect
    }

    fn translate(&self, x: i32, y: i32) -> Self {
        let mut rect = *self;
        rect.x += x;
        rect.y += y;
        rect
    }

    fn scale(&self, width: Unit, height: Unit, container: &Rect) -> Self {
        let mut rect = *self;

        let width = match width {
            Unit::Percentage(width) => (container.width as f32 * (width / 100.0)).round() as u32,
            Unit::Pixels(width) => width,
        };

        let height = match height {
            Unit::Percentage(height) => (container.height as f32 * (height / 100.0)).round() as u32,
            Unit::Pixels(height) => height,
        };

        rect.width = width as i32;
        rect.height = height as i32;

        rect
    }

    fn scale_to_match_height(&self, height: Unit, container: &Rect) -> Self {
        let mut rect = *self;

        let height = match height {
            Unit::Percentage(height) => (container.height as f32 * (height / 100.0)).round() as u32,
            Unit::Pixels(height) => height,
        };

        let ratio = rect.width as f32 / rect.height as f32;
        rect.width = (height as f32 * ratio) as i32;
        rect.height = height as i32;

        rect
    }

    fn scale_to_match_width(&self, width: Unit, container: &Rect) -> Self {
        let mut rect = *self;

        let width = match width {
            Unit::Percentage(width) => (container.width as f32 * (width / 100.0)).round() as u32,
            Unit::Pixels(width) => width,
        };

        let ratio = rect.height as f32 / rect.width as f32;
        rect.width = width as i32;
        rect.height = (width as f32 * ratio) as i32;

        rect
    }

    fn get_pos_for_rect_of_size(&self, pos: Position, rect: &Rect) -> Rect {
        let v_offset = match pos.0 {
            Vertical::Top => 0.0,
            Vertical::Middle => 0.5,
            Vertical::Bottom => 1.0,
        };

        let h_offset = match pos.1 {
            Horizontal::Left => 0.0,
            Horizontal::Middle => 0.5,
            Horizontal::Right => 1.0,
        };

        let x = (self.width as f32 * h_offset) - (rect.width as f32 * h_offset);
        let y = (self.height as f32 * v_offset) - (rect.height as f32 * v_offset);
        let (x, y) = (x as i32, y as i32);

        Rect {
            x,
            y,
            width: rect.width,
            height: rect.height,
        }
    }
}

impl From<swayipc::Rect> for Rect {
    fn from(rect: swayipc::Rect) -> Self {
        Self {
            x: rect.x,
            y: rect.y,
            width: rect.width,
            height: rect.height,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rect_with_padding() {
        let rect = Rect::_new(0, 0, 100, 100);
        let rect = rect.with_padding(10);

        assert_eq!(rect.x, 10);
        assert_eq!(rect.y, 10);
        assert_eq!(rect.width, 80);
        assert_eq!(rect.height, 80);
    }

    #[test]
    fn test_get_pos_for_rect_of_size() {
        let workspace = Rect::_new(0, 0, 100, 100);
        let window = Rect::_new(0, 0, 33, 33);

        let pos = Position(Vertical::Top, Horizontal::Left);
        let rect = workspace.get_pos_for_rect_of_size(pos, &window);

        assert_eq!(rect.x, 0);
        assert_eq!(rect.y, 0);
        assert_eq!(rect.width, 33);
        assert_eq!(rect.height, 33);

        let pos = Position(Vertical::Middle, Horizontal::Middle);
        let rect = workspace.get_pos_for_rect_of_size(pos, &window);

        assert_eq!(rect.x, 33);
        assert_eq!(rect.y, 33);
        assert_eq!(rect.width, 33);
        assert_eq!(rect.height, 33);

        let pos = Position(Vertical::Bottom, Horizontal::Right);
        let rect = workspace.get_pos_for_rect_of_size(pos, &window);

        assert_eq!(rect.x, 67);
        assert_eq!(rect.y, 67);
        assert_eq!(rect.width, 33);
        assert_eq!(rect.height, 33);
    }

    #[test]
    fn test_scale_to_match_height() {
        let container = Rect::_new(0, 0, 200, 100);
        let rect = Rect::_new(0, 0, 200, 100);
        let rect = rect.scale_to_match_height(Unit::Pixels(50), &container);

        assert_eq!(rect.width, 100);
        assert_eq!(rect.height, 50);

        let container = Rect::_new(0, 0, 100, 50);
        let rect = Rect::_new(0, 0, 100, 50);
        let rect = rect.scale_to_match_height(Unit::Pixels(200), &container);

        assert_eq!(rect.width, 400);
        assert_eq!(rect.height, 200);
    }

    #[test]
    fn test_scale_to_match_width() {
        let container = Rect::_new(0, 0, 200, 100);
        let rect = Rect::_new(0, 0, 200, 100);
        let rect = rect.scale_to_match_width(Unit::Pixels(400), &container);

        assert_eq!(rect.width, 400);
        assert_eq!(rect.height, 200);

        let container = Rect::_new(0, 0, 100, 50);
        let rect = Rect::_new(0, 0, 100, 50);
        let rect = rect.scale_to_match_width(Unit::Pixels(25), &container);

        assert_eq!(rect.width, 25);
        assert_eq!(rect.height, 12);
    }

    #[test]
    fn test_scale_to_match_width_percentage() {
        let container = Rect::_new(0, 0, 200, 100);
        let rect = Rect::_new(0, 0, 200, 100);
        let rect = rect.scale_to_match_width(Unit::Percentage(10.0), &container);

        assert_eq!(rect.width, 20);
        assert_eq!(rect.height, 10);

        let container = Rect::_new(0, 0, 100, 50);
        let rect = Rect::_new(0, 0, 100, 50);
        let rect = rect.scale_to_match_width(Unit::Percentage(10.0), &container);

        assert_eq!(rect.width, 10);
        assert_eq!(rect.height, 5);
    }
}
