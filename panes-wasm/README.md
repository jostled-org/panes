[![crates.io](https://img.shields.io/crates/v/panes-wasm.svg)](https://crates.io/crates/panes-wasm)
[![docs.rs](https://docs.rs/panes-wasm/badge.svg)](https://docs.rs/panes-wasm)
[![license](https://img.shields.io/crates/l/panes-wasm.svg)](https://github.com/jostled-org/panes/blob/main/LICENSE-MIT)

You're rendering to a canvas or passing layout data to JavaScript. You need f64 coordinates, not f32, and optionally `wasm-bindgen` interop.

**panes-wasm** converts [`panes`](https://crates.io/crates/panes) layouts into `WasmRect` values with f64 fields for JavaScript consumption.

## Install

```
cargo add panes panes-wasm
```

Enable `wasm-bindgen` interop:

```
cargo add panes-wasm --features js
```

## Usage

```rust
use panes::Layout;

let resolved = Layout::grid(3, ["a", "b", "c", "d", "e", "f"])
    .resolve(800.0, 600.0)?;

let rects = panes_wasm::convert(&resolved);

for (pid, rect) in &rects {
    // rect is WasmRect { x: f64, y: f64, w: f64, h: f64 }
    ctx.fill_rect(rect.x, rect.y, rect.w, rect.h);
}
```

With the `js` feature enabled, `WasmRect` derives `wasm_bindgen` for direct JS access.

## License

MIT or Apache 2.0, at your option. See [LICENSE-MIT](../LICENSE-MIT) and [LICENSE-APACHE](../LICENSE-APACHE).
