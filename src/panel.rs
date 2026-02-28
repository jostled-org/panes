use crate::error::PaneError;
use crate::node::PanelId;

/// Generates sequential, unique `PanelId` values.
#[derive(Default)]
pub struct PanelIdGenerator {
    counter: u32,
}

impl PanelIdGenerator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Produce the next unique `PanelId`.
    ///
    /// Returns an error if the counter reaches `u32::MAX`.
    pub fn next_id(&mut self) -> Result<PanelId, PaneError> {
        let id = PanelId::from_raw(self.counter);
        self.counter = self
            .counter
            .checked_add(1)
            .ok_or(PaneError::InvalidTree("panel ID counter exhausted".into()))?;
        Ok(id)
    }
}

/// Spatial constraints for a panel within a layout.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Constraints {
    pub grow: Option<f32>,
    pub fixed: Option<f32>,
    pub min: Option<f32>,
    pub max: Option<f32>,
}

impl Constraints {
    pub fn min(mut self, value: f32) -> Self {
        self.min = Some(value);
        self
    }

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
                "grow and fixed are mutually exclusive".into(),
            )),
            (_, _, Some(lo), Some(hi)) if lo > hi => {
                Err(PaneError::InvalidConstraint("min exceeds max".into()))
            }
            _ => Ok(()),
        }
    }

    fn reject_bad_f32(name: &str, value: Option<f32>) -> Result<(), PaneError> {
        match value {
            Some(v) if v.is_nan() => Err(PaneError::InvalidConstraint(
                format!("{name} is NaN").into(),
            )),
            Some(v) if v < 0.0 => Err(PaneError::InvalidConstraint(
                format!("{name} is negative").into(),
            )),
            _ => Ok(()),
        }
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
