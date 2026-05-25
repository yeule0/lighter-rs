#[cfg(target_arch = "x86_64")]
mod cpuid;

#[cfg(target_arch = "x86_64")]
pub use cpuid::has_avx2;
#[cfg(target_arch = "x86_64")]
pub use cpuid::has_avx512f;
