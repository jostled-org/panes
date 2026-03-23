use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use panes::runtime::LayoutRuntime;
use panes::{Layout, Overlay};

fn build_dashboard(n: usize) -> Layout {
    let cards: Vec<(String, usize)> = (0..n).map(|i| (format!("card_{i}"), 1)).collect();
    let Ok(layout) = Layout::dashboard(cards).columns(4).gap(8.0).build() else {
        unreachable!("valid dashboard params");
    };
    layout
}

fn build_overlay_defs(n: usize) -> Vec<panes::OverlayDef> {
    let Ok(layout) = Layout::row(["a", "b"]) else {
        unreachable!("valid row params");
    };
    let mut rt = LayoutRuntime::from(layout);
    for i in 0..n {
        let kind = format!("overlay_{i}");
        let Ok(_id) = rt.add_overlay(kind, Overlay::center().fixed(400.0, 300.0)) else {
            unreachable!("valid overlay params");
        };
    }
    rt.overlays().to_vec()
}

fn bench_emit_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("emit_throughput");
    for n in [5, 50, 500] {
        let layout = build_dashboard(n);
        group.bench_with_input(BenchmarkId::from_parameter(n), &layout, |b, layout| {
            b.iter(|| panes_css::emit(layout));
        });
    }
    group.finish();
}

fn bench_emit_variants(c: &mut Criterion) {
    let mut group = c.benchmark_group("emit_variants");
    let layout = build_dashboard(50);
    let overlay_defs = build_overlay_defs(5);

    group.bench_function("emit_with_overlays", |b| {
        b.iter(|| panes_css::emit_with_overlays(&layout, &overlay_defs));
    });

    group.bench_function("emit_with_transitions", |b| {
        b.iter(|| panes_css::emit_with_transitions(&layout));
    });

    group.bench_function("emit_full", |b| {
        b.iter(|| panes_css::emit_full(&layout, &overlay_defs));
    });

    group.finish();
}

criterion_group!(benches, bench_emit_throughput, bench_emit_variants);
criterion_main!(benches);
