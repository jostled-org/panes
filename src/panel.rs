use crate::error::{ConstraintError, PaneError, TreeError};
use crate::node::PanelId;
use crate::validate::{check_f32_non_negative, float_invalid_to_constraint};

/// Cross-axis alignment for a panel within its container.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum Align {
    /// Align to the start of the cross axis.
    Start,
    /// Center along the cross axis.
    Center,
    /// Align to the end of the cross axis.
    End,
    /// Stretch to fill the cross axis (default behavior).
    Stretch,
}

/// Generates sequential, unique `PanelId` values.
#[derive(Default)]
pub struct PanelIdGenerator {
    counter: u32,
}

impl PanelIdGenerator {
    /// Create a generator starting at zero.
    pub fn new() -> Self {
        Self::default()
    }

    /// The number of IDs generated so far (one past the last issued ID).
    pub fn high_water(&self) -> u32 {
        self.counter
    }

    /// Produce the next unique `PanelId`.
    ///
    /// Returns an error if the counter reaches `u32::MAX`.
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
    /// Flex grow factor. Mutually exclusive with `fixed`.
    pub grow: Option<f32>,
    /// Fixed size in layout units. Mutually exclusive with `grow`.
    pub fixed: Option<f32>,
    /// Minimum size along the parent axis.
    pub min: Option<f32>,
    /// Maximum size along the parent axis.
    pub max: Option<f32>,
    /// Minimum width regardless of parent axis.
    pub min_width: Option<f32>,
    /// Maximum width regardless of parent axis.
    pub max_width: Option<f32>,
    /// Minimum height regardless of parent axis.
    pub min_height: Option<f32>,
    /// Maximum height regardless of parent axis.
    pub max_height: Option<f32>,
    /// Cross-axis alignment. None means Stretch (default).
    pub align: Option<Align>,
}

impl Constraints {
    /// Set the minimum size constraint.
    pub fn min(mut self, value: f32) -> Self {
        self.min = Some(value);
        self
    }

    /// Set the maximum size constraint.
    pub fn max(mut self, value: f32) -> Self {
        self.max = Some(value);
        self
    }

    /// Set the minimum width constraint (cross-axis, absolute).
    pub fn min_width(mut self, value: f32) -> Self {
        self.min_width = Some(value);
        self
    }

    /// Set the maximum width constraint (cross-axis, absolute).
    pub fn max_width(mut self, value: f32) -> Self {
        self.max_width = Some(value);
        self
    }

    /// Set the minimum height constraint (cross-axis, absolute).
    pub fn min_height(mut self, value: f32) -> Self {
        self.min_height = Some(value);
        self
    }

    /// Set the maximum height constraint (cross-axis, absolute).
    pub fn max_height(mut self, value: f32) -> Self {
        self.max_height = Some(value);
        self
    }

    /// Set cross-axis alignment.
    pub fn align(mut self, value: Align) -> Self {
        self.align = Some(value);
        self
    }

    /// Reject invalid constraint combinations.
    pub fn validate(&self) -> Result<(), PaneError> {
        Self::reject_bad_f32("grow", self.grow)?;
        Self::reject_bad_f32("fixed", self.fixed)?;
        Self::reject_bad_f32("min", self.min)?;
        Self::reject_bad_f32("max", self.max)?;
        Self::reject_bad_f32("min_width", self.min_width)?;
        Self::reject_bad_f32("max_width", self.max_width)?;
        Self::reject_bad_f32("min_height", self.min_height)?;
        Self::reject_bad_f32("max_height", self.max_height)?;

        match (self.grow, self.fixed, self.min, self.max) {
            (Some(_), Some(_), _, _) => Err(PaneError::InvalidConstraint(
                ConstraintError::GrowFixedExclusive,
            )),
            (_, _, Some(lo), Some(hi)) if lo > hi => {
                Err(PaneError::InvalidConstraint(ConstraintError::MinExceedsMax))
            }
            _ => Ok(()),
        }?;

        match (self.min_width, self.max_width) {
            (Some(lo), Some(hi)) if lo > hi => {
                Err(PaneError::InvalidConstraint(ConstraintError::MinExceedsMax))
            }
            _ => Ok(()),
        }?;

        match (self.min_height, self.max_height) {
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

/// Create constraints with a grow factor.
pub fn grow(value: f32) -> Constraints {
    Constraints {
        grow: Some(value),
        ..Constraints::default()
    }
}

/// Create constraints with a fixed size.
pub fn fixed(value: f32) -> Constraints {
    Constraints {
        fixed: Some(value),
        ..Constraints::default()
    }
}
