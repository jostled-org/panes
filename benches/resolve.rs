#![allow(clippy::unwrap_used, clippy::panic)]
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use panes::runtime::LayoutRuntime;
use panes::{LayoutTree, PanelId, fixed, grow};

fn build_flat_row(n: usize) -> LayoutTree {
    let mut tree = LayoutTree::new();
    let nids: Vec<_> = (0..n)
        .map(|i| {
            let kind: &str = match i % 3 {
                0 => "editor",
                1 => "sidebar",
                _ => "status",
            };
            tree.add_panel(kind, grow(1.0))
                .map(|(_, nid)| nid)
                .unwrap_or_else(|e| panic!("build_flat_row add_panel failed: {e}"))
        })
        .collect();
    let root = tree
        .add_row(0.0, nids)
        .unwrap_or_else(|e| panic!("build_flat_row add_row failed: {e}"));
    tree.set_root(root);
    tree
}

fn build_nested_tree(depth: usize, leaves_per: usize) -> LayoutTree {
    let mut tree = LayoutTree::new();
    let root = build_nested_level(&mut tree, depth, leaves_per, true);
    tree.set_root(root);
    tree
}

fn add_leaf(tree: &mut LayoutTree) -> panes::NodeId {
    tree.add_panel("leaf", grow(1.0))
        .map(|(_, nid)| nid)
        .unwrap_or_else(|e| panic!("add_panel failed: {e}"))
}

fn add_container(
    tree: &mut LayoutTree,
    is_row: bool,
    children: Vec<panes::NodeId>,
) -> panes::NodeId {
    match is_row {
        true => tree
            .add_row(2.0, children)
            .unwrap_or_else(|e| panic!("add_row failed: {e}")),
        false => tree
            .add_col(2.0, children)
            .unwrap_or_else(|e| panic!("add_col failed: {e}")),
    }
}

fn build_nested_level(
    tree: &mut LayoutTree,
    depth: usize,
    leaves_per: usize,
    is_row: bool,
) -> panes::NodeId {
    match depth {
        0 => add_leaf(tree),
        _ => {
            let children: Vec<_> = (0..leaves_per)
                .map(|_| build_nested_level(tree, depth - 1, leaves_per, !is_row))
                .collect();
            add_container(tree, is_row, children)
        }
    }
}

fn resolve_or_panic(rt: &mut LayoutRuntime, w: f32, h: f32) -> panes::runtime::Frame {
    rt.resolve(w, h)
        .unwrap_or_else(|e| panic!("resolve failed: {e}"))
}

fn bench_static_frame(c: &mut Criterion) {
    let mut group = c.benchmark_group("static_frame");
    for n in [5, 50, 500] {
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            let tree = build_flat_row(n);
            let mut rt = LayoutRuntime::from(tree);
            resolve_or_panic(&mut rt, 1920.0, 1080.0);
            b.iter(|| resolve_or_panic(&mut rt, 1920.0, 1080.0));
        });
    }
    group.finish();
}

fn next_width(w: f32) -> f32 {
    match w > 1900.0 {
        true => 800.0,
        false => w + 1.0,
    }
}

fn bench_resize(c: &mut Criterion) {
    let mut group = c.benchmark_group("resize");
    for n in [5, 50, 500] {
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            let tree = build_flat_row(n);
            let mut rt = LayoutRuntime::from(tree);
            resolve_or_panic(&mut rt, 1920.0, 1080.0);
            let mut width = 800.0_f32;
            b.iter(|| {
                width = next_width(width);
                resolve_or_panic(&mut rt, width, 1080.0)
            });
        });
    }
    group.finish();
}

fn toggle_constraint(toggle: bool) -> panes::Constraints {
    match toggle {
        true => fixed(200.0),
        false => grow(1.0),
    }
}

fn bench_mutate_resolve(c: &mut Criterion) {
    let mut group = c.benchmark_group("mutate_resolve");
    for n in [5, 50, 500] {
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            let tree = build_flat_row(n);
            let mut rt = LayoutRuntime::from(tree);
            resolve_or_panic(&mut rt, 1920.0, 1080.0);
            let pid = PanelId::from_raw(0);
            let mut toggle = false;
            b.iter(|| {
                toggle = !toggle;
                rt.tree_mut()
                    .set_constraints(pid, toggle_constraint(toggle))
                    .unwrap_or_else(|e| panic!("set_constraints failed: {e}"));
                resolve_or_panic(&mut rt, 1920.0, 1080.0)
            });
        });
    }
    group.finish();
}

