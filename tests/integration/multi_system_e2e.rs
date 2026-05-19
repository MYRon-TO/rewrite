use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
};

use chrono::Duration;
use rewrite::{
    ir::Ir,
    runtime::{
        event::EventRecord,
        rng::RngManager,
        scenario::{ScenarioRunnerConfig, generate_scenario},
    },
    validate::{validate_drift, validate_identity, validate_machines, validate_references},
};
use serde_json::Value;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("multi_system.json")
}

fn load_validated_fixture() -> Ir {
    let path = fixture_path();
    let content = fs::read_to_string(&path).expect("fixture IR should be readable");
    let raw: Value = serde_json::from_str(&content).expect("fixture IR should be JSON");
    validate_drift(&raw).expect("fixture must stay inside MVP boundary");

    let ir = Ir::from_json_str(&content, path.display().to_string())
        .expect("fixture IR should deserialize");
    validate_identity(&ir).expect("fixture object identities should be valid");
    validate_references(&ir).expect("fixture references should be valid");
    validate_machines(&ir).expect("fixture machines should be valid");
    ir
}

#[test]
fn multi_system_fixture_runs_independent_systems_with_shared_global_context() {
    let ir = load_validated_fixture();
    let rng = RngManager::new(42);

    let generated = generate_scenario(&ir, &rng, ScenarioRunnerConfig::default())
        .expect("multi-system scenario should generate events");

    assert_eq!(generated.plans.len(), 4);
    assert_eq!(generated.events.len(), 4);
    assert_eq!(
        system_counts(&generated.events),
        BTreeMap::from([("linux_auth".to_string(), 2), ("web_access".to_string(), 2),])
    );

    let tenant_ids = string_field_values(&generated.events, "tenant_id");
    assert_eq!(
        tenant_ids.len(),
        1,
        "global tenant_id should be shared across systems"
    );
    assert_eq!(
        tenant_ids.iter().next().map(String::as_str),
        Some("tenant-multi-system")
    );

    let auth_events = events_for_system(&generated.events, "linux_auth");
    let web_events = events_for_system(&generated.events, "web_access");
    assert!(auth_events.iter().all(|event| {
        event.machine == "ssh_login_once"
            && event.template == "ssh_login_success"
            && event.fields["host"] == "ssh-edge-01"
            && event.fields.get("request_id").is_none()
            && !event.label.is_attack
    }));
    assert!(web_events.iter().all(|event| {
        event.machine == "web_request_once"
            && event.template == "http_access"
            && event.fields["host"] == "web-edge-01"
            && event.fields.get("session_id").is_none()
            && !event.label.is_attack
    }));

    let first_auth_time = auth_events[0].timestamp;
    let first_web_time = web_events[0].timestamp;
    assert_eq!(
        first_web_time - first_auth_time,
        Duration::milliseconds(250),
        "system clock skew is local per system; don't smuggle cross-system causality in here, tiny config gremlin"
    );
}

fn system_counts(events: &[EventRecord]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for event in events {
        *counts.entry(event.system.clone()).or_insert(0) += 1;
    }
    counts
}

fn string_field_values(events: &[EventRecord], field: &str) -> BTreeSet<String> {
    events
        .iter()
        .map(|event| {
            event.fields[field]
                .as_str()
                .unwrap_or_else(|| panic!("field `{field}` should be a string"))
                .to_owned()
        })
        .collect()
}

fn events_for_system<'a>(events: &'a [EventRecord], system: &str) -> Vec<&'a EventRecord> {
    events
        .iter()
        .filter(|event| event.system == system)
        .collect()
}
