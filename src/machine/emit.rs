use crate::{error::RuntimeError, ir::EmitCount, runtime::rng::RuntimeRng};

/// Samples how many events a state emit declaration should produce.
///
/// `EmitCount::Fixed(n)` returns `n` directly. `EmitCount::Range { min, max }`
/// samples from the inclusive range `[min, max]`. The sampler owns only count
/// semantics; it does not know templates, states, or labels, because stuffing
/// all that in here would turn a dice roll into a tiny DMV queue.
pub fn sample_emit_count(count: &EmitCount, rng: &mut RuntimeRng) -> Result<usize, RuntimeError> {
    sample_emit_count_at(count, rng, "<unknown>")
}

/// Same as [`sample_emit_count`], but with a caller-provided location for
/// diagnostics from future machine-runner integration.
pub fn sample_emit_count_at(
    count: &EmitCount,
    rng: &mut RuntimeRng,
    location: impl AsRef<str>,
) -> Result<usize, RuntimeError> {
    validate_emit_count_at(count, location.as_ref())?;

    match count {
        EmitCount::Fixed(value) => Ok(*value),
        EmitCount::Range(range) => Ok(rng.range_usize(range.min, range.max)),
    }
}

/// Validates emit-count shape without sampling.
pub fn validate_emit_count(count: &EmitCount) -> Result<(), RuntimeError> {
    validate_emit_count_at(count, "<unknown>")
}

/// Same as [`validate_emit_count`], but with a caller-provided location.
pub fn validate_emit_count_at(
    count: &EmitCount,
    location: impl AsRef<str>,
) -> Result<(), RuntimeError> {
    match count {
        EmitCount::Fixed(_) => Ok(()),
        EmitCount::Range(range) => {
            if range.min > range.max {
                return Err(RuntimeError::invalid_emit_count(
                    location.as_ref(),
                    format!(
                        "range min must be <= max, got min={} and max={}",
                        range.min, range.max
                    ),
                ));
            }

            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::EmitCountRange;

    fn range(min: usize, max: usize) -> EmitCount {
        EmitCount::Range(EmitCountRange { min, max })
    }

    #[test]
    fn returns_fixed_count_without_randomizing() {
        let mut rng = RuntimeRng::new(42);

        assert_eq!(
            sample_emit_count(&EmitCount::Fixed(0), &mut rng).unwrap(),
            0
        );
        assert_eq!(
            sample_emit_count(&EmitCount::Fixed(7), &mut rng).unwrap(),
            7
        );
    }
    #[test]
    fn samples_inclusive_range() {
        let mut rng = RuntimeRng::new(7);

        for _ in 0..256 {
            let count = sample_emit_count(&range(2, 5), &mut rng).unwrap();
            assert!(
                (2..=5).contains(&count),
                "sampled count {count} must be in range"
            );
        }
    }

    #[test]
    fn range_boundaries_are_allowed_when_equal() {
        let mut rng = RuntimeRng::new(99);

        assert_eq!(sample_emit_count(&range(3, 3), &mut rng).unwrap(), 3);
        assert_eq!(sample_emit_count(&range(0, 0), &mut rng).unwrap(), 0);
    }

    #[test]
    fn rejects_range_with_min_greater_than_max() {
        let err = validate_emit_count_at(&range(8, 3), "machine.m.states.s.emit[0].count")
            .expect_err("inverted range must fail");

        assert!(err.to_string().contains("machine.m.states.s.emit[0].count"));
        assert!(err.to_string().contains("min must be <= max"));
    }

    #[test]
    fn deterministic_for_same_seed() {
        let count = range(20, 80);
        let mut left_rng = RuntimeRng::new(2026);
        let mut right_rng = RuntimeRng::new(2026);

        let left: Vec<usize> = (0..64)
            .map(|_| sample_emit_count(&count, &mut left_rng).unwrap())
            .collect();
        let right: Vec<usize> = (0..64)
            .map(|_| sample_emit_count(&count, &mut right_rng).unwrap())
            .collect();

        assert_eq!(left, right);
    }

    #[test]
    fn range_sampler_can_hit_both_edges_for_small_ranges() {
        let count = range(0, 1);
        let mut rng = RuntimeRng::new(1);
        let samples: Vec<usize> = (0..64)
            .map(|_| sample_emit_count(&count, &mut rng).unwrap())
            .collect();

        assert!(samples.contains(&0), "lower boundary should be reachable");
        assert!(samples.contains(&1), "upper boundary should be reachable");
    }

    #[test]
    fn sample_with_location_reports_invalid_range() {
        let mut rng = RuntimeRng::new(1);
        let err = sample_emit_count_at(&range(3, 2), &mut rng, "machine.m.states.s.emit[0].count")
            .unwrap_err();

        assert!(matches!(err, RuntimeError::InvalidEmitCount { .. }));
        assert!(err.to_string().contains("machine.m.states.s.emit[0].count"));
    }
}
