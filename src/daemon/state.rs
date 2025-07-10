use std::error::Error;

use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use swayipc::Error as SwayIPCError;

use crate::{
    cli::Args,
    daemon::{
        unit::{AbsolutePixels, AbsoluteUnit, Unit},
        DaemonError,
    },
    sway::Window,
};

pub struct InitialStateOptions {
    pub position: PositionUpdate,
    pub padding: Option<u32>,
    pub width: Option<AbsoluteUnit>,
    pub height: Option<AbsoluteUnit>,
    pub natural: Option<bool>,
}

impl TryFrom<Args> for InitialStateOptions {
    type Error = DaemonError;

    fn try_from(args: Args) -> Result<Self, Self::Error> {
        let width = match args.width {
            Some(Unit::Absolute(width)) => Some(width),
            Some(Unit::Relative(_)) => {
                return Err(DaemonError::InvalidInitialState(
                    "The initial width must not be a relative value".to_string(),
                ))
            }
            None => None,
        };

        let height = match args.height {
            Some(Unit::Absolute(height)) => Some(height),
            Some(Unit::Relative(_)) => {
                return Err(DaemonError::InvalidInitialState(
                    "The initial height must not be a relative value".to_string(),
                ))
            }
            None => None,
        };

        Ok(Self {
            position: PositionUpdate(args.vertical, args.horizontal),
            padding: args.padding,
            width,
            height,
            natural: args.natural,
        })
    }
}

#[derive(Debug, Default, Clone)]
pub struct State {
    pub position: Position,
    pub padding: u32,
    pub width: Option<AbsoluteUnit>,
    pub height: Option<AbsoluteUnit>,
    pub natural: bool,
}

impl State {
    pub fn update(&mut self, update: StateUpdate, context: &Window) {
        self.position.update(update.position);
        if let Some(padding) = update.padding {
            self.padding = padding;
        }
        if let Some(natural) = update.natural {
            self.natural = natural;
        }

        let default_width = AbsolutePixels::from(context.dimensions.width as u32).into();
        let default_height = AbsolutePixels::from(context.dimensions.height as u32).into();

        let parent_width: AbsolutePixels = (context.working_area.width as u32).into();
        let parent_height: AbsolutePixels = (context.working_area.height as u32).into();

        // If only one dimension is provided, we probably want to set the other to None
        match (update.width, update.height) {
            (Some(width), Some(height)) => {
                self.width = Some(
                    width.to_absolute(self.width.clone().unwrap_or(default_width), parent_width),
                );
                self.height = Some(
                    height
                        .to_absolute(self.height.clone().unwrap_or(default_height), parent_height),
                );
            }
            (Some(width), None) => {
                self.width = Some(
                    width.to_absolute(self.width.clone().unwrap_or(default_width), parent_width),
                );
                self.height = None;
            }
            (None, Some(height)) => {
                self.width = None;
                self.height = Some(
                    height
                        .to_absolute(self.height.clone().unwrap_or(default_height), parent_height),
                );
            }
            _ => {}
        }
    }

    pub fn with_initial(initial: InitialStateOptions) -> Self {
        Self {
            position: Position(
                initial.position.0.unwrap_or_default(),
                initial.position.1.unwrap_or_default(),
            ),
            padding: initial.padding.unwrap_or_default(),
            width: initial.width,
            height: initial.height,
            natural: initial.natural.unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Vertical {
    /// Top-aligned in the top third of the space
    Top,
    /// Centered on the middle third of the space
    Middle,
    /// Bottom-aligned in the bottom third of the space
    Bottom,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Horizontal {
    /// Left-aligned in the left third of the space
    Left,
    /// Centered on the middle third of the space
    Middle,
    /// Right-aligned in the right third of the space
    Right,
}

impl Default for Vertical {
    fn default() -> Self {
        Self::Bottom
    }
}

impl Default for Horizontal {
    fn default() -> Self {
        Self::Right
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Position(pub Vertical, pub Horizontal);

impl Position {
    pub fn update(&mut self, update: PositionUpdate) {
        if let Some(vertical) = update.0 {
            self.0 = vertical;
        }
        if let Some(horizontal) = update.1 {
            self.1 = horizontal;
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PositionUpdate(pub Option<Vertical>, pub Option<Horizontal>);

impl From<Position> for PositionUpdate {
    fn from(state: Position) -> Self {
        Self(Some(state.0), Some(state.1))
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct StateUpdate {
    pub position: PositionUpdate,
    pub padding: Option<u32>,
    pub width: Option<Unit>,
    pub height: Option<Unit>,
    pub natural: Option<bool>,
}

impl From<State> for StateUpdate {
    fn from(state: State) -> Self {
        Self {
            position: state.position.into(),
            padding: Some(state.padding),
            width: state.width.map(Unit::Absolute),
            height: state.height.map(Unit::Absolute),
            natural: Some(state.natural),
        }
    }
}

#[derive(Debug)]
pub enum StateUpdateError {
    SwayIPC(swayipc::Error),
    NoApplicableNode,
    MultipleApplicableNodes,
}

impl std::fmt::Display for StateUpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StateUpdateError::SwayIPC(err) => write!(f, "SwayIPC error: {}", err),
            StateUpdateError::NoApplicableNode => write!(f, "No applicable node found"),
            StateUpdateError::MultipleApplicableNodes => {
                write!(f, "Multiple applicable nodes found")
            }
        }
    }
}

impl Error for StateUpdateError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            StateUpdateError::SwayIPC(err) => Some(err),
            _ => None,
        }
    }
}

impl From<SwayIPCError> for StateUpdateError {
    fn from(err: SwayIPCError) -> Self {
        Self::SwayIPC(err)
    }
}
