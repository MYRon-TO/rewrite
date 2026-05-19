use std::{fs, path::PathBuf};

use rewrite::{
    ir::Ir,
    output::jsonl::write_events_jsonl,
    runtime::{
        rng::RngManager,
        scenario::{ScenarioRunnerConfig, generate_scenario},
    },
    validate::{validate_drift, validate_identity, validate_machines, validate_references},
};
use serde_json::{Value, json};

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("ssh_bruteforce.json")
}

fn temp_output_path() -> PathBuf {
    std::env::temp_dir().join(format!(
        "rewrite-ssh-bruteforce-e2e-{}-{}.jsonl",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ))
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
fn ssh_bruteforce_fixture_generates_benign_and_attack_jsonl() {
    let ir = load_validated_fixture();
    let rng = RngManager::new(42);

    let generated = generate_scenario(&ir, &rng, ScenarioRunnerConfig::default())
        .expect("SSH brute-force scenario should generate events");

    assert!(!generated.events.is_empty());
    assert!(
        generated.events.iter().any(|event| !event.label.is_attack),
        "generated logs must include benign events"
    );
    assert!(
        generated.events.iter().any(|event| event.label.is_attack),
        "generated logs must include attack events; if this breaks, don't blame probability, pin the fixture or seed"
    );
    assert!(
        generated.events.iter().any(|event| {
            event.label.is_attack && event.label.technique.as_deref() == Some("T1110")
        }),
        "brute-force attack events should carry ATT&CK technique T1110"
    );
    assert!(
        generated.events.iter().any(|event| {
            event.label.is_attack
                && event.state == "attack_success"
                && event.label.technique.as_deref() == Some("T1078")
        }),
        "attack_success should demonstrate state-level label override to T1078"
    );

    let out = temp_output_path();
    write_events_jsonl(&out, generated.events.iter()).expect("JSONL output should be writable");
    let content = fs::read_to_string(&out).expect("JSONL output should be readable");
    let _ = fs::remove_file(&out);

    let rows = content
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("each JSONL line must be JSON"))
        .collect::<Vec<_>>();

    assert_eq!(rows.len(), generated.events.len());
    assert!(
        rows.iter()
            .any(|row| row["label"]["is_attack"] == json!(false))
    );
    assert!(
        rows.iter()
            .any(|row| row["label"]["is_attack"] == json!(true))
    );
    assert!(rows.iter().any(|row| {
        row["scenario"] == json!("ssh_bruteforce_demo")
            && row["system"] == json!("linux_auth")
            && row["machine"] == json!("ssh_login_flow")
            && row["fields"]["message"].is_string()
    }));
}
