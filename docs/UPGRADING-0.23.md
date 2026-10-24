# Upgrading to panes 0.23

This guide covers breaking changes from 0.22.x to 0.23.0 and how to migrate.

## Breaking changes at a glance

| Area | Change | Migration |
|------|--------|-----------|
| Grid spans | `CardSpan` is now a struct with `columns: ColumnSpan` and `rows: usize` | Use the constructors below |
| Grid spans | `CardSpan::Columns(n)` removed | `CardSpan::columns(n)` |
| Grid spans | `CardSpan::FullWidth` removed | `CardSpan::full_width()` |
| Grid spans | Pattern matches on `CardSpan` | Match on `span.columns` with `ColumnSpan::Count(n)` / `ColumnSpan::FullWidth` |
| Validation | A span of 0 on either axis is rejected | Handle the new `ConstraintError::GridSpanZero` |
| Error types | `ConstraintError` gains `GridSpanZero` | Add a match arm if you match exhaustively |
| Error types | `MutationError` gains `NotGridItem(PanelId)` | Add a match arm if you match exhaustively |
| Runtime | `set_card_span` on a runtime without a strategy now changes the span in the tree instead of returning `NoStrategy` | A panel outside a grid returns `NotGridItem` |
| panes-css | The `emit`, `emit_with_overlays`, `emit_with_transitions`, and `emit_full` functions are generic over `AsRef<LayoutTree>` | Call sites with `&layout` compile unchanged. Code that uses these functions as `fn` pointers must name the type, e.g. `emit::<Layout>` |
| panes-css | Rows, columns, and containers nested directly inside a grid now emit rules | Before, they got no CSS. Markup now needs a `data-pane-node` element for each one. `skeleton()` lists them |

## Row spans

Grid items can now span rows. Row spans default to 1.

```rust
use panes::{CardSpan, Layout};

// (columns, rows)
Layout::dashboard([("chart", (1, 2)), ("stats", (1, 1)), ("events", (1, 1))]).columns(2);

// Builder
CardSpan::columns(2).with_rows(3);
CardSpan::full_width().with_rows(2);
```

The `layout!` macro accepts `rows: N` on grid panels, alone or after `span:` or `full_width:`. TOML dashboard cards accept `rows = N`. `panes-css` emits `grid-row: span N` when N is above 1.

## Markup that matches the CSS

`panes_css::skeleton(&layout)` lists the elements that `emit` styles, in document order, with each one's selector, tree node, and parent index. Build markup from it instead of repeating the tree walk. `emit` renders from the same list, so the numbering always matches.

## Snapshots

`CardSpan` now serializes as `{ "columns": ..., "rows": N }`. Deserialization still accepts the 0.22 form (`{ "Columns": N }` and `"FullWidth"`), so you can restore snapshots saved by 0.22 without changes. Snapshots written by 0.23 do not load in 0.22.
