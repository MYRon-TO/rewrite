use serde_json::Value;

use crate::error::ValidationError;

/// Validate that raw IR JSON does not contain fields/objects that are outside the MVP boundary.
///
/// This intentionally works on `serde_json::Value` instead of typed IR structs, but it only scans
/// structural IR fields. Flexible business-data payloads such as `params`, `template.fields`, and
/// `const` values are ignored here to avoid rejecting ordinary log field names. Typed
/// deserialization with `deny_unknown_fields` is still responsible for ordinary schema mismatch;
/// this module reports explicit MVP-boundary violations with a clearer error category.
pub fn validate_drift(raw: &Value) -> Result<(), ValidationError> {
    scan_value(raw, "$", false)
}

fn scan_value(value: &Value, location: &str, data_payload: bool) -> Result<(), ValidationError> {
    match value {
        Value::Object(object) => {
            for (field, child) in object {
                if !data_payload {
                    if let Some(message) = forbidden_field_message(field) {
                        return Err(ValidationError::drift(location, field, message));
                    }
                }

                let child_location = child_location(location, field);
                let child_is_data_payload = data_payload || is_data_payload_field(field);
                scan_value(child, &child_location, child_is_data_payload)?;
            }

            Ok(())
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                scan_value(child, &format!("{location}[{index}]"), data_payload)?;
            }

            Ok(())
        }
        _ => Ok(()),
    }
}

fn is_data_payload_field(field: &str) -> bool {
    matches!(field, "params" | "fields" | "const")
}

fn forbidden_field_message(field: &str) -> Option<&'static str> {
    match field {
        // Future identity graph / entity model.
        "entity" | "entities" => Some(
            "entity modeling is outside IR v0.1; use system/context/instance_group boundaries only",
        ),

        // Cross-machine or top-level attack orchestration.
        "attack_chain" | "attack_chains" => Some(
            "cross-machine attack chains are outside MVP; model only single-machine attack branches",
        ),

        // Global or per-group quota control. MVP uses local transition weights only.
        "attack_ratio" | "attack_quota" | "attack_rate" => Some(
            "attack ratio/quota control is not supported; use local transition.weight sampling instead",
        ),

        // Attack stage objects are intentionally not part of the first definition layer.
        "stage" | "stages" | "attack_stage" | "attack_stages" => Some(
            "attack stages are outside MVP; state.kind and state.attack_label are the only attack markers",
        ),

        // Event/causal DAG and dependency modeling.
        "dag" | "event_dag" | "events_dag" | "causal_dag" | "causality" | "causal_links"
        | "causal_edges" | "depends_on" | "dependencies" => Some(
            "event DAG or causal dependency fields are outside MVP; machines may only use local transitions",
        ),

        // Explicit cross-boundary dependencies.
        "cross_system"
        | "cross_system_dependency"
        | "cross_system_dependencies"
        | "system_dependency"
        | "system_dependencies"
        | "depends_on_system"
        | "target_system"
        | "source_system"
        | "cross_instance"
        | "cross_instance_dependency"
        | "cross_instance_dependencies"
        | "instance_dependency"
        | "instance_dependencies"
        | "depends_on_instance"
        | "target_instance"
        | "source_instance" => Some(
            "cross-system or cross-instance dependencies are outside MVP; systems run in parallel without causality",
        ),

        // Runtime code execution / extension points. Generator/template params stay pure JSON data.
        "script"
        | "scripts"
        | "eval"
        | "exec"
        | "shell"
        | "hook"
        | "hooks"
        | "plugin"
        | "plugins"
        | "external"
        | "external_generator"
        | "external_generators" => {
            Some("scripts, hooks, plugins, and external execution hooks are outside MVP")
        }

        _ => None,
    }
}

fn child_location(parent: &str, field: &str) -> String {
    if is_path_ident(field) {
        format!("{parent}.{field}")
    } else {
        format!(
            "{parent}[{}]",
            serde_json::to_string(field).expect("string serialization")
        )
    }
}

fn is_path_ident(input: &str) -> bool {
    let mut chars = input.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch == '-' || ch.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn generated_value() -> Value {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/generated/examples/ssh_bruteforce.json"
        );
        let input = std::fs::read_to_string(path).expect("generated example should be readable");
        serde_json::from_str(&input).expect("generated example should be valid json")
    }

    #[test]
    fn accepts_generated_ssh_bruteforce_ir() {
        validate_drift(&generated_value()).unwrap();
    }

    #[test]
    fn rejects_top_level_attack_chain_object() {
        let mut value = generated_value();
        value["attack_chain"] = json!({"id": "chain_1"});

        let err = validate_drift(&value).expect_err("attack_chain is outside MVP");

        assert!(matches!(err, ValidationError::Drift { .. }));
        assert!(err.to_string().contains("field `attack_chain`"));
        assert!(err.to_string().contains("cross-machine attack chains"));
    }

    #[test]
    fn rejects_instance_group_attack_ratio() {
        let mut value = generated_value();
        value["instance_groups"]["ssh_login_sessions"]["attack_ratio"] = json!(0.2);

        let err = validate_drift(&value).expect_err("attack_ratio is outside MVP");

        assert!(
            err.to_string()
                .contains("$.instance_groups.ssh_login_sessions")
        );
        assert!(err.to_string().contains("field `attack_ratio`"));
    }

    #[test]
    fn rejects_event_dag_dependency_fields() {
        let mut value = generated_value();
        value["machines"]["ssh_login_flow"]["states"]["start"]["depends_on"] =
            json!(["other_event"]);

        let err = validate_drift(&value).expect_err("event DAG dependencies are outside MVP");

        assert!(err.to_string().contains("field `depends_on`"));
        assert!(err.to_string().contains("event DAG"));
    }

    #[test]
    fn allows_forbidden_words_inside_data_payloads() {
        let mut value = generated_value();
        value["generators"]["username_pool"]["params"]["script"] = json!("literal string only");
        value["templates"]["ssh_login_success"]["fields"]["stage"] = json!({"const": "prod"});
        value["templates"]["ssh_login_success"]["fields"]["payload"] =
            json!({"const": {"external": true}});

        validate_drift(&value).expect("drift scan should ignore business data payload keys");
    }

    #[test]
    fn rejects_attack_stage_inside_state() {
        let mut value = generated_value();
        value["machines"]["ssh_login_flow"]["states"]["bruteforce"]["stage"] =
            json!("credential_access");

        let err = validate_drift(&value).expect_err("stage is outside MVP");

        assert!(err.to_string().contains("field `stage`"));
        assert!(err.to_string().contains("attack stages"));
    }
}
