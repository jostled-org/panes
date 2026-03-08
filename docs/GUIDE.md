# panes User Guide

## Table of Contents

- [Quick Start](#quick-start)
- [Core Concepts](#core-concepts)
- [Layout vs LayoutRuntime](#layout-vs-layoutruntime)
- [Constraints](#constraints)
- [Presets](#presets)
- [Builder API](#builder-api)
- [Layout Macro](#layout-macro)
- [TOML Configuration](#toml-configuration)
- [Runtime](#runtime)
- [Frame Diffing](#frame-diffing)
- [Animation](#animation)
- [Render Adapters](#render-adapters)
- [Escape Hatch: Raw Taffy](#escape-hatch-raw-taffy)

---

## Quick Start

```rust
use panes::{Layout, Rect};

let resolved = Layout::master_stack(["editor", "chat", "status"])
    .master_ratio(0.6)
    .gap(1.0)
    .resolve(80.0, 24.0)?;

for (id, rect) in resolved.iter() {
    println!("{id}: {rect:?}");
}
```

panes computes rectangles. You render them however you want.

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

Two primary sizing modes, plus optional bounds.

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

Equal-width vertical columns. Panels are distributed round-robin across columns.

```rust
// 6 panels into 3 columns: col0=[a,d], col1=[b,e], col2=[c,f]
Layout::columns(3, ["a", "b", "c", "d", "e", "f"])
    .gap(1.0)
    .resolve(90.0, 100.0)?;
```

#### grid

Equal-sized cells in an N-column arrangement. Panels fill left-to-right, top-to-bottom.

```rust
Layout::grid(3, ["a", "b", "c", "d", "e", "f"])
    .gap(1.0)
    .resolve(90.0, 100.0)?;
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

### Simple Layouts

```rust
// Equal-grow panels in a row
Layout::row(["a", "b", "c"])?;

// Equal-grow panels in a column
Layout::col(["a", "b", "c"])?;
```

---

## Builder API

For layouts that don't fit a preset, use `LayoutBuilder` directly.

```rust
use panes::{LayoutBuilder, gap, grow, fixed};

let mut b = LayoutBuilder::new();
b.row(gap(8.0), |r| {
    r.panel("editor", grow(2.0))?;
    r.col(gap(0.0), |c| {
        c.panel("chat", grow(1.0))?;
        c.panel("status", fixed(3.0))?;
        Ok(())
    })
})?;
let layout = b.build()?;
let resolved = layout.resolve(80.0, 24.0)?;
```

Key rules:
- The root must be a single `row` or `col`
- Containers nest freely — rows inside columns, columns inside rows
- `gap(n)` sets spacing between children of a container
- Every `build()` validates the tree and returns `Result<Layout, PaneError>`

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

`LayoutRuntime` wraps a layout tree with viewport state, compile caching, and frame diffing.

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

Add, remove, move, and focus panels at runtime. The strategy handles the spatial rules — where new panels go, how focus shifts, whether the tree rebuilds or just adjusts constraints.

```rust
// Add a panel — focus shifts to the new panel
let pid = rt.add_panel("terminal".into())?;

// Remove the focused panel — focus shifts to neighbor
rt.remove_panel(pid)?;

// Move a panel to a new position in the sequence
rt.move_panel(pid, 0)?;

// Focus navigation
rt.focus_next()?;
rt.focus_prev()?;
rt.focus(pid)?;

// Query focus state
let focused: Option<PanelId> = rt.focused();
let kind: Option<&str> = rt.focused_kind();
```

### Viewport operations

```rust
// Collapse/uncollapse a panel (saves and restores constraints)
rt.toggle_collapsed(panel_id)?;

// Scroll (for scrollable layouts)
rt.scroll_by(10.0);
rt.scroll_to(0.0);
```

### Legacy runtime

For layouts built with the builder API or macro, wrap them in a runtime without a strategy. Mutations go through `tree_mut()` directly.

```rust
let layout = Layout::master_stack(["editor", "chat"]).build()?;
let mut rt = LayoutRuntime::from(layout);

let tree = rt.tree_mut();
let (pid, _) = tree.add_panel("terminal", grow(1.0))?;
tree.set_constraints(pid, fixed(30.0))?;
tree.remove_panel(pid)?;
```

The runtime recompiles automatically when the tree is dirty.

---

## Frame Diffing

Every `runtime.resolve()` call returns a `Frame` containing a `LayoutDiff`:

```rust
let frame = rt.resolve(80.0, 24.0)?;
let diff = frame.diff();

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

The first frame reports all panels as `added`.

---

## Animation

`ResolvedLayout` supports linear interpolation for smooth transitions between layout states:

```rust
let from = layout_a.resolve(80.0, 24.0)?;
let to = layout_b.resolve(80.0, 24.0)?;

// t ranges from 0.0 (from) to 1.0 (to)
let mid = from.lerp(&to, 0.5);
```

`Rect` also has a standalone `lerp`:

```rust
let interpolated = rect_a.lerp(rect_b, t);
```

---

## Render Adapters

panes computes abstract `Rect { x, y, w, h }` values. Adapter crates convert these to renderer-native types.

Each adapter provides two APIs:
- `convert()` — returns a `FxHashMap<PanelId, TargetRect>` for random access
- `panels()` — returns a lazy iterator of `PanelEntry { id, kind, rect, kind_index }` for rendering loops

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

`PanelEntry` is `#[non_exhaustive]`, so it cannot be constructed outside the crate. Use `map_rect` to transform the rectangle type while preserving identity fields:

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

## Escape Hatch: Raw Taffy

When panes' spatial vocabulary doesn't cover your use case, drop down to raw Taffy styles via `taffy_node`:

```rust
let custom_style = taffy::Style {
    display: taffy::Display::Grid,
    grid_template_columns: vec![taffy::prelude::fr(1.0); 3],
    ..Default::default()
};

let mut b = LayoutBuilder::new();
b.row(gap(0.0), |r| {
    r.taffy_node(custom_style, |grid| {
        grid.panel("a", grow(1.0))?;
        grid.panel("b", grow(1.0))?;
        grid.panel("c", grow(1.0))?;
        Ok(())
    })
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
