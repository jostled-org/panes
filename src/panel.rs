use crate::error::{ConstraintError, PaneError, TreeError};
use crate::node::PanelId;
use crate::validate::{check_f32_non_negative, float_invalid_to_constraint};

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

    /// Reject invalid constraint combinations.
    pub fn validate(&self) -> Result<(), PaneError> {
        Self::reject_bad_f32("grow", self.grow)?;
        Self::reject_bad_f32("fixed", self.fixed)?;
        Self::reject_bad_f32("min", self.min)?;
        Self::reject_bad_f32("max", self.max)?;

        match (self.grow, self.fixed, self.min, self.max) {
            (Some(_), Some(_), _, _) => Err(PaneError::InvalidConstraint(
                ConstraintError::GrowFixedExclusive,
            )),
            (_, _, Some(lo), Some(hi)) if lo > hi => {
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
