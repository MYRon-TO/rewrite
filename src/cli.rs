use std::{ffi::OsString, path::PathBuf};

use clap::{Arg, ArgAction, Command, value_parser};

use crate::{
    error::Result,
    ir::Ir,
    validate::{MachineValidationWarning, validate_machines, validate_references},
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
    let ir = Ir::from_path(&ir_path)?;

    validate_references(&ir)?;
    let report = validate_machines(&ir)?;

    for warning in report.warnings {
        eprintln!("warning: {}", format_machine_warning(&warning));
    }

    println!("IR validation passed: {}", ir_path.display());

    Ok(())
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
}
