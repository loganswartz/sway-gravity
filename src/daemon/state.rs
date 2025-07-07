use std::{error::Error, fmt::Display, num::ParseIntError};

use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use swayipc::Error as SwayIPCError;

#[derive(Debug, Default, Clone)]
pub struct State {
    pub position: Position,
    pub padding: u32,
    pub width: Option<Unit>,
    pub height: Option<Unit>,
    pub natural: bool,
}

impl State {
    pub fn update(&mut self, update: StateUpdate) {
        self.position.update(update.position);
        if let Some(padding) = update.padding {
            self.padding = padding;
        }
        if let Some(natural) = update.natural {
            self.natural = natural;
        }

        // If only one dimension is provided, we probably want to set the other to None
        match (update.width, update.height) {
            (Some(width), Some(height)) => {
                self.width = width;
                self.height = height;
            }
            (Some(width), None) => {
                self.width = width;
                self.height = None;
            }
            (None, Some(height)) => {
                self.width = None;
                self.height = height;
            }
            _ => {}
        }
    }

    pub fn with(self, update: StateUpdate) -> Self {
        let mut new_state = self.clone();
        new_state.update(update);
        new_state
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Unit {
    /// A relative dimension, which can be a percentage or a pixel value (ex: `+100px` or `-5%`)
    Relative(RelativeUnit),
    /// An absolute dimension, which can be a percentage or a pixel value (ex: `100px` or `33.333%`)
    Absolute(AbsoluteUnit),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AbsoluteUnit {
    /// A dimension in pixels (ex: `100` or `100px`)
    Pixels(u32),
    /// A dimension as a percentage (ex: `33.333%`)
    Percentage(f32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelativeUnit {
    /// A relative dimension in pixels (ex: `+100` or `-100px`)
    Pixels(i32),
    /// A dimension as a percentage (ex: `-5%`)
    Percentage(f32),
}

#[derive(Debug, Clone)]
pub enum ParseUnitError {
    ParseIntError(ParseIntError),
    ParseFloatError(std::num::ParseFloatError),
}

impl Display for ParseUnitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseUnitError::ParseIntError(err) => write!(f, "ParseIntError: {}", err),
            ParseUnitError::ParseFloatError(err) => write!(f, "ParseFloatError: {}", err),
        }
    }
}

impl Error for ParseUnitError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ParseUnitError::ParseIntError(err) => Some(err),
            ParseUnitError::ParseFloatError(err) => Some(err),
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
        if s.starts_with(['-', '+']) {
            Ok(Self::Relative(s.parse()?))
        } else {
            Ok(Self::Absolute(s.parse()?))
        }
    }
}

impl std::fmt::Display for Unit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Relative(value) => write!(f, "{}", value),
            Self::Absolute(value) => write!(f, "{}", value),
        }
    }
}

impl std::str::FromStr for RelativeUnit {
    type Err = ParseUnitError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(value) = s.strip_suffix("%") {
            Ok(Self::Percentage(value.parse()?))
        } else {
            // Default to pixels if no suffix is provided
            let value = s.strip_suffix("px").unwrap_or(s);
            Ok(Self::Pixels(value.parse()?))
        }
    }
}

impl std::fmt::Display for RelativeUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pixels(value) => write!(f, "{} px", value),
            Self::Percentage(value) => write!(f, "{} ppt", value),
        }
    }
}

impl std::str::FromStr for AbsoluteUnit {
    type Err = ParseUnitError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(value) = s.strip_suffix("%") {
            Ok(Self::Percentage(value.parse()?))
        } else {
            // Default to pixels if no suffix is provided
            let value = s.strip_suffix("px").unwrap_or(s);
            Ok(Self::Pixels(value.parse()?))
        }
    }
}

impl std::fmt::Display for AbsoluteUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pixels(value) => write!(f, "{} px", value),
            Self::Percentage(value) => write!(f, "{} ppt", value),
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
    pub width: Option<Option<Unit>>,
    pub height: Option<Option<Unit>>,
    pub natural: Option<bool>,
}

impl From<State> for StateUpdate {
    fn from(state: State) -> Self {
        Self {
            position: state.position.into(),
            padding: Some(state.padding),
            width: Some(state.width),
            height: Some(state.height),
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
