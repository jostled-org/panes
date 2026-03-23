//! Comparative benchmark: panes vs ratatui::Layout vs taffy (direct).
//!
//! Each scenario builds an equivalent layout and measures resolve time.

use criterion::{criterion_group, criterion_main};

// ---------------------------------------------------------------------------
// Scenario 1: Flat row of N equal-weight panels
// ---------------------------------------------------------------------------

mod flat_row {
    use criterion::{BenchmarkId, Criterion};

    pub fn panes(c: &mut Criterion) {
        let mut group = c.benchmark_group("flat_row/panes");
        for n in [5, 20, 100] {
            group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
                let panels: Vec<String> = (0..n).map(|i| format!("p{i}")).collect();
                let layout = panes::Layout::row(panels.iter().map(String::as_str)).unwrap();
                b.iter(|| layout.resolve(1920.0, 1080.0).unwrap());
            });
        }
        group.finish();
    }

    pub fn panes_runtime(c: &mut Criterion) {
        use panes::runtime::LayoutRuntime;

        let mut group = c.benchmark_group("flat_row/panes_rt");
        for n in [5, 20, 100] {
            group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
                let panels: Vec<String> = (0..n).map(|i| format!("p{i}")).collect();
                let layout = panes::Layout::row(panels.iter().map(String::as_str)).unwrap();
                let mut rt = LayoutRuntime::from(layout);
                rt.set_collect_boundaries(false);
                rt.resolve(1920.0, 1080.0).unwrap();
                b.iter(|| rt.resolve(1920.0, 1080.0).unwrap());
            });
        }
        group.finish();
    }

    pub fn ratatui(c: &mut Criterion) {
        use ratatui::layout::{Constraint, Direction, Layout, Rect};

        let mut group = c.benchmark_group("flat_row/ratatui");
        // ratatui's cassowary solver panics at 100 constraints (ObjectiveUnbounded)
        for n in [5, 20] {
            group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
                let constraints: Vec<Constraint> = (0..n).map(|_| Constraint::Fill(1)).collect();
                let layout = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(constraints);
                let area = Rect::new(0, 0, 1920, 1080);
                b.iter(|| layout.split(area));
            });
        }
        group.finish();
    }

    pub fn taffy(c: &mut Criterion) {
        use taffy::prelude::*;

        let mut group = c.benchmark_group("flat_row/taffy");
        for n in [5, 20, 100] {
            group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
                let mut tree = TaffyTree::<()>::new();
                let children: Vec<_> = (0..n).map(|_| taffy_grow_leaf(&mut tree)).collect();
                let root = tree
                    .new_with_children(
                        Style {
                            display: Display::Flex,
                            flex_direction: FlexDirection::Row,
                            size: Size {
                                width: length(1920.0),
                                height: length(1080.0),
                            },
                            ..Default::default()
                        },
                        &children,
                    )
                    .unwrap();
                tree.compute_layout(root, Size::MAX_CONTENT).unwrap();
                b.iter(|| tree.compute_layout(root, Size::MAX_CONTENT).unwrap());
            });
        }
        group.finish();
    }

    fn taffy_grow_leaf(tree: &mut taffy::TaffyTree<()>) -> taffy::NodeId {
        tree.new_leaf(taffy::Style {
            flex_grow: 1.0,
            ..Default::default()
        })
        .unwrap()
    }
}

// ---------------------------------------------------------------------------
// Scenario 2: Nested 2-level layout (outer cols, inner rows)
//   e.g., 3 columns each containing N/3 rows
// ---------------------------------------------------------------------------

mod nested {
    use criterion::{BenchmarkId, Criterion};

