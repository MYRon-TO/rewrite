use serde::{Deserialize, Serialize};

use crate::{
    ir::{AttackLabel, AttackMetadata, Id, Machine, State, StateKind},
    machine::runner::EventDraft,
};

/// Final attack label attached to an output event.
///
/// The resolver only looks at machine/state metadata. It does not inspect or
/// mutate rendered template fields; otherwise a harmless log line would become a
/// tiny policy engine wearing a fake moustache.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedAttackLabel {
    pub is_attack: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub attack_type: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub technique: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tactic: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub attack_state: Option<Id>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub attack_machine: Option<Id>,
}

/// Resolve an attack label for an emitted event draft.
///
/// Rules implemented here:
/// - `state.kind == attack` sets `is_attack = true`.
/// - `state.kind == benign` sets `is_attack = false`.
/// - `state.attack_label` fields override `machine.attack` fields.
/// - `attack_state` and `attack_machine` are emitted for attack events.
pub fn resolve_attack_label(
    machine: &Machine,
    state_id: &str,
    state: &State,
) -> ResolvedAttackLabel {
    let is_attack = state.kind == StateKind::Attack;

    if !is_attack {
        return ResolvedAttackLabel::benign();
    }

    let source = LabelSource::new(&machine.attack, state.attack_label.as_ref());
    ResolvedAttackLabel {
        is_attack: true,
        attack_type: source.attack_type(),
        technique: source.technique(),
        tactic: source.tactic(),
        name: source.name(),
        description: source.description(),
        attack_state: Some(state_id.to_owned()),
        attack_machine: Some(machine.id.clone()),
    }
}

/// Resolve an attack label using the state referenced by an event draft.
pub fn resolve_attack_label_for_draft(
    machine: &Machine,
    draft: &EventDraft,
) -> Result<ResolvedAttackLabel, AttackLabelResolveError> {
    let state = machine.states.get(&draft.state_id).ok_or_else(|| {
        AttackLabelResolveError::missing_state(machine.id.clone(), draft.state_id.clone())
    })?;

    Ok(resolve_attack_label(machine, &draft.state_id, state))
}

impl ResolvedAttackLabel {
    pub fn benign() -> Self {
        Self {
            is_attack: false,
            attack_type: None,
            technique: None,
            tactic: None,
            name: None,
            description: None,
            attack_state: None,
            attack_machine: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttackLabelResolveError {
    pub machine_id: Id,
    pub state_id: Id,
}

impl AttackLabelResolveError {
    fn missing_state(machine_id: Id, state_id: Id) -> Self {
        Self {
            machine_id,
            state_id,
        }
    }
}

impl std::fmt::Display for AttackLabelResolveError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "machine `{}` has no state `{}` for attack label resolution",
            self.machine_id, self.state_id
        )
    }
}

impl std::error::Error for AttackLabelResolveError {}

struct LabelSource<'a> {
    machine: &'a AttackMetadata,
    state: Option<&'a AttackLabel>,
}

impl<'a> LabelSource<'a> {
    fn new(machine: &'a AttackMetadata, state: Option<&'a AttackLabel>) -> Self {
        Self { machine, state }
    }

    fn attack_type(&self) -> Option<String> {
        self.state
            .and_then(|label| label.attack_type.clone())
            .or_else(|| self.machine.attack_type.clone())
    }

    fn technique(&self) -> Option<String> {
        self.state
            .and_then(|label| label.technique.clone())
            .or_else(|| self.machine.technique.clone())
    }

    fn tactic(&self) -> Option<String> {
        self.state
            .and_then(|label| label.tactic.clone())
            .or_else(|| self.machine.tactic.clone())
    }

    fn name(&self) -> Option<String> {
        self.state
            .and_then(|label| label.name.clone())
            .or_else(|| self.machine.name.clone())
    }

    fn description(&self) -> Option<String> {
        self.state
            .and_then(|label| label.description.clone())
            .or_else(|| self.machine.description.clone())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use chrono::{TimeZone, Utc};
    use serde_json::Value;

    use super::*;
    use crate::{
        context::{ContextStore, ContextValueGenerator},
        error::{Result, RuntimeError},
        ir::*,
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
            unreachable!("label tests do not use generator-backed context")
        }
    }

    fn machine_attack() -> AttackMetadata {
        AttackMetadata {
            enabled: true,
            attack_type: Some("credential_access".to_owned()),
            technique: Some("T1110".to_owned()),
            tactic: Some("Credential Access".to_owned()),
            name: Some("Brute Force".to_owned()),
            description: Some("many failed logins".to_owned()),
        }
    }

    fn state(kind: StateKind, attack_label: Option<AttackLabel>) -> State {
        State {
            kind,
            attack_label,
            emit: vec![Emit {
                template: "shared_template".to_owned(),
                count: EmitCount::Fixed(1),
            }],
            transitions: vec![],
            terminal: true,
            description: None,
        }
    }

