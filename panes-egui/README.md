You want layout math separate from your egui widget code. Every frame you're nesting `ui.horizontal()` and `ui.vertical()` to approximate what a layout engine should compute once.

**panes-egui** converts [`panes`](https://crates.io/crates/panes) layouts into `egui::Rect` values for direct use in your render pass.

[![crates.io](https://img.shields.io/crates/v/panes-egui.svg)](https://crates.io/crates/panes-egui)
[![docs.rs](https://docs.rs/panes-egui/badge.svg)](https://docs.rs/panes-egui)
[![license](https://img.shields.io/crates/l/panes-egui.svg)](https://github.com/jostled-org/panes/blob/main/LICENSE-MIT)

## Install

```
cargo add panes panes-egui
```

## Usage

```rust
use panes::Layout;

let resolved = Layout::sidebar("nav", "content")
    .sidebar_width(200.0)
    .resolve(width, height)?;

let rects = panes_egui::convert(&resolved);

for (pid, rect) in &rects {
    // rect is egui::Rect — f32, direct mapping
    let panel_ui = ui.child_ui(*rect, egui::Layout::default(), None);
}
```

## License

MIT or Apache 2.0, at your option. See [LICENSE-MIT](../LICENSE-MIT) and [LICENSE-APACHE](../LICENSE-APACHE).
