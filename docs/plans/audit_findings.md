# Audit Findings — Post-Overlay Remediation

## Phase 1: Safety & Correctness ✅

All items addressed.

### 1.1 Discarded `move_panel` error in `swap_by` ✅
- **Resolution**: Plan D1 deliberately kept `swap_next`/`swap_prev` infallible. Comment documents invariant: `move_panel` can only fail on no strategy (impossible), OOB index (impossible via `rem_euclid`), or empty kinds (impossible, `len > 1` checked). Not a bug — documented design choice.

### 1.2 `set_window_size` accepts 0 ✅
- Fixed: validates `size > 0`, returns `Result<(), PaneError>`

### 1.3 `size - 1` underflow when `size == 0` ✅
- Fixed: `debug_assert!(size > 0)` guard added. Unreachable through public API after 1.2 fix.

### 1.4 NaN/infinity in `scroll_by`/`scroll_to` ✅
- Fixed: `check_f32_finite` validation, returns `Result`

### 1.5 NaN/infinity in overlay builder params ✅
- Fixed: `Overlay::validate()` called at `add_overlay` time

### 1.6 `insert_child_at` / `PanelSequence::insert` panic on OOB ✅
- Fixed: bounds check returns `Result` / clamps with `debug_assert`

### 1.7 `tree_to_snapshot` failure silently produces empty tree ✅
- Fixed: `snapshot()` returns `Result<LayoutSnapshot, PaneError>`, propagates `SnapshotNoRoot`

### 1.8 `let _ = apply_window_constraints(...)` in best-effort helper ✅
- **Resolution**: Plan D1 kept `let _ =`. Function renamed to `apply_window_constraints_best_effort` with doc explaining: errors from missing/corrupted panels are ignored because focus has already been set and partial constraint application is preferable to propagating mid-focus. Not a bug — documented design choice.

### 1.9 `if/else` in `focus_deck_full` ✅
- Fixed: changed to `match spid == pid { true => ..., false => ... }`

### 1.10 `from_snapshot` propagate `toggle_collapsed` ✅
- Verified: uses `?` propagation

---

## Phase 2: Per-Frame Allocation ⚠️

6 of 7 implementation steps completed. Step 7 skipped — plan was incorrect.

### 2.1 `diff_same_panels` allocates 3 Vecs per frame ✅
- Fixed: `DiffScratch` expanded with `moved`, `resized`, `unchanged` Vecs. `diff_same_panels_reuse` clears and reuses.

### 2.2 `overlay_rects_buf` not reclaimed after frame ✅
- Fixed: `take_overlay_rects()` added, reclaimed via `Arc::try_unwrap` pattern

### 2.3 Double `Arc::clone` of overlay kind strings per frame ⚠️ OPEN
- Step 7 (swap `prev_overlay_rects`) was skipped. The plan's swap approach put stale overlay data into the layout returned to callers — 11 overlay tests failed. The layout is wrapped in `Arc` and consumed by users; its overlay rects must remain valid. Correct fix requires shared ownership between layout and `prev_overlay_rects`, not a swap.

### 2.4 `lerp_into` clones overlay_rects unnecessarily ✅
- Fixed: returns empty `Vec::new()` — interpolation doesn't affect overlays

---

## Phase 3: Structure & DRY ✅

All items addressed.

### 3.1 `strategy/mod.rs` contains type definitions ✅
- Fixed: types moved to `src/strategy/types.rs`, mod.rs is re-exports only

### 3.2 `overlay.rs` — 550 lines, multiple responsibilities ✅
- Fixed: split into `src/overlay/{mod,types,builder,resolve}.rs`

### 3.3 `add_panel_adjacent_no_strategy` — 71 lines, multiple jobs ✅
- Fixed: extracted `find_focused_position` free function + `parent_axis_direction` helper

### 3.4 `set_active` vs `focus` naming confusion ✅
- Fixed: renamed to `set_focus_unchecked`, removed duplicate `active_panel` (use `focused()`)

### 3.5 `StrategyConfig` ↔ `StrategyKind` boilerplate ✅
- Fixed: `strategy_convert!` macro generates both `From` impls

---

## Phase 4: Lower Priority ⚠️

### 4.1 `LayoutDiff`/`OverlayDiff` own `Box<[T]>` — allocation every frame ✅
- Fixed: `LayoutDiff<'a>` and `OverlayDiff<'a>` now borrow from scratch buffers. `DiffScratch` owns all 7 Vecs. `OverlayDiffScratch` added. `Frame` no longer carries diff data — new `last_diff()`/`last_overlay_diff()` methods on `LayoutRuntime`.

### 4.2 `TaffyPassthrough` nodes dropped from snapshots ✅
- **Resolution**: documented on `snapshot()` method. TaffyPassthrough is an internal escape-hatch with no serializable representation. Returns `SnapshotNoRoot` if at root.

### 4.3 `compile()` allocates fresh `TaffyTree` per dirty frame — ACCEPTED
- Inherent to Taffy API. Only runs on dirty frames. Not a per-frame cost.

### 4.4 `compile_children` allocates `Vec<NodeId>` per container — ACCEPTED
- Required by `TaffyTree::new_with_children`. Only runs on dirty frames.

### 4.5 `format!("{kind}_tab")` / `_title` per panel per rebuild — ACCEPTED
- Runs on mutation, not per-frame. Low frequency, low impact.

### 4.6 `ResolvedLayout` derives `Clone` ✅
- Fixed: `Clone` derive removed. All consumers use `Arc<ResolvedLayout>`.

---

## New Findings (Post-Phase 4 Audit)

### N1 `set_overlay_height`/`set_overlay_width` accept unvalidated `f32`
- `src/runtime.rs:650-661` — NaN injection bypasses `add_overlay` validation
- Severity: medium

### N2 `tree_mut()` exposes unchecked mutation
- `src/runtime.rs:157-159` — no invariant protection for sequence, viewport, or cached state
- Severity: medium (API design concern)

### N3 Snapshot types use `Vec<T>` for immutable data
- `src/snapshot.rs:35,37,71,136,155,189,196` — six fields constructed once, never mutated. Should be `Box<[T]>` per CLAUDE.md.
- Severity: medium (CLAUDE.md conformance)

### N4 `row_gap`/`col_gap`/`taffy_node` triplicate in builder
- `src/builder.rs:174-237` — three copies of same 15-line error/validate/dispatch skeleton
- Severity: high (DRY violation)

### N5 14 preset files duplicate `into_runtime` boilerplate
- `src/preset/*.rs` — identical shape not covered by `impl_preset!` macro
- Severity: medium (DRY)

### N6 Dirty-tree path bypasses scratch buffers
- `src/runtime.rs:725` — `resolver::resolve()` ignores `resolve_scratch` and `rects_buf`, uses recursive DFS
- Severity: low (dirty frames only)

### N7 `LayoutRuntime` is 912 lines
- Overlay management methods could extract to a separate module
- Severity: medium (structure)

---

## Verification

```
cargo test --workspace
cargo test --workspace --features serde
cargo clippy --workspace --features serde
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --features serde
```