    fn empty_ir() -> Ir {
        Ir {
            ir_version: "0.1".to_owned(),
            systems: ObjectMap::new(),
            generators: ObjectMap::new(),
            context: Context::default(),
            templates: ObjectMap::new(),
            machines: ObjectMap::new(),
            instance_groups: ObjectMap::new(),
            time_profiles: ObjectMap::new(),
            scenario: Scenario {
                id: "s".to_owned(),
                systems: vec![],
                instance_groups: vec![],
                time_profile: "t".to_owned(),
                description: None,
            },
        }
    }

    fn test_machine() -> Machine {
        Machine {
            id: "ssh_login_flow".to_owned(),
            system: "linux_auth".to_owned(),
            start: "benign".to_owned(),
            attack: machine_attack(),
            states: BTreeMap::from([
                ("benign".to_owned(), state(StateKind::Benign, None)),
                ("bruteforce".to_owned(), state(StateKind::Attack, None)),
                (
                    "compromised_login".to_owned(),
                    state(
                        StateKind::Attack,
                        Some(AttackLabel {
                            attack_type: None,
                            technique: Some("T1078".to_owned()),
                            tactic: Some("Defense Evasion".to_owned()),
                            name: Some("Valid Accounts".to_owned()),
                            description: None,
                        }),
                    ),
                ),
            ]),
            description: None,
        }
    }

    fn draft_for_state(state_id: &str, state_kind: StateKind) -> EventDraft {
        let ir = empty_ir();
        let mut store = ContextStore::new(&ir);
        let mut generator = NoopGenerator;
        let event_context = store
            .event_context("linux_auth", "ssh_login_flow", "i-1", &mut generator)
            .unwrap();

        EventDraft {
            scenario_id: "s".to_owned(),
            system_id: "linux_auth".to_owned(),
            instance_id: "i-1".to_owned(),
            machine_id: "ssh_login_flow".to_owned(),
            state_id: state_id.to_owned(),
            state_kind,
            template_id: "shared_template".to_owned(),
            timestamp: Utc.with_ymd_and_hms(2026, 5, 17, 0, 0, 0).single().unwrap(),
            event_context,
        }
    }

    #[test]
    fn benign_state_is_never_attack_and_has_no_attack_metadata() {
        let machine = test_machine();
        let label = resolve_attack_label(&machine, "benign", &machine.states["benign"]);

        assert_eq!(label, ResolvedAttackLabel::benign());
    }

    #[test]
    fn attack_state_inherits_machine_attack_metadata() {
        let machine = test_machine();
        let label = resolve_attack_label(&machine, "bruteforce", &machine.states["bruteforce"]);

        assert!(label.is_attack);
        assert_eq!(label.attack_type.as_deref(), Some("credential_access"));
        assert_eq!(label.technique.as_deref(), Some("T1110"));
        assert_eq!(label.tactic.as_deref(), Some("Credential Access"));
        assert_eq!(label.attack_state.as_deref(), Some("bruteforce"));
        assert_eq!(label.attack_machine.as_deref(), Some("ssh_login_flow"));
    }

    #[test]
    fn state_attack_label_overrides_machine_metadata_field_by_field() {
        let machine = test_machine();
        let label = resolve_attack_label(
            &machine,
            "compromised_login",
            &machine.states["compromised_login"],
        );

        assert!(label.is_attack);
        assert_eq!(label.attack_type.as_deref(), Some("credential_access"));
        assert_eq!(label.technique.as_deref(), Some("T1078"));
        assert_eq!(label.tactic.as_deref(), Some("Defense Evasion"));
        assert_eq!(label.name.as_deref(), Some("Valid Accounts"));
        assert_eq!(label.description.as_deref(), Some("many failed logins"));
    }

    #[test]
    fn same_template_can_have_different_labels_in_benign_and_attack_states() {
        let machine = test_machine();
        let benign = resolve_attack_label(&machine, "benign", &machine.states["benign"]);
        let attack = resolve_attack_label(&machine, "bruteforce", &machine.states["bruteforce"]);

        assert_eq!(machine.states["benign"].emit[0].template, "shared_template");
        assert_eq!(
            machine.states["bruteforce"].emit[0].template,
            "shared_template"
        );
        assert!(!benign.is_attack);
        assert!(attack.is_attack);
        assert_ne!(benign, attack);
    }

    #[test]
    fn resolves_label_for_event_draft_state() {
        let machine = test_machine();
        let draft = draft_for_state("bruteforce", StateKind::Attack);

        let label = resolve_attack_label_for_draft(&machine, &draft).unwrap();

        assert!(label.is_attack);
        assert_eq!(label.technique.as_deref(), Some("T1110"));
    }

    #[test]
    fn draft_resolution_reports_missing_state() {
        let machine = test_machine();
        let draft = draft_for_state("missing", StateKind::Attack);

        let err = resolve_attack_label_for_draft(&machine, &draft).unwrap_err();

        assert_eq!(err.machine_id, "ssh_login_flow");
        assert_eq!(err.state_id, "missing");
    }
}
