use crate::{
    context::{ContextStore, ContextValueGenerator},
    error::RuntimeError,
    generator::BuiltinGenerator,
    ir::Ir,
    machine::runner::{EventDraft, MachineRunnerConfig, run_machine},
    runtime::{
        instance_group::{InstancePlan, expand_instance_groups, initialize_plan_contexts},
        rng::RngManager,
        time::TimeModel,
    },
};

/// Scenario-level execution settings for phase-4 event draft generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ScenarioRunnerConfig {
    pub machine: MachineRunnerConfig,
}

/// Result of executing every enabled instance group referenced by a scenario.
#[derive(Debug, Clone, PartialEq)]
pub struct ScenarioRun {
    pub plans: Vec<InstancePlan>,
    pub events: Vec<EventDraft>,
}

/// Execute the scenario with the built-in generator implementation.
///
/// This is the default runtime entry point future CLI generation can call. All
/// random streams are derived from the same root seed, but time, generator, and
/// per-instance machine sampling are separated so adding one random draw to a
/// generator does not shove every transition into a different universe.
pub fn run_scenario(
    ir: &Ir,
    rng_manager: &RngManager,
    config: ScenarioRunnerConfig,
) -> Result<ScenarioRun, RuntimeError> {
    let generator_seed = rng_manager.stream("context-generator").seed();
    let mut generator = BuiltinGenerator::with_seed(generator_seed);
    run_scenario_with_generator(ir, rng_manager, &mut generator, config)
}

