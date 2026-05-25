#[inline]
pub fn has_avx2() -> bool {
    is_x86_feature_detected!("avx2")
}

#[inline]
pub fn has_avx512f() -> bool {
    is_x86_feature_detected!("avx512f")
}