    pub fn panes(c: &mut Criterion) {
        let mut group = c.benchmark_group("nested/panes");
        for n in [9, 27, 81] {
            group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
                let tree = build_panes_nested(n);
                b.iter(|| tree.resolve(1920.0, 1080.0).unwrap());
            });
        }
        group.finish();
    }

    pub fn panes_runtime(c: &mut Criterion) {
        let mut group = c.benchmark_group("nested/panes_rt");
        for n in [9, 27, 81] {
            group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
                let tree = build_panes_nested(n);
                let mut rt = panes::runtime::LayoutRuntime::from(tree);
                rt.resolve(1920.0, 1080.0).unwrap();
                b.iter(|| rt.resolve(1920.0, 1080.0).unwrap());
            });
        }
        group.finish();
    }

    fn build_panes_nested(n: usize) -> panes::LayoutTree {
        use panes::{LayoutTree, grow};

        let cols = 3;
        let rows_per = n / cols;
        let mut tree = LayoutTree::new();
        let col_nodes: Vec<_> = (0..cols)
            .map(|c_idx| {
                let row_nodes: Vec<_> = (0..rows_per)
                    .map(|r_idx| {
                        let kind = format!("p{}", c_idx * rows_per + r_idx);
                        let (_, nid) = tree.add_panel(&*kind, grow(1.0)).unwrap();
                        nid
                    })
                    .collect();
                tree.add_col(0.0, row_nodes).unwrap()
            })
            .collect();
        let root = tree.add_row(0.0, col_nodes).unwrap();
        tree.set_root(root);
        tree
    }

    pub fn ratatui(c: &mut Criterion) {
        use ratatui::layout::{Constraint, Direction, Layout, Rect};

        let mut group = c.benchmark_group("nested/ratatui");
        // 81 panels = 27 per column; cassowary may struggle at that scale
        for n in [9, 27] {
            group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
                let cols = 3;
                let rows_per = n / cols;
                let outer = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(vec![Constraint::Fill(1); cols]);
                let inner = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(vec![Constraint::Fill(1); rows_per]);
                let area = Rect::new(0, 0, 1920, 1080);
                b.iter(|| resolve_nested_ratatui(&outer, &inner, area));
            });
        }
        group.finish();
    }

    fn resolve_nested_ratatui(
        outer: &ratatui::layout::Layout,
        inner: &ratatui::layout::Layout,
        area: ratatui::layout::Rect,
    ) {
        let col_rects = outer.split(area);
        for col_rect in col_rects.iter() {
            std::hint::black_box(inner.split(*col_rect));
        }
    }

    pub fn taffy(c: &mut Criterion) {
        use taffy::prelude::*;

        let mut group = c.benchmark_group("nested/taffy");
        for n in [9, 27, 81] {
            group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
                let (mut tree, root) = build_taffy_nested(n);
                tree.compute_layout(root, Size::MAX_CONTENT).unwrap();
                b.iter(|| tree.compute_layout(root, Size::MAX_CONTENT).unwrap());
            });
        }
        group.finish();

        fn build_taffy_nested(n: usize) -> (TaffyTree<()>, NodeId) {
            let cols = 3;
            let rows_per = n / cols;
            let mut tree = TaffyTree::<()>::new();
            let col_nodes: Vec<_> = (0..cols)
                .map(|_| build_taffy_col(&mut tree, rows_per))
                .collect();
            let root = tree
                .new_with_children(
                    Style {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        size: Size {
                            width: length(1920.0),
                            height: length(1080.0),
                        },
                        ..Default::default()
                    },
                    &col_nodes,
                )
                .unwrap();
            (tree, root)
        }

        fn build_taffy_col(tree: &mut TaffyTree<()>, rows: usize) -> NodeId {
            let row_nodes: Vec<_> = (0..rows)
                .map(|_| {
                    tree.new_leaf(Style {
                        flex_grow: 1.0,
                        ..Default::default()
                    })
                    .unwrap()
                })
                .collect();
            tree.new_with_children(
                Style {
                    flex_grow: 1.0,
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    ..Default::default()
                },
                &row_nodes,
            )
            .unwrap()
        }
    }
}

// ---------------------------------------------------------------------------
// Scenario 3: Holy-grail layout (header, footer, 3-col body)
// ---------------------------------------------------------------------------