/// Execute all enabled instance groups referenced by `ir.scenario`.
///
/// Coordination is intentionally flat: every instance receives its own machine
/// RNG and clock, while one shared `ContextStore` provides global/system sharing
/// and instance isolation. There is no cross-system trigger or dependency graph;
/// sneaking one in here would be architectural raccoon behavior.
pub fn run_scenario_with_generator<G>(
    ir: &Ir,
    rng_manager: &RngManager,
    generator: &mut G,
    config: ScenarioRunnerConfig,
) -> Result<ScenarioRun, RuntimeError>
where
    G: ContextValueGenerator,
{
    let plans = expand_instance_groups(ir)?;
    let mut store = ContextStore::new(ir);
    initialize_plan_contexts(ir, &plans, &mut store, generator)?;

    let time_profile = ir
        .time_profiles
        .get(&ir.scenario.time_profile)
        .ok_or_else(|| {
            RuntimeError::invalid_runtime_plan(
                format!("scenario.{}.time_profile", ir.scenario.id),
                format!("time_profile `{}` does not exist", ir.scenario.time_profile),
            )
        })?;
    let mut time_rng = rng_manager.stream("time-profile");
    let time_model = TimeModel::from_profile(time_profile, &mut time_rng)?;

    let mut events = Vec::new();
    for (plan_index, plan) in plans.iter().enumerate() {
        let machine = ir.machines.get(&plan.machine_id).ok_or_else(|| {
            RuntimeError::invalid_runtime_plan(
                format!("{}.machine", plan.location()),
                format!("machine `{}` does not exist", plan.machine_id),
            )
        })?;

        if machine.system != plan.system_id {
            return Err(RuntimeError::invalid_runtime_plan(
                format!("{}.system", plan.location()),
                format!(
                    "plan system `{}` does not match machine `{}` system `{}`",
                    plan.system_id, machine.id, machine.system
                ),
            ));
        }

        let mut clock = time_model.instance_clock(&plan.system_id);
        let mut machine_rng = rng_manager.indexed_stream("machine-runner", plan_index as u64);
        let mut instance_events = run_machine(
            machine,
            plan,
            &mut store,
            generator,
            &mut clock,
            &mut machine_rng,
            config.machine,
        )?;
        events.append(&mut instance_events);
    }

    Ok(ScenarioRun { plans, events })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use chrono::Duration;
    use serde_json::{Value, json};

    use super::*;
    use crate::{
        context::ContextValueGenerator, error::Result, ir::*, machine::runner::MachineRunnerConfig,
    };

    #[derive(Default)]
    struct CountingGenerator {
        next: usize,
        calls: Vec<String>,
    }

    impl ContextValueGenerator for CountingGenerator {
        fn generate(
            &mut self,
            generator: &Generator,
            _params: &JsonObject,
            location: &str,
        ) -> Result<Value, RuntimeError> {
            self.next += 1;
            self.calls.push(format!("{}@{}", generator.id, location));
            Ok(json!(format!("{}-{}", generator.id, self.next)))
        }
    }

    fn var(source: ValueSource) -> ContextVariable {
        ContextVariable {
            source,
            description: None,
        }
    }

    fn const_var(value: Value) -> ContextVariable {
        var(ValueSource::Const(ConstValue { value }))
    }

    fn generator_var(id: &str) -> ContextVariable {
        var(ValueSource::Generator(GeneratorValue {
            generator: id.to_string(),
            params: JsonObject::new(),
        }))
    }

    fn no_attack() -> AttackMetadata {
        AttackMetadata {
            enabled: false,
            attack_type: None,
            technique: None,
            tactic: None,
            name: None,
            description: None,
        }
    }

    fn one_state_machine(id: &str, system: &str, template: &str) -> Machine {
        Machine {
            id: id.to_string(),
            system: system.to_string(),
            start: "start".to_string(),
            attack: no_attack(),
            states: BTreeMap::from([(
                "start".to_string(),
                State {
                    kind: StateKind::Benign,
                    attack_label: None,
                    emit: vec![Emit {
                        template: template.to_string(),
                        count: EmitCount::Fixed(1),
                    }],
                    transitions: vec![],
                    terminal: true,
                    description: None,
                },
            )]),
            description: None,
        }
    }

    fn test_ir() -> Ir {
        Ir {
            ir_version: "0.1".to_string(),
            systems: BTreeMap::from([
                (
                    "linux".to_string(),
                    System {
                        id: "linux".to_string(),
                        kind: None,
                        description: None,
                    },
                ),
                (
                    "app".to_string(),
                    System {
                        id: "app".to_string(),
                        kind: None,
                        description: None,
                    },
                ),
            ]),
            generators: BTreeMap::from([(
                "g".to_string(),
                Generator {
                    id: "g".to_string(),
                    generator_type: "fake".to_string(),
                    params: JsonObject::new(),
                    description: None,
                },
            )]),
            context: Context {
                global: BTreeMap::from([("tenant".to_string(), generator_var("g"))]),
                system: BTreeMap::from([
                    (
                        "linux".to_string(),
                        BTreeMap::from([("host".to_string(), generator_var("g"))]),
                    ),
                    (
                        "app".to_string(),
                        BTreeMap::from([("host".to_string(), generator_var("g"))]),
                    ),
                ]),
                instance: BTreeMap::from([
                    (
                        "linux_flow".to_string(),
                        BTreeMap::from([("session".to_string(), generator_var("g"))]),
                    ),
                    (
                        "app_flow".to_string(),
                        BTreeMap::from([("session".to_string(), generator_var("g"))]),
                    ),
                ]),
                event: BTreeMap::from([("pid".to_string(), const_var(json!(1234)))]),
            },
            templates: BTreeMap::new(),
            machines: BTreeMap::from([
                (
                    "linux_flow".to_string(),
                    one_state_machine("linux_flow", "linux", "linux_event"),
                ),
                (
                    "app_flow".to_string(),
                    one_state_machine("app_flow", "app", "app_event"),
                ),
            ]),
            instance_groups: BTreeMap::from([
                (
                    "linux_sessions".to_string(),
                    InstanceGroup {
                        id: "linux_sessions".to_string(),
                        machine: "linux_flow".to_string(),
                        enabled: true,
                        count: 2,
                        params: None,
                        description: None,
                    },
                ),
                (
                    "app_sessions".to_string(),
                    InstanceGroup {
                        id: "app_sessions".to_string(),
                        machine: "app_flow".to_string(),
                        enabled: true,
                        count: 1,
                        params: None,
                        description: None,
                    },
                ),
            ]),
            time_profiles: BTreeMap::from([(
                "default_time".to_string(),
                TimeProfile {
                    id: "default_time".to_string(),
                    start: "2026-05-17T00:00:00Z".to_string(),
                    timezone: "UTC".to_string(),
                    granularity: TimeGranularity::Millisecond,
                    monotonic_per_instance: true,
                    system_clock_skew: BTreeMap::from([(
                        "app".to_string(),
                        ClockSkew::Fixed(FixedClockSkew {
                            milliseconds: 1000.0,
                        }),
                    )]),
                    description: None,
                },
            )]),
            scenario: Scenario {
                id: "multi".to_string(),
                systems: vec!["linux".to_string(), "app".to_string()],
                instance_groups: vec!["linux_sessions".to_string(), "app_sessions".to_string()],
                time_profile: "default_time".to_string(),
                description: None,
            },
        }
    }

    #[test]
    fn executes_multiple_systems_with_shared_global_context() {
        let ir = test_ir();
        let rng_manager = RngManager::new(42);
        let mut generator = CountingGenerator::default();

        let run = run_scenario_with_generator(
            &ir,
            &rng_manager,
            &mut generator,
            ScenarioRunnerConfig::default(),
        )
        .unwrap();

        assert_eq!(run.plans.len(), 3);
        assert_eq!(run.events.len(), 3);
        assert_eq!(
            run.events
                .iter()
                .map(|event| event.system_id.as_str())
                .collect::<Vec<_>>(),
            vec!["linux", "linux", "app"]
        );
        assert_eq!(
            generator
                .calls
                .iter()
                .filter(|call| call.contains("context.global.tenant"))
                .count(),
            1
        );
        assert_eq!(
            generator
                .calls
                .iter()
                .filter(|call| call.contains("context.system.linux.host"))
                .count(),
            1
        );
        assert_eq!(
            generator
                .calls
                .iter()
                .filter(|call| call.contains("context.system.app.host"))
                .count(),
            1
        );
    }

    #[test]
    fn applies_independent_system_clocks_without_cross_system_causality() {
        let ir = test_ir();
        let rng_manager = RngManager::new(42);
        let mut generator = CountingGenerator::default();

        let run = run_scenario_with_generator(
            &ir,
            &rng_manager,
            &mut generator,
            ScenarioRunnerConfig::default(),
        )
        .unwrap();

        let linux_time = run
            .events
            .iter()
            .find(|event| event.system_id == "linux")
            .unwrap()
            .timestamp;
        let app_time = run
            .events
            .iter()
            .find(|event| event.system_id == "app")
            .unwrap()
            .timestamp;

        assert_eq!(app_time - linux_time, Duration::seconds(1));
    }

    #[test]
    fn propagates_machine_step_limit_errors_with_location() {
        let mut ir = test_ir();
        let machine = ir.machines.get_mut("linux_flow").unwrap();
        let state = machine.states.get_mut("start").unwrap();
        state.terminal = false;
        state.transitions = vec![Transition {
            to: "start".to_string(),
            weight: 1.0,
        }];
        state.emit.clear();

        let rng_manager = RngManager::new(42);
        let mut generator = CountingGenerator::default();
        let err = run_scenario_with_generator(
            &ir,
            &rng_manager,
            &mut generator,
            ScenarioRunnerConfig {
                machine: MachineRunnerConfig { max_steps: 2 },
            },
        )
        .unwrap_err();

        assert!(matches!(
            err,
            RuntimeError::StepLimitExceeded {
                machine_id,
                max_steps: 2
            } if machine_id == "linux_flow"
        ));
    }
}
