You defined a layout in Rust and now you need the same arrangement in the browser. Translating row/column/grow/fixed into `display: flex; flex-direction: row; flex-grow: 2; flex-basis: 0px` by hand is error-prone and tedious.

**panes-css** transpiles [`panes`](https://crates.io/crates/panes) layout trees into CSS flexbox and grid declarations. The browser becomes the solver — Taffy is not invoked.

[![crates.io](https://img.shields.io/crates/v/panes-css.svg)](https://crates.io/crates/panes-css)
[![docs.rs](https://docs.rs/panes-css/badge.svg)](https://docs.rs/panes-css)
[![license](https://img.shields.io/crates/l/panes-css.svg)](https://github.com/jostled-org/panes/blob/main/LICENSE-MIT)

## Install

```
cargo add panes panes-css
```

## Usage

```rust
use panes::{Layout, layout, grow, fixed};

let layout = layout! {
    row(gap: 8.0) {
        panel("editor", grow: 2.0)
        panel("sidebar", fixed: 300.0)
    }
}?;

let css = panes_css::emit(&layout);
```

Output:

```css
[data-pane-root] { display: flex; flex-direction: row; gap: 8px; }
[data-pane="editor"] { flex-grow: 2; flex-basis: 0px; flex-shrink: 1; }
[data-pane="sidebar"] { flex-grow: 0; flex-basis: 300px; flex-shrink: 0; }
```

Panels use `[data-pane="kind"]` selectors. Containers use `[data-pane-node="N"]`. The root uses `[data-pane-root]`. Dashboard layouts emit CSS Grid.

## License

MIT or Apache 2.0, at your option. See [LICENSE-MIT](../LICENSE-MIT) and [LICENSE-APACHE](../LICENSE-APACHE).
