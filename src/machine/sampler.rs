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
    fn rejects_empty_transition_list() {
        let mut rng = RuntimeRng::new(42);
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
