use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
};

use rewrite::{
    ir::Ir,
    output::jsonl::write_events_jsonl,
    runtime::{
        rng::RngManager,
        scenario::{ScenarioRunnerConfig, generate_scenario},
    },
    validate::{
        validate_drift, validate_generators_with_asset_root, validate_identity, validate_machines,
        validate_references,
    },
};
use serde_json::Value;

const IR: &str = "generated/examples/credential_generators.json";
const GOLDEN: &str = "tests/fixtures/golden/credential_generators_seed_42.jsonl";

fn load_fixture() -> Ir {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = root.join(IR);
    let content = fs::read_to_string(&path).expect("credential generator IR should be readable");
    let raw: Value =
        serde_json::from_str(&content).expect("credential generator IR should be JSON");
    validate_drift(&raw).expect("fixture should remain within the definition boundary");
    let ir = Ir::from_json_str(&content, path.display().to_string())
        .expect("fixture should deserialize");
    validate_identity(&ir).expect("fixture identities should be valid");
    validate_references(&ir).expect("fixture references should be valid");
    validate_machines(&ir).expect("fixture machines should be valid");
    validate_generators_with_asset_root(&ir, &root)
        .expect("fixture generators and built-in corpus should be valid");
    ir
}

#[test]
fn credential_generators_cover_profiles_context_lifecycles_and_object_fields() {
    let ir = load_fixture();
    let generated = generate_scenario(&ir, &RngManager::new(42), ScenarioRunnerConfig::default())
        .expect("credential generator scenario should run");

    assert_eq!(generated.events.len(), 4);
    assert_eq!(
        generated
            .events
            .iter()
            .filter(|event| event.label.is_attack)
            .count(),
        2
    );
    assert_eq!(
        generated
            .events
            .iter()
            .filter(|event| !event.label.is_attack)
            .count(),
        2
    );

    let values = |field: &str| {
        generated
            .events
            .iter()
            .map(|event| event.fields[field].as_str().unwrap().to_owned())
            .collect::<BTreeSet<_>>()
    };
    assert_eq!(
        values("global_hex").len(),
        1,
        "global context must be shared"
    );
    assert_eq!(
        values("system_base64").len(),
        1,
        "system context must be shared"
    );
    assert_eq!(
        values("uuid_secret").len(),
        4,
        "event context must be regenerated per event"
    );

    let mut instances: BTreeMap<&str, Vec<_>> = BTreeMap::new();
    for event in &generated.events {
        instances.entry(&event.instance).or_default().push(event);
    }
    assert_eq!(instances.len(), 2);
    for events in instances.values() {
        assert_eq!(events.len(), 2);
        for field in [
            "human_password",
            "api_key",
            "access_key_id",
            "secret_access_key",
        ] {
            assert_eq!(
                events[0].fields[field], events[1].fields[field],
                "instance field {field} must be stable"
            );
        }
    }
    assert_eq!(
        values("api_key").len(),
        2,
        "instance context must be isolated"
    );

    let row = &generated.events[0].fields;
    assert_eq!(row["global_hex"].as_str().unwrap().len(), 32);
    assert_eq!(row["system_base64"].as_str().unwrap().len(), 32);
    assert!(
        row["system_base64"]
            .as_str()
            .unwrap()
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '+' | '/' | '='))
    );
    assert_eq!(row["base64url_secret"].as_str().unwrap().len(), 32);
    assert_eq!(row["uuid_secret"].as_str().unwrap().split('-').count(), 5);
    assert!(row["api_key"].as_str().unwrap().starts_with("sim_sk_"));
    assert!(
        row["bearer_token"]
            .as_str()
            .unwrap()
            .starts_with("sim_bearer_")
    );
    assert_eq!(row["jwt"].as_str().unwrap().split('.').count(), 3);
    assert!(
        row["pem"]
            .as_str()
            .unwrap()
            .starts_with("-----BEGIN SIMULATED PRIVATE KEY-----\n")
    );
    assert!(
        row["cloud_access_key"]
            .as_str()
            .unwrap()
            .starts_with("SIMAKIA")
    );
    assert!(
        row["access_key_id"]
            .as_str()
            .unwrap()
            .starts_with("SIMAKIA")
    );
    assert_eq!(row["secret_access_key"].as_str().unwrap().len(), 40);
    assert!(
        row["connection"]
            .as_str()
            .unwrap()
            .starts_with("postgresql://sim_user:")
    );
    assert!(
        row["private_token"]
            .as_str()
            .unwrap()
            .starts_with("sim_tok_")
    );
    assert!(
        ["123456", "password", "admin", "qwerty"].contains(&row["weak_password"].as_str().unwrap())
    );
    assert!(!row["human_password"].as_str().unwrap().is_empty());
}

#[test]
fn credential_generators_seed_42_matches_golden_jsonl() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let generated = generate_scenario(
        &load_fixture(),
        &RngManager::new(42),
        ScenarioRunnerConfig::default(),
    )
    .expect("credential generator scenario should run");
    let out = std::env::temp_dir().join(format!(
        "rewrite-credential-generators-{}.jsonl",
        std::process::id()
    ));
    write_events_jsonl(&out, generated.events.iter()).expect("JSONL should be writable");
    let actual = fs::read_to_string(&out).expect("generated JSONL should be readable");
    let _ = fs::remove_file(out);
    let expected = fs::read_to_string(root.join(GOLDEN)).expect("golden JSONL should be readable");
    assert_eq!(actual, expected, "seed 42 credential output drifted");
}
