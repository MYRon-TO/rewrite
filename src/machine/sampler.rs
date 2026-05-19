use crate::{
    error::RuntimeError,
    ir::{Id, Transition},
    runtime::rng::RuntimeRng,
};

/// Sample one outgoing transition according to its local `weight`.
pub fn sample_transition(
    machine_id: &str,
    state_id: &str,
    transitions: &[Transition],
    rng: &mut RuntimeRng,
) -> Result<Id, RuntimeError> {
    if transitions.is_empty() {
        return Err(RuntimeError::machine_stuck(
            machine_id,
            state_id,
            "non-terminal state has no outgoing transitions",
        ));
    }

    let total = transitions.iter().try_fold(0.0, |acc, transition| {
        if !transition.weight.is_finite() || transition.weight <= 0.0 {
            Err(RuntimeError::machine_stuck(
                machine_id,
                state_id,
                format!(
                    "transition to `{}` has invalid weight {}; expected a finite positive number",
                    transition.to, transition.weight
                ),
            ))
        } else {
            Ok(acc + transition.weight)
        }
    })?;

    let mut cursor = rng.next_f64() * total;
    for transition in transitions {
        if cursor < transition.weight {
            return Ok(transition.to.clone());
        }
        cursor -= transition.weight;
    }

    // Floating point round-off can leave `cursor` exactly at the end. Fall back
    // to the final transition instead of inventing a spooky missing branch.
    Ok(transitions
        .last()
        .expect("non-empty transitions checked above")
        .to
        .clone())
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
    fn samples_degenerate_single_transition() {
        let transitions = vec![Transition {
            to: "end".to_string(),
            weight: 1.0,
        }];
        let mut rng = RuntimeRng::new(42);

        assert_eq!(
            sample_transition("m", "start", &transitions, &mut rng).unwrap(),
            "end"
        );
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
            .map(|_| sample_transition("m", "start", &transitions, &mut left_rng).unwrap())
            .collect();
        let right: Vec<String> = (0..64)
            .map(|_| sample_transition("m", "start", &transitions, &mut right_rng).unwrap())
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
            match sample_transition("m", "start", &transitions, &mut rng)
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
            .map(|_| sample_transition("m", "start", &transitions, &mut rng).unwrap())
            .collect();

        assert_eq!(samples.len(), 8);
        assert!(samples.iter().all(|state| state == "a" || state == "b"));
        let err = sample_transition("m", "start", &[], &mut rng).unwrap_err();

        assert!(matches!(err, RuntimeError::MachineStuck { .. }));
    }

    #[test]
    fn rejects_invalid_weight() {
        let transitions = vec![Transition {
            to: "end".to_string(),
            weight: 0.0,
        }];
        let mut rng = RuntimeRng::new(42);
        let err = sample_transition("m", "start", &transitions, &mut rng).unwrap_err();

        assert!(matches!(err, RuntimeError::MachineStuck { .. }));
    }
}
