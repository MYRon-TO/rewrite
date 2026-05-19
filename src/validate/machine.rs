use std::collections::{BTreeSet, VecDeque};

use crate::{
    error::ValidationError,
    ir::{EmitCount, Ir, Machine, StateKind},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineValidationReport {
    pub warnings: Vec<MachineValidationWarning>,
}

impl MachineValidationReport {
    pub fn is_clean(&self) -> bool {
        self.warnings.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MachineValidationWarning {
    UnreachableState {
        machine_id: String,
        state_id: String,
    },
}

pub fn validate_machines(ir: &Ir) -> Result<MachineValidationReport, ValidationError> {
    let mut warnings = Vec::new();

    for (machine_id, machine) in &ir.machines {
        warnings.extend(validate_machine(machine_id, machine)?);
    }

    Ok(MachineValidationReport { warnings })
}

pub fn validate_machine(
    machine_id: &str,
    machine: &Machine,
) -> Result<Vec<MachineValidationWarning>, ValidationError> {
    validate_start_state(machine_id, machine)?;
    validate_attack_boundaries(machine_id, machine)?;
    validate_state_emits(machine_id, machine)?;
    validate_state_transitions(machine_id, machine)?;

    Ok(find_unreachable_states(machine_id, machine))
}

fn validate_start_state(machine_id: &str, machine: &Machine) -> Result<(), ValidationError> {
    if machine.states.is_empty() {
        return Err(ValidationError::invalid_machine(
            machine_id,
            "states must not be empty",
        ));
    }

    if !machine.states.contains_key(&machine.start) {
        return Err(ValidationError::invalid_machine(
            machine_id,
            format!(
                "start state `{}` does not exist in machine.states",
                machine.start
            ),
        ));
    }

    Ok(())
}

fn validate_attack_boundaries(machine_id: &str, machine: &Machine) -> Result<(), ValidationError> {
    for (state_id, state) in &machine.states {
        if !machine.attack.enabled && state.kind == StateKind::Attack {
            return Err(ValidationError::invalid_machine(
                machine_id,
                format!(
                    "attack.enabled is false but state `{state_id}` has kind `attack`; enable machine.attack or make the state benign"
                ),
            ));
        }

        if state.kind == StateKind::Benign && state.attack_label.is_some() {
            return Err(ValidationError::invalid_machine(
                machine_id,
                format!(
                    "benign state `{state_id}` must not declare attack_label; attack metadata belongs only on attack states"
                ),
            ));
        }
    }

    Ok(())
}

fn validate_state_emits(machine_id: &str, machine: &Machine) -> Result<(), ValidationError> {
    for (state_id, state) in &machine.states {
        for (emit_index, emit) in state.emit.iter().enumerate() {
            if let EmitCount::Range(range) = &emit.count {
                if range.min > range.max {
                    return Err(ValidationError::invalid_machine(
                        machine_id,
                        format!(
                            "state `{state_id}` emit[{emit_index}].count range min must be <= max, got min={} and max={}",
                            range.min, range.max
                        ),
                    ));
                }
            }
        }
    }

    Ok(())
}

fn validate_state_transitions(machine_id: &str, machine: &Machine) -> Result<(), ValidationError> {
    for (state_id, state) in &machine.states {
        if state.terminal && !state.transitions.is_empty() {
            return Err(ValidationError::invalid_machine(
                machine_id,
                format!(
                    "terminal state `{state_id}` must not declare transitions; terminal means execution stops"
                ),
            ));
        }

        if !state.terminal && state.transitions.is_empty() {
            return Err(ValidationError::invalid_machine(
                machine_id,
                format!(
                    "non-terminal state `{state_id}` has no transitions; set terminal=true or add transitions"
                ),
            ));
        }

        for (transition_index, transition) in state.transitions.iter().enumerate() {
            if !machine.states.contains_key(&transition.to) {
                return Err(ValidationError::invalid_machine(
                    machine_id,
                    format!(
                        "state `{state_id}` transition[{transition_index}].to references missing state `{}`",
                        transition.to
                    ),
                ));
            }

            if !transition.weight.is_finite() || transition.weight <= 0.0 {
                return Err(ValidationError::invalid_machine(
                    machine_id,
                    format!(
                        "state `{state_id}` transition[{transition_index}].weight must be a finite number greater than 0, got {}",
                        transition.weight
                    ),
                ));
            }
        }
    }

    Ok(())
}

fn find_unreachable_states(machine_id: &str, machine: &Machine) -> Vec<MachineValidationWarning> {
    let mut visited = BTreeSet::new();
    let mut queue = VecDeque::from([machine.start.as_str()]);

    while let Some(state_id) = queue.pop_front() {
        if !visited.insert(state_id.to_owned()) {
            continue;
        }

        let Some(state) = machine.states.get(state_id) else {
            continue;
        };

        for transition in &state.transitions {
            if !visited.contains(&transition.to) {
                queue.push_back(&transition.to);
            }
        }
    }

    machine
        .states
        .keys()
        .filter(|state_id| !visited.contains(*state_id))
        .map(|state_id| MachineValidationWarning::UnreachableState {
            machine_id: machine_id.to_owned(),
            state_id: state_id.to_owned(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_machine(json: &str) -> Machine {
        serde_json::from_str(json).expect("machine json should deserialize")
    }

    fn valid_machine() -> Machine {
        parse_machine(
            r#"
            {
              "id": "login_flow",
              "system": "linux_auth",
              "start": "start",
              "attack": {
                "enabled": false,
                "type": null,
                "technique": null,
                "tactic": null,
                "description": null
              },
              "states": {
                "start": {
                  "kind": "benign",
                  "attack_label": null,
                  "emit": [],
                  "transitions": [{"to": "done", "weight": 1}],
                  "terminal": false,
                  "description": null
                },
                "done": {
                  "kind": "benign",
                  "attack_label": null,
                  "emit": [],
                  "transitions": [],
                  "terminal": true,
                  "description": null
                }
              },
              "description": null
            }
            "#,
        )
    }

    #[test]
    fn accepts_generated_ssh_bruteforce_machine() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/generated/examples/ssh_bruteforce.json"
        );
        let input = std::fs::read_to_string(path).expect("generated example should be readable");
        let ir: Ir = serde_json::from_str(&input).expect("generated example should deserialize");

        let report = validate_machines(&ir).expect("example machines should be valid");

        assert!(report.is_clean());
    }

    #[test]
    fn rejects_missing_start_state() {
        let mut machine = valid_machine();
        machine.start = "missing".to_owned();

        let err = validate_machine("login_flow", &machine).expect_err("missing start must fail");

        assert!(matches!(err, ValidationError::InvalidMachine { .. }));
        assert!(err.to_string().contains("start state `missing`"));
    }

    #[test]
    fn rejects_attack_state_when_machine_attack_is_disabled() {
        let mut machine = valid_machine();
        machine.attack.enabled = false;
        machine.states.get_mut("start").unwrap().kind = crate::ir::StateKind::Attack;

        let err = validate_machine("login_flow", &machine)
            .expect_err("disabled attack machine must reject attack states");

        assert!(err.to_string().contains("attack.enabled is false"));
        assert!(err.to_string().contains("state `start`"));
    }

    #[test]
    fn rejects_attack_label_on_benign_state() {
        let mut machine = valid_machine();
        machine.states.get_mut("start").unwrap().attack_label = Some(crate::ir::AttackLabel {
            attack_type: None,
            technique: Some("T1110".to_owned()),
            tactic: None,
            name: None,
        });

        let err = validate_machine("login_flow", &machine)
            .expect_err("benign state attack_label must fail");

        assert!(err.to_string().contains("benign state `start`"));
        assert!(err.to_string().contains("attack_label"));
    }

    #[test]
    fn rejects_emit_count_range_with_min_greater_than_max() {
        let mut machine = valid_machine();
        machine.states.get_mut("start").unwrap().emit = vec![crate::ir::Emit {
            template: "login".to_owned(),
            count: crate::ir::EmitCount::Range(crate::ir::EmitCountRange { min: 9, max: 2 }),
        }];

        let err = validate_machine("login_flow", &machine)
            .expect_err("inverted emit count range must fail");

        assert!(err.to_string().contains("emit[0].count"));
        assert!(err.to_string().contains("min must be <= max"));
    }

    #[test]
    fn rejects_transition_to_missing_state() {
        let mut machine = valid_machine();
        machine.states.get_mut("start").unwrap().transitions[0].to = "ghost".to_owned();

        let err = validate_machine("login_flow", &machine).expect_err("bad transition must fail");

        assert!(err.to_string().contains("transition[0].to"));
        assert!(err.to_string().contains("ghost"));
    }

    #[test]
    fn rejects_non_terminal_state_without_transition() {
        let mut machine = valid_machine();
        machine.states.get_mut("start").unwrap().transitions.clear();

        let err = validate_machine("login_flow", &machine).expect_err("stuck state must fail");

        assert!(err.to_string().contains("non-terminal state `start`"));
    }

    #[test]
    fn rejects_terminal_state_with_transition() {
        let mut machine = valid_machine();
        let done = machine.states.get_mut("done").unwrap();
        done.transitions.push(crate::ir::Transition {
            to: "done".to_owned(),
            weight: 1.0,
        });

        let err = validate_machine("login_flow", &machine)
            .expect_err("ambiguous terminal state must fail");

        assert!(err.to_string().contains("terminal state `done`"));
    }

    #[test]
    fn rejects_non_positive_transition_weight() {
        let mut machine = valid_machine();
        machine.states.get_mut("start").unwrap().transitions[0].weight = 0.0;

        let err = validate_machine("login_flow", &machine).expect_err("zero weight must fail");

        assert!(err.to_string().contains("weight must be"));
    }

    #[test]
    fn reports_unreachable_states_as_warning() {
        let mut machine = valid_machine();
        machine.states.insert(
            "orphan".to_owned(),
            serde_json::from_value(serde_json::json!({
                "kind": "benign",
                "attack_label": null,
                "emit": [],
                "transitions": [],
                "terminal": true,
                "description": null
            }))
            .expect("orphan state should deserialize"),
        );

        let warnings =
            validate_machine("login_flow", &machine).expect("unreachable is warning only");

        assert_eq!(
            warnings,
            vec![MachineValidationWarning::UnreachableState {
                machine_id: "login_flow".to_owned(),
                state_id: "orphan".to_owned(),
            }]
        );
    }
}
