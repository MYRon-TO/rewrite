use crate::{
    context::{ContextStore, ContextValueGenerator},
    error::{Result, RuntimeError},
    ir::{Id, Ir},
};

/// One executable instance produced from a scenario `instance_group`.
///
/// The plan is intentionally data-only: later phase-4 modules can feed it into
/// the machine runner without knowing how groups were expanded. Context sharing
/// semantics come from these IDs:
/// - same scenario/global store for all plans;
/// - same `system_id` shares system context;
/// - unique `instance_id` per plan isolates instance context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstancePlan {
    pub scenario_id: Id,
    pub group_id: Id,
    pub group_index: usize,
    pub instance_index: usize,
    pub instance_id: Id,
    pub system_id: Id,
    pub machine_id: Id,
}

impl InstancePlan {
    fn new(
        scenario_id: &str,
        group_id: &str,
        group_index: usize,
        instance_index: usize,
        system_id: &str,
        machine_id: &str,
    ) -> Self {
        Self {
            scenario_id: scenario_id.to_string(),
            group_id: group_id.to_string(),
            group_index,
            instance_index,
            instance_id: format!("{group_id}-{:04}", instance_index + 1),
            system_id: system_id.to_string(),
            machine_id: machine_id.to_string(),
        }
    }

    pub fn location(&self) -> String {
        format!(
            "scenario.{}.instance_groups[{}].instances[{}]",
            self.scenario_id, self.group_index, self.instance_index
        )
    }
}

/// Expand enabled instance groups referenced by the scenario into concrete plans.
///
/// Disabled groups are skipped even if present in the top-level IR map. Groups
/// not referenced by `scenario.instance_groups` are not expanded; otherwise a
/// stray definition would secretly run itself, which is exactly how configs grow
/// legs and commit crimes.
pub fn expand_instance_groups(ir: &Ir) -> Result<Vec<InstancePlan>, RuntimeError> {
    let mut plans = Vec::new();

    for (group_index, group_id) in ir.scenario.instance_groups.iter().enumerate() {
        let group = ir.instance_groups.get(group_id).ok_or_else(|| {
            RuntimeError::invalid_runtime_plan(
                format!("scenario.{}.instance_groups[{group_index}]", ir.scenario.id),
                format!("instance_group `{group_id}` does not exist"),
            )
        })?;

        if !group.enabled {
            continue;
        }

        let machine = ir.machines.get(&group.machine).ok_or_else(|| {
            RuntimeError::invalid_runtime_plan(
                format!("instance_group.{group_id}.machine"),
                format!("machine `{}` does not exist", group.machine),
            )
        })?;

        for instance_index in 0..group.count {
            plans.push(InstancePlan::new(
                &ir.scenario.id,
                group_id,
                group_index,
                instance_index,
                &machine.system,
                &group.machine,
            ));
        }
    }

    Ok(plans)
}

