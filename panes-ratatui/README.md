[![crates.io](https://img.shields.io/crates/v/panes-ratatui.svg)](https://crates.io/crates/panes-ratatui)
[![docs.rs](https://docs.rs/panes-ratatui/badge.svg)](https://docs.rs/panes-ratatui)
[![license](https://img.shields.io/crates/l/panes-ratatui.svg)](https://github.com/jostled-org/panes/blob/main/LICENSE-MIT)

You computed layout rectangles with float precision. Now you need them as `ratatui::layout::Rect` without gaps between adjacent panels.

**panes-ratatui** converts [`panes`](https://crates.io/crates/panes) layouts into ratatui rects with pixel-perfect edge rounding.

## Install

```
cargo add panes panes-ratatui
```

## Usage

```rust
use panes::Layout;

let resolved = Layout::master_stack(["editor", "chat", "status"])
    .resolve(80.0, 24.0)?;

let rects = panes_ratatui::convert(&resolved);

for (pid, rect) in &rects {
    // rect is ratatui::layout::Rect { x, y, width, height } — all u16
    frame.render_widget(my_widget, *rect);
}
```

Adjacent panels sharing a float boundary produce matching integer edges. No gaps, no overlaps.

## License

MIT or Apache 2.0, at your option. See [LICENSE-MIT](../LICENSE-MIT) and [LICENSE-APACHE](../LICENSE-APACHE).
