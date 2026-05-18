use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

use clap::{Arg, ArgAction, Command, value_parser};

use crate::{
    error::{IrParseError, Result},
    ir::Ir,
    validate::{MachineValidationWarning, validate_drift, validate_machines, validate_references},
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
                .about("Read and statically validate a generated JSON IR file; does not generate logs")
                .arg(
                    Arg::new("ir")
                        .long("ir")
                        .value_name("PATH")
                        .help("Path to generated JSON IR, for example generated/examples/ssh_bruteforce.json")
                        .required(true)
                        .action(ArgAction::Set)
                        .value_parser(value_parser!(PathBuf)),
                ),
        )
}

fn validate_command(ir_path: PathBuf) -> Result<()> {
    let ir_content = read_json_ir_content(&ir_path)?;
    let raw_ir = serde_json::from_str(&ir_content)
        .map_err(|source| IrParseError::json(ir_path.display().to_string(), source))?;

    validate_drift(&raw_ir)?;

    let ir = Ir::from_json_str(&ir_content, ir_path.display().to_string())?;

    validate_references(&ir)?;
    let report = validate_machines(&ir)?;

    for warning in report.warnings {
        eprintln!("warning: {}", format_machine_warning(&warning));
    }

    println!("IR validation passed: {}", ir_path.display());

    Ok(())
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
