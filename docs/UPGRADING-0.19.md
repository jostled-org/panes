# Upgrading to panes 0.19

This guide covers breaking changes from 0.18.x to 0.19.0 and how to migrate.

## Breaking changes at a glance

| Area | Change | Migration |
|------|--------|-----------|
| Focus API | `focus()` returns `FocusOutcome` instead of `bool` | Replace `if rt.focus(pid)` with `if rt.focus(pid).is_applied()` or match on `Applied`/`Unchanged`/`Rejected` |
| Focus API | `focus_direction`/`focus_direction_current` return `(Option<PanelId>, FocusOutcome)` | Destructure the tuple: `let (target, outcome) = rt.focus_direction_current(dir)?;` |
| Swap API | `swap_next()`/`swap_prev()` return `Result<(), PaneError>` | Add `?` to call sites |
| Orientation | `Direction` enum removed, replaced by `Axis` | `Direction::Horizontal` → `Axis::Row`, `Direction::Vertical` → `Axis::Col` |
| Strategy types | `StrategyKind::Sequence { direction }` → `{ axis }` | Rename field access |
| Strategy types | `StrategyKind::Window { size }` → `{ panel_count }` | Rename field access |
| Snapshot format | `SnapshotNode::Row`/`Col` gain `constraints: Option<Constraints>` | Pattern matches need the new field |
| Snapshot format | New `SnapshotNode::Grid` variant | Add match arm if exhaustively matching |
| Error types | `TreeError` gains 6 new variants | Add match arms if exhaustively matching |
| Error types | `ConstraintError` gains `ExceedsOne` variant | Add match arm if exhaustively matching |
| Overlay diff | `OverlayDiff` is now a struct (was a type alias) | No change needed if accessing fields by name |
| Overlay diff | `OverlayDiff` has new `anchor_failed` field | Use it or ignore it — no code change required |
| CSS emission | `emit_adaptive()` returns `Result<String, AdaptiveCssError>` | Add `?` to call sites |
| Node types | `Node::Row`/`Col` gain `constraints: Option<Constraints>` | Add field to pattern matches |
| Node types | New `Node::Grid` and `Node::GridItemWrapper` variants | Add match arms if exhaustively matching |
| Builder naming | `set_window_size()` → `set_window_panel_count()` | Rename call |
| Builder naming | `Layout::window_size()` → `Layout::window_panel_count()` | Rename call |

## Focus API

The biggest user-facing change. `focus()` previously returned `bool`. It now returns `FocusOutcome`, a three-state enum:

```rust
use panes::FocusOutcome;

match rt.focus(pid) {
    FocusOutcome::Applied => { /* focus moved to pid */ }
    FocusOutcome::Unchanged => { /* pid was already focused */ }
    FocusOutcome::Rejected(reason) => { /* panel missing or strategy rejected */ }
}
```

For callers that only need a boolean, `is_on_target()` collapses `Applied` and `Unchanged` into `true`:

```rust
// Before
if rt.focus(pid) { /* ... */ }

// After — same semantics
if rt.focus(pid).is_on_target() { /* ... */ }
```

`focus_direction` and `focus_direction_current` now return `(Option<PanelId>, FocusOutcome)` instead of `Option<PanelId>`:

```rust
// Before
if let Some(target) = rt.focus_direction_current(FocusDirection::Right)? {
    // focus moved to target
}

// After
let (target, outcome) = rt.focus_direction_current(FocusDirection::Right)?;
if outcome.is_applied() {
    // focus moved to target.unwrap()
}
```

## Swap API

`swap_next()` and `swap_prev()` now return `Result<(), PaneError>` instead of `()`. Strategies that don't support reordering (slotted) still no-op, but internal errors propagate:

```rust
// Before
rt.swap_next();

// After
rt.swap_next()?;
```

## Direction → Axis

The `Direction` enum is removed. Use `Axis` everywhere:

```rust
// Before
use panes::Direction;
let kind = StrategyKind::Sequence { direction: Direction::Horizontal, gap: 0.0 };

// After
use panes::Axis;
let kind = StrategyKind::Sequence { axis: Axis::Row, gap: 0.0 };
```

Mapping: `Direction::Horizontal` → `Axis::Row`, `Direction::Vertical` → `Axis::Col`.

This also affects `StrategyKind::Slotted { axis }` and snapshot `StrategyConfig` variants.

## Window panel_count rename

```rust
// Before
builder.set_window_size(3)?;
let n = layout.window_size();

// After
builder.set_window_panel_count(3)?;
let n = layout.window_panel_count();
```

## Snapshot format changes

`SnapshotNode::Row` and `SnapshotNode::Col` now carry `constraints: Option<Constraints>` for constrained container support. If you pattern-match on snapshot nodes:

```rust
// Before
SnapshotNode::Row { gap, children } => { /* ... */ }

// After
SnapshotNode::Row { gap, constraints, children } => { /* ... */ }
```

New variant `SnapshotNode::Grid` for CSS Grid topology. Add a match arm if your code exhaustively matches `SnapshotNode`.

Snapshots now include `focused_key: Option<PanelKey>` and `collapsed_keys: Box<[PanelKey]>` for deterministic restore with repeated kinds. These are `#[serde(default)]`, so old serialized snapshots deserialize without error — they just fall back to kind-based restore.

## Overlay anchor failures

Overlay anchors that fail to resolve (kind not found, kind ambiguous, stale panel key) are no longer silently dropped. They appear in `ResolvedLayout::overlay_failures()`:

```rust
for (id, kind, failure) in resolved.overlay_failures() {
    log::warn!("overlay {kind}: {failure:?}");
}
```

The overlay diff now tracks `anchor_failed` — overlays that were resolved last frame but failed this frame:

```rust
let diff = rt.last_overlay_diff();
for &oid in diff.anchor_failed.iter() {
    // overlay was visible, now broken
}
```

## CSS emission

`panes_css::emit_adaptive()` now validates breakpoint ordering and returns `Result`:

```rust
// Before
let css = panes_css::emit_adaptive(&breakpoints);

// After
let css = panes_css::emit_adaptive(&breakpoints)?;
```

## New capabilities (non-breaking)

These are additions that don't require migration but are worth knowing about:

- **Grid builder**: `Grid::columns(3).gap(8.0)`, `Grid::auto_fit(200.0)`, and grid syntax in the `layout!` macro
- **Container constraints**: `row_with(constraints, f)`, `col_with(constraints, f)` for weighted nested containers
- **O(1) panel queries**: `kind_of(pid)`, `kind_index_of_panel(pid)` on `ResolvedLayout`
- **Decoration panels**: `decoration_panels()`, `decoration_role(pid)`, `decoration_entries(kind)` for tab/title chrome
- **PanelKey**: Stable identity surviving tree rebuilds, for deterministic snapshot restore with repeated kinds
- **Key-based overlay anchoring**: `Overlay::above_key(key)` etc. for overlays on specific panel instances
- **AdapterFrame**: Shared shell for renderer backends — `overlay_failures()` available on all adapter frames
- **Boundary hit-test fast path**: `boundary_at_point_buf()` in panes-wasm for typed-array results
- **Dirty-state tracking**: Two-level invalidation (topology vs layout) for fewer cache rebuilds
