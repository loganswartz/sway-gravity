use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};
use swayipc::{Connection, Floating, Fallible, Error as SwayIPCError};

fn find_working_area_for(con: &mut Connection, node_id: i64) -> Fallible<Option<swayipc::Rect>> {
    Ok(con.get_workspaces()?.iter().find(|w| w.focus.contains(&node_id)).map(|w| w.rect))
}

fn move_node_to_position(con: &mut Connection, node_id: i64, x: i32, y: i32) -> Fallible<()> {
    let cmd = format!(r#"[con_id="{}"] move position {} {}"#, node_id, x, y);
    con.run_command(cmd)?;

    Ok(())
}

fn resize_node(con: &mut Connection, node_id: i64, width: u32, height: u32) -> Fallible<()> {
    let cmd = format!(r#"[con_id="{}"] resize set {} px {} px"#, node_id, width, height);
    con.run_command(cmd)?;

    Ok(())
}

/// Move a floating window to the specified position
#[derive(Debug, Parser)]
#[command(version, about)]
struct Args {
    vertical: Vertical,
    horizontal: Horizontal,

    #[arg(short, long, default_value_t = 0)]
    padding: u32,

    #[arg(long)]
    width: Option<u32>,

    #[arg(long)]
    height: Option<u32>,
}

#[derive(Debug)]
enum Error {
    SwayIPC(swayipc::Error),
    NoApplicableNode,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::SwayIPC(err) => write!(f, "SwayIPC error: {}", err),
            Error::NoApplicableNode => write!(f, "No applicable node found"),
        }
    }
}

impl From<SwayIPCError> for Error {
    fn from(err: SwayIPCError) -> Self {
        Self::SwayIPC(err)
    }
}

fn main() {
    if let Err(e) = submain() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn submain() -> Result<(), Error> {
    let args = Args::parse();
    let mut con = Connection::new()?;

    let tree = con.get_tree()?;
    let floating_nodes: Vec<_> = tree.iter().filter(|node| node.floating.is_some_and(|state| state == Floating::AutoOn || state == Floating::UserOn)).collect();
    let target_node = match floating_nodes.len() {
        1 => {
            println!("Only one floating node found, using it.");
            floating_nodes[0]
        },
        _ => tree.find_focused_as_ref(|node| node.focused && node.floating.is_some_and(|state| state == Floating::AutoOn || state == Floating::UserOn)).ok_or(Error::NoApplicableNode)?,
    };

    let working_area = find_working_area_for(&mut con, target_node.id)?.map(Rect::from).ok_or(Error::NoApplicableNode)?;
    let proper_area = working_area.with_padding(args.padding as i32);

    let mut rect: Rect = target_node.rect.into();
    // TODO: do this properly
    rect.height += target_node.deco_rect.height;

    let (width, height) = match (args.width, args.height) {
        (Some(width), Some(height)) => (width as i32, height as i32),
        (Some(width), None) => {
            let rect = &rect.scale_to_match_width(width as i32);
            (rect.width, rect.height)
        },
        (None, Some(height)) => {
            let rect = &rect.scale_to_match_height(height as i32);
            (rect.width, rect.height)
        },
        _ => (target_node.rect.width, target_node.rect.height),
    };
    resize_node(&mut con, target_node.id, width as u32, height as u32)?;

    rect.height = height;
    rect.width = width;

    let pos = Position(args.vertical, args.horizontal);
    let rect = proper_area.get_pos_for_rect_of_size(pos, &rect);
    // we added a padding to our working area, but the center of the new area is not the same as the
    // center of the old area, so we need to adjust the position of the window
    let final_pos = rect.translate(args.padding as i32, args.padding as i32);

    move_node_to_position(&mut con, target_node.id, final_pos.x, final_pos.y)?;

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
    fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
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

    fn scale_to_match_height(&self, height: i32) -> Self {
        let mut rect = *self;

        let ratio = rect.width as f32 / rect.height as f32;
        rect.width = (height as f32 * ratio) as i32;
        rect.height = height;

        rect
    }

    fn scale_to_match_width(&self, width: i32) -> Self {
        let mut rect = *self;

        let ratio = rect.height as f32 / rect.width as f32;
        rect.width = width;
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

struct Position(Vertical, Horizontal);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rect_with_padding() {
        let rect = Rect::new(0, 0, 100, 100);
        let rect = rect.with_padding(10);

        assert_eq!(rect.x, 10);
        assert_eq!(rect.y, 10);
        assert_eq!(rect.width, 80);
        assert_eq!(rect.height, 80);
    }

    #[test]
    fn test_get_pos_for_rect_of_size() {
        let workspace = Rect::new(0, 0, 100, 100);
        let window = Rect::new(0, 0, 33, 33);

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
        let rect = Rect::new(0, 0, 200, 100);
        let rect = rect.scale_to_match_height(50);

        assert_eq!(rect.width, 100);
        assert_eq!(rect.height, 50);

        let rect = Rect::new(0, 0, 100, 50);
        let rect = rect.scale_to_match_height(200);

        assert_eq!(rect.width, 400);
        assert_eq!(rect.height, 200);
    }

    #[test]
    fn test_scale_to_match_width() {
        let rect = Rect::new(0, 0, 200, 100);
        let rect = rect.scale_to_match_width(400);

        assert_eq!(rect.width, 400);
        assert_eq!(rect.height, 200);

        let rect = Rect::new(0, 0, 100, 50);
        let rect = rect.scale_to_match_width(25);

        assert_eq!(rect.width, 25);
        assert_eq!(rect.height, 12);
    }
}
