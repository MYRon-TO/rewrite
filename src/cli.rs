use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

use clap::{Arg, ArgAction, Command, value_parser};

use crate::{
    error::{IrParseError, Result},
    ir::Ir,
    machine::runner::MachineRunnerConfig,
    output::{csv::write_events_csv, jsonl::write_events_jsonl},
    runtime::{
        rng::{DEFAULT_SEED, RngManager},
        scenario::{ScenarioRunnerConfig, generate_scenario},
    },
    validate::{
        MachineValidationWarning, validate_drift, validate_identity, validate_machines,
        validate_references,
    },
};

pub fn run() -> Result<()> {
    run_with_args(std::env::args_os())
}

pub fn run_with_args<I, T>(args: I) -> Result<()>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let matches = command().get_matches_from(args);

    match matches.subcommand() {
        Some(("validate", subcommand)) => {
            let ir_path = subcommand
                .get_one::<PathBuf>("ir")
                .expect("clap requires --ir")
                .clone();
            validate_command(ir_path)
        }
        Some(("generate", subcommand)) => {
            let ir_path = subcommand
                .get_one::<PathBuf>("ir")
                .expect("clap requires --ir")
                .clone();
            let out_path = subcommand
                .get_one::<PathBuf>("out")
                .expect("clap requires --out")
                .clone();
            let format = subcommand
                .get_one::<String>("format")
                .map(|value| OutputFormat::parse(value))
                .transpose()?
                .unwrap_or(OutputFormat::Jsonl);
            let seed = subcommand.get_one::<u64>("seed").copied();
            let enable_attack = subcommand
                .get_one::<bool>("enable-attack")
                .copied()
                .unwrap_or(true);
            generate_command(ir_path, out_path, format, seed, enable_attack)
        }
        _ => unreachable!("clap requires a known subcommand"),
    }
}

fn command() -> Command {
    Command::new("rewrite")
        .about("Log generator CLI")
        .version(env!("CARGO_PKG_VERSION"))
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("validate")
                .about(
                    "Read and statically validate a generated JSON IR file; does not generate logs",
                )
                .arg(ir_arg()),
        )
        .subcommand(
            Command::new("generate")
                .about("Generate JSON Lines logs from a generated JSON IR file")
                .arg(ir_arg())
                .arg(
                    Arg::new("out")
                        .long("out")
                        .value_name("PATH")
                        .help("Output JSONL file path, for example out/events.jsonl")
                        .required(true)
                        .action(ArgAction::Set)
                        .value_parser(value_parser!(PathBuf)),
                )
                .arg(
                    Arg::new("format")
                        .long("format")
                        .value_name("FORMAT")
                        .help("Output format: jsonl writes full event records; csv writes only logs and attack labels.")
                        .required(false)
                        .default_value("jsonl")
                        .action(ArgAction::Set)
                        .value_parser(["jsonl", "csv"]),
                )
                .arg(
                    Arg::new("seed")
                        .long("seed")
                        .value_name("U64")
                        .help("Root RNG seed. If omitted, a fixed reproducible default is used.")
                        .required(false)
                        .action(ArgAction::Set)
                        .value_parser(value_parser!(u64)),
                )
                .arg(
                    Arg::new("enable-attack")
                        .long("enable-attack")
                        .value_name("BOOL")
                        .help("Whether generation may enter attack states. Set to false for clean mode; defaults to true.")
                        .required(false)
                        .default_value("true")
                        .action(ArgAction::Set)
                        .value_parser(value_parser!(bool)),
                )
        )
}

fn ir_arg() -> Arg {
    Arg::new("ir")
        .long("ir")
        .value_name("PATH")
        .help("Path to generated JSON IR, for example generated/examples/ssh_bruteforce.json")
        .required(true)
        .action(ArgAction::Set)
        .value_parser(value_parser!(PathBuf))
}

fn validate_command(ir_path: PathBuf) -> Result<()> {
    let _ir = load_and_validate_ir(&ir_path)?;

    println!("IR validation passed: {}", ir_path.display());

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    Jsonl,
    Csv,
}

