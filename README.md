[![crates.io](https://img.shields.io/crates/v/panes.svg)](https://crates.io/crates/panes)
[![docs.rs](https://docs.rs/panes/badge.svg)](https://docs.rs/panes)
[![CI](https://github.com/jostled-org/panes/actions/workflows/ci.yml/badge.svg)](https://github.com/jostled-org/panes/actions)
[![license](https://img.shields.io/crates/l/panes.svg)](https://github.com/jostled-org/panes/blob/main/LICENSE-MIT)

You keep solving the same panel layout problem — splits, stacks, grids, resize — from scratch in every project.

**panes** is a spatial layout engine that computes rectangles without rendering them.

Describe panels in rows, columns, and presets. panes solves the geometry via Taffy's flexbox engine and hands back a map of `PanelId → Rect`. No framework. No widget system. No opinions about pixels.

## Proof

```rust
use panes::{layout, grow, fixed};

// Game HUD: health bar pinned at 40px, viewport fills the rest
let layout = layout! {
    col {
        row {
            panel("viewport", grow: 1.0)
        }
        row(gap: 4.0) {
            panel("health", fixed: 40.0)
            panel("inventory", grow: 1.0)
            panel("minimap", fixed: 48.0)
        }
    }
}?;

let resolved = layout.resolve(800.0, 600.0)?;
for entry in resolved.panels() {
    println!("{}: {:?}", entry.kind, entry.rect);
}
```

## Install

```
cargo add panes
```

## Usage

Build custom layouts or pick from 15 presets. Pass any coordinate system — pixels, logical points, terminal cells.

```rust
// Custom — full control with the layout macro
let layout = layout! {
    row(gap: 8.0) {
        panel("editor", grow: 2.0)
        col {
            panel("chat")
            panel("status", fixed: 3.0)
        }
    }
}?;
```

```rust
// Preset — one-liner for common patterns
Layout::master_stack(["editor", "chat", "status"]).master_ratio(0.6).gap(1.0)
```

```rust
// Runtime — add/remove panels, focus navigation, frame diffing
let mut rt = Layout::master_stack(["editor", "chat", "status"])
    .master_ratio(0.6).gap(1.0).into_runtime()?;

rt.add_panel("terminal".into())?;
rt.focus_next()?;

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

Each adapter provides a `panels()` iterator that yields `PanelEntry { id, kind, rect, kind_index }` — no hashmap, no cross-referencing:

```rust
for entry in panes_ratatui::panels(&resolved) {
    println!("{}: {} at {:?}", entry.id, entry.kind, entry.rect);
}
```

## Performance

`LayoutRuntime` resolves a 100-panel flat layout in ~1µs on the hot path — roughly 3x over raw Taffy. The abstraction layer (tree compilation, rect resolution, frame diffing) adds minimal overhead.

See [`benches/`](benches/) for methodology.

## Documentation

See the [User Guide](docs/GUIDE.md) for the full API: all 15 presets, the layout macro, TOML configuration, runtime mutations, frame diffing, animation, and render adapters.

## License

MIT or Apache 2.0, at your option. See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE).
