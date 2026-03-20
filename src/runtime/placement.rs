/// Where to place the new panel relative to the focused panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Placement {
    /// New panel goes before focused (left or above).
    Before,
    /// New panel goes after focused (right or below).
    #[default]
    After,
    /// Append to the end of the sequence.
    End,
}
