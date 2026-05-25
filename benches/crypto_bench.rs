use criterion::{black_box, criterion_group, criterion_main, Criterion};
use lighter_rs::curve::scalar::Scalar;
use lighter_rs::curve::point::Point;
use lighter_rs::signature::schnorr;
use lighter_rs::field::quintic::Fp5;
use std::sync::LazyLock;

static TEST_SK: LazyLock<Scalar> = LazyLock::new(|| {
    Scalar::from_bytes_le(&[42u8; 40])
});
static TEST_MSG: LazyLock<Fp5> = LazyLock::new(|| {
    Fp5::from_u64_arr([1, 2, 3, 4, 5])
});
static TEST_NONCE: LazyLock<Scalar> = LazyLock::new(|| {
    Scalar::from_bytes_le(&{ let mut b = [0u8; 40]; b[0] = 99; b[1] = 1; b })
});

fn bench_sign_full(c: &mut Criterion) {
    c.bench_function("schnorr_sign_full", |bench| {
        bench.iter(|| {
            schnorr::sign_hashed_message(
                black_box(&TEST_MSG),
                black_box(&TEST_SK),
            );
        });
    });
}

fn bench_k_times_g(c: &mut Criterion) {
    c.bench_function("k_times_G", |bench| {
        bench.iter(|| {
            Point::mul_generator(black_box(&TEST_NONCE));
        });
    });
}

fn bench_encode(c: &mut Criterion) {
    let r = Point::mul_generator(&TEST_NONCE);
    c.bench_function("point_encode", |bench| {
        bench.iter(|| {
            black_box(black_box(&r).encode());
        });
    });
}

criterion_group!(benches, bench_sign_full, bench_k_times_g, bench_encode);
criterion_main!(benches);
