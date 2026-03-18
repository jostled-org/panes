# panes User Guide

## Table of Contents

- [Quick Start](#quick-start)
- [Core Concepts](#core-concepts)
- [Layout vs LayoutRuntime](#layout-vs-layoutruntime)
- [Constraints](#constraints)
- [Builder API](#builder-api)
- [Layout Macro](#layout-macro)
- [Presets](#presets)
- [TOML Configuration](#toml-configuration)
- [Runtime](#runtime)
- [Overlays](#overlays)
- [Frame Diffing](#frame-diffing)
- [Animation](#animation)
- [Render Adapters](#render-adapters)
- [Serialization](#serialization)
- [Snapshots](#snapshots)
- [Escape Hatch: Raw Taffy](#escape-hatch-raw-taffy)

---

## Quick Start

Pick a preset or build from scratch. Either way you get a bag of rectangles. For a working example, see the [demo app](https://github.com/jostled-org/p3-demo).

```rust
// Preset: one-liner for common patterns
use panes::Layout;

let resolved = Layout::master_stack(["editor", "chat", "status"])
    .master_ratio(0.6)
    .gap(1.0)
    .resolve(80.0, 24.0)?;

for (id, rect) in resolved.iter() {
    println!("{id}: {rect:?}");
}
```

```rust
// Custom: full control with the layout macro
use panes::{layout, grow, fixed};

let layout = layout! {
    col {
        panel("viewport", grow: 1.0)
        row(gap: 4.0) {
            panel("health", fixed: 40.0)
            panel("inventory", grow: 1.0)
            panel("minimap", fixed: 48.0)
        }
    }
}?;
let resolved = layout.resolve(800.0, 600.0)?;
```

Pass any units to `resolve()` — pixels, logical points, terminal cells. panes doesn't care.

---

## Core Concepts

**Panel** — A named region in the layout. Each panel has a string _kind_ (e.g. `"editor"`) and a set of constraints that govern its size. Panels are the leaves of the layout tree.

**Container** — A `row` or `col` that arranges its children along one axis. Rows are horizontal, columns are vertical. Containers can nest.

**Constraints** — Rules that determine a panel's size within its container: `grow`, `fixed`, `min`, `max`.

**Layout** — An immutable, validated tree of panels and containers. Call `.resolve(width, height)` to compute rectangles.

**ResolvedLayout** — The output: a map from `PanelId` to `Rect`. Look up panels by id or by kind.

**Rect** — `{ x, y, w, h }` in f32. Origin is top-left.

---

## Layout vs LayoutRuntime

Two ways to resolve a layout. Pick based on how often you resolve.

**`Layout::resolve()`** — Stateless. Compiles and resolves from scratch every call. No caching, no diffing. Use for one-shot rendering: CLI tools, static output, tests, or any case where you resolve once and discard.

```rust
let resolved = Layout::row(["a", "b", "c"])?.resolve(80.0, 24.0)?;
```

**`LayoutRuntime::resolve()`** — Stateful. Caches the compiled tree, reuses allocations across frames, and returns a diff against the previous frame. Use for render loops: TUI apps, game UIs, or anything that resolves repeatedly with mutations between frames.

```rust
let mut rt = Layout::master_stack(["editor", "chat"])
    .into_runtime()?;

// Each call reuses cached state and produces a diff
let frame = rt.resolve(80.0, 24.0)?;
let diff = frame.diff();  // what changed since last frame
```

Start with `Layout`. Switch to `LayoutRuntime` when you need per-frame updates, mutations, or diffing. The conversion is one line: `Layout::preset(...).into_runtime()`.

---

## Constraints

Two sizing modes. Use `fixed` when you know the size. Use `grow` when you want the panel to fill available space.

```rust
use panes::{grow, fixed};

grow(1.0)             // fill available space, weight 1
grow(2.0)             // fill with double weight
fixed(20.0)           // exactly 20 units

grow(1.0).min(10.0)   // grow, but at least 10
grow(1.0).max(100.0)  // grow, but at most 100
fixed(50.0).min(30.0) // fixed 50, floor at 30
```

`grow` and `fixed` are mutually exclusive. `min` and `max` can be added to either.

A bare panel in the macro (no constraint specified) defaults to `grow(1.0)`.

### When to use which

**`fixed`** — panels with a known size: toolbars, status bars, sidebars, health bars, tab headers. The panel gets exactly the size you specify, in whatever units you pass to `resolve()`.

**`grow`** — panels that fill remaining space after fixed panels are placed. The weight controls how leftover space is divided. In a row with `grow(2.0)` and `grow(1.0)`, the first panel gets 2/3 and the second gets 1/3 of the space not consumed by fixed panels or gaps.

### Mixing fixed and grow

Fixed panels are allocated first. Grow panels split what remains.

```rust
// 100px wide row: sidebar gets 30, editor gets 70
row {
    panel("sidebar", fixed: 30.0)
    panel("editor", grow: 1.0)
}
```

```rust
// 100px wide row with gap: sidebar=30, gap=4, editor=66
row(gap: 4.0) {
    panel("sidebar", fixed: 30.0)
    panel("editor", grow: 1.0)
}
```

```rust
// Three grow panels with weights: 50, 25, 25
row {
    panel("main", grow: 2.0)
    panel("left", grow: 1.0)
    panel("right", grow: 1.0)
}
```

---

## Builder API

For layouts that don't fit a preset, use `LayoutBuilder` directly. Think of it as nested flexbox: `row` lays out children horizontally, `col` lays out children vertically.

```rust
use panes::{LayoutBuilder, grow, fixed};

let mut b = LayoutBuilder::new();
b.row_gap(8.0, |r| {
    r.panel_with("editor", grow(2.0));
    r.col(|c| {
        c.panel("chat");
        c.panel_with("status", fixed(3.0));
    });
})?;
let layout = b.build()?;
let resolved = layout.resolve(80.0, 24.0)?;
```

Or use the convenience constructors on `Layout`:

```rust
use panes::{Layout, fixed};

let layout = Layout::build_row_gap(8.0, |r| {
    r.panel_with("editor", panes::grow(2.0));
    r.col(|c| {
        c.panel("chat");
        c.panel_with("status", fixed(3.0));
    });
})?;
```

Key rules:
- The root must be a single `row` or `col`
- Containers nest freely — rows inside columns, columns inside rows
- `row_gap(n, ...)` / `col_gap(n, ...)` sets spacing; bare `row(...)` / `col(...)` uses zero gap
- Bare `panel("kind")` defaults to `grow(1.0)`; use `panel_with("kind", constraints)` for explicit sizing
- Every `build()` validates the tree and returns `Result<Layout, PaneError>`

### Example: game HUD

A viewport that fills most of the screen, a fixed bottom bar with health, inventory, and minimap, and a fixed right column for a quest log:

```rust
//  ┌─────────────────────────┬──────────┐
//  │                         │          │
//  │       viewport          │  quest   │
//  │       (grow)            │  (fixed) │
//  │                         │          │
//  ├────────┬────────┬───────┤          │
//  │ health │ inven  │ mini  │          │
//  │ (fixed)│ (grow) │(fixed)│          │
//  └────────┴────────┴───────┴──────────┘

let mut b = LayoutBuilder::new();
b.row(|r| {
    r.col_gap(2.0, |c| {
        c.panel("viewport");
        c.row_gap(4.0, |bar| {
            bar.panel_with("health", fixed(120.0));
            bar.panel("inventory");
            bar.panel_with("minimap", fixed(120.0));
        });
    });
    r.panel_with("quest_log", fixed(200.0));
})?;
let resolved = b.build()?.resolve(1920.0, 1080.0)?;
```

The same layout works at any resolution — pass `800.0, 600.0` and the fixed panels keep their sizes while grow panels absorb the difference.

---

## Layout Macro

The `layout!` macro provides a declarative shorthand for the builder API.

```rust
use panes::layout;

let layout = layout! {
    row(gap: 8.0) {
        panel("editor", grow: 2.0, min: 40.0)
        col {
            panel("chat")
            panel("status", fixed: 3.0)
        }
    }
}?;
```

Syntax:
- Root is a single `row` or `col`, with optional `(gap: N)`
- `panel("kind")` — defaults to `grow(1.0)`
- `panel("kind", grow: N)` or `panel("kind", fixed: N)`
- Optional `min:` and `max:` after the primary constraint
- Nested `row { ... }` and `col { ... }` containers

The macro returns `Result<Layout, PaneError>`.

The same game HUD from the Builder API section:

```rust
let layout = layout! {
    row {
        col(gap: 2.0) {
            panel("viewport", grow: 1.0)
            row(gap: 4.0) {
                panel("health", fixed: 120.0)
                panel("inventory", grow: 1.0)
                panel("minimap", fixed: 120.0)
            }
        }
        panel("quest_log", fixed: 200.0)
    }
}?;
```

---

## Presets

Every preset follows the same pattern: construct via `Layout::preset_name(...)`, optionally chain configuration methods, then call `.build()` for a `Layout` or `.resolve(w, h)` directly.

### Preset Catalog

`Layout::presets()` returns metadata for all 15 built-in presets — name, input style, and description. Use it to build preset browsers, populate selection menus, or generate documentation without hard-coding the list.

```rust
use panes::{Layout, PanelInputKind};

for preset in Layout::presets() {
    println!("{}: {}", preset.name, preset.description);
}

// Filter by input style
let dynamic: Vec<_> = Layout::presets()
    .iter()
    .filter(|p| p.input == PanelInputKind::DynamicList)
    .collect();
```

Each `PresetInfo` has three fields:

- `name` — kebab-case identifier matching the TOML `strategy` field (e.g. `"master-stack"`)
- `input` — `DynamicList` (accepts N panels) or `FixedSlots` (accepts named slots)
- `description` — one-line summary

The three `FixedSlots` presets are `sidebar`, `holy-grail`, and `split`. All others accept a dynamic list.

### Simple Layouts

```rust
// Equal-grow panels in a row
Layout::row(["a", "b", "c"])?;

// Equal-grow panels in a column
Layout::col(["a", "b", "c"])?;
```

### Tiling Presets

#### master_stack

One primary pane on the left, remaining panes stacked vertically on the right.

```rust
Layout::master_stack(["editor", "chat", "status"])
    .master_ratio(0.6)  // default: 0.5
    .gap(1.0)            // default: 0.0
    .resolve(80.0, 24.0)?;
```

#### centered_master

Master pane in the center, remaining panes distributed alternately to left and right stacks.

```rust
Layout::centered_master(["editor", "a", "b", "c", "d"])
    .master_ratio(0.5)
    .gap(2.0)
    .resolve(120.0, 40.0)?;
```

#### dwindle

Recursive split alternating horizontal/vertical. Each new pane takes half the remaining space.

```rust
Layout::dwindle(["a", "b", "c", "d", "e"])
    .ratio(0.5)  // split ratio at each level
    .gap(1.0)
    .resolve(100.0, 100.0)?;
```

#### spiral

Like dwindle but reverses child order on even-depth levels, creating a spiral pattern.

```rust
Layout::spiral(["a", "b", "c", "d", "e"])
    .ratio(0.5)
    .gap(1.0)
    .resolve(100.0, 100.0)?;
```

#### columns

CSS Grid with equal columns. Panels fill left-to-right, top-to-bottom (row-major). Supports responsive reflow.

```rust
// Fixed 3 columns
Layout::columns(3, ["a", "b", "c", "d", "e", "f"])
    .gap(1.0)
    .resolve(90.0, 100.0)?;

// Responsive: columns reflow based on min width
Layout::columns(3, ["a", "b", "c", "d", "e", "f"])
    .auto_fill(200.0)  // repeat(auto-fill, minmax(200px, 1fr))
    .gap(1.0)
    .resolve(900.0, 600.0)?;
```

#### grid

CSS Grid with equal cells in an N-column arrangement. Panels fill left-to-right, top-to-bottom. Supports responsive reflow.

```rust
Layout::grid(3, ["a", "b", "c", "d", "e", "f"])
    .gap(1.0)
    .resolve(90.0, 100.0)?;

// Responsive grid
Layout::grid(3, ["a", "b", "c", "d"])
    .auto_fill(200.0)
    .gap(8.0)
    .resolve(800.0, 600.0)?;
```

### Stateful Presets

These presets have an `active` index controlling which panel is visible.

#### monocle

Single fullscreen pane. Other panels exist in the tree but have zero size.

```rust
Layout::monocle(["editor", "chat", "settings"])
    .active(0)  // default: 0
    .resolve(80.0, 24.0)?;
```

#### deck

Master pane visible on the left. The stack on the right shows only the active card.

```rust
Layout::deck(["master", "a", "b", "c"])
    .master_ratio(0.5)
    .active(0)  // index within the stack (not including master)
    .gap(1.0)
    .resolve(80.0, 24.0)?;
```

#### tabbed

Tab header bar over a single visible content pane. Each panel gets a `{kind}_tab` panel in the header.

```rust
Layout::tabbed(["editor", "chat", "terminal"])
    .active(0)
    .tab_height(1.0)  // default: 1.0
    .gap(0.0)
    .resolve(80.0, 24.0)?;

// Panels created: "editor_tab", "chat_tab", "terminal_tab" (in header row)
//                 "editor", "chat", "terminal" (content, only active visible)
```

#### stacked

Vertical list of title bars over a single visible content pane. Each panel gets a `{kind}_title` panel.

```rust
Layout::stacked(["editor", "chat", "terminal"])
    .active(0)
    .title_height(1.0)  // default: 1.0
    .gap(0.0)
    .resolve(80.0, 24.0)?;
```

### Application Layout Presets

#### split

Two panels, horizontal or vertical.

```rust
Layout::split("left", "right")
    .ratio(0.7)  // default: 0.5
    .gap(1.0)
    .resolve(100.0, 24.0)?;

Layout::split("top", "bottom")
    .vertical()
    .resolve(80.0, 24.0)?;
```

#### sidebar

Fixed-width sidebar with a growing content area.

```rust
Layout::sidebar("nav", "content")
    .sidebar_width(30.0)  // default: 20.0
    .gap(0.0)
    .resolve(100.0, 24.0)?;
```

#### holy_grail

Header, footer, left sidebar, main content, right sidebar.

```rust
Layout::holy_grail("header", "footer", "left", "main", "right")
    .header_height(3.0)   // default: 1.0
    .footer_height(2.0)   // default: 1.0
    .sidebar_width(15.0)  // default: 20.0
    .gap(1.0)
    .resolve(100.0, 100.0)?;
```

#### dashboard

Mixed-size cards in a CSS Grid. Each card has a column span.

```rust
Layout::dashboard([("metrics", 2), ("chart", 2), ("log", 1), ("alerts", 1)])
    .columns(4)  // default: 4
    .gap(2.0)
    .resolve(100.0, 100.0)?;
```

#### scrollable

Horizontal strip of fixed-width columns. Panels never shrink — the layout can exceed the viewport width.

```rust
Layout::scrollable(["project-a", "project-b", "project-c"])
    .col_width(80.0)  // default: 80.0
    .gap(1.0)
    .resolve(100.0, 24.0)?;
```

### Grid-Based vs Flexbox-Based Presets

Presets fall into two families based on their CSS layout model.

**Grid-based** (`dashboard`, `grid`, `columns`) use CSS Grid under the hood. They support responsive column reflow via `.auto_fill(min_width)` and `.auto_fit(min_width)`. Panels are placed in a flat grid and wrap to new rows as viewport width changes.

```rust
// Fixed 3-column grid
Layout::grid(3, ["a", "b", "c", "d", "e", "f"]).build()?;

// Responsive: columns reflow based on min width
Layout::grid(3, ["a", "b", "c", "d"])
    .auto_fill(200.0)  // repeat(auto-fill, minmax(200px, 1fr))
    .gap(8.0)
    .build()?;

// auto-fit expands items to fill remaining space
Layout::columns(3, ["a", "b", "c"])
    .auto_fit(200.0)   // repeat(auto-fit, minmax(200px, 1fr))
    .build()?;
```

**Flexbox-based** (`master_stack`, `centered_master`, `deck`, `dwindle`, `spiral`, `split`, `monocle`, `tabbed`, `stacked`, `scrollable`, `sidebar`, `holy_grail`) use nested flexbox containers. Their topology is structural — panels have specific spatial relationships (master/stack, recursive splits, fixed slots) that can't be expressed as a flat grid. These don't support column reflow.

**Direct tree mode** (via `LayoutBuilder` or `layout!` macro) builds topology with per-split flexbox nesting. Auto-fill/auto-fit doesn't apply.

For responsive layout across preset families, use `Layout::adaptive()` to switch between strategies at width breakpoints.

---

## TOML Configuration

Enable the `toml` feature:

```toml
[dependencies]
panes = { version = "0.1", features = ["toml"] }
```

Then load layouts from TOML strings or files:

```rust
let layout = Layout::from_toml(toml_str)?;
let layout = Layout::from_toml_file("layout.toml")?;
```

### Preset strategies

```toml
[layout]
strategy = "master-stack"
panels = ["editor", "chat", "status"]
master_ratio = 0.6
gap = 1.0
```

Every preset is available as a strategy name: `master-stack`, `centered-master`, `monocle`, `scrollable`, `dwindle`, `spiral`, `columns`, `grid`, `deck`, `tabbed`, `stacked`, `sidebar`, `split`, `holy-grail`, `dashboard`.

### Named-parameter strategies

```toml
[layout]
strategy = "sidebar"
sidebar = "nav"
content = "main"
sidebar_width = 30.0
gap = 0.0
```

```toml
[layout]
strategy = "split"
first = "left"
second = "right"
direction = "vertical"
ratio = 0.7
```

```toml
[layout]
strategy = "holy-grail"
header = "toolbar"
footer = "status"
left = "nav"
main = "content"
right = "inspector"
header_height = 3.0
footer_height = 2.0
sidebar_width = 15.0
```

### Responsive columns (grid, columns, dashboard)

Grid-based strategies accept `min_column_width` for responsive reflow. When set, panels wrap into fewer columns as the viewport narrows. Use `column_mode` to choose between `auto-fill` (default) and `auto-fit`.

`columns` and `min_column_width` are mutually exclusive.

```toml
# Grid with responsive auto-fill
[layout]
strategy = "grid"
min_column_width = 200.0
gap = 8.0
panels = ["a", "b", "c", "d"]

# Columns with auto-fit (items expand to fill remaining space)
[layout]
strategy = "columns"
min_column_width = 200.0
column_mode = "auto-fit"
panels = ["a", "b", "c"]
```

### Dashboard

```toml
[layout]
strategy = "dashboard"
columns = 4
gap = 2.0

[[layout.panels]]
kind = "metrics"
span = 2

[[layout.panels]]
kind = "chart"
span = 2

[[layout.panels]]
kind = "log"
span = 1
```

Dashboard also supports `min_column_width` and `column_mode`:

```toml
[layout]
strategy = "dashboard"
min_column_width = 250.0
column_mode = "auto-fill"
panels = ["chart", "stats", "logs"]
```

### Custom tree

For layouts that don't fit any preset, define the tree directly:

```toml
[layout]
strategy = "custom"

[layout.root]
type = "row"
gap = 8.0

[[layout.root.children]]
kind = "editor"
grow = 2.0
min = 40.0

[[layout.root.children]]
type = "col"

[[layout.root.children.children]]
kind = "chat"
grow = 1.0

[[layout.root.children.children]]
kind = "status"
fixed = 3.0
```

---

## Runtime

`LayoutRuntime` wraps a layout tree with viewport state, compile caching, and frame diffing. Two modes:

| Mode | Entry point | Panel mutations | Tree control |
|------|-------------|-----------------|--------------|
| **Strategy** | `preset.into_runtime()` | Strategy rebuilds the tree (add/remove/move) | Managed |
| **Direct** | `LayoutRuntime::new(tree)` | Hyprland-style splits by aspect ratio | Full (`tree_mut()`, `resize_boundary()`) |

Pick strategy mode when a preset fits your layout. Pick direct mode when you need custom tree topology or manual split control.

### Strategy-based runtime

Every preset has an `into_runtime()` method that creates a runtime with a mutation strategy. The strategy defines how panels are added, removed, moved, and focused.

```rust
let mut rt = Layout::master_stack(["editor", "chat", "status"])
    .master_ratio(0.6)
    .gap(1.0)
    .into_runtime()?;

// Resolve produces a Frame with layout + diff
let frame = rt.resolve(80.0, 24.0)?;
let resolved = frame.layout();
```

### Panel mutations

Add, remove, move, and focus panels at runtime. The strategy handles the spatial rules — where new panels go, how focus shifts, and how the tree rebuilds.

```rust
// Add a panel — inserts after focused, rebuilds via strategy
let pid = rt.add_panel("terminal".into())?;

// Explicit placement — before focused, after focused, or at end
use panes::Placement;
rt.add_panel_with("sidebar".into(), Placement::Before)?;
rt.add_panel_with("logs".into(), Placement::End)?;

// Remove the focused panel — focus shifts to neighbor
rt.remove_panel(pid)?;

// Move a panel to a new position in the sequence
rt.move_panel(pid, 0)?;

// Swap focused panel with neighbors — infallible, no-op when impossible
rt.swap_next();
rt.swap_prev();

// Sequential focus navigation
rt.focus_next();
rt.focus_prev();
rt.focus(pid);  // returns bool

// Spatial focus navigation — move to nearest panel in a direction.
// Returns Result<Option<PanelId>, PaneError>.
// Returns Err(SpatialNavUnsupported) for ActivePanel and Window strategies —
// use focus_next/focus_prev for Monocle, Tabbed, Stacked, and Scrollable presets.
use panes::FocusDirection;
rt.focus_direction_current(FocusDirection::Left)?;
rt.focus_direction_current(FocusDirection::Right)?;
rt.focus_direction_current(FocusDirection::Up)?;
rt.focus_direction_current(FocusDirection::Down)?;

// Or pass an explicit layout (e.g. during animation)
let frame = rt.resolve(80.0, 24.0)?;
rt.focus_direction(frame.layout(), FocusDirection::Right)?;

// Query focus state
let focused: Option<PanelId> = rt.focused();
let kind: Option<&str> = rt.focused_kind();
```

#### Without a strategy

`add_panel` and `add_panel_adjacent_with` also work on direct tree runtimes — see [Direct tree runtime](#direct-tree-runtime) for details. On a strategy runtime, `add_panel_adjacent_with` delegates to the strategy (direction and constraints are ignored).

### Viewport operations

```rust
// Collapse/uncollapse a panel (saves and restores constraints)
rt.toggle_collapsed(panel_id)?;

// Scroll (for scrollable layouts)
rt.scroll_by(10.0);
rt.scroll_to(0.0);
```

### Direct tree runtime

For layouts that don't fit a preset — tiling window managers, custom editor layouts, or anything that needs manual split control.

```rust
use panes::{Layout, grow, fixed, Direction, Placement};
use panes::runtime::LayoutRuntime;

// Start from any tree
let layout = Layout::build_row(|r| {
    r.panel("editor");
    r.panel("terminal");
})?;
let mut rt = LayoutRuntime::from(layout);
```

**Hyprland-style splits:** `add_panel` auto-picks the split direction from the focused panel's aspect ratio (wider panels split horizontally, taller panels split vertically).

```rust
rt.set_focus_unchecked(some_pid);
rt.add_panel("chat".into())?;
```

**Explicit splits:** Control direction, constraints, and placement.

```rust
rt.add_panel_adjacent_with(
    "sidebar".into(),
    Direction::Horizontal,
    fixed(30.0),
    Placement::Before,
)?;
```

**Direct tree surgery:** `tree_mut()` gives full access to the tree for operations that don't map to add/remove.

```rust
let tree = rt.tree_mut();
let (pid, _) = tree.add_panel("terminal", grow(1.0))?;
tree.set_constraints(pid, fixed(30.0))?;
```

**Resize boundaries:** Only available in direct mode. Adjusts a panel's share of its container.

```rust
rt.resize_boundary(pid, 0.1)?;   // give 10% more space
rt.resize_boundary(pid, -0.05)?; // give 5% less
```

Use `from_tree_and_sequence` when you need focus navigation ordering:

```rust
let mut rt = LayoutRuntime::from_tree_and_sequence(tree, sequence);
```

The runtime recompiles automatically when the tree is dirty.

---

## Overlays

Overlays are floating UI elements rendered above the base layout — command palettes, tooltips, dropdown menus, modal dialogs. They don't participate in the flexbox tree; instead they're positioned relative to the viewport or a specific panel.

### Adding overlays

```rust
use panes::Overlay;

// Centered modal
let id = rt.add_overlay("picker", Overlay::center().fixed(400.0, 300.0))?;

// Bottom-anchored command palette (10px margin, full width, 50px tall)
rt.add_overlay("palette", Overlay::bottom(10.0).full_width().height(50.0))?;

// Tooltip above a specific panel
rt.add_overlay("tooltip", Overlay::above("editor").fixed(200.0, 30.0))?;
```

`add_overlay` returns the existing `OverlayId` if the kind already exists. Anchors include `center()`, `top(margin)`, `bottom(margin)`, `left(margin)`, `right(margin)`, `above(panel_kind)`, and `below(panel_kind)`.

### Controlling overlays

```rust
// Show/hide without removing
rt.set_overlay_visible("palette", false);
rt.set_overlay_visible("palette", true);

// Resize dynamically
rt.set_overlay_height("palette", 80.0)?;
rt.set_overlay_width("palette", 400.0)?;

// Remove entirely
rt.remove_overlay("palette");

// Look up definition
if let Some(def) = rt.overlay("palette") {
    println!("visible: {}", def.visible());
}
```

### Reading overlay rects

After `resolve()`, overlay rects appear in the `ResolvedLayout`:

```rust
let frame = rt.resolve(800.0, 600.0)?;

// Iterate resolved overlays in z-order
for entry in frame.layout().overlays() {
    println!("{}: {:?}", entry.kind, entry.rect);
}

// Look up by id
if let Some(rect) = frame.layout().overlay_rect(id) {
    draw_overlay(rect);
}
```

Adapter crates provide `overlays()` iterators that convert to renderer-native rects, just like `panels()`.

Hidden overlays and panel-anchored overlays referencing missing panels are excluded from the resolved output.

---

## Frame Diffing

Every `runtime.resolve()` call computes a diff against the previous frame. Access it via `last_diff()` and `last_overlay_diff()`:

```rust
let _frame = rt.resolve(80.0, 24.0)?;
let diff = rt.last_diff();

// Panels that appeared this frame
for &pid in diff.added.iter() { /* ... */ }

// Panels that disappeared
for &pid in diff.removed.iter() { /* ... */ }

// Panels that moved (position changed)
for change in diff.moved.iter() {
    println!("{}: {:?} -> {:?}", change.id, change.from, change.to);
}

// Panels that resized
for change in diff.resized.iter() { /* ... */ }

// Panels with no change
for &pid in diff.unchanged.iter() { /* ... */ }
```

Overlay diffing works identically:

```rust
let overlay_diff = rt.last_overlay_diff();
for &oid in overlay_diff.added.iter() { /* new overlay appeared */ }
for &oid in overlay_diff.removed.iter() { /* overlay disappeared */ }
for change in overlay_diff.moved.iter() { /* overlay repositioned */ }
for change in overlay_diff.resized.iter() { /* overlay size changed */ }
```

The first frame reports all panels and overlays as `added`. Diffs borrow from internal scratch buffers and are valid until the next `resolve()` call.

---

## Animation

`ResolvedLayout` supports linear interpolation for smooth transitions between layout states:

```rust
let from = layout_a.resolve(80.0, 24.0)?;
let to = layout_b.resolve(80.0, 24.0)?;

// t ranges from 0.0 (from) to 1.0 (to)
let mid = from.lerp(&to, 0.5);
```

With the runtime, `Frame` is `Clone` (cheap — it's an `Arc` internally). Store the previous frame to lerp from:

```rust
let prev_frame = rt.resolve(800.0, 600.0)?;

// ... mutation happens ...
let next_frame = rt.resolve(800.0, 600.0)?;

// Lerp between the two layouts
let mid = prev_frame.layout().lerp(next_frame.layout(), t);
```

For shared ownership, `frame.arc()` returns the underlying `Arc<ResolvedLayout>`:

```rust
let layout_arc = frame.arc(); // cheap clone of the Arc
```

`Rect` also has a standalone `lerp`:

```rust
let interpolated = rect_a.lerp(rect_b, t);
```

---

## Render Adapters

panes computes abstract `Rect { x, y, w, h }` values. Adapter crates convert these to renderer-native types.

Each adapter provides three APIs:
- `convert()` — returns a `FxHashMap<PanelId, TargetRect>` for random access
- `panels()` — returns a lazy iterator of `PanelEntry { id, kind, rect, kind_index }` for rendering loops
- `overlays()` — returns a lazy iterator of `OverlayEntry { id, kind, rect }` for overlay rendering

### PanelEntry

`PanelEntry` carries four fields:

- `id` — the `PanelId` for this panel
- `kind` — the kind string (e.g. `"editor"`)
- `rect` — the computed rectangle in the target coordinate system
- `kind_index` — zero-based index of this panel's kind group in iteration order

`kind_index` lets you apply per-kind styling without reconstructing group boundaries. All panels sharing a kind have the same `kind_index`; distinct kinds get distinct indices.

```rust
for entry in resolved.panels() {
    let color = PALETTE[entry.kind_index % PALETTE.len()];
    draw(entry.rect, color);
}
```

Use `map_rect` to transform the rectangle type while preserving identity fields:

```rust
let entries: Vec<_> = resolved
    .panels()
    .map(|e| e.map_rect(|r| my_rect_from(r)))
    .collect();
```

### panes-ratatui

```toml
[dependencies]
panes-ratatui = "0.1"
```

```rust
let resolved = layout.resolve(80.0, 24.0)?;

// Iterator — preferred for render loops
for entry in panes_ratatui::panels(&resolved) {
    println!("{}: {} at {:?}", entry.id, entry.kind, entry.rect);
}

// Hashmap — for random access by PanelId
let rects: FxHashMap<PanelId, ratatui::layout::Rect> = panes_ratatui::convert(&resolved);
```

Uses edge-rounding quantization: adjacent panels sharing a float boundary produce matching integer edges. No gaps, no overlaps.

`panels_at()` and `convert_at()` offset rects by a parent origin, for rendering a panes layout inside another panel.

`focused_panels()` pairs each entry with a focus `bool`. A panel is focused when its id matches the given `PanelId`, or when its kind is a decoration (`_tab` / `_title` suffix) of the focused panel's kind. Pass `runtime.focused()` or any `Option<PanelId>`:

```rust
for (entry, is_focused) in panes_ratatui::focused_panels(&resolved, rt.focused()) {
    let style = if is_focused { focused_style } else { normal_style };
    frame.render_widget(styled_block(entry.kind, style), entry.rect);
}
```

`focused_panels_at()` combines focus logic with origin offset.

### panes-egui

```toml
[dependencies]
panes-egui = "0.1"
```

```rust
let resolved = layout.resolve(width, height)?;

for entry in panes_egui::panels(&resolved) {
    ui.put(entry.rect, egui::Label::new(entry.kind));
}

// Or hashmap access
let rects: FxHashMap<PanelId, egui::Rect> = panes_egui::convert(&resolved);
```

Direct f32 mapping.

### panes-css

```toml
[dependencies]
panes-css = "0.1"
```

```rust
let css: String = panes_css::emit(&layout);
```

Transpiles the layout tree into CSS flexbox/grid declarations. The browser acts as the solver — Taffy is not invoked. Panels use `[data-pane="kind"]` selectors, containers use `[data-pane-node="N"]`, and the root uses `[data-pane-root]`.

Grid-based presets (`dashboard`, `grid`, `columns`) emit `display: grid` with `grid-template-columns`. When using `auto_fill` or `auto_fit`, the output uses `repeat(auto-fill, minmax(...))` or `repeat(auto-fit, minmax(...))` for responsive reflow. Flexbox-based presets emit `display: flex` with `flex-direction`.

### panes-wasm

```toml
[dependencies]
panes-wasm = "0.1"
```

```rust
let resolved = layout.resolve(width, height)?;

for entry in panes_wasm::panels(&resolved) {
    // entry.rect is WasmRect with f64 fields for JavaScript interop
}

let rects: FxHashMap<PanelId, WasmRect> = panes_wasm::convert(&resolved);
```

---

## Serialization

Enable the `serde` feature to derive `Serialize` and `Deserialize` on core types:

```toml
[dependencies]
panes = { version = "0.1", features = ["serde"] }
```

Types with serde derives: `Rect`, `PanelId`, `NodeId`, `Constraints`, `Direction`, `ActivePanelVariant`, all snapshot types (`LayoutSnapshot`, `SnapshotSource`, `StrategyConfig`, `SnapshotNode`, `SnapshotSlotDef`, `SnapshotOverlay`), and overlay types (`OverlayAnchor`, `OverlayExtent`, `ExtentValue`, `HAlign`, `VAlign`).

This lets resolved layouts flow directly into JSON responses, IPC messages, or config files without manual conversion:

```rust
let resolved = layout.resolve(800.0, 600.0)?;
for entry in resolved.panels() {
    // entry.rect, entry.id are now serializable
    let json = serde_json::to_string(&entry.rect)?;
}
```

The `toml` feature implies `serde` — if you're already using TOML configuration, the derives are active.

For JavaScript/browser interop, use `panes-wasm` instead — its `WasmRect` uses f64 to match the browser's number model.

---

## Snapshots

`LayoutSnapshot` captures a runtime's state for session persistence. Snapshot, serialize with any serde format, restore next session.

```rust
use panes::Layout;
use panes::runtime::LayoutRuntime;

let mut rt = Layout::master_stack(["editor", "chat", "status"])
    .master_ratio(0.6).gap(1.0).into_runtime()?;
rt.focus_next(); // focus "chat"

// Capture
let snapshot = rt.snapshot()?;
let json = serde_json::to_string(&snapshot)?;

// Restore (next session)
let snapshot: panes::LayoutSnapshot = serde_json::from_str(&json)?;
let mut rt = LayoutRuntime::from_snapshot(snapshot)?;
// rt has the same strategy, panel order, and focus
```

### What gets persisted

**Strategy runtimes** snapshot the recipe — strategy config + panel kinds in sequence order. On restore, the tree is rebuilt through the preset builder, so decorative panels (tabs, titles) are recreated automatically.

**Non-strategy runtimes** snapshot the tree topology — a recursive structure of rows, columns, and panels with their constraints. On restore, the tree is rebuilt via the layout builder.

Both variants also persist:
- **Focused panel** (by kind, not PanelId — IDs are regenerated on rebuild)
- **Collapsed panels** (by kind — collapse state is re-applied after rebuild)
- **Overlays** (anchor, extents, visibility)

### What doesn't persist

- **Resize deltas** — `resize_boundary` modifies tree constraints directly. To persist resize state, track the deltas in your app and replay them after restore.
- **Scroll offset** — transient viewport state.
- **Cached frames** — recomputed on the next `resolve()` call.

### Format freedom

`LayoutSnapshot` implements `Serialize`/`Deserialize` behind the `serde` feature gate. Pick whatever format fits your stack — JSON, TOML, MessagePack, bincode:

```rust
// JSON for config files
let json = serde_json::to_string_pretty(&snapshot)?;

// bincode for IPC
let bytes = bincode::serialize(&snapshot)?;
```

---

## Escape Hatch: Raw Taffy

When panes' spatial vocabulary doesn't cover your use case, drop down to raw Taffy styles via `taffy_node`:

```rust
let custom_style = taffy::Style {
    display: taffy::Display::Grid,
    grid_template_columns: vec![taffy::prelude::fr(1.0); 3],
    ..Default::default()
};

let mut b = LayoutBuilder::new();
b.row(|r| {
    r.taffy_node(custom_style, |grid| {
        grid.panel("a");
        grid.panel("b");
        grid.panel("c");
    });
})?;
```

Raw Taffy nodes compose freely with panes nodes in the same tree. Use this when panes adds no value over the underlying CSS property.

---

## Querying Results

```rust
let resolved = layout.resolve(80.0, 24.0)?;

// Iterate with identity — id, kind, rect, and kind group index
for entry in resolved.panels() {
    println!("{}: {} [group {}] at {:?}", entry.id, entry.kind, entry.kind_index, entry.rect);
}

// By PanelId
let rect: Option<&Rect> = resolved.get(panel_id);

// By kind — returns all PanelIds with that kind
let editors: &[PanelId] = resolved.by_kind("editor");

// Iterate all (PanelId, Rect) pairs
for (pid, rect) in resolved.iter() {
    println!("{pid}: x={} y={} w={} h={}", rect.x, rect.y, rect.w, rect.h);
}

// Geometric queries on Rect
let area = rect.area();
let (cx, cy) = rect.center();
let hit = rect.contains(mouse_x, mouse_y);
let overlap = rect_a.intersects(rect_b);
```
