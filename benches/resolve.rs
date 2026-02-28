use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use panes::runtime::LayoutRuntime;
use panes::{LayoutTree, fixed, grow};

fn build_flat_row(n: usize) -> LayoutTree {
    let mut tree = LayoutTree::new();
    let nids: Vec<_> = (0..n)
        .map(|i| {
            let kind: &str = match i % 3 {
                0 => "editor",
                1 => "sidebar",
                _ => "status",
            };
            let (_, nid) = tree.add_panel(kind, grow(1.0)).unwrap();
            nid
        })
        .collect();
    let root = tree.add_row(0.0, nids).unwrap();
    tree.set_root(root);
    tree
}

fn build_nested_tree(depth: usize, leaves_per: usize) -> LayoutTree {
    let mut tree = LayoutTree::new();
    let root = build_nested_level(&mut tree, depth, leaves_per, true);
    tree.set_root(root);
    tree
}

fn build_nested_level(
    tree: &mut LayoutTree,
    depth: usize,
    leaves_per: usize,
    is_row: bool,
) -> panes::NodeId {
    if depth == 0 {
        let (_, nid) = tree.add_panel("leaf", grow(1.0)).unwrap();
        return nid;
    }
    let children: Vec<_> = (0..leaves_per)
        .map(|_| build_nested_level(tree, depth - 1, leaves_per, !is_row))
        .collect();
    match is_row {
        true => tree.add_row(2.0, children).unwrap(),
        false => tree.add_col(2.0, children).unwrap(),
    }
}

fn bench_static_frame(c: &mut Criterion) {
    let mut group = c.benchmark_group("static_frame");
    for n in [5, 50, 500] {
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            let tree = build_flat_row(n);
            let mut rt = LayoutRuntime::from(tree);
            rt.resolve(1920.0, 1080.0).unwrap();
            b.iter(|| rt.resolve(1920.0, 1080.0).unwrap());
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
            rt.resolve(1920.0, 1080.0).unwrap();
            let mut width = 800.0_f32;
            b.iter(|| {
                width = next_width(width);
                rt.resolve(width, 1080.0).unwrap()
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
            rt.resolve(1920.0, 1080.0).unwrap();
            let pid = panes::PanelId::from_raw(0);
            let mut toggle = false;
            b.iter(|| {
                toggle = !toggle;
                rt.tree_mut()
                    .set_constraints(pid, toggle_constraint(toggle))
                    .unwrap();
                rt.resolve(1920.0, 1080.0).unwrap()
            });
        });
    }
    group.finish();
}

fn bench_nested_resolve(c: &mut Criterion) {
    let mut group = c.benchmark_group("nested_static");
    // depth=3, leaves=3 => 27 panels, depth=4 => 81, depth=5 => 243
    for depth in [3, 4, 5] {
        let n_panels = 3_usize.pow(depth);
        group.bench_with_input(
            BenchmarkId::new("depth", format!("{depth}_panels_{n_panels}")),
            &depth,
            |b, &depth| {
                let tree = build_nested_tree(depth as usize, 3);
                let mut rt = LayoutRuntime::from(tree);
                rt.resolve(1920.0, 1080.0).unwrap();
                b.iter(|| rt.resolve(1920.0, 1080.0).unwrap());
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
            let layout_a = tree_a.resolve(1920.0, 1080.0).unwrap();

            let tree_b = build_flat_row(n);
            let layout_b = tree_b.resolve(800.0, 600.0).unwrap();

            b.iter(|| layout_a.lerp(&layout_b, 0.5));
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
);
criterion_main!(benches);
