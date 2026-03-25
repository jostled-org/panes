use crate::error::{ConstraintError, PaneError, TreeError};
use crate::node::PanelId;
use crate::validate::{check_f32_non_negative, float_invalid_to_constraint};

/// Content-based sizing mode for a panel.
///
/// Overrides `flex-basis` when set, emitting CSS intrinsic-sizing keywords.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "kebab-case"))]
pub enum SizeMode {
    MinContent,
    MaxContent,
    FitContent(f32),
}

/// Cross-axis alignment for a panel within its container.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum Align {
    Start,
    Center,
    End,
    Stretch,
}

/// Generates sequential, unique `PanelId` values.
#[derive(Default)]
pub struct PanelIdGenerator {
    counter: u32,
}

impl PanelIdGenerator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn high_water(&self) -> u32 {
        self.counter
    }

    pub fn next_id(&mut self) -> Result<PanelId, PaneError> {
        let id = PanelId::from_raw(self.counter);
        self.counter = self
            .counter
            .checked_add(1)
            .ok_or(PaneError::InvalidTree(TreeError::PanelIdExhausted))?;
        Ok(id)
    }
}

/// Spatial constraints for a panel within a layout.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Constraints {
    /// Mutually exclusive with `fixed`.
    pub grow: Option<f32>,
    /// Mutually exclusive with `grow`.
    pub fixed: Option<f32>,
    pub min: Option<f32>,
    pub max: Option<f32>,
    pub min_width: Option<f32>,
    pub max_width: Option<f32>,
    pub min_height: Option<f32>,
    pub max_height: Option<f32>,
    pub align: Option<Align>,
    /// Overrides flex-basis in CSS emission.
    pub size_mode: Option<SizeMode>,
}

impl Constraints {
    crate::macros::builder_mapped_setters!(
        min(value: f32) -> min = Some(value);
        max(value: f32) -> max = Some(value);
        min_width(value: f32) -> min_width = Some(value);
        max_width(value: f32) -> max_width = Some(value);
        min_height(value: f32) -> min_height = Some(value);
        max_height(value: f32) -> max_height = Some(value);
        align(value: Align) -> align = Some(value);
        size_mode(value: SizeMode) -> size_mode = Some(value)
    );

    pub fn validate(&self) -> Result<(), PaneError> {
        Self::reject_bad_f32("grow", self.grow)?;
        Self::reject_bad_f32("fixed", self.fixed)?;
        Self::reject_bad_f32("min", self.min)?;
        Self::reject_bad_f32("max", self.max)?;
        Self::reject_bad_f32("min_width", self.min_width)?;
        Self::reject_bad_f32("max_width", self.max_width)?;
        Self::reject_bad_f32("min_height", self.min_height)?;
        Self::reject_bad_f32("max_height", self.max_height)?;

        match self.size_mode {
            Some(SizeMode::FitContent(v)) => Self::reject_bad_f32("fit_content", Some(v))?,
            _ => {}
        }

        match (self.grow, self.fixed) {
            (Some(_), Some(_)) => {
                return Err(PaneError::InvalidConstraint(
                    ConstraintError::GrowFixedExclusive,
                ));
            }
            _ => {}
        }

        Self::check_range(self.min, self.max)?;
        Self::check_range(self.min_width, self.max_width)?;
        Self::check_range(self.min_height, self.max_height)
    }

    fn check_range(lo: Option<f32>, hi: Option<f32>) -> Result<(), PaneError> {
        match (lo, hi) {
            (Some(lo), Some(hi)) if lo > hi => {
                Err(PaneError::InvalidConstraint(ConstraintError::MinExceedsMax))
            }
            _ => Ok(()),
        }
    }

    fn reject_bad_f32(name: &'static str, value: Option<f32>) -> Result<(), PaneError> {
        let Some(v) = value else { return Ok(()) };
        check_f32_non_negative(v)
            .map_err(|e| PaneError::InvalidConstraint(float_invalid_to_constraint(name, e)))
    }
}

pub fn grow(value: f32) -> Constraints {
    Constraints {
        grow: Some(value),
        ..Constraints::default()
    }
}

pub fn fixed(value: f32) -> Constraints {
    Constraints {
        fixed: Some(value),
        ..Constraints::default()
    }
}
