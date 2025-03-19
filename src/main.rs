use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};
use swayipc::{Connection, Workspace};

fn get_focused_workspace(con: &mut Connection) -> Option<Workspace> {
    let workspaces = con.get_workspaces().unwrap();
    workspaces.into_iter().find(|w| w.focused)
}

fn find_working_area(con: &mut Connection) -> Option<swayipc::Rect> {
    Some(get_focused_workspace(con)?.rect)
}

fn move_window_to_position(con: &mut Connection, x: i32, y: i32) {
    let cmd = format!("move position {} {}", x, y);
    con.run_command(cmd).unwrap();
}

fn resize_window(con: &mut Connection, width: u32, height: u32) {
    let cmd = format!("resize set {} px {} px", width, height);
    con.run_command(cmd).unwrap();
}

/// Move the focused window to the specified position
#[derive(Debug, Parser)]
#[command(version, about)]
struct Args {
    vertical: Vertical,
    horizontal: Horizontal,

    #[arg(short, long, default_value_t = 0)]
    margin: u32,

    #[arg(long)]
    width: Option<u32>,

    #[arg(long)]
    height: Option<u32>,
}

fn main() {
    let args = Args::parse();
    let mut con = Connection::new().unwrap();

    let working_area: Rect = find_working_area(&mut con).unwrap().into();
    let proper_area = working_area.with_margin(args.margin as i32);

    let tree = con.get_tree().unwrap();
    let focused_node = tree.find_focused_as_ref(|node| node.focused).unwrap();
    let mut rect: Rect = focused_node.rect.into();

    let (width, height) = match (args.width, args.height) {
        (Some(width), Some(height)) => {
            resize_window(&mut con, width, height);
            (width as i32, height as i32)
        },
        (Some(width), None) => {
            let rect = &rect.scale_to_match_width(width as i32);
            resize_window(&mut con, rect.width as u32, rect.height as u32);
            (rect.width, rect.height)
        },
        (None, Some(height)) => {
            let rect = &rect.scale_to_match_height(height as i32);
            resize_window(&mut con, rect.width as u32, rect.height as u32);
            (rect.width, rect.height)
        },
        _ => (focused_node.rect.width, focused_node.rect.height),
    };
    rect.height = height;
    rect.width = width;

    let pos = Position(args.vertical, args.horizontal);
    let rect = proper_area.get_pos_for_rect_of_size(pos, &rect);
    // we added a margin to our working area, but the center of the new area is not the same as the
    // center of the old area, so we need to adjust the position of the window
    let final_pos = rect.translate(args.margin as i32, args.margin as i32);

    move_window_to_position(&mut con, final_pos.x, final_pos.y);
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

    fn with_margin(&self, margin: i32) -> Self {
        let mut rect = *self;

        rect.x += margin;
        rect.y += margin;
        rect.width -= margin * 2;
        rect.height -= margin * 2;

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

        let ratio = height as f32 / rect.height as f32;
        rect.width = (rect.width as f32 * ratio) as i32;
        rect.height = height;

        rect
    }

    fn scale_to_match_width(&self, width: i32) -> Self {
        let mut rect = *self;

        let ratio = width as f32 / rect.width as f32;
        rect.width = width;
        rect.height = (rect.height as f32 * ratio) as i32;

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
    fn test_rect_with_margin() {
        let rect = Rect::new(0, 0, 100, 100);
        let rect = rect.with_margin(10);

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
        let rect = Rect::new(0, 0, 100, 50);
        let rect = rect.scale_to_match_height(25);

        assert_eq!(rect.width, 50);
        assert_eq!(rect.height, 25);

        let rect = Rect::new(0, 0, 100, 50);
        let rect = rect.scale_to_match_height(200);

        assert_eq!(rect.width, 400);
        assert_eq!(rect.height, 200);
    }
}