/// Eagerly initialize cacheable context scopes for the expanded plans.
///
/// This is a small bridge for phase 4: it proves and preserves the intended
/// cache boundaries before machine execution exists. Event context is purposely
/// excluded because phase 3 defines it as per-emitted-event, not per-instance.
pub fn initialize_plan_contexts(
    ir: &Ir,
    plans: &[InstancePlan],
    store: &mut ContextStore<'_>,
    generator: &mut impl ContextValueGenerator,
) -> Result<(), RuntimeError> {
    for name in ir.context.global.keys() {
        store.global_value(name, generator)?;
    }

    for plan in plans {
        if let Some(system_vars) = ir.context.system.get(&plan.system_id) {
            for name in system_vars.keys() {
                store.system_value(&plan.system_id, name, generator)?;
            }
        }

        if let Some(instance_vars) = ir.context.instance.get(&plan.machine_id) {
            for name in instance_vars.keys() {
                store.instance_value(
                    &plan.system_id,
                    &plan.machine_id,
                    &plan.instance_id,
                    name,
                    generator,
                )?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::{Value, json};

    use super::*;
    use crate::ir::*;

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

    fn generator_var(id: &str) -> ContextVariable {
        var(ValueSource::Generator(GeneratorValue {
            generator: id.to_string(),
            params: JsonObject::new(),
        }))
    }

    fn test_ir() -> Ir {
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
                system: BTreeMap::from([(
                    "linux".to_string(),
                    BTreeMap::from([("host".to_string(), generator_var("g"))]),
                )]),
                instance: BTreeMap::from([(
                    "login".to_string(),
                    BTreeMap::from([("session".to_string(), generator_var("g"))]),
                )]),
                event: BTreeMap::new(),
            },
            templates: BTreeMap::new(),
            machines: BTreeMap::from([(
                "login".to_string(),
                Machine {
                    id: "login".to_string(),
                    system: "linux".to_string(),
                    start: "start".to_string(),
                    attack: AttackMetadata {
                        enabled: false,
                        attack_type: None,
                        technique: None,
                        tactic: None,
                        description: None,
                    },
                    states: BTreeMap::new(),
                    description: None,
                },
            )]),
            instance_groups: BTreeMap::from([
                (
                    "sessions".to_string(),
                    InstanceGroup {
                        id: "sessions".to_string(),
                        machine: "login".to_string(),
                        enabled: true,
                        count: 3,
                        params: None,
                        description: None,
                    },
                ),
                (
                    "disabled".to_string(),
                    InstanceGroup {
                        id: "disabled".to_string(),
                        machine: "login".to_string(),
                        enabled: false,
                        count: 9,
                        params: None,
                        description: None,
                    },
                ),
            ]),
            time_profiles: BTreeMap::new(),
            scenario: Scenario {
                id: "scenario".to_string(),
                systems: vec!["linux".to_string()],
                instance_groups: vec!["sessions".to_string(), "disabled".to_string()],
                time_profile: "t".to_string(),
                description: None,
            },
        }
    }

    #[test]
    fn expands_enabled_scenario_groups_into_stable_instance_plans() {
        let ir = test_ir();
        let plans = expand_instance_groups(&ir).expect("expand groups");

        assert_eq!(plans.len(), 3);
        assert_eq!(plans[0].instance_id, "sessions-0001");
        assert_eq!(plans[1].instance_id, "sessions-0002");
        assert_eq!(plans[2].instance_id, "sessions-0003");
        assert!(plans.iter().all(|plan| plan.system_id == "linux"));
        assert!(plans.iter().all(|plan| plan.machine_id == "login"));
        assert!(plans.iter().all(|plan| plan.group_id == "sessions"));
    }

    #[test]
    fn initializes_shared_global_and_system_but_isolated_instance_contexts() {
        let ir = test_ir();
        let plans = expand_instance_groups(&ir).expect("expand groups");
        let mut store = ContextStore::new(&ir);
        let mut generator = CountingGenerator::default();

        initialize_plan_contexts(&ir, &plans, &mut store, &mut generator).expect("init contexts");

        let global_calls = generator
            .calls
            .iter()
            .filter(|call| call.contains("context.global.tenant"))
            .count();
        let system_calls = generator
            .calls
            .iter()
            .filter(|call| call.contains("context.system.linux.host"))
            .count();
        let instance_calls = generator
            .calls
            .iter()
            .filter(|call| call.contains("context.instance.login.sessions-"))
            .count();

        assert_eq!(global_calls, 1);
        assert_eq!(system_calls, 1);
        assert_eq!(instance_calls, 3);

        let first = store
            .cached_instance_values("login", "sessions-0001")
            .and_then(|values| values.get("session"))
            .cloned()
            .expect("first instance session");
        let second = store
            .cached_instance_values("login", "sessions-0002")
            .and_then(|values| values.get("session"))
            .cloned()
            .expect("second instance session");

        assert_ne!(first, second);
    }

    #[test]
    fn reports_missing_scenario_group_as_runtime_plan_error() {
        let mut ir = test_ir();
        ir.scenario.instance_groups = vec!["ghost".to_string()];

        let err = expand_instance_groups(&ir).expect_err("missing group");

        assert!(matches!(err, RuntimeError::InvalidRuntimePlan { .. }));
    }
}
