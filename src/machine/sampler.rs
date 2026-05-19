use crate::{error::RuntimeError, ir::Transition, runtime::rng::RuntimeRng};

/// Selects one outgoing transition from a state using local transition weights.
///
/// Weights are normalized implicitly by drawing from `[0, sum(weights))` and
/// walking the cumulative weight. The caller is expected to pass transitions
/// from a single state; this sampler deliberately does not know about machines
/// or templates, keeping Phase 4.2 small instead of growing a tiny probability
/// wheel into a casino management system.
pub fn sample_next_state(
    transitions: &[Transition],
    rng: &mut RuntimeRng,
) -> Result<String, RuntimeError> {
    validate_transitions(transitions)?;

    let total_weight: f64 = transitions.iter().map(|transition| transition.weight).sum();
    let mut draw = rng.next_f64() * total_weight;

    for transition in transitions {
        if draw < transition.weight {
            return Ok(transition.to.clone());
        }

        draw -= transition.weight;
    }

    // Floating-point edge guard. `rng.next_f64()` should be < 1.0, but returning
    // the last transition keeps behavior well-defined if an implementation ever
    // hands us the exact upper boundary.
    Ok(transitions
        .last()
        .expect("validated transitions are non-empty")
        .to
        .clone())
}

fn validate_transitions(transitions: &[Transition]) -> Result<(), RuntimeError> {
    if transitions.is_empty() {
        return Err(RuntimeError::machine_stuck(
            "<unknown>",
            "<unknown>",
            "cannot sample next state from an empty transition list",
        ));
    }

    for (index, transition) in transitions.iter().enumerate() {
        if !transition.weight.is_finite() || transition.weight <= 0.0 {
            return Err(RuntimeError::machine_stuck(
                "<unknown>",
                "<unknown>",
                format!(
                    "transition[{index}].weight must be a finite number greater than 0, got {}",
                    transition.weight
                ),
            ));
        }
    }

    let total_weight: f64 = transitions.iter().map(|transition| transition.weight).sum();
    if !total_weight.is_finite() || total_weight <= 0.0 {
        return Err(RuntimeError::machine_stuck(
            "<unknown>",
            "<unknown>",
            "transition weights overflowed or summed to a non-positive value",
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn transition(to: &str, weight: f64) -> Transition {
        Transition {
            to: to.to_owned(),
            weight,
        }
    }

    #[test]
    fn samples_the_only_transition() {
        let transitions = vec![transition("done", 1.0)];
        let mut rng = RuntimeRng::new(42);

        for _ in 0..16 {
            let next = sample_next_state(&transitions, &mut rng).expect("sample should succeed");
            assert_eq!(next, "done");
        }
    }

    #[test]
    fn rejects_empty_transition_list() {
        let mut rng = RuntimeRng::new(42);

        let err = sample_next_state(&[], &mut rng).expect_err("empty transitions must fail");

        assert!(err.to_string().contains("empty transition list"));
    }

    #[test]
    fn rejects_invalid_weights() {
        let mut rng = RuntimeRng::new(42);

        for bad_weight in [0.0, -1.0, f64::INFINITY, f64::NAN] {
            let transitions = vec![transition("bad", bad_weight)];
            let err = sample_next_state(&transitions, &mut rng)
                .expect_err("invalid transition weight must fail");

            assert!(err.to_string().contains("weight must be"));
        }
    }

    #[test]
    fn deterministic_for_same_seed() {
        let transitions = vec![
            transition("benign", 7.0),
            transition("attack", 2.0),
            transition("done", 1.0),
        ];
        let mut left_rng = RuntimeRng::new(2026);
        let mut right_rng = RuntimeRng::new(2026);

        let left: Vec<String> = (0..64)
            .map(|_| sample_next_state(&transitions, &mut left_rng).unwrap())
            .collect();
        let right: Vec<String> = (0..64)
            .map(|_| sample_next_state(&transitions, &mut right_rng).unwrap())
            .collect();

        assert_eq!(left, right);
    }

    #[test]
    fn statistical_distribution_tracks_relative_weights() {
        let transitions = vec![transition("low", 1.0), transition("high", 3.0)];
        let mut rng = RuntimeRng::new(7);
        let samples = 20_000;
        let mut low = 0usize;
        let mut high = 0usize;

        for _ in 0..samples {
            match sample_next_state(&transitions, &mut rng)
                .expect("valid weighted sample")
                .as_str()
            {
                "low" => low += 1,
                "high" => high += 1,
                other => panic!("unexpected state {other}"),
            }
        }

        let low_ratio = low as f64 / samples as f64;
        let high_ratio = high as f64 / samples as f64;

        assert!(
            (low_ratio - 0.25).abs() < 0.02,
            "low ratio {low_ratio} should be close to 0.25"
        );
        assert!(
            (high_ratio - 0.75).abs() < 0.02,
            "high ratio {high_ratio} should be close to 0.75"
        );
    }

    #[test]
    fn accepts_non_normalized_weights() {
        let transitions = vec![transition("a", 10.0), transition("b", 30.0)];
        let mut rng = RuntimeRng::new(99);

        let samples: Vec<String> = (0..8)
            .map(|_| sample_next_state(&transitions, &mut rng).unwrap())
            .collect();

        assert_eq!(samples.len(), 8);
        assert!(samples.iter().all(|state| state == "a" || state == "b"));
    }
}
