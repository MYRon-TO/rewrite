use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    ir::{Id, JsonObject},
    label::attack::ResolvedAttackLabel,
    machine::runner::EventDraft,
};

/// Stable metadata copied from the runtime draft into the final output event.
///
/// Keep these names boring and explicit: JSONL consumers should not need a
/// treasure map to find `scenario`, `system`, or `template`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventMetadata {
    pub scenario: Id,
    pub system: Id,
    pub instance: Id,
    pub machine: Id,
    pub state: Id,
    pub template: Id,
}

impl EventMetadata {
    pub fn new(
        scenario: impl Into<Id>,
        system: impl Into<Id>,
        instance: impl Into<Id>,
        machine: impl Into<Id>,
        state: impl Into<Id>,
        template: impl Into<Id>,
    ) -> Self {
        Self {
            scenario: scenario.into(),
            system: system.into(),
            instance: instance.into(),
            machine: machine.into(),
            state: state.into(),
            template: template.into(),
        }
    }
}

impl From<&EventDraft> for EventMetadata {
    fn from(draft: &EventDraft) -> Self {
        Self {
            scenario: draft.scenario_id.clone(),
            system: draft.system_id.clone(),
            instance: draft.instance_id.clone(),
            machine: draft.machine_id.clone(),
            state: draft.state_id.clone(),
            template: draft.template_id.clone(),
        }
    }
}

/// Final event record written by output backends.
///
/// Shape intentionally follows the phase-5 JSONL contract:
/// `timestamp`, stable runtime identity fields, rendered `fields`, and an
/// always-present `label`. Do not flatten business fields into the top level;
/// that would invite collisions like `fields.state` vs runtime `state`, which
/// is the kind of tiny foot-gun that later becomes a conference talk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventRecord {
    pub timestamp: DateTime<Utc>,
    pub scenario: Id,
    pub system: Id,
    pub instance: Id,
    pub machine: Id,
    pub state: Id,
    pub template: Id,
    pub fields: JsonObject,
    pub label: ResolvedAttackLabel,
}

/// Assemble a final event from rendered fields, resolved label, metadata and
/// timestamp.
pub fn assemble_event(
    fields: JsonObject,
    label: ResolvedAttackLabel,
    metadata: EventMetadata,
    timestamp: DateTime<Utc>,
) -> EventRecord {
    EventRecord {
        timestamp,
        scenario: metadata.scenario,
        system: metadata.system,
        instance: metadata.instance,
        machine: metadata.machine,
        state: metadata.state,
        template: metadata.template,
        fields,
        label,
    }
}

/// Convenience assembler for the common runtime path where metadata and
/// timestamp both come from an `EventDraft`.
pub fn assemble_event_from_draft(
    draft: &EventDraft,
    fields: JsonObject,
    label: ResolvedAttackLabel,
) -> EventRecord {
    assemble_event(fields, label, EventMetadata::from(draft), draft.timestamp)
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use serde_json::{Value, json};

    use super::*;
    use crate::{
        context::{ContextStore, ContextValueGenerator},
        error::{Result, RuntimeError},
        ir::*,
        machine::runner::EventDraft,
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
            unreachable!("event assembler tests do not use generator-backed context")
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
                id: "scenario".to_owned(),
                systems: vec![],
                instance_groups: vec![],
                time_profile: "time".to_owned(),
                description: None,
            },
        }
    }

    fn draft() -> EventDraft {
        let ir = empty_ir();
        let mut store = ContextStore::new(&ir);
        let mut generator = NoopGenerator;
        let event_context = store
            .event_context(
                "linux_auth",
                "ssh_login_flow",
                "sessions-0001",
                &mut generator,
            )
            .unwrap();

        EventDraft {
            scenario_id: "ssh_bruteforce_demo".to_owned(),
            system_id: "linux_auth".to_owned(),
            instance_id: "sessions-0001".to_owned(),
            machine_id: "ssh_login_flow".to_owned(),
            state_id: "bruteforce".to_owned(),
            state_kind: StateKind::Attack,
            template_id: "ssh_login_failed".to_owned(),
            timestamp: Utc.with_ymd_and_hms(2026, 5, 17, 0, 0, 0).single().unwrap(),
            event_context,
        }
    }

    fn attack_label() -> ResolvedAttackLabel {
        ResolvedAttackLabel {
            is_attack: true,
            attack_type: Some("credential_access".to_owned()),
            technique: Some("T1110".to_owned()),
            tactic: Some("Credential Access".to_owned()),
            name: Some("Brute Force".to_owned()),
            attack_state: Some("bruteforce".to_owned()),
            attack_machine: Some("ssh_login_flow".to_owned()),
        }
    }

    #[test]
    fn assembles_record_with_stable_runtime_metadata() {
        let draft = draft();
        let fields = JsonObject::from([
            ("user".to_owned(), json!("root")),
            ("result".to_owned(), json!("failure")),
        ]);

        let event = assemble_event_from_draft(&draft, fields.clone(), attack_label());

        assert_eq!(event.timestamp, draft.timestamp);
        assert_eq!(event.scenario, "ssh_bruteforce_demo");
        assert_eq!(event.system, "linux_auth");
        assert_eq!(event.instance, "sessions-0001");
        assert_eq!(event.machine, "ssh_login_flow");
        assert_eq!(event.state, "bruteforce");
        assert_eq!(event.template, "ssh_login_failed");
        assert_eq!(event.fields, fields);
        assert!(event.label.is_attack);
    }

    #[test]
    fn serializes_to_contract_field_names_without_flattening_business_fields() {
        let metadata = EventMetadata::new(
            "scenario", "system", "instance", "machine", "state", "template",
        );
        let fields = JsonObject::from([("state".to_owned(), json!("business-state"))]);
        let timestamp = Utc.with_ymd_and_hms(2026, 5, 17, 1, 2, 3).single().unwrap();

        let event = assemble_event(fields, ResolvedAttackLabel::benign(), metadata, timestamp);
        let value = serde_json::to_value(&event).unwrap();

        assert_eq!(value["timestamp"], json!("2026-05-17T01:02:03Z"));
        assert_eq!(value["scenario"], json!("scenario"));
        assert_eq!(value["system"], json!("system"));
        assert_eq!(value["instance"], json!("instance"));
        assert_eq!(value["machine"], json!("machine"));
        assert_eq!(value["state"], json!("state"));
        assert_eq!(value["template"], json!("template"));
        assert_eq!(value["fields"]["state"], json!("business-state"));
        assert_eq!(value["label"]["is_attack"], json!(false));
    }
}