mod holy_grail {
    use criterion::Criterion;

    pub fn panes(c: &mut Criterion) {
        c.bench_function("holy_grail/panes", |b| {
            let layout = panes::Layout::holy_grail("header", "footer", "left", "main", "right")
                .header_height(3.0)
                .footer_height(3.0)
                .sidebar_width(15.0)
                .build()
                .unwrap();
            b.iter(|| layout.resolve(1920.0, 1080.0).unwrap());
        });
    }

    pub fn panes_runtime(c: &mut Criterion) {
        c.bench_function("holy_grail/panes_rt", |b| {
            let layout = panes::Layout::holy_grail("header", "footer", "left", "main", "right")
                .header_height(3.0)
                .footer_height(3.0)
                .sidebar_width(15.0)
                .into_runtime()
                .unwrap();
            let mut rt = layout;
            rt.resolve(1920.0, 1080.0).unwrap();
            b.iter(|| rt.resolve(1920.0, 1080.0).unwrap());
        });
    }

    pub fn ratatui(c: &mut Criterion) {
        use ratatui::layout::{Constraint, Direction, Layout, Rect};

        c.bench_function("holy_grail/ratatui", |b| {
            let outer = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(0),
                    Constraint::Length(3),
                ]);
            let inner = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(15),
                    Constraint::Fill(1),
                    Constraint::Length(15),
                ]);
            let area = Rect::new(0, 0, 1920, 1080);
            b.iter(|| {
                let rows = outer.split(area);
                std::hint::black_box(&rows[0]);
                std::hint::black_box(inner.split(rows[1]));
                std::hint::black_box(&rows[2]);
            });
        });
    }

    pub fn taffy(c: &mut Criterion) {
        use taffy::prelude::TaffyMaxContent;

        c.bench_function("holy_grail/taffy", |b| {
            let (mut tree, root) = build_taffy_holy_grail();
            tree.compute_layout(root, taffy::prelude::Size::MAX_CONTENT)
                .unwrap();
            b.iter(|| {
                tree.compute_layout(root, taffy::prelude::Size::MAX_CONTENT)
                    .unwrap()
            });
        });
    }

    fn build_taffy_holy_grail() -> (taffy::TaffyTree<()>, taffy::NodeId) {
        use taffy::prelude::*;

        let mut tree = TaffyTree::<()>::new();
        let header = tree
            .new_leaf(Style {
                size: Size {
                    width: auto(),
                    height: length(3.0),
                },
                ..Default::default()
            })
            .unwrap();
        let left = tree
            .new_leaf(Style {
                size: Size {
                    width: length(15.0),
                    height: auto(),
                },
                ..Default::default()
            })
            .unwrap();
        let main = tree
            .new_leaf(Style {
                flex_grow: 1.0,
                ..Default::default()
            })
            .unwrap();
        let right = tree
            .new_leaf(Style {
                size: Size {
                    width: length(15.0),
                    height: auto(),
                },
                ..Default::default()
            })
            .unwrap();
        let body = tree
            .new_with_children(
                Style {
                    flex_grow: 1.0,
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    ..Default::default()
                },
                &[left, main, right],
            )
            .unwrap();
        let footer = tree
            .new_leaf(Style {
                size: Size {
                    width: auto(),
                    height: length(3.0),
                },
                ..Default::default()
            })
            .unwrap();
        let root = tree
            .new_with_children(
                Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    size: Size {
                        width: length(1920.0),
                        height: length(1080.0),
                    },
                    ..Default::default()
                },
                &[header, body, footer],
            )
            .unwrap();
        (tree, root)
    }
}

criterion_group!(
    benches,
    flat_row::panes,
    flat_row::panes_runtime,
    flat_row::ratatui,
    flat_row::taffy,
    nested::panes,
    nested::panes_runtime,
    nested::ratatui,
    nested::taffy,
    holy_grail::panes,
    holy_grail::panes_runtime,
    holy_grail::ratatui,
    holy_grail::taffy,
);
criterion_main!(benches);
