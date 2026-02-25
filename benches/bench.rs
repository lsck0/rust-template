use criterion::{Criterion, criterion_group, criterion_main};
use project_name::sum_digits;

fn bench(_c: &mut Criterion) {
    let mut group = _c.benchmark_group("sum_digits");
    group.bench_function("simple", |b| {
        b.iter(|| {
            sum_digits("12345".as_bytes());
        })
    });
    group.bench_function("long", |b| {
        b.iter(|| {
            sum_digits("12345678901234567890".as_bytes());
        })
    });
    group.bench_function("with_non_digits", |b| {
        b.iter(|| {
            sum_digits("1a2b3c4d5e".as_bytes());
        })
    });
    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
