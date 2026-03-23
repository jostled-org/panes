use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use panes::ResolvedLayout;

fn build_resolved(n: usize) -> ResolvedLayout {
    let panels: Vec<String> = (0..n).map(|i| format!("p{i}")).collect();
    let layout = panes::Layout::row(panels.iter().map(String::as_str)).unwrap();
    layout.resolve(1920.0, 1080.0).unwrap()
}

fn bench_convert(c: &mut Criterion) {
    let mut group = c.benchmark_group("convert");
    for n in [5, 50, 500] {
        let resolved = build_resolved(n);
        group.bench_with_input(BenchmarkId::from_parameter(n), &resolved, |b, resolved| {
            b.iter(|| panes_ratatui::convert(resolved));
        });
    }
    group.finish();
}

fn bench_panels(c: &mut Criterion) {
    let mut group = c.benchmark_group("panels");
    for n in [5, 50, 500] {
        let resolved = build_resolved(n);
        group.bench_with_input(BenchmarkId::from_parameter(n), &resolved, |b, resolved| {
            b.iter(|| panes_ratatui::panels(resolved).count());
        });
    }
    group.finish();
}

fn bench_focused_panels(c: &mut Criterion) {
    let mut group = c.benchmark_group("focused_panels");
    for n in [5, 50, 500] {
        let resolved = build_resolved(n);
        let focused = resolved.iter().next().map(|(pid, _)| pid);
        group.bench_with_input(BenchmarkId::from_parameter(n), &resolved, |b, resolved| {
            b.iter(|| panes_ratatui::focused_panels(resolved, focused).count());
        });
    }
    group.finish();
}

criterion_group!(benches, bench_convert, bench_panels, bench_focused_panels);
criterion_main!(benches);
