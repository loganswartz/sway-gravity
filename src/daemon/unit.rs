use std::{
    error::Error,
    fmt::Display,
    num::ParseIntError,
    ops::{Add, Sub},
    str::FromStr,
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbsolutePixels(pub u32);

impl AbsolutePixels {
    pub fn as_absolute_percentage(&self, container_px: i32) -> AbsolutePercentage {
        let absolute_value = self.0 as f32 / container_px as f32;
        AbsolutePercentage((absolute_value * 100.0).round())
    }
}

impl<T: Into<u32>> From<T> for AbsolutePixels {
    fn from(value: T) -> Self {
        AbsolutePixels(value.into())
    }
}

impl Add<AbsolutePixels> for AbsolutePixels {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        AbsolutePixels(self.0 + other.0)
    }
}

impl Sub<AbsolutePixels> for AbsolutePixels {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        AbsolutePixels(self.0.saturating_sub(other.0))
    }
}

impl Add<RelativePixels> for AbsolutePixels {
    type Output = Self;

    fn add(self, other: RelativePixels) -> Self::Output {
        AbsolutePixels((self.0 as i32 + other.0) as u32)
    }
}

impl Sub<RelativePixels> for AbsolutePixels {
    type Output = Self;

    fn sub(self, other: RelativePixels) -> Self::Output {
        AbsolutePixels((self.0 as i32 - other.0) as u32)
    }
}

impl FromStr for AbsolutePixels {
    type Err = ParseUnitError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value = s.strip_suffix("px").unwrap_or(s);
        let parsed_value: u32 = value.parse()?;
        Ok(Self(parsed_value))
    }
}

impl Display for AbsolutePixels {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} px", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelativePixels(pub i32);

impl<T: Into<i32>> From<T> for RelativePixels {
    fn from(value: T) -> Self {
        RelativePixels(value.into())
    }
}

impl Add<RelativePixels> for RelativePixels {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        RelativePixels(self.0 + other.0)
    }
}

impl Sub<RelativePixels> for RelativePixels {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        RelativePixels(self.0 - other.0)
    }
}

impl Add<AbsolutePixels> for RelativePixels {
    type Output = AbsolutePixels;

    fn add(self, other: AbsolutePixels) -> Self::Output {
        AbsolutePixels((self.0 + other.0 as i32) as u32)
    }
}

impl Sub<AbsolutePixels> for RelativePixels {
    type Output = AbsolutePixels;

    fn sub(self, other: AbsolutePixels) -> Self::Output {
        AbsolutePixels((self.0 - other.0 as i32) as u32)
    }
}

impl FromStr for RelativePixels {
    type Err = ParseUnitError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value = s.strip_suffix("px").unwrap_or(s);
        let parsed_value: i32 = value.parse()?;
        Ok(Self(parsed_value))
    }
}

impl Display for RelativePixels {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0 >= 0 {
            write!(f, "+{} px", self.0)
        } else {
            write!(f, "-{} px", self.0)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AbsolutePercentage(pub f32);

impl AbsolutePercentage {
    pub fn as_absolute_pixels(&self, container_px: i32) -> AbsolutePixels {
        let absolute_value = (self.0 / 100.0) * container_px as f32;
        AbsolutePixels(absolute_value.round() as u32)
    }
}

impl<T: Into<f32>> From<T> for AbsolutePercentage {
    fn from(value: T) -> Self {
        AbsolutePercentage(value.into())
    }
}

impl Add<AbsolutePercentage> for AbsolutePercentage {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        AbsolutePercentage(self.0 + other.0)
    }
}

impl Sub<AbsolutePercentage> for AbsolutePercentage {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        AbsolutePercentage(self.0 - other.0)
    }
}

impl Add<RelativePercentage> for AbsolutePercentage {
    type Output = Self;

    fn add(self, other: RelativePercentage) -> Self::Output {
        AbsolutePercentage(self.0 + other.0)
    }
}

