use std::ops::{Deref, DerefMut};

use swayipc::{Connection, Fallible, Node};

use crate::daemon::unit::{AbsolutePercentage, AbsoluteUnit};

pub struct SwayConnection(Connection);

impl Deref for SwayConnection {
    type Target = Connection;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SwayConnection {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl SwayConnection {
    pub fn new() -> Fallible<Self> {
        Ok(Self(Connection::new()?))
    }

    pub fn find_working_area_for(&mut self, node_id: i64) -> Fallible<Option<swayipc::Rect>> {
        Ok(self
            .get_workspaces()?
            .iter()
            .find(|w| w.focus.contains(&node_id))
            .map(|w| w.rect))
    }

    pub fn move_node_to_position(&mut self, node_id: i64, x: i32, y: i32) -> Fallible<()> {
        let cmd = format!(r#"[con_id="{}"] move position {} {}"#, node_id, x, y);
        self.run_command(cmd)?;

        Ok(())
    }

    pub fn resize_node<W: Into<AbsoluteUnit>, H: Into<AbsoluteUnit>>(
        &mut self,
        node_id: i64,
        width: W,
        height: H,
    ) -> Fallible<()> {
        let width: AbsoluteUnit = width.into();
        let height: AbsoluteUnit = height.into();

        let cmd = format!(r#"[con_id="{}"] resize set {} {}"#, node_id, width, height);
        self.run_command(cmd)?;

        Ok(())
    }

    pub fn _get_parent_node(&mut self, node_id: i64) -> Fallible<Option<swayipc::Node>> {
        let tree = self.get_tree()?;

        Ok(tree.find_focused(|node| {
            node.nodes.iter().any(|n| n.id == node_id)
                || node.floating_nodes.iter().any(|n| n.id == node_id)
        }))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dimension {
    Width(i32),
    Height(i32),
}

#[derive(Debug, Clone)]
pub struct WindowDimension {
    pub width: i32,
    pub height: i32,
}

impl WindowDimension {
    pub fn ratio(&self) -> f32 {
        if self.height == 0 {
            0.0
        } else {
            self.width as f32 / self.height as f32
        }
    }
}

#[derive(Debug, Clone)]
pub struct Coordinate {
    x: i32,
    y: i32,
}

impl Coordinate {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone)]
pub struct Window {
    pub position: Coordinate,
    pub dimensions: WindowDimension,
    pub natural_dimensions: Option<WindowDimension>,
    pub working_area: swayipc::Rect,
}

impl Window {
    pub fn from_node(node: Node, con: &mut SwayConnection) -> Fallible<Self> {
        let working_area = con
            .find_working_area_for(node.id)?
            .expect("Node should have a working area");

        Ok(Self {
            position: Coordinate::new(node.rect.x, node.rect.y),
            dimensions: WindowDimension {
                width: node.rect.width,
                height: node.rect.height,
            },
            natural_dimensions: Some(WindowDimension {
                width: node.geometry.width,
                height: node.geometry.height,
            }),
            working_area,
        })
    }

    pub fn width_in_parent_percentage(&self) -> AbsolutePercentage {
        AbsolutePercentage::from(
            self.dimensions.width as f32 / self.working_area.width as f32 * 100.0,
        )
    }

    pub fn height_in_parent_percentage(&self) -> AbsolutePercentage {
        AbsolutePercentage::from(
            self.dimensions.height as f32 / self.working_area.height as f32 * 100.0,
        )
    }
}