impl OutputFormat {
    fn parse(value: &str) -> std::result::Result<Self, IrParseError> {
        match value {
            "jsonl" => Ok(Self::Jsonl),
            "csv" => Ok(Self::Csv),
            other => Err(IrParseError::invalid(
                "generate.--format",
                format!("unsupported output format `{other}`; expected `jsonl` or `csv`"),
            )),
        }
    }
}

fn generate_command(
    ir_path: PathBuf,
    out_path: PathBuf,
    format: OutputFormat,
    seed: Option<u64>,
    enable_attack: bool,
) -> Result<()> {
    let ir = load_and_validate_ir(&ir_path)?;
    let rng_manager = RngManager::from_cli_seed(seed);
    let config = ScenarioRunnerConfig {
        machine: MachineRunnerConfig {
            enable_attack,
            ..MachineRunnerConfig::default()
        },
    };
    let generated = generate_scenario(&ir, &rng_manager, config)?;

    match format {
        OutputFormat::Jsonl => write_events_jsonl(&out_path, generated.events.iter())?,
        OutputFormat::Csv => write_events_csv(&out_path, generated.events.iter())?,
    }

    println!(
        "Generated {} events from {} to {} as {:?} with seed {} and enable_attack={}",
        generated.events.len(),
        ir_path.display(),
        out_path.display(),
        format,
        seed.unwrap_or(DEFAULT_SEED),
        enable_attack
    );

    Ok(())
}

fn load_and_validate_ir(ir_path: &Path) -> Result<Ir> {
    let ir_content = read_json_ir_content(ir_path)?;
    let raw_ir = serde_json::from_str(&ir_content)
        .map_err(|source| IrParseError::json(ir_path.display().to_string(), source))?;

    validate_drift(&raw_ir)?;

    let ir = Ir::from_json_str(&ir_content, ir_path.display().to_string())?;

    validate_identity(&ir)?;
    validate_references(&ir)?;
    let report = validate_machines(&ir)?;

    for warning in report.warnings {
        eprintln!("warning: {}", format_machine_warning(&warning));
    }

    Ok(ir)
}

fn read_json_ir_content(path: &Path) -> std::result::Result<String, IrParseError> {
    if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
        return Err(IrParseError::unsupported_format(
            path.display().to_string(),
            "JSON IR (*.json)",
        ));
    }

    fs::read_to_string(path)
        .map_err(|source| IrParseError::read(path.display().to_string(), source))
}