impl Sub<RelativePercentage> for AbsolutePercentage {
    type Output = Self;

    fn sub(self, other: RelativePercentage) -> Self::Output {
        AbsolutePercentage(self.0 - other.0)
    }
}

impl FromStr for AbsolutePercentage {
    type Err = ParseUnitError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value = s.strip_suffix("%").unwrap_or(s);
        let parsed_value: f32 = value.parse()?;
        Ok(Self(parsed_value))
    }
}

impl Display for AbsolutePercentage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ppt", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RelativePercentage(pub f32);

impl<T: Into<f32>> From<T> for RelativePercentage {
    fn from(value: T) -> Self {
        RelativePercentage(value.into())
    }
}

impl Add<RelativePercentage> for RelativePercentage {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        RelativePercentage(self.0 + other.0)
    }
}

impl Sub<RelativePercentage> for RelativePercentage {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        RelativePercentage(self.0 - other.0)
    }
}

impl Add<AbsolutePercentage> for RelativePercentage {
    type Output = AbsolutePercentage;

    fn add(self, other: AbsolutePercentage) -> Self::Output {
        AbsolutePercentage(self.0 + other.0)
    }
}

impl Sub<AbsolutePercentage> for RelativePercentage {
    type Output = AbsolutePercentage;

    fn sub(self, other: AbsolutePercentage) -> Self::Output {
        AbsolutePercentage(self.0 - other.0)
    }
}

impl FromStr for RelativePercentage {
    type Err = ParseUnitError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value = s.strip_suffix("%").unwrap_or(s);
        let parsed_value: f32 = value.parse()?;
        Ok(Self(parsed_value))
    }
}

