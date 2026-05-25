use crate::field::quintic::Fp5;
use crate::curve::scalar::Scalar;
use crate::signature::schnorr;
use std::time::Instant;
use rayon::prelude::*;

pub fn batch_sign_parallel(hashed_msgs: &[Fp5], sk: &Scalar) -> (Vec<[u8; 80]>, BatchMetrics) {
    let start = Instant::now();
    let count = hashed_msgs.len();

    let sigs: Vec<[u8; 80]> = hashed_msgs
        .par_iter()
        .map(|msg| schnorr::sign_hashed_message(msg, sk).to_bytes())
        .collect();

    let elapsed = start.elapsed();
    let total_us = elapsed.as_micros() as f64;
    let metrics = BatchMetrics {
        count,
        total_us,
        us_per_sig: total_us / count as f64,
        throughput_per_sec: count as f64 / elapsed.as_secs_f64(),
    };
    (sigs, metrics)
}

pub fn batch_sign_seq(hashed_msgs: &[Fp5], sk: &Scalar) -> (Vec<[u8; 80]>, BatchMetrics) {
    let start = Instant::now();
    let count = hashed_msgs.len();

    let sigs: Vec<[u8; 80]> = hashed_msgs
        .iter()
        .map(|msg| schnorr::sign_hashed_message(msg, sk).to_bytes())
        .collect();

    let elapsed = start.elapsed();
    let total_us = elapsed.as_micros() as f64;
    let metrics = BatchMetrics {
        count,
        total_us,
        us_per_sig: total_us / count as f64,
        throughput_per_sec: count as f64 / elapsed.as_secs_f64(),
    };
    (sigs, metrics)
}

#[derive(Debug, Clone)]
pub struct BatchMetrics {
    pub count: usize,
    pub total_us: f64,
    pub us_per_sig: f64,
    pub throughput_per_sec: f64,
}
