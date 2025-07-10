use std::{env, error::Error, fmt::Display};

use clap::Parser;
use sway::SwayConnection;
use swayipc::{Node, NodeType};

use crate::{
    cli::Args,
    client::{send_message, ClientError},
    daemon::{
        run_daemon,
        state::{
            Horizontal, InitialStateOptions, Position, State, StateUpdate, StateUpdateError,
            Vertical,
        },
        unit::{AbsolutePixels, AbsoluteUnit, RelativeUnit, Unit},
        DaemonError,
    },
    sway::{Dimension, Window},
};

mod cli;
mod client;
mod daemon;
mod sway;

#[derive(Debug)]
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

impl Error for ApplicationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ApplicationError::Daemon(err) => Some(err),
            ApplicationError::Client(err) => Some(err),
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

fn submain(args: Args) -> Result<(), ApplicationError> {
    let Ok(_) = env::var("WAYLAND_DISPLAY") else {
        eprintln!("No WAYLAND_DISPLAY environment variable found");
        return Ok(());
    };
    let socket = args.socket.clone();
    let sway_delay = args.sway_event_delay;

    if args.daemon {
        let initial: InitialStateOptions = args.try_into()?;

        Ok(run_daemon(
            socket,
            State::with_initial(initial),
            sway_delay,
        )?)
    } else {
        Ok(send_message(&socket, args.into())?)
    }
}