impl Display for RelativePercentage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0 >= 0.0 {
            write!(f, "+{} ppt", self.0)
        } else {
            write!(f, "-{} ppt", -self.0)
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

impl From<AbsoluteUnit> for Unit {
    fn from(value: AbsoluteUnit) -> Self {
        Self::Absolute(value)
    }
}

impl From<RelativeUnit> for Unit {
    fn from(value: RelativeUnit) -> Self {
        Self::Relative(value)
    }
}

impl From<AbsolutePixels> for Unit {
    fn from(value: AbsolutePixels) -> Self {
        Self::Absolute(AbsoluteUnit::Pixels(value))
    }
}

impl From<RelativePixels> for Unit {
    fn from(value: RelativePixels) -> Self {
        Self::Relative(RelativeUnit::Pixels(value))
    }
}

impl From<AbsolutePercentage> for Unit {
    fn from(value: AbsolutePercentage) -> Self {
        Self::Absolute(AbsoluteUnit::Percentage(value))
    }
}

impl From<RelativePercentage> for Unit {
    fn from(value: RelativePercentage) -> Self {
        Self::Relative(RelativeUnit::Percentage(value))
    }
}

impl Unit {
    pub fn to_absolute<B: Into<AbsoluteUnit>, C: Into<AbsolutePixels>>(
        &self,
        baseline: B,
        container_px: C,
    ) -> AbsoluteUnit {
        let baseline: AbsoluteUnit = baseline.into();
        let container_px: AbsolutePixels = container_px.into();

        let relative = match self {
            Self::Absolute(absolute) => return absolute.clone(),
            Self::Relative(relative) => relative,
        };

        match (baseline, relative) {
            (AbsoluteUnit::Pixels(current), RelativeUnit::Pixels(pixels)) => {
                (current + *pixels).into()
            }
            (AbsoluteUnit::Percentage(current), RelativeUnit::Percentage(percentage)) => {
                (current + *percentage).into()
            }
            (AbsoluteUnit::Percentage(current), RelativeUnit::Pixels(pixels)) => {
                let as_pixels = current.as_absolute_pixels(container_px.0 as i32);
                (as_pixels + *pixels).into()
            }
            (AbsoluteUnit::Pixels(current), RelativeUnit::Percentage(percentage)) => {
                let as_percentage = current.as_absolute_percentage(container_px.0 as i32);
                (as_percentage + *percentage).into()
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AbsoluteUnit {
    /// A dimension in pixels (ex: `100` or `100px`)
    Pixels(AbsolutePixels),
    /// A dimension as a percentage (ex: `33.333%`)
    Percentage(AbsolutePercentage),
}

impl From<AbsolutePixels> for AbsoluteUnit {
    fn from(value: AbsolutePixels) -> Self {
        Self::Pixels(value)
    }
}

impl From<AbsolutePercentage> for AbsoluteUnit {
    fn from(value: AbsolutePercentage) -> Self {
        Self::Percentage(value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelativeUnit {
    /// A relative dimension in pixels (ex: `+100` or `-100px`)
    Pixels(RelativePixels),
    /// A dimension as a percentage (ex: `-5%`)
    Percentage(RelativePercentage),
}

impl From<RelativePixels> for RelativeUnit {
    fn from(value: RelativePixels) -> Self {
        Self::Pixels(value)
    }
}

impl From<RelativePercentage> for RelativeUnit {
    fn from(value: RelativePercentage) -> Self {
        Self::Percentage(value)
    }
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

impl FromStr for Unit {
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

impl FromStr for RelativeUnit {
    type Err = ParseUnitError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.strip_suffix("%").is_some() {
            Ok(Self::Percentage(s.parse()?))
        } else {
            Ok(Self::Pixels(s.parse()?))
        }
    }
}

impl Display for RelativeUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pixels(value) => write!(f, "{}", value),
            Self::Percentage(value) => write!(f, "{}", value),
        }
    }
}

impl FromStr for AbsoluteUnit {
    type Err = ParseUnitError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.strip_suffix("%").is_some() {
            Ok(Self::Percentage(s.parse()?))
        } else {
            // Default to pixels if no suffix is provided
            Ok(Self::Pixels(s.parse()?))
        }
    }
}

impl Display for AbsoluteUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pixels(value) => write!(f, "{}", value),
            Self::Percentage(value) => write!(f, "{}", value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_absolute_pixels_from_str() {
        assert_eq!(
            AbsolutePixels::from_str("100px").unwrap(),
            AbsolutePixels(100)
        );
        assert_eq!(
            AbsolutePixels::from_str("100").unwrap(),
            AbsolutePixels(100)
        );
        assert!(AbsolutePixels::from_str("100%").is_err());
    }

    #[test]
    fn test_relative_pixels_from_str() {
        assert_eq!(
            RelativePixels::from_str("+50px").unwrap(),
            RelativePixels(50)
        );
        assert_eq!(
            RelativePixels::from_str("-50px").unwrap(),
            RelativePixels(-50)
        );
        assert!(RelativePixels::from_str("50%").is_err());
    }

    #[test]
    fn test_absolute_percentage_from_str() {
        assert_eq!(
            AbsolutePercentage::from_str("33.333%").unwrap(),
            AbsolutePercentage(33.333)
        );
        assert!(AbsolutePercentage::from_str("33.333px").is_err());
    }

    #[test]
    fn test_relative_percentage_from_str() {
        assert_eq!(
            RelativePercentage::from_str("+10%").unwrap(),
            RelativePercentage(10.0)
        );
        assert_eq!(
            RelativePercentage::from_str("-10%").unwrap(),
            RelativePercentage(-10.0)
        );
        assert!(RelativePercentage::from_str("10px").is_err());
    }

    #[test]
    fn test_adding_units() {
        let abs_px1 = AbsolutePixels::from(100u32);
        let abs_px2 = AbsolutePixels::from(50u32);
        let rel_px = RelativePixels::from(20);

        assert_eq!(abs_px1 + abs_px2, AbsolutePixels::from(150u32));
        assert_eq!(abs_px1 + rel_px, AbsolutePixels::from(120u32));
        assert_eq!(abs_px1 - rel_px, AbsolutePixels::from(80u32));
    }
}
