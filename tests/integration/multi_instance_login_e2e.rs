use std::{collections::BTreeSet, fs, path::PathBuf};

use rewrite::{
    ir::Ir,
    runtime::{
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
        .join("multi_instance_login.json")
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
fn multi_instance_fixture_shares_global_and_system_but_not_instance_context() {
    let ir = load_validated_fixture();
    let rng = RngManager::new(42);

    let generated = generate_scenario(&ir, &rng, ScenarioRunnerConfig::default())
        .expect("multi-instance login scenario should generate events");

    assert_eq!(generated.plans.len(), 3);
    assert_eq!(generated.events.len(), 3);
    assert_eq!(
        generated
            .plans
            .iter()
            .map(|plan| plan.instance_id.as_str())
            .collect::<Vec<_>>(),
        vec![
            "ssh_login_sessions-0001",
            "ssh_login_sessions-0002",
            "ssh_login_sessions-0003"
        ]
    );

    let tenant_ids = string_field_values(&generated.events, "tenant_id");
    let hosts = string_field_values(&generated.events, "host");
    let session_ids = string_field_values(&generated.events, "session_id");

    assert_eq!(tenant_ids.len(), 1, "global tenant_id should be shared");
    assert_eq!(
        tenant_ids.iter().next().map(String::as_str),
        Some("tenant-shared")
    );
    assert_eq!(hosts.len(), 1, "system hostname should be shared");
    assert_eq!(
        hosts.iter().next().map(String::as_str),
        Some("ssh-shared-01")
    );
    assert_eq!(
        session_ids.len(),
        3,
        "each instance must have its own session_id; sharing it would be config clown-car behavior"
    );

    assert!(generated.events.iter().all(|event| {
        !event.label.is_attack
            && event.scenario == "multi_instance_login_demo"
            && event.system == "linux_auth"
            && event.machine == "ssh_login_once"
            && event.template == "ssh_login_success"
            && event.fields["message"].is_string()
    }));
}

fn string_field_values(
    events: &[rewrite::runtime::event::EventRecord],
    field: &str,
) -> BTreeSet<String> {
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
