/// Minimal deterministic RNG for fuzz testing.
///
/// Uses a linear congruential generator (LCG) with the same constants as
/// glibc's `drand48`. Kept as a custom 6-line implementation rather than
/// depending on `fastrand`/`rand` because:
/// 1. Fuzz tests require deterministic, seed-reproducible output — an external
///    crate could change its algorithm across versions, silently breaking
///    existing fuzz corpora.
/// 2. Statistical quality doesn't matter for fuzzing; reproducibility does.
/// 3. Zero dependencies in the fuzz crate avoids nightly toolchain conflicts.
pub struct SimpleRng(u64);

impl SimpleRng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        self.0
    }
}
