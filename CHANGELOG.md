# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.18.3](https://github.com/jostled-org/panes/compare/panes-v0.18.2...panes-v0.18.3) - 2026-03-27

### Added

- *(panes-egui)* add EguiFrame resolve wrapper for DX consistency

### Other

- add .mcp.json to gitignore

## [0.18.2](https://github.com/jostled-org/panes/compare/panes-v0.18.1...panes-v0.18.2) - 2026-03-26

### Fixed

- *(panes-ratatui)* fix resolve_layout lifetime, update docs for TerminalFrame

## [0.18.1](https://github.com/jostled-org/panes/compare/panes-v0.18.0...panes-v0.18.1) - 2026-03-26

### Added

- *(panes-ratatui)* add TerminalFrame resolve-and-convert pipeline

## [0.18.0](https://github.com/jostled-org/panes/compare/panes-v0.17.2...panes-v0.18.0) - 2026-03-25

### Other

- strip redundant docs, merge DFS, reduce duplication

## [0.17.2](https://github.com/jostled-org/panes/compare/panes-v0.17.1...panes-v0.17.2) - 2026-03-23

### Other

- cache kind sort in resolver, replace serde Value with Serialize derives

## [0.17.1](https://github.com/jostled-org/panes/compare/panes-v0.17.0...panes-v0.17.1) - 2026-03-23

### Other

- add hit-testing, sizing, diff cost, and CSS emit benchmarks
- audit fixes — boundary DFS, deterministic ordering, builder macros

## [0.17.0](https://github.com/jostled-org/panes/compare/panes-v0.16.0...panes-v0.17.0) - 2026-03-22

### Added

- *(panes-wasm)* WasmRuntime/WasmLayout with diffs, scroll, hit-testing
- [**breaking**] content-sizing keywords, overlay/scroll/transition CSS emission

## [0.16.0](https://github.com/jostled-org/panes/compare/panes-v0.15.0...panes-v0.16.0) - 2026-03-22

### Added

- [**breaking**] content-driven sizing and hit-testing primitives

## [0.15.0](https://github.com/jostled-org/panes/compare/panes-v0.14.0...panes-v0.15.0) - 2026-03-21

### Fixed

- *(ci)* remove stale pedant config path

### Other

- [**breaking**] collapse dashboard variants, remove deprecated APIs, double-buffer resolve

## [0.14.0](https://github.com/jostled-org/panes/compare/panes-v0.13.1...panes-v0.14.0) - 2026-03-20

### Added

- cross-container resize with runtime and strategy refactoring

## [0.13.1](https://github.com/jostled-org/panes/compare/panes-v0.13.0...panes-v0.13.1) - 2026-03-19

### Other

- update for CardSpan::FullWidth, deprecated grid/columns, render_overlays

## [0.13.0](https://github.com/jostled-org/panes/compare/panes-v0.12.0...panes-v0.13.0) - 2026-03-18

### Added

- [**breaking**] add CardSpan::FullWidth, consolidate grid strategies into Dashboard, optimize hot paths

## [0.12.0](https://github.com/jostled-org/panes/compare/panes-v0.11.2...panes-v0.12.0) - 2026-03-18

### Added

- [**breaking**] complete responsive layout implementation

## [0.11.2](https://github.com/jostled-org/panes/compare/panes-v0.11.1...panes-v0.11.2) - 2026-03-14

### Other

- link live wasm demo in README

## [0.11.1](https://github.com/jostled-org/panes/compare/panes-v0.11.0...panes-v0.11.1) - 2026-03-13

### Other

- skip CI on docs-only changes and link demo app

## [0.11.0](https://github.com/jostled-org/panes/compare/panes-v0.10.1...panes-v0.11.0) - 2026-03-13

### Fixed

- *(focus)* [**breaking**] prefer cross-axis overlap in spatial navigation and error on unsupported strategies

## [0.10.1](https://github.com/jostled-org/panes/compare/panes-v0.10.0...panes-v0.10.1) - 2026-03-11

### Added

- *(runtime)* make Frame cloneable and expose arc() accessor

## [0.10.0](https://github.com/jostled-org/panes/compare/panes-v0.9.0...panes-v0.10.0) - 2026-03-11

### Added

- [**breaking**] add overlay system, frame diffing, and audit remediation
- [**breaking**] add overlay system, frame diffing, and audit remediation
- [**breaking**] add overlay system, frame diffing, and audit remediation

## [0.9.0](https://github.com/jostled-org/panes/compare/panes-v0.8.2...panes-v0.9.0) - 2026-03-10

### Added

- [**breaking**] unify add_panel API with strategy-aware rebuild and add snapshots

## [0.8.2](https://github.com/jostled-org/panes/compare/panes-v0.8.1...panes-v0.8.2) - 2026-03-09

### Added

- *(runtime)* add swap_next and swap_prev methods

## [0.8.1](https://github.com/jostled-org/panes/compare/panes-v0.8.0...panes-v0.8.1) - 2026-03-09

### Other

- split strategy.rs into module directory and extract resize helpers

## [0.8.0](https://github.com/jostled-org/panes/compare/panes-v0.7.0...panes-v0.8.0) - 2026-03-09

### Other

- [**breaking**] replace stringly-typed errors with structured enums and audit remediation

## [0.7.0](https://github.com/jostled-org/panes/compare/panes-v0.6.0...panes-v0.7.0) - 2026-03-08

### Other

- *(runtime)* [**breaking**] make focus methods infallible and simplify from_tree_and_strategy

## [0.6.0](https://github.com/jostled-org/panes/compare/panes-v0.5.2...panes-v0.6.0) - 2026-03-08

### Added

- [**breaking**] add strategy-independent panel splitting with auto direction

## [0.5.2](https://github.com/jostled-org/panes/compare/panes-v0.5.1...panes-v0.5.2) - 2026-03-08

### Added

- add directional focus navigation

## [0.5.1](https://github.com/jostled-org/panes/compare/panes-v0.5.0...panes-v0.5.1) - 2026-03-08

### Added

- *(ratatui)* add focus-enriched panel iterators

## [0.5.0](https://github.com/jostled-org/panes/compare/panes-v0.4.0...panes-v0.5.0) - 2026-03-08

### Other

- [**breaking**] ergonomic builder API with infallible closures and method splits

## [0.4.0](https://github.com/jostled-org/panes/compare/panes-v0.3.0...panes-v0.4.0) - 2026-03-08

### Added

- [**breaking**] add preset catalog and kind-group index to PanelEntry

## [0.3.0](https://github.com/jostled-org/panes/compare/panes-v0.2.0...panes-v0.3.0) - 2026-03-08

### Other

- [**breaking**] replace FxHashMap with Vec-indexed storage for rects and node_map

## [0.2.0](https://github.com/jostled-org/panes/compare/panes-v0.1.2...panes-v0.2.0) - 2026-03-08

### Added

- add strategy-based runtime mutations and unified panel iteration

## [0.1.2](https://github.com/jostled-org/panes/compare/panes-v0.1.1...panes-v0.1.2) - 2026-03-07

### Other

- add doc comments to all public items and reorganize READMEs

## [0.1.1](https://github.com/jostled-org/panes/compare/panes-v0.1.0...panes-v0.1.1) - 2026-03-07

### Other

- cargo fmt
