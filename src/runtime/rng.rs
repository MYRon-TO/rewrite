use rand::{Rng, RngExt, SeedableRng, rngs::StdRng};

/// Seed used when the CLI does not provide one.
///
/// A fixed default keeps MVP runs reproducible instead of silently depending on
/// OS entropy. Users can still choose a different experiment by passing a seed
/// through the CLI once generation is wired in later phases.
pub const DEFAULT_SEED: u64 = 0x5EED_2026_0000_0001;

/// Resolve the optional CLI seed into the concrete seed used by runtime code.
pub fn resolve_seed(cli_seed: Option<u64>) -> u64 {
    cli_seed.unwrap_or(DEFAULT_SEED)
}

/// Factory for deterministic RNG streams derived from one root seed.
///
/// The manager intentionally does not mutate while creating streams. Deriving a
/// stream from `(root_seed, namespace, index)` makes independent components
/// reproducible even if execution order changes later. This avoids the classic
/// "one extra random draw moved the whole scenario" foot-gun; stepping on that
/// rake is funny exactly once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RngManager {
    root_seed: u64,
}

impl RngManager {
    pub fn new(seed: u64) -> Self {
        Self { root_seed: seed }
    }

    pub fn from_cli_seed(cli_seed: Option<u64>) -> Self {
        Self::new(resolve_seed(cli_seed))
    }

    pub fn root_seed(&self) -> u64 {
        self.root_seed
    }

    pub fn root_stream(&self) -> RuntimeRng {
        RuntimeRng::new(self.root_seed)
    }

    pub fn stream(&self, namespace: impl AsRef<str>) -> RuntimeRng {
        self.indexed_stream(namespace, 0)
    }

    pub fn indexed_stream(&self, namespace: impl AsRef<str>, index: u64) -> RuntimeRng {
        RuntimeRng::new(derive_seed(self.root_seed, namespace.as_ref(), index))
    }
}

/// Deterministic random source used by runtime modules.
#[derive(Debug)]
pub struct RuntimeRng {
    seed: u64,
    inner: StdRng,
}

impl RuntimeRng {
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            inner: StdRng::seed_from_u64(seed),
        }
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub fn inner_mut(&mut self) -> &mut StdRng {
        &mut self.inner
    }

    pub fn next_u64(&mut self) -> u64 {
        self.inner.random()
    }

    pub fn next_u32(&mut self) -> u32 {
        self.inner.random()
    }

    pub fn next_f64(&mut self) -> f64 {
        self.inner.random()
    }

    pub fn range_usize(&mut self, min: usize, max_inclusive: usize) -> usize {
        assert!(
            min <= max_inclusive,
            "invalid random usize range: min must be <= max"
        );
        self.inner.random_range(min..=max_inclusive)
    }

    pub fn range_u64(&mut self, min: u64, max_inclusive: u64) -> u64 {
        assert!(
            min <= max_inclusive,
            "invalid random u64 range: min must be <= max"
        );
        self.inner.random_range(min..=max_inclusive)
    }
}

/// Stable FNV-1a based derivation for child stream seeds.
fn derive_seed(root_seed: u64, namespace: &str, index: u64) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

    let mut hash = FNV_OFFSET;

    for byte in root_seed
        .to_le_bytes()
        .into_iter()
        .chain(namespace.as_bytes().iter().copied())
        .chain(index.to_le_bytes())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }

    // Mix once more so similar short namespaces do not produce visually similar
    // seeds when printed in diagnostics.
    hash ^= hash >> 33;
    hash = hash.wrapping_mul(0xff51_afd7_ed55_8ccd);
    hash ^= hash >> 33;
    hash = hash.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    hash ^ (hash >> 33)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_cli_seed_or_default_seed() {
        assert_eq!(resolve_seed(Some(42)), 42);
        assert_eq!(resolve_seed(None), DEFAULT_SEED);
        assert_eq!(RngManager::from_cli_seed(None).root_seed(), DEFAULT_SEED);
    }

    #[test]
    fn same_seed_produces_same_sequence() {
        let mut left = RuntimeRng::new(42);
        let mut right = RuntimeRng::new(42);

        let left_values: Vec<u64> = (0..16).map(|_| left.next_u64()).collect();
        let right_values: Vec<u64> = (0..16).map(|_| right.next_u64()).collect();

        assert_eq!(left_values, right_values);
    }

    #[test]
    fn different_seed_changes_sequence() {
        let mut left = RuntimeRng::new(42);
        let mut right = RuntimeRng::new(43);

        let left_values: Vec<u64> = (0..8).map(|_| left.next_u64()).collect();
        let right_values: Vec<u64> = (0..8).map(|_| right.next_u64()).collect();

        assert_ne!(left_values, right_values);
    }

    #[test]
    fn named_streams_are_stable_and_independent_of_creation_order() {
        let manager = RngManager::new(7);

        let mut first_a = manager.indexed_stream("instance", 1);
        let mut first_b = manager.indexed_stream("generator", 1);
        let mut second_b = manager.indexed_stream("generator", 1);
        let mut second_a = manager.indexed_stream("instance", 1);

        assert_eq!(first_a.seed(), second_a.seed());
        assert_eq!(first_b.seed(), second_b.seed());
        assert_ne!(first_a.seed(), first_b.seed());

        assert_eq!(first_a.next_u64(), second_a.next_u64());
        assert_eq!(first_b.next_u64(), second_b.next_u64());
    }

    #[test]
    fn inclusive_ranges_include_boundaries_when_degenerate() {
        let mut rng = RuntimeRng::new(1);

        assert_eq!(rng.range_usize(3, 3), 3);
        assert_eq!(rng.range_u64(9, 9), 9);
    }
}
