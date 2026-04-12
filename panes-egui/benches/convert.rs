#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use std::sync::Arc;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use panes::runtime::LayoutRuntime;
use panes::{ResolvedLayout, StrategyKind};

fn build_resolved(n: usize) -> ResolvedLayout {
    let panels: Vec<String> = (0..n).map(|i| format!("p{i}")).collect();
    let layout = panes::Layout::row(panels.iter().map(String::as_str)).unwrap();
    layout.resolve(1920.0, 1080.0).unwrap()
}

fn build_runtime(n: usize) -> LayoutRuntime {
    let kinds: Vec<Arc<str>> = (0..n)
        .map(|i| Arc::from(format!("p{i}").as_str()))
        .collect();
    LayoutRuntime::from_strategy(
        StrategyKind::MasterStack {
            master_ratio: 0.5,
            gap: 0.0,
        },
        &kinds,
    )
    .unwrap()
}

fn bench_convert(c: &mut Criterion) {
    let mut group = c.benchmark_group("convert");
    for n in [5, 50, 500] {
        let resolved = build_resolved(n);
        group.bench_with_input(BenchmarkId::from_parameter(n), &resolved, |b, resolved| {
            b.iter(|| panes_egui::convert(resolved));
        });
    }
    group.finish();
}

fn bench_panels(c: &mut Criterion) {
    let mut group = c.benchmark_group("panels");
    for n in [5, 50, 500] {
        let resolved = build_resolved(n);
        group.bench_with_input(BenchmarkId::from_parameter(n), &resolved, |b, resolved| {
            b.iter(|| panes_egui::panels(resolved).count());
        });
    }
    group.finish();
}

fn bench_resolve(c: &mut Criterion) {
    let area = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1920.0, 1080.0));
    let mut group = c.benchmark_group("resolve");
    for n in [5, 50, 500] {
        group.bench_function(BenchmarkId::from_parameter(n), |b| {
            b.iter_batched_ref(
                || build_runtime(n),
                |rt| {
                    let frame = panes_egui::resolve(rt, area).unwrap();
                    frame.panels().count()
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

fn bench_resolve_layout(c: &mut Criterion) {
    let area = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1920.0, 1080.0));
    let mut group = c.benchmark_group("resolve_layout");
    for n in [5, 50, 500] {
        let panels: Vec<String> = (0..n).map(|i| format!("p{i}")).collect();
        let layout = panes::Layout::row(panels.iter().map(String::as_str)).unwrap();
        group.bench_with_input(BenchmarkId::from_parameter(n), &layout, |b, layout| {
            b.iter(|| panes_egui::resolve_layout(layout, area).unwrap());
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_convert,
    bench_panels,
    bench_resolve,
    bench_resolve_layout
);
criterion_main!(benches);
