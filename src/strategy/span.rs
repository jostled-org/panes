/// Grid span for a dashboard card: how many columns and rows it occupies.
///
/// Serializes as `{ "columns": ..., "rows": N }`. Deserialization also
/// accepts the pre-0.23 column-only encoding (`{ "Columns": N }` or
/// `"FullWidth"`), so saved snapshots keep loading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(from = "CardSpanRepr")
)]
pub struct CardSpan {
    /// Horizontal extent.
    pub columns: ColumnSpan,
    /// Number of rows to span. Emits `grid-row: span N` in CSS when above 1.
    pub rows: usize,
}

/// Horizontal extent of a grid card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ColumnSpan {
    /// Span a fixed number of columns.
    #[cfg_attr(feature = "serde", serde(alias = "Columns"))]
    Count(usize),
    /// Span all columns regardless of how many the viewport produces.
    /// Emits `grid-column: 1 / -1` in CSS.
    FullWidth,
}

impl CardSpan {
    /// Span `n` columns and one row.
    pub const fn columns(n: usize) -> Self {
        Self {
            columns: ColumnSpan::Count(n),
            rows: 1,
        }
    }

    /// Span every column and one row.
    pub const fn full_width() -> Self {
        Self {
            columns: ColumnSpan::FullWidth,
            rows: 1,
        }
    }

    /// Return this span with `rows` rows.
    pub const fn with_rows(self, rows: usize) -> Self {
        Self { rows, ..self }
    }
}

impl Default for CardSpan {
    fn default() -> Self {
        Self::columns(1)
    }
}

impl From<ColumnSpan> for CardSpan {
    fn from(columns: ColumnSpan) -> Self {
        Self { columns, rows: 1 }
    }
}

impl From<usize> for CardSpan {
    fn from(n: usize) -> Self {
        Self::columns(n)
    }
}

/// `(columns, rows)`.
impl From<(usize, usize)> for CardSpan {
    fn from((columns, rows): (usize, usize)) -> Self {
        Self::columns(columns).with_rows(rows)
    }
}

/// Wire forms accepted when deserializing a [`CardSpan`].
#[cfg(feature = "serde")]
#[derive(serde::Deserialize)]
#[serde(untagged)]
enum CardSpanRepr {
    Current {
        columns: ColumnSpan,
        #[serde(default = "one_row")]
        rows: usize,
    },
    Legacy(ColumnSpan),
}

#[cfg(feature = "serde")]
fn one_row() -> usize {
    1
}

#[cfg(feature = "serde")]
impl From<CardSpanRepr> for CardSpan {
    fn from(repr: CardSpanRepr) -> Self {
        match repr {
            CardSpanRepr::Current { columns, rows } => Self { columns, rows },
            CardSpanRepr::Legacy(columns) => columns.into(),
        }
    }
}
