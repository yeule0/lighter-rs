pub mod field;
pub mod hash;
pub mod curve;
pub mod signature;
pub mod types;
pub mod signer;
pub mod batch;
pub mod x86;

pub use field::goldilocks::GoldilocksField;

#[cfg(all(target_arch = "x86_64", not(feature = "no-simd")))]
pub fn has_avx2() -> bool {
    x86::has_avx2()
}

#[cfg(all(target_arch = "x86_64", not(feature = "no-simd")))]
pub fn has_avx512f() -> bool {
    x86::has_avx512f()
}

#[cfg(any(not(target_arch = "x86_64"), feature = "no-simd"))]
pub fn has_avx2() -> bool { false }

#[cfg(any(not(target_arch = "x86_64"), feature = "no-simd"))]
pub fn has_avx512f() -> bool { false }