fn bench_nested_resolve(c: &mut Criterion) {
    let mut group = c.benchmark_group("nested_static");
    for depth in [3, 4, 5] {
        let n_panels = 3_usize.pow(depth);
        group.bench_with_input(
            BenchmarkId::new("depth", format!("{depth}_panels_{n_panels}")),
            &depth,
            |b, &depth| {
                let tree = build_nested_tree(depth as usize, 3);
                let mut rt = LayoutRuntime::from(tree);
                resolve_or_panic(&mut rt, 1920.0, 1080.0);
                b.iter(|| resolve_or_panic(&mut rt, 1920.0, 1080.0));
            },
        );
    }
    group.finish();
}

fn bench_lerp(c: &mut Criterion) {
    let mut group = c.benchmark_group("lerp");
    for n in [5, 50, 500] {
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            let tree_a = build_flat_row(n);
            let layout_a = tree_a
                .resolve(1920.0, 1080.0)
                .unwrap_or_else(|e| panic!("resolve_a failed: {e}"));

            let tree_b = build_flat_row(n);
            let layout_b = tree_b
                .resolve(800.0, 600.0)
                .unwrap_or_else(|e| panic!("resolve_b failed: {e}"));

            b.iter(|| layout_a.lerp(&layout_b, 0.5));
        });
    }
    group.finish();
}

fn bench_hit_testing(c: &mut Criterion) {
    let mut group = c.benchmark_group("hit_testing");
    for n in [5, 50, 500] {
        let tree = build_flat_row(n);
        let mut rt = LayoutRuntime::from(tree);
        let frame = resolve_or_panic(&mut rt, 1920.0, 1080.0);
        let layout = frame.layout();

        // First panel center = worst case for reverse iteration (checked last)
        let first_panel_center_x = 1920.0 / (2 * n) as f32;
        let mid_y = 540.0;
        let first_boundary_x = 1920.0 / n as f32;

        group.bench_with_input(BenchmarkId::new("panel_at_point", n), &n, |b, _| {
            b.iter(|| {
                layout.panel_at_point(
                    std::hint::black_box(first_panel_center_x),
                    std::hint::black_box(mid_y),
                )
            });
        });

        group.bench_with_input(BenchmarkId::new("boundary_at_point", n), &n, |b, _| {
            b.iter(|| {
                layout.boundary_at_point(
                    std::hint::black_box(first_boundary_x),
                    std::hint::black_box(mid_y),
                    std::hint::black_box(4.0),
                )
            });
        });
    }
    group.finish();
}

fn apply_sizing_toggle(rt: &mut LayoutRuntime, pid: PanelId, set: bool) {
    match set {
        true => rt
            .set_panel_size(pid, 300.0, 200.0)
            .unwrap_or_else(|e| panic!("set_panel_size failed: {e}")),
        false => rt
            .clear_panel_size(pid)
            .unwrap_or_else(|e| panic!("clear_panel_size failed: {e}")),
    }
}

fn bench_sizing_resolve(c: &mut Criterion) {
    let mut group = c.benchmark_group("sizing_resolve");
    for n in [5, 50, 500] {
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            let tree = build_flat_row(n);
            let mut rt = LayoutRuntime::from(tree);
            resolve_or_panic(&mut rt, 1920.0, 1080.0);
            let pid = PanelId::from_raw(0);
            let mut toggle = false;
            b.iter(|| {
                toggle = !toggle;
                apply_sizing_toggle(&mut rt, pid, toggle);
                resolve_or_panic(&mut rt, 1920.0, 1080.0)
            });
        });
    }
    group.finish();
}

fn bench_diff_cost(c: &mut Criterion) {
    let mut group = c.benchmark_group("diff_cost");
    for n in [5, 50, 500] {
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            let tree = build_flat_row(n);
            let mut rt = LayoutRuntime::from(tree);
            resolve_or_panic(&mut rt, 1920.0, 1080.0);
            let pid = PanelId::from_raw(0);
            let mut toggle = false;
            b.iter(|| {
                toggle = !toggle;
                rt.tree_mut()
                    .set_constraints(pid, toggle_constraint(toggle))
                    .unwrap_or_else(|e| panic!("set_constraints failed: {e}"));
                resolve_or_panic(&mut rt, 1920.0, 1080.0)
            });
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_static_frame,
    bench_resize,
    bench_mutate_resolve,
    bench_nested_resolve,
    bench_lerp,
    bench_hit_testing,
    bench_sizing_resolve,
    bench_diff_cost,
);
criterion_main!(benches);
