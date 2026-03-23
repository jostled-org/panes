use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use panes::ResolvedLayout;
use panes_wasm::WasmRuntime;

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
            b.iter(|| panes_wasm::convert(resolved));
        });
    }
    group.finish();
}

fn bench_panels_json(c: &mut Criterion) {
    let mut group = c.benchmark_group("panels_json");
    for n in [5, 50, 500] {
        let panels: Vec<String> = (0..n).map(|i| format!("p{i}")).collect();
        let refs: Vec<&str> = panels.iter().map(String::as_str).collect();
        let mut runtime = WasmRuntime::from_preset("master-stack", &refs).unwrap();
        let wasm_layout = runtime.resolve(1920.0, 1080.0).unwrap();
        group.bench_with_input(
            BenchmarkId::from_parameter(n),
            &wasm_layout,
            |b, wasm_layout| {
                b.iter(|| wasm_layout.panels().unwrap());
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench_convert, bench_panels_json);
criterion_main!(benches);
