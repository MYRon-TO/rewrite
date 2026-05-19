use crate::{
    error::RuntimeError,
    ir::{EmitCount, EmitCountRange},
    runtime::rng::RuntimeRng,
};

/// Sample how many events one `emit` declaration produces in the current state.
pub fn sample_emit_count(
    machine_id: &str,
    state_id: &str,
    emit_index: usize,
    count: &EmitCount,
    rng: &mut RuntimeRng,
) -> Result<usize, RuntimeError> {
    match count {
        EmitCount::Fixed(value) => Ok(*value),
        EmitCount::Range(EmitCountRange { min, max }) => {
            if min > max {
                return Err(RuntimeError::machine_stuck(
                    machine_id,
                    state_id,
                    format!("emit[{emit_index}].count range has min > max"),
                ));
            }
            Ok(rng.range_usize(*min, *max))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_fixed_count() {
        let mut rng = RuntimeRng::new(1);
        assert_eq!(
            sample_emit_count("m", "s", 0, &EmitCount::Fixed(3), &mut rng).unwrap(),
            3
        );
    }

    #[test]
    fn samples_inclusive_range() {
        let mut rng = RuntimeRng::new(1);
        let count = sample_emit_count(
            "m",
            "s",
            0,
            &EmitCount::Range(EmitCountRange { min: 2, max: 2 }),
            &mut rng,
        )
        .unwrap();

        assert_eq!(count, 2);
    }

    #[test]
    fn rejects_invalid_range() {
        let mut rng = RuntimeRng::new(1);
        let err = sample_emit_count(
            "m",
            "s",
            0,
            &EmitCount::Range(EmitCountRange { min: 3, max: 2 }),
            &mut rng,
        )
        .unwrap_err();

        assert!(matches!(err, RuntimeError::MachineStuck { .. }));
    }
}