fn main() {
    let args = Args::parse();

    if let Err(e) = submain(args) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn find_target_node(con: &mut SwayConnection) -> Result<swayipc::Node, StateUpdateError> {
    let tree = con.get_tree()?;

    let floating_nodes: Vec<_> = tree
        .iter()
        .filter(|node| node.node_type == NodeType::FloatingCon)
        .collect();

    let target_node = match floating_nodes.len() {
        1 => {
            println!("Only one floating node found, using it.");
            let floating_node_id = floating_nodes[0].id;
            tree.find(|node| node.id == floating_node_id)
                .expect("Node should exist")
        }
        0 => return Err(StateUpdateError::NoApplicableNode),
        _ => tree
            .find_focused(|node| node.focused && node.node_type == NodeType::FloatingCon)
            .ok_or(StateUpdateError::MultipleApplicableNodes)?,
    };

    Ok(target_node)
}

fn move_window(
    con: &mut SwayConnection,
    target_node: Node,
    mut state: State,
    update: StateUpdate,
) -> Result<State, StateUpdateError> {
    let context = Window::from_node(target_node.clone(), con).map_err(StateUpdateError::SwayIPC)?;
    state.update(update, &context);

    let working_area: Rect = context.working_area.into();
    let proper_area = working_area.with_padding(state.padding as i32);

    let original_rect: Rect = target_node.rect.into();
    let mut rect: Rect = target_node.rect.into();
    // TODO: do this properly
    rect.height += target_node.deco_rect.height;

    let ratio = if state.natural {
        Some(aspect_ratio(
            target_node.geometry.width,
            target_node.geometry.height,
        ))
    } else {
        None
    };
    let scaled = &rect.scale(
        state.width.clone().map(|w| w.into()),
        state.height.clone().map(|h| h.into()),
        &original_rect,
        &proper_area,
        ratio,
    );

    con.resize_node(
        target_node.id,
        AbsolutePixels::from(scaled.width as u32),
        AbsolutePixels::from(scaled.height as u32),
    )?;

    rect.height = scaled.height;
    rect.width = scaled.width;

    let rect = proper_area.get_pos_for_rect_of_size(&state.position, &rect);
    // we added a padding to our working area, but the center of the new area is not the same as the
    // center of the old area, so we need to adjust the position of the window
    let final_pos = rect.translate(state.padding as i32, state.padding as i32);

    con.move_node_to_position(target_node.id, final_pos.x, final_pos.y)?;

    Ok(state)
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
        Self {
            x,
            y,
            width,
            height,
        }
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

    fn scale(
        &self,
        width: Option<Unit>,
        height: Option<Unit>,
        target: &Rect,
        container: &Rect,
        ratio: Option<f32>,
    ) -> Self {
        let mut rect = *self;
        let aspect = ratio.unwrap_or(aspect_ratio(target.width, target.height));

        let (width, height) = match (width, height) {
            (Some(w), Some(h)) => (
                Dimension::Width(unit_to_real_pixels(w, target.width, container.width)),
                Dimension::Height(unit_to_real_pixels(h, target.height, container.height)),
            ),
            (Some(w), None) => {
                let width = unit_to_real_pixels(w, target.width, container.width);
                (
                    Dimension::Width(width),
                    scale_to_ratio(Dimension::Width(width), aspect),
                )
            }
            (None, Some(h)) => {
                let height = unit_to_real_pixels(h, target.height, container.height);
                (
                    scale_to_ratio(Dimension::Height(height), aspect),
                    Dimension::Height(height),
                )
            }
            (None, None) => (
                Dimension::Width(target.width),
                Dimension::Height(target.height),
            ),
        };

        let width = match width {
            Dimension::Width(w) => w,
            _ => unreachable!(),
        };

        let height = match height {
            Dimension::Height(h) => h,
            _ => unreachable!(),
        };

        rect.width = width;
        rect.height = height;

        rect
    }

    fn get_pos_for_rect_of_size(&self, pos: &Position, rect: &Rect) -> Rect {
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

fn aspect_ratio(width: i32, height: i32) -> f32 {
    if height == 0 {
        return 0.0;
    }
    width as f32 / height as f32
}

fn scale_to_ratio(dimension: Dimension, ratio: f32) -> Dimension {
    if ratio == 0.0 {
        return match dimension {
            Dimension::Width(_) => Dimension::Height(0),
            Dimension::Height(_) => Dimension::Width(0),
        };
    }

    match dimension {
        Dimension::Width(width) => Dimension::Height((width as f32 / ratio).round() as i32),
        Dimension::Height(height) => Dimension::Width((height as f32 * ratio).round() as i32),
    }
}

fn unit_to_real_pixels(unit: Unit, target_px: i32, container_px: i32) -> i32 {
    let real = match unit {
        Unit::Absolute(AbsoluteUnit::Pixels(pixels)) => pixels.0 as f32,
        Unit::Absolute(AbsoluteUnit::Percentage(percentage)) => {
            container_px as f32 * (percentage.0 / 100.0)
        }
        Unit::Relative(RelativeUnit::Pixels(pixels)) => target_px.saturating_add(pixels.0) as f32,
        Unit::Relative(RelativeUnit::Percentage(percentage)) => {
            let current = target_px as f32 / container_px as f32;
            let adjusted = current + (percentage.0 / 100.0);
            container_px as f32 * adjusted
        }
    };

    real.max(0.0).round() as i32
}

#[cfg(test)]
mod tests {
    use crate::daemon::unit::{AbsolutePercentage, RelativePercentage, RelativePixels};

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
        let rect = workspace.get_pos_for_rect_of_size(&pos, &window);

        assert_eq!(rect.x, 0);
        assert_eq!(rect.y, 0);
        assert_eq!(rect.width, 33);
        assert_eq!(rect.height, 33);

        let pos = Position(Vertical::Middle, Horizontal::Middle);
        let rect = workspace.get_pos_for_rect_of_size(&pos, &window);

        assert_eq!(rect.x, 33);
        assert_eq!(rect.y, 33);
        assert_eq!(rect.width, 33);
        assert_eq!(rect.height, 33);

        let pos = Position(Vertical::Bottom, Horizontal::Right);
        let rect = workspace.get_pos_for_rect_of_size(&pos, &window);

        assert_eq!(rect.x, 67);
        assert_eq!(rect.y, 67);
        assert_eq!(rect.width, 33);
        assert_eq!(rect.height, 33);
    }

    #[test]
    fn test_scale_to_match_height_absolute_pixels() {
        let container = Rect::_new(0, 0, 200, 100);
        let rect = Rect::_new(0, 0, 200, 100);
        let rect = rect.scale(
            None,
            Some(AbsolutePixels(50).into()),
            &rect,
            &container,
            None,
        );

        assert_eq!(rect.width, 100);
        assert_eq!(rect.height, 50);

        let container = Rect::_new(0, 0, 100, 50);
        let rect = Rect::_new(0, 0, 100, 50);
        let rect = rect.scale(
            None,
            Some(AbsolutePixels(200).into()),
            &rect,
            &container,
            None,
        );

        assert_eq!(rect.width, 400);
        assert_eq!(rect.height, 200);
    }

    #[test]
    fn test_scale_to_match_height_absolute_percentage() {
        let container = Rect::_new(0, 0, 200, 100);
        let rect = Rect::_new(0, 0, 200, 100);
        let rect = rect.scale(
            None,
            Some(AbsolutePercentage(10.0).into()),
            &rect,
            &container,
            None,
        );

        assert_eq!(rect.width, 20);
        assert_eq!(rect.height, 10);

        let container = Rect::_new(0, 0, 100, 50);
        let rect = Rect::_new(0, 0, 100, 50);
        let rect = rect.scale(
            None,
            Some(AbsolutePercentage(10.0).into()),
            &rect,
            &container,
            None,
        );

        assert_eq!(rect.width, 10);
        assert_eq!(rect.height, 5);
    }

    #[test]
    fn test_scale_to_match_height_relative_percentage() {
        let container = Rect::_new(0, 0, 200, 100);
        let rect = Rect::_new(0, 0, 200, 100);
        let rect = rect.scale(
            None,
            Some(RelativePercentage(10.0).into()),
            &rect,
            &container,
            None,
        );

        assert_eq!(rect.width, 220);
        assert_eq!(rect.height, 110);

        let container = Rect::_new(0, 0, 100, 50);
        let rect = Rect::_new(0, 0, 100, 50);
        let rect = rect.scale(
            None,
            Some(RelativePercentage(10.0).into()),
            &rect,
            &container,
            None,
        );

        assert_eq!(rect.width, 110);
        assert_eq!(rect.height, 55);
    }

    #[test]
    fn test_scale_to_match_height_relative_pixels() {
        let container = Rect::_new(0, 0, 200, 100);
        let rect = Rect::_new(0, 0, 200, 100);
        let rect = rect.scale(
            None,
            Some(RelativePixels(50).into()),
            &rect,
            &container,
            None,
        );

        assert_eq!(rect.width, 300);
        assert_eq!(rect.height, 150);

        let container = Rect::_new(0, 0, 100, 50);
        let rect = Rect::_new(0, 0, 100, 50);
        let rect = rect.scale(
            None,
            Some(RelativePixels(200).into()),
            &rect,
            &container,
            None,
        );

        assert_eq!(rect.width, 500);
        assert_eq!(rect.height, 250);
    }

    #[test]
    fn test_scale_to_match_width_absolute_pixels() {
        let container = Rect::_new(0, 0, 200, 100);
        let rect = Rect::_new(0, 0, 200, 100);
        let rect = rect.scale(
            Some(AbsolutePixels(400).into()),
            None,
            &rect,
            &container,
            None,
        );

        assert_eq!(rect.width, 400);
        assert_eq!(rect.height, 200);

        let container = Rect::_new(0, 0, 100, 50);
        let rect = Rect::_new(0, 0, 100, 50);
        let rect = rect.scale(
            Some(AbsolutePixels(25).into()),
            None,
            &rect,
            &container,
            None,
        );

        assert_eq!(rect.width, 25);
        assert_eq!(rect.height, 13);
    }

    #[test]
    fn test_scale_to_match_width_absolute_percentage() {
        let container = Rect::_new(0, 0, 200, 100);
        let rect = Rect::_new(0, 0, 200, 100);
        let rect = rect.scale(
            Some(AbsolutePercentage(10.0).into()),
            None,
            &rect,
            &container,
            None,
        );

        assert_eq!(rect.width, 20);
        assert_eq!(rect.height, 10);

        let container = Rect::_new(0, 0, 100, 50);
        let rect = Rect::_new(0, 0, 100, 50);
        let rect = rect.scale(
            Some(AbsolutePercentage(10.0).into()),
            None,
            &rect,
            &container,
            None,
        );

        assert_eq!(rect.width, 10);
        assert_eq!(rect.height, 5);
    }

    #[test]
    fn test_scale_to_match_width_relative_pixels() {
        let container = Rect::_new(0, 0, 200, 100);
        let rect = Rect::_new(0, 0, 200, 100);
        let rect = rect.scale(
            Some(RelativePixels(400).into()),
            None,
            &rect,
            &container,
            None,
        );

        assert_eq!(rect.width, 600);
        assert_eq!(rect.height, 300);

        let container = Rect::_new(0, 0, 100, 50);
        let rect = Rect::_new(0, 0, 100, 50);
        let rect = rect.scale(
            Some(RelativePixels(25).into()),
            None,
            &rect,
            &container,
            None,
        );

        assert_eq!(rect.width, 125);
        assert_eq!(rect.height, 63);
    }

    #[test]
    fn test_scale_to_match_width_relative_percentage() {
        let container = Rect::_new(0, 0, 200, 100);
        let rect = Rect::_new(0, 0, 200, 100);
        let rect = rect.scale(
            Some(RelativePercentage(10.0).into()),
            None,
            &rect,
            &container,
            None,
        );

        assert_eq!(rect.width, 220);
        assert_eq!(rect.height, 110);

        let container = Rect::_new(0, 0, 100, 50);
        let rect = Rect::_new(0, 0, 100, 50);
        let rect = rect.scale(
            Some(RelativePercentage(10.0).into()),
            None,
            &rect,
            &container,
            None,
        );

        assert_eq!(rect.width, 110);
        assert_eq!(rect.height, 55);
    }

    #[test]
    fn test_aspect_ratio() {
        assert_eq!(aspect_ratio(1920, 1080), 16.0 / 9.0);
        assert_eq!(aspect_ratio(640, 480), 4.0 / 3.0);
        assert_eq!(aspect_ratio(100, 100), 1.0);

        assert_eq!(aspect_ratio(0, 100), 0.0);
        assert_eq!(aspect_ratio(100, 0), 0.0);
        assert_eq!(aspect_ratio(0, 0), 0.0);
    }

    #[test]
    fn test_scale_to_ratio() {
        assert_eq!(
            scale_to_ratio(Dimension::Width(1920), 16.0 / 9.0),
            Dimension::Height(1080)
        );
        assert_eq!(
            scale_to_ratio(Dimension::Height(1080), 16.0 / 9.0),
            Dimension::Width(1920)
        );
        assert_eq!(
            scale_to_ratio(Dimension::Width(640), 4.0 / 3.0),
            Dimension::Height(480)
        );
        assert_eq!(
            scale_to_ratio(Dimension::Height(480), 4.0 / 3.0),
            Dimension::Width(640)
        );
        assert_eq!(
            scale_to_ratio(Dimension::Width(640), 16.0 / 9.0),
            Dimension::Height(360)
        );
        assert_eq!(
            scale_to_ratio(Dimension::Height(480), 16.0 / 9.0),
            Dimension::Width(853)
        );
        assert_eq!(
            scale_to_ratio(Dimension::Width(100), 1.0),
            Dimension::Height(100)
        );
    }

    #[test]
    fn test_unit_to_real_pixels() {
        assert_eq!(
            unit_to_real_pixels(AbsolutePixels(100).into(), 200, 1000),
            100
        );
        assert_eq!(
            unit_to_real_pixels(AbsolutePercentage(50.0).into(), 200, 1000),
            500
        );
        assert_eq!(
            unit_to_real_pixels(RelativePixels(-50).into(), 200, 1000),
            150
        );
        assert_eq!(
            unit_to_real_pixels(RelativePercentage(50.0).into(), 250, 1000),
            750
        );
    }
}