fn format_machine_warning(warning: &MachineValidationWarning) -> String {
    match warning {
        MachineValidationWarning::UnreachableState {
            machine_id,
            state_id,
        } => format!("machine `{machine_id}` has unreachable state `{state_id}`"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_command_accepts_generated_example() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/generated/examples/ssh_bruteforce.json"
        );

        run_with_args(["rewrite", "validate", "--ir", path]).expect("valid IR should pass");
    }

    #[test]
    fn validate_command_rejects_missing_ir_file() {
        let err = run_with_args(["rewrite", "validate", "--ir", "missing.json"])
            .expect_err("missing IR should fail");

        assert!(err.to_string().contains("failed to read IR file"));
    }

    #[test]
    fn generate_command_writes_jsonl_output() {
        let ir_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/generated/examples/ssh_bruteforce.json"
        );
        let out_path = std::env::temp_dir().join(format!(
            "rewrite-cli-generate-{}-{}.jsonl",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));

        run_with_args([
            OsString::from("rewrite"),
            OsString::from("generate"),
            OsString::from("--ir"),
            OsString::from(ir_path),
            OsString::from("--out"),
            out_path.clone().into_os_string(),
            OsString::from("--seed"),
            OsString::from("42"),
        ])
        .expect("generate command should write JSONL");

        let content = std::fs::read_to_string(&out_path).expect("output should be readable");
        let _ = std::fs::remove_file(out_path);
        let lines = content.lines().collect::<Vec<_>>();

        assert!(!lines.is_empty());
        let first: serde_json::Value = serde_json::from_str(lines[0]).expect("line is JSON");
        assert_eq!(first["scenario"], serde_json::json!("ssh_bruteforce_demo"));
        assert!(first["label"].get("is_attack").is_some());
    }

    #[test]
    fn generate_command_respects_enable_attack_false() {
        let ir_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/generated/examples/ssh_bruteforce.json"
        );
        let out_path = std::env::temp_dir().join(format!(
            "rewrite-cli-generate-clean-{}-{}.jsonl",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));

        run_with_args([
            OsString::from("rewrite"),
            OsString::from("generate"),
            OsString::from("--ir"),
            OsString::from(ir_path),
            OsString::from("--out"),
            out_path.clone().into_os_string(),
            OsString::from("--seed"),
            OsString::from("42"),
            OsString::from("--enable-attack"),
            OsString::from("false"),
        ])
        .expect("clean generation should write JSONL");

        let content = std::fs::read_to_string(&out_path).expect("output should be readable");
        let _ = std::fs::remove_file(out_path);
        let events = content
            .lines()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).expect("line is JSON"))
            .collect::<Vec<_>>();

        assert!(!events.is_empty());
        assert!(
            events
                .iter()
                .all(|event| event["label"]["is_attack"] == serde_json::json!(false))
        );
        assert!(
            events
                .iter()
                .all(|event| event["state"].as_str() != Some("bruteforce"))
        );
    }

    #[test]
    fn generate_command_writes_csv_output() {
        let ir_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/generated/examples/ssh_bruteforce.json"
        );
        let out_path = std::env::temp_dir().join(format!(
            "rewrite-cli-generate-{}-{}.csv",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));

        run_with_args([
            OsString::from("rewrite"),
            OsString::from("generate"),
            OsString::from("--ir"),
            OsString::from(ir_path),
            OsString::from("--out"),
            out_path.clone().into_os_string(),
            OsString::from("--format"),
            OsString::from("csv"),
            OsString::from("--seed"),
            OsString::from("42"),
        ])
        .expect("generate command should write CSV");

        let content = std::fs::read_to_string(&out_path).expect("output should be readable");
        let _ = std::fs::remove_file(out_path);

        let mut reader = csv::Reader::from_reader(content.as_bytes());
        let headers = reader.headers().unwrap().clone();
        assert_eq!(
            headers.iter().collect::<Vec<_>>(),
            vec![
                "host",
                "content",
                "is_attack",
                "attack_type",
                "technique",
                "tactic",
                "attack_name",
            ]
        );
        assert!(!headers.iter().any(|header| header == "scenario"));
        assert!(!headers.iter().any(|header| header == "log"));
        assert!(reader.records().next().is_some());
    }

    #[test]
    fn generate_command_is_reproducible_for_same_seed() {
        let ir_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/generated/examples/ssh_bruteforce.json"
        );
        let base = std::env::temp_dir().join(format!(
            "rewrite-cli-generate-repro-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let left = base.with_extension("left.jsonl");
        let right = base.with_extension("right.jsonl");

        for path in [&left, &right] {
            run_with_args([
                OsString::from("rewrite"),
                OsString::from("generate"),
                OsString::from("--ir"),
                OsString::from(ir_path),
                OsString::from("--out"),
                path.clone().into_os_string(),
                OsString::from("--seed"),
                OsString::from("42"),
            ])
            .expect("generate command should write JSONL");
        }

        let left_content = std::fs::read_to_string(&left).expect("left output readable");
        let right_content = std::fs::read_to_string(&right).expect("right output readable");
        let _ = std::fs::remove_file(left);
        let _ = std::fs::remove_file(right);

        assert_eq!(left_content, right_content);
    }

    #[test]
    fn validate_command_runs_drift_before_typed_parse() {
        let source_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/generated/examples/ssh_bruteforce.json"
        );
        let input = std::fs::read_to_string(source_path).expect("fixture should be readable");
        let mut value: serde_json::Value =
            serde_json::from_str(&input).expect("fixture should be json");
        value["attack_chain"] = serde_json::json!({"id": "chain_1"});

        let path = std::env::temp_dir().join(format!(
            "rewrite-cli-drift-{}-{}.json",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        std::fs::write(&path, serde_json::to_string_pretty(&value).unwrap())
            .expect("temp IR should be writable");

        let err = run_with_args([
            OsString::from("rewrite"),
            OsString::from("validate"),
            OsString::from("--ir"),
            path.clone().into_os_string(),
        ])
        .expect_err("drift must fail");

        let _ = std::fs::remove_file(path);

        assert!(err.to_string().contains("definition boundary drift"));
        assert!(err.to_string().contains("field `attack_chain`"));
    }
}
