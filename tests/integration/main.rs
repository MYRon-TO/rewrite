use std::{fs, path::PathBuf};

use rewrite::{
    error::RewriteError,
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
fn invalid_fixtures_fail_with_locatable_errors() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let cases: &[(&str, &[&str])] = &[
        (
            "tests/fixtures/invalid/missing_template.json",
            &[
                "missing template reference `missing_template`",
                "machine.m.states.a.emit[0].template",
            ],
        ),
        (
            "tests/fixtures/invalid/missing_state.json",
            &[
                "invalid machine `m`",
                "state `a` transition[0].to references missing state `missing_state`",
            ],
        ),
        (
            "tests/fixtures/invalid/attack_disabled_has_attack_state.json",
            &[
                "invalid machine `m`",
                "attack.enabled is false",
                "state `a`",
            ],
        ),
        (
            "tests/fixtures/invalid/benign_state_attack_label.json",
            &["invalid machine `m`", "benign state `a`", "attack_label"],
        ),
        (
            "tests/fixtures/invalid/transition_weight_zero.json",
            &[
                "invalid machine `m`",
                "state `a` transition[0].weight",
                "greater than 0",
            ],
        ),
        (
            "tests/fixtures/invalid/emit_count_range_reversed.json",
            &[
                "invalid machine `m`",
                "state `a` emit[0].count",
                "min must be <= max",
            ],
        ),
        (
            "tests/fixtures/invalid/top_level_attack_chain.json",
            &[
                "definition boundary drift",
                "field `attack_chain`",
                "cross-machine attack chains",
            ],
        ),
    ];

    for (relative_path, expected_substrings) in cases {
        let path = manifest_dir.join(relative_path);
        let err = match validate_ir_file(&path) {
            Ok(_) => panic!("invalid fixture `{relative_path}` unexpectedly passed validation"),
            Err(err) => err,
        };
        let message = err.to_string();

        for expected in *expected_substrings {
            assert!(
                message.contains(expected),
                "fixture `{relative_path}` error did not contain `{expected}`; actual: {message}"
            );
        }
    }
}

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
    validate_ir_file(path).expect("IR fixture should be valid")
}

fn validate_ir_file(path: &std::path::Path) -> Result<Ir, RewriteError> {
    let content = fs::read_to_string(path)
        .map_err(|source| rewrite::error::IrParseError::read(path.display().to_string(), source))?;
    let raw: Value = serde_json::from_str(&content)
        .map_err(|source| rewrite::error::IrParseError::json(path.display().to_string(), source))?;
    validate_drift(&raw)?;

    let ir = Ir::from_json_str(&content, path.display().to_string())?;
    validate_identity(&ir)?;
    validate_references(&ir)?;
    validate_machines(&ir)?;
    Ok(ir)
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
