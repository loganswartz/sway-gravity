use std::ops::{Deref, DerefMut};

use swayipc::{Connection, Fallible};

use crate::Unit;

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
        Ok(self.0.get_workspaces()?.iter().find(|w| w.focus.contains(&node_id)).map(|w| w.rect))
    }

    pub fn move_node_to_position(&mut self, node_id: i64, x: i32, y: i32) -> Fallible<()> {
        let cmd = format!(r#"[con_id="{}"] move position {} {}"#, node_id, x, y);
        self.0.run_command(cmd)?;

        Ok(())
    }

    pub fn resize_node(&mut self, node_id: i64, width: Unit, height: Unit) -> Fallible<()> {
        let cmd = format!(r#"[con_id="{}"] resize set {} {}"#, node_id, width, height);
        self.0.run_command(cmd)?;

        Ok(())
    }

    pub fn _get_parent_node(&mut self, node_id: i64) -> Fallible<Option<swayipc::Node>> {
        let tree = self.0.get_tree()?;

        Ok(tree.find_focused(|node| node.nodes.iter().any(|n| n.id == node_id) || node.floating_nodes.iter().any(|n| n.id == node_id)))
    }
}
