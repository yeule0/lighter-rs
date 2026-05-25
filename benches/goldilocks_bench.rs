use criterion::{black_box, criterion_group, criterion_main, Criterion};
use lighter_rs::field::goldilocks::GoldilocksField;

fn bench_add(c: &mut Criterion) {
    let a = GoldilocksField::from_canonical_u64(0x1234567890ABCDEF % GoldilocksField::MODULUS);
    let b = GoldilocksField::from_canonical_u64(0xFEDCBA0987654321 % GoldilocksField::MODULUS);
    c.bench_function("goldilocks_add", |bench| {
        bench.iter(|| black_box(black_box(&a).add(black_box(&b))));
    });
}

fn bench_mul(c: &mut Criterion) {
    let a = GoldilocksField::from_canonical_u64(0x1234567890ABCDEF % GoldilocksField::MODULUS);
    let b = GoldilocksField::from_canonical_u64(0xFEDCBA0987654321 % GoldilocksField::MODULUS);
    c.bench_function("goldilocks_mul", |bench| {
        bench.iter(|| black_box(black_box(&a).mul(black_box(&b))));
    });
}

fn bench_square(c: &mut Criterion) {
    let a = GoldilocksField::from_canonical_u64(0x1234567890ABCDEF % GoldilocksField::MODULUS);
    c.bench_function("goldilocks_square", |bench| {
        bench.iter(|| black_box(black_box(&a).square()));
    });
}

fn bench_inverse(c: &mut Criterion) {
    let a = GoldilocksField::from_canonical_u64(12345);
    c.bench_function("goldilocks_inverse", |bench| {
        bench.iter(|| black_box(black_box(&a).inverse()));
    });
}

fn bench_add_4way(c: &mut Criterion) {
    use lighter_rs::field::goldilocks_avx2::GoldilocksFieldx4;
    let vals = [100u64, 200, 300, 400].map(GoldilocksField::from_canonical_u64);
    let a = GoldilocksFieldx4(vals);
    let vals = [50u64, 60, 70, 80].map(GoldilocksField::from_canonical_u64);
    let b = GoldilocksFieldx4(vals);
    c.bench_function("goldilocks_add_4way", |bench| {
        bench.iter(|| black_box(black_box(&a).add(black_box(&b))));
    });
}

criterion_group!(benches, bench_add, bench_mul, bench_square, bench_inverse, bench_add_4way);
criterion_main!(benches);
