[![crates.io](https://img.shields.io/crates/v/panes.svg)](https://crates.io/crates/panes)
[![docs.rs](https://docs.rs/panes/badge.svg)](https://docs.rs/panes)
[![CI](https://github.com/jostled-org/panes/actions/workflows/ci.yml/badge.svg)](https://github.com/jostled-org/panes/actions)
[![license](https://img.shields.io/crates/l/panes.svg)](https://github.com/jostled-org/panes/blob/main/LICENSE-MIT)

Every layout engine you've found is welded to a renderer. You need the math without the framework.

**panes** is a spatial layout engine that computes rectangles without rendering them.

Describe panels in rows, columns, and presets. panes solves the geometry via Taffy's flexbox engine and hands back a map of `PanelId → Rect`. No framework. No widget system. No opinions about pixels.

## Proof

```rust
use panes::{Layout, Rect};

let resolved = Layout::master_stack(["editor", "chat", "status"])
    .master_ratio(0.6)
    .gap(1.0)
    .resolve(80.0, 24.0)?;

for (id, rect) in resolved.iter() {
    println!("{id}: {rect:?}");
}
// PanelId(0): Rect { x: 0.0, y: 0.0, w: 47.5, h: 24.0 }
// PanelId(1): Rect { x: 48.5, y: 0.0, w: 31.5, h: 11.5 }
// PanelId(2): Rect { x: 48.5, y: 12.5, w: 31.5, h: 11.5 }
```

## Install

```
cargo add panes
```

## Usage

Pick a preset, build custom, or mutate at runtime.

```rust
// Preset — 15 built-in strategies
Layout::master_stack(["editor", "chat", "status"]).master_ratio(0.6).gap(1.0)
```

```rust
// Builder — nest rows, columns, constraints
let mut b = LayoutBuilder::new();
b.row(gap(8.0), |r| {
    r.panel("editor", grow(2.0))?;
    r.col(gap(0.0), |c| {
        c.panel("chat", grow(1.0))?;
        c.panel("status", fixed(3.0))?;
        Ok(())
    })
})?;
```

```rust
// Runtime — mutations, viewport, frame diffing
let mut rt = LayoutRuntime::from(layout);
rt.tree_mut().add_panel("terminal", grow(1.0))?;
let frame = rt.resolve(80.0, 24.0)?;
let diff = frame.diff();
```

Adapters convert rects to renderer-native types:

| Crate | Target | Rect type |
|---|---|---|
| [`panes-ratatui`](https://crates.io/crates/panes-ratatui) | ratatui | `ratatui::layout::Rect` (u16, edge-rounded) |
| [`panes-egui`](https://crates.io/crates/panes-egui) | egui | `egui::Rect` (f32) |
| [`panes-css`](https://crates.io/crates/panes-css) | Browser | CSS declarations (browser solves layout) |
| [`panes-wasm`](https://crates.io/crates/panes-wasm) | Canvas/JS | `WasmRect` (f64) |

## Documentation

See the [User Guide](docs/GUIDE.md) for the full API: all 15 presets, the layout macro, TOML configuration, runtime mutations, frame diffing, animation, and render adapters.

## License

MIT or Apache 2.0, at your option. See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE).
