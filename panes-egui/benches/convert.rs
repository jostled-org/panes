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

criterion_group!(benches, bench_convert, bench_panels);
criterion_main!(benches);
