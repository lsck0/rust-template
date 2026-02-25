use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};

const INPUT_PATTERN: &[u8] = b"1a2b3c4d5e";
const INPUT_SIZES_BYTES: [usize; 3] = [16, 1024, 1024 * 1024];

fn sum_digits_throughput(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("sum_digits");

    for size_bytes in INPUT_SIZES_BYTES {
        let input: Vec<u8> = INPUT_PATTERN.iter().copied().cycle().take(size_bytes).collect();
        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size_bytes), &input, |bencher, input| {
            bencher.iter(|| {{crate_name}}::sum_digits(black_box(input)));
        });
    }

    group.finish();
}

criterion_group!(benches, sum_digits_throughput);
criterion_main!(benches);
