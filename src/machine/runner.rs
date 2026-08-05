use chrono::{DateTime, Utc};

use crate::{
    context::{ContextStore, ContextValueGenerator, EventContext},
    error::RuntimeError,
    ir::{Id, Machine, ObjectMap, StateKind, Template, Transition},
    machine::{emit::sample_emit_count_at, sampler::sample_transition},
    runtime::{instance_group::InstancePlan, rng::RuntimeRng, time::InstanceClock},
};

/// Default loop guard for machine execution.
pub const DEFAULT_MAX_STEPS: usize = 10_000;

/// One event produced by the machine before template rendering and labels.
#[derive(Debug, Clone, PartialEq)]
pub struct EventDraft {
    pub scenario_id: Id,
    pub system_id: Id,
    pub instance_id: Id,
    pub machine_id: Id,
    pub state_id: Id,
    pub state_kind: StateKind,
    pub template_id: Id,
    pub timestamp: DateTime<Utc>,
    pub event_context: EventContext,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MachineRunnerConfig {
    pub max_steps: usize,
    pub enable_attack: bool,
}

impl Default for MachineRunnerConfig {
    fn default() -> Self {
        Self {
            max_steps: DEFAULT_MAX_STEPS,
            enable_attack: true,
        }
    }
}

/// Execute one machine for one concrete instance and return event drafts.
///
/// The runner deliberately does not render templates and does not resolve attack
/// labels. Its job is only to walk states, expand `emit.count`, attach fresh
/// per-event context, and advance the instance clock. Otherwise this module
/// would become the runtime junk drawer; nobody wants to debug that goblin nest.
pub fn run_machine<G>(
    machine: &Machine,
    templates: &ObjectMap<Template>,
    plan: &InstancePlan,
    store: &mut ContextStore<'_>,
    generator: &mut G,
    clock: &mut InstanceClock,
    rng: &mut RuntimeRng,
    config: MachineRunnerConfig,
) -> Result<Vec<EventDraft>, RuntimeError>
where
    G: ContextValueGenerator,
{
    if config.max_steps == 0 {
        return Err(RuntimeError::step_limit_exceeded(
            &machine.id,
            config.max_steps,
        ));
    }

    let mut drafts = Vec::new();
    let mut current_state_id = machine.start.clone();

    for step in 0..config.max_steps {
        let state = machine.states.get(&current_state_id).ok_or_else(|| {
            RuntimeError::machine_stuck(&machine.id, &current_state_id, "state does not exist")
        })?;

        if !config.enable_attack && state.kind == StateKind::Attack {
            return Err(RuntimeError::machine_stuck(
                &machine.id,
                &current_state_id,
                "enable_attack=false but runner reached an attack state; clean mode must not execute attack behavior",
            ));
        }

        for (emit_index, emit) in state.emit.iter().enumerate() {
            let count = sample_emit_count_at(
                &emit.count,
                rng,
                format!(
                    "machine.{}.states.{}.emit[{emit_index}].count",
                    machine.id, current_state_id
                ),
            )?;

            for _ in 0..count {
                let event_context = store.event_context(
                    plan.system_id.clone(),
                    machine.id.clone(),
                    plan.instance_id.clone(),
                    generator,
                )?;
                let event_type = templates
                    .get(&emit.template)
                    .and_then(|template| template.event_type.as_deref());

                drafts.push(EventDraft {
                    scenario_id: plan.scenario_id.clone(),
                    system_id: plan.system_id.clone(),
                    instance_id: plan.instance_id.clone(),
                    machine_id: machine.id.clone(),
                    state_id: current_state_id.clone(),
                    state_kind: state.kind,
                    template_id: emit.template.clone(),
                    timestamp: clock.next_timestamp_for_behavior(
                        Some(&machine.id),
                        Some(&current_state_id),
                        event_type,
                        Some(rng),
                    ),
                    event_context,
                });
            }
        }

        if state.terminal {
            return Ok(drafts);
        }

        current_state_id = sample_next_state(
            machine,
            &current_state_id,
            &state.transitions,
            rng,
            config.enable_attack,
        )?;

        if step + 1 == config.max_steps {
            return Err(RuntimeError::step_limit_exceeded(
                &machine.id,
                config.max_steps,
            ));
        }
    }

    Err(RuntimeError::step_limit_exceeded(
        &machine.id,
        config.max_steps,
    ))
}

fn sample_next_state(
    machine: &Machine,
    state_id: &str,
    transitions: &[Transition],
    rng: &mut RuntimeRng,
    enable_attack: bool,
) -> Result<Id, RuntimeError> {
    if enable_attack {
        return sample_transition(&machine.id, state_id, transitions, rng);
    }

    if transitions.is_empty() {
        return sample_transition(&machine.id, state_id, transitions, rng);
    }

    let mut allowed = Vec::new();
    let mut blocked_attack_targets = Vec::new();

    for transition in transitions {
        let target = machine.states.get(&transition.to).ok_or_else(|| {
            RuntimeError::machine_stuck(
                &machine.id,
                state_id,
                format!("transition targets missing state `{}`", transition.to),
            )
        })?;

        if target.kind == StateKind::Attack {
            blocked_attack_targets.push(transition.to.clone());
        } else {
            allowed.push(transition.clone());
        }
    }

    if allowed.is_empty() {
        return Err(RuntimeError::machine_stuck(
            &machine.id,
            state_id,
            format!(
                "enable_attack=false filtered out all outgoing transitions because they target attack states [{}]; clean mode requires the CUE machine to provide at least one non-attack transition",
                blocked_attack_targets.join(", ")
            ),
        ));
    }

    sample_transition(&machine.id, state_id, &allowed, rng)
}

pub fn run_machine_with_defaults<G>(
    machine: &Machine,
    plan: &InstancePlan,
    store: &mut ContextStore<'_>,
    generator: &mut G,
    clock: &mut InstanceClock,
    rng: &mut RuntimeRng,
) -> Result<Vec<EventDraft>, RuntimeError>
where
    G: ContextValueGenerator,
{
    let templates = ObjectMap::new();
    run_machine(
        machine,
        &templates,
        plan,
        store,
        generator,
        clock,
        rng,
        MachineRunnerConfig::default(),
    )
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use chrono::{Duration, TimeZone};
    use serde_json::Value;

    use super::*;
    use crate::{
        context::ContextValueGenerator, error::Result, ir::*, runtime::instance_group::InstancePlan,
    };

    #[derive(Default)]
    struct NoopGenerator;

    impl ContextValueGenerator for NoopGenerator {
        fn generate(
            &mut self,
            _generator: &Generator,
            _params: &JsonObject,
            _location: &str,
        ) -> Result<Value, RuntimeError> {
            unreachable!("tests only use const context")
        }
    }

    fn empty_attack() -> AttackMetadata {
        AttackMetadata {
            enabled: false,
            attack_type: None,
            technique: None,
            tactic: None,
            description: None,
        }
    }

    fn emit(template: &str, count: EmitCount) -> Emit {
        Emit {
            template: template.to_string(),
            count,
        }
    }

    fn state_with_kind(
        kind: StateKind,
        terminal: bool,
        emit: Vec<Emit>,
        transitions: Vec<Transition>,
    ) -> State {
        State {
            kind,
            attack_label: None,
            emit,
            transitions,
            terminal,
            description: None,
        }
    }

    fn state(terminal: bool, emit: Vec<Emit>, transitions: Vec<Transition>) -> State {
        state_with_kind(StateKind::Benign, terminal, emit, transitions)
    }

    fn base_ir(machine: Machine) -> Ir {
        Ir {
            ir_version: "0.2".to_string(),
            systems: BTreeMap::from([(
                "linux".to_string(),
                System {
                    id: "linux".to_string(),
                    kind: None,
                    description: None,
                },
            )]),
            generators: BTreeMap::new(),
            context: Context::default(),
            templates: BTreeMap::new(),
            machines: BTreeMap::from([("login".to_string(), machine)]),
            instance_groups: BTreeMap::new(),
            time_profiles: BTreeMap::new(),
            scenario: Scenario {
                id: "scenario".to_string(),
                systems: vec!["linux".to_string()],
                instance_groups: vec![],
                time_profile: "t".to_string(),
                description: None,
            },
        }
    }

    fn plan() -> InstancePlan {
        InstancePlan {
            scenario_id: "scenario".to_string(),
            group_id: "sessions".to_string(),
            group_index: 0,
            instance_index: 0,
            instance_id: "sessions-0001".to_string(),
            system_id: "linux".to_string(),
            machine_id: "login".to_string(),
        }
    }

    fn clock() -> InstanceClock {
        InstanceClock::fixed_tick(
            Utc.with_ymd_and_hms(2026, 5, 17, 0, 0, 0).single().unwrap(),
            Duration::milliseconds(1),
            true,
        )
    }

    #[test]
    fn emits_current_state_then_stops_at_terminal() {
        let machine = Machine {
            id: "login".to_string(),
            system: "linux".to_string(),
            start: "start".to_string(),
            attack: empty_attack(),
            states: BTreeMap::from([(
                "start".to_string(),
                state(true, vec![emit("ssh_login", EmitCount::Fixed(2))], vec![]),
            )]),
            description: None,
        };
        let ir = base_ir(machine.clone());
        let mut store = ContextStore::new(&ir);
        let mut generator = NoopGenerator;
        let mut clock = clock();
        let mut rng = RuntimeRng::new(42);

        let drafts = run_machine_with_defaults(
            &machine,
            &plan(),
            &mut store,
            &mut generator,
            &mut clock,
            &mut rng,
        )
        .unwrap();

        assert_eq!(drafts.len(), 2);
        assert!(drafts.iter().all(|draft| draft.state_id == "start"));
        assert!(drafts.iter().all(|draft| draft.template_id == "ssh_login"));
        assert!(drafts[0].timestamp < drafts[1].timestamp);
    }

    #[test]
    fn follows_transition_until_terminal() {
        let machine = Machine {
            id: "login".to_string(),
            system: "linux".to_string(),
            start: "start".to_string(),
            attack: empty_attack(),
            states: BTreeMap::from([
                (
                    "start".to_string(),
                    state(
                        false,
                        vec![emit("begin", EmitCount::Fixed(1))],
                        vec![Transition {
                            to: "end".to_string(),
                            weight: 1.0,
                        }],
                    ),
                ),
                (
                    "end".to_string(),
                    state(true, vec![emit("finish", EmitCount::Fixed(1))], vec![]),
                ),
            ]),
            description: None,
        };
        let ir = base_ir(machine.clone());
        let mut store = ContextStore::new(&ir);
        let mut generator = NoopGenerator;
        let mut clock = clock();
        let mut rng = RuntimeRng::new(42);

        let drafts = run_machine_with_defaults(
            &machine,
            &plan(),
            &mut store,
            &mut generator,
            &mut clock,
            &mut rng,
        )
        .unwrap();

        let states = drafts
            .iter()
            .map(|draft| draft.state_id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(states, vec!["start", "end"]);
    }

    #[test]
    fn stops_infinite_loop_with_step_limit() {
        let machine = Machine {
            id: "login".to_string(),
            system: "linux".to_string(),
            start: "loop".to_string(),
            attack: empty_attack(),
            states: BTreeMap::from([(
                "loop".to_string(),
                state(
                    false,
                    vec![],
                    vec![Transition {
                        to: "loop".to_string(),
                        weight: 1.0,
                    }],
                ),
            )]),
            description: None,
        };
        let ir = base_ir(machine.clone());
        let mut store = ContextStore::new(&ir);
        let mut generator = NoopGenerator;
        let mut clock = clock();
        let mut rng = RuntimeRng::new(42);

        let err = run_machine(
            &machine,
            &ir.templates,
            &plan(),
            &mut store,
            &mut generator,
            &mut clock,
            &mut rng,
            MachineRunnerConfig {
                max_steps: 3,
                ..MachineRunnerConfig::default()
            },
        )
        .unwrap_err();

        assert!(matches!(
            err,
            RuntimeError::StepLimitExceeded { max_steps: 3, .. }
        ));
    }

    #[test]
    fn non_terminal_without_transition_gets_stuck() {
        let machine = Machine {
            id: "login".to_string(),
            system: "linux".to_string(),
            start: "start".to_string(),
            attack: empty_attack(),
            states: BTreeMap::from([("start".to_string(), state(false, vec![], vec![]))]),
            description: None,
        };
        let ir = base_ir(machine.clone());
        let mut store = ContextStore::new(&ir);
        let mut generator = NoopGenerator;
        let mut clock = clock();
        let mut rng = RuntimeRng::new(42);

        let err = run_machine_with_defaults(
            &machine,
            &plan(),
            &mut store,
            &mut generator,
            &mut clock,
            &mut rng,
        )
        .unwrap_err();

        assert!(matches!(err, RuntimeError::MachineStuck { .. }));
    }

    #[test]
    fn disabled_attack_filters_transitions_into_attack_states() {
        let machine = Machine {
            id: "login".to_string(),
            system: "linux".to_string(),
            start: "start".to_string(),
            attack: AttackMetadata {
                enabled: true,
                attack_type: Some("ssh_bruteforce".to_string()),
                technique: None,
                tactic: None,
                description: None,
            },
            states: BTreeMap::from([
                (
                    "start".to_string(),
                    state(
                        false,
                        vec![emit("begin", EmitCount::Fixed(1))],
                        vec![
                            Transition {
                                to: "benign_done".to_string(),
                                weight: 1.0,
                            },
                            Transition {
                                to: "bruteforce".to_string(),
                                weight: 999.0,
                            },
                        ],
                    ),
                ),
                (
                    "benign_done".to_string(),
                    state(true, vec![emit("clean", EmitCount::Fixed(1))], vec![]),
                ),
                (
                    "bruteforce".to_string(),
                    state_with_kind(
                        StateKind::Attack,
                        true,
                        vec![emit("attack", EmitCount::Fixed(1))],
                        vec![],
                    ),
                ),
            ]),
            description: None,
        };
        let ir = base_ir(machine.clone());
        let mut store = ContextStore::new(&ir);
        let mut generator = NoopGenerator;
        let mut clock = clock();
        let mut rng = RuntimeRng::new(42);

        let drafts = run_machine(
            &machine,
            &ir.templates,
            &plan(),
            &mut store,
            &mut generator,
            &mut clock,
            &mut rng,
            MachineRunnerConfig {
                enable_attack: false,
                ..MachineRunnerConfig::default()
            },
        )
        .unwrap();

        let states = drafts
            .iter()
            .map(|draft| draft.state_id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(states, vec!["start", "benign_done"]);
        assert!(
            drafts
                .iter()
                .all(|draft| draft.state_kind == StateKind::Benign)
        );
    }

    #[test]
    fn disabled_attack_reports_when_clean_path_is_missing() {
        let machine = Machine {
            id: "login".to_string(),
            system: "linux".to_string(),
            start: "start".to_string(),
            attack: AttackMetadata {
                enabled: true,
                attack_type: Some("ssh_bruteforce".to_string()),
                technique: None,
                tactic: None,
                description: None,
            },
            states: BTreeMap::from([
                (
                    "start".to_string(),
                    state(
                        false,
                        vec![],
                        vec![Transition {
                            to: "bruteforce".to_string(),
                            weight: 1.0,
                        }],
                    ),
                ),
                (
                    "bruteforce".to_string(),
                    state_with_kind(StateKind::Attack, true, vec![], vec![]),
                ),
            ]),
            description: None,
        };
        let ir = base_ir(machine.clone());
        let mut store = ContextStore::new(&ir);
        let mut generator = NoopGenerator;
        let mut clock = clock();
        let mut rng = RuntimeRng::new(42);

        let err = run_machine(
            &machine,
            &ir.templates,
            &plan(),
            &mut store,
            &mut generator,
            &mut clock,
            &mut rng,
            MachineRunnerConfig {
                enable_attack: false,
                ..MachineRunnerConfig::default()
            },
        )
        .unwrap_err();

        assert!(matches!(err, RuntimeError::MachineStuck { .. }));
        assert!(err.to_string().contains("enable_attack=false"));
        assert!(err.to_string().contains("clean mode requires"));
        assert!(err.to_string().contains("bruteforce"));
    }

    #[test]
    fn disabled_attack_reports_if_attack_state_is_reached_anyway() {
        let machine = Machine {
            id: "login".to_string(),
            system: "linux".to_string(),
            start: "bruteforce".to_string(),
            attack: AttackMetadata {
                enabled: true,
                attack_type: Some("ssh_bruteforce".to_string()),
                technique: None,
                tactic: None,
                description: None,
            },
            states: BTreeMap::from([(
                "bruteforce".to_string(),
                state_with_kind(StateKind::Attack, true, vec![], vec![]),
            )]),
            description: None,
        };
        let ir = base_ir(machine.clone());
        let mut store = ContextStore::new(&ir);
        let mut generator = NoopGenerator;
        let mut clock = clock();
        let mut rng = RuntimeRng::new(42);

        let err = run_machine(
            &machine,
            &ir.templates,
            &plan(),
            &mut store,
            &mut generator,
            &mut clock,
            &mut rng,
            MachineRunnerConfig {
                enable_attack: false,
                ..MachineRunnerConfig::default()
            },
        )
        .unwrap_err();

        assert!(err.to_string().contains("enable_attack=false"));
        assert!(err.to_string().contains("reached an attack state"));
    }
}
