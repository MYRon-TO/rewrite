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
use serde_json::Value;

const SSH_IR: &str = "generated/examples/ssh_bruteforce.json";
const SSH_GOLDEN_SEED_42: &str = "tests/fixtures/golden/ssh_bruteforce_seed_42.jsonl";

#[test]
fn ssh_bruteforce_seed_42_matches_golden_jsonl() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let ir_path = manifest_dir.join(SSH_IR);
    let golden_path = manifest_dir.join(SSH_GOLDEN_SEED_42);
    let out_path = std::env::temp_dir().join(format!(
        "rewrite-golden-ssh-{}-{}.jsonl",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ));

    let ir = load_validated_ir(&ir_path);
    let generated = generate_scenario(&ir, &RngManager::new(42), ScenarioRunnerConfig::default())
        .expect("fixed IR and seed should generate events");

    write_events_jsonl(&out_path, generated.events.iter()).expect("golden candidate should write");

    let actual = fs::read_to_string(&out_path).expect("generated JSONL should be readable");
    let _ = fs::remove_file(&out_path);
    let expected = fs::read_to_string(&golden_path).expect("golden JSONL should be readable");

    assert_jsonl_contract(&actual);
    assert_eq!(actual, expected, "seed 42 output drifted from golden file");
}

fn load_validated_ir(path: &std::path::Path) -> Ir {
    let content = fs::read_to_string(path).expect("IR fixture should be readable");
    let raw: Value = serde_json::from_str(&content).expect("IR fixture should be JSON");
    validate_drift(&raw).expect("IR fixture should pass drift validation");

    let ir = Ir::from_json_str(&content, path.display().to_string()).expect("IR fixture parses");
    validate_identity(&ir).expect("IR fixture identity is valid");
    validate_references(&ir).expect("IR fixture references are valid");
    validate_machines(&ir).expect("IR fixture machines are valid");
    ir
}

fn assert_jsonl_contract(content: &str) {
    let mut total = 0usize;
    let mut benign = 0usize;
    let mut attack = 0usize;

    assert!(
        content.ends_with('\n'),
        "JSONL output must end with a newline"
    );

    for (index, line) in content.lines().enumerate() {
        total += 1;
        let value: Value = serde_json::from_str(line)
            .unwrap_or_else(|err| panic!("line {index} is not valid JSON: {err}"));

        for field in [
            "timestamp",
            "scenario",
            "system",
            "instance",
            "machine",
            "state",
            "template",
            "fields",
            "label",
        ] {
            assert!(value.get(field).is_some(), "line {index} missing `{field}`");
        }

        match value["label"]["is_attack"].as_bool() {
            Some(true) => attack += 1,
            Some(false) => benign += 1,
            None => panic!("line {index} missing boolean label.is_attack"),
        }
    }

    assert!(total > 0, "golden candidate should contain events");
    assert!(benign > 0, "golden candidate should contain benign events");
    assert!(attack > 0, "golden candidate should contain attack events");
}
