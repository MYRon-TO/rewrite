use std::{collections::BTreeSet, path::Path};

use serde_json::Value;

use crate::{
    error::ValidationError,
    generator::BUILTIN_GENERATOR_TYPES,
    ir::{Generator, Ir, JsonObject, ValueSource},
};

pub fn validate_generators(ir: &Ir) -> Result<(), ValidationError> {
    validate_generators_with_asset_root(ir, Path::new("."))
}

pub fn validate_generators_with_asset_root(
    ir: &Ir,
    asset_root: &Path,
) -> Result<(), ValidationError> {
    if ir.ir_version != "0.2" {
        return Err(invalid("ir", "ir_version", "expected `0.2`"));
    }
    for (id, generator) in &ir.generators {
        validate_one(generator, &generator.params, &format!("generators.{id}"))?;
    }
    for (name, variable) in &ir.context.global {
        validate_source(ir, &variable.source, &format!("context.global.{name}"))?;
    }
    for (scope, groups) in [
        ("system", &ir.context.system),
        ("instance", &ir.context.instance),
    ] {
        for (owner, variables) in groups {
            for (name, variable) in variables {
                validate_source(
                    ir,
                    &variable.source,
                    &format!("context.{scope}.{owner}.{name}"),
                )?;
            }
        }
    }
    for (name, variable) in &ir.context.event {
        validate_source(ir, &variable.source, &format!("context.event.{name}"))?;
    }

    let mut cache = crate::generator::new_corpus_cache();
    for (id, generator) in &ir.generators {
        if generator.generator_type == "corpus_choice" {
            crate::generator::validate_corpus(
                &mut cache,
                asset_root,
                generator,
                &generator.params,
                &format!("generators.{id}"),
            )
            .map_err(|message| invalid(format!("generators.{id}"), "source", message))?;
        }
    }
    validate_corpus_sources(ir, asset_root, &mut cache)?;
    Ok(())
}

fn validate_corpus_sources(
    ir: &Ir,
    asset_root: &Path,
    cache: &mut crate::generator::corpus_cache::CorpusCache,
) -> Result<(), ValidationError> {
    let mut check = |source: &ValueSource, location: String| -> Result<(), ValidationError> {
        let ValueSource::Generator(value) = source else {
            return Ok(());
        };
        let Some(generator) = ir.generators.get(&value.generator) else {
            return Ok(());
        };
        if generator.generator_type != "corpus_choice" {
            return Ok(());
        }
        let mut merged = generator.params.clone();
        merged.extend(value.params.clone());
        crate::generator::validate_corpus(cache, asset_root, generator, &merged, &location)
            .map_err(|message| invalid(location, "source", message))
    };
    for (name, variable) in &ir.context.global {
        check(&variable.source, format!("context.global.{name}"))?;
    }
    for (scope, groups) in [
        ("system", &ir.context.system),
        ("instance", &ir.context.instance),
    ] {
        for (owner, variables) in groups {
            for (name, variable) in variables {
                check(&variable.source, format!("context.{scope}.{owner}.{name}"))?;
            }
        }
    }
    for (name, variable) in &ir.context.event {
        check(&variable.source, format!("context.event.{name}"))?;
    }
    Ok(())
}

fn validate_source(ir: &Ir, source: &ValueSource, location: &str) -> Result<(), ValidationError> {
    let ValueSource::Generator(value) = source else {
        return Ok(());
    };
    let Some(generator) = ir.generators.get(&value.generator) else {
        return Ok(());
    };
    let mut merged = generator.params.clone();
    merged.extend(value.params.clone());
    validate_one(generator, &merged, location)
}

fn validate_one(
    generator: &Generator,
    params: &JsonObject,
    location: &str,
) -> Result<(), ValidationError> {
    match generator.generator_type.as_str() {
        "machine_secret" => validate_machine(params, location),
        "corpus_choice" => validate_corpus(params, location),
        kind if BUILTIN_GENERATOR_TYPES.contains(&kind) => Ok(()),
        kind => Err(invalid(
            location,
            "type",
            format!("unsupported generator type `{kind}`"),
        )),
    }
}

fn validate_machine(p: &JsonObject, location: &str) -> Result<(), ValidationError> {
    let profile = string(p, "profile", location)?;
    let allowed: &[&str] = match profile {
        "hex" | "base64" | "base64url" => &["profile", "byte_length"],
        "uuid" => &["profile"],
        "api_key" | "bearer_token" | "private_token" => {
            &["profile", "prefix", "body_length", "encoding"]
        }
        "jwt_shape" => &["profile", "issuer", "subject", "signature_length"],
        "pem_shape" => &["profile", "label", "byte_length", "line_width"],
        "cloud_access_key" => &["profile", "prefix", "body_length"],
        "cloud_secret_pair" => &["profile", "id_prefix", "id_body_length", "secret_length"],
        "connection_string" => &[
            "profile",
            "scheme",
            "host",
            "port",
            "database",
            "username",
            "password_length",
        ],
        _ => {
            return Err(invalid(
                location,
                "profile",
                format!("unsupported profile `{profile}`"),
            ));
        }
    };
    reject_unknown(p, allowed, location)?;
    for key in [
        "prefix",
        "issuer",
        "subject",
        "label",
        "id_prefix",
        "scheme",
        "host",
        "database",
        "username",
    ] {
        if p.get(key).is_some_and(|value| !value.is_string()) {
            return Err(invalid(location, key, "must be a string"));
        }
    }
    match profile {
        "hex" | "base64" | "base64url" => length(p, "byte_length", 1, 4096, true, location)?,
        "api_key" | "bearer_token" | "private_token" => {
            length(p, "body_length", 1, 4096, false, location)?;
            encoding(p, location)?;
        }
        "jwt_shape" => length(p, "signature_length", 1, 4096, false, location)?,
        "pem_shape" => {
            length(p, "byte_length", 1, 4096, false, location)?;
            length(p, "line_width", 1, 256, false, location)?;
        }
        "cloud_access_key" => length(p, "body_length", 1, 4096, false, location)?,
        "cloud_secret_pair" => {
            length(p, "id_body_length", 1, 4096, false, location)?;
            length(p, "secret_length", 1, 4096, false, location)?;
        }
        "connection_string" => {
            for key in ["scheme", "host", "database", "username"] {
                if let Some(v) = p.get(key) {
                    if v.as_str().is_none_or(str::is_empty) {
                        return Err(invalid(location, key, "must be a non-empty string"));
                    }
                }
            }
            length(p, "port", 1, 65535, false, location)?;
            length(p, "password_length", 1, 4096, false, location)?;
        }
        _ => {}
    }
    Ok(())
}

fn validate_corpus(p: &JsonObject, location: &str) -> Result<(), ValidationError> {
    reject_unknown(
        p,
        &[
            "source",
            "min_length",
            "max_length",
            "sampling",
            "deduplicate",
            "keep_empty",
        ],
        location,
    )?;
    let source = p
        .get("source")
        .and_then(Value::as_object)
        .ok_or_else(|| invalid(location, "source", "is required and must be an object"))?;
    let keys: BTreeSet<_> = source.keys().map(String::as_str).collect();
    if keys != BTreeSet::from(["builtin"]) && keys != BTreeSet::from(["path"]) {
        return Err(invalid(
            location,
            "source",
            "must contain exactly one of `builtin` or `path`",
        ));
    }
    if let Some(builtin) = source.get("builtin") {
        if builtin.as_str() != Some("human-passwords-v1") {
            return Err(invalid(
                location,
                "source.builtin",
                "unsupported builtin corpus",
            ));
        }
    }
    if source.values().next().and_then(Value::as_str).is_none() {
        return Err(invalid(location, "source", "source value must be a string"));
    }
    let min = optional_integer(p, "min_length", 1, location)?;
    let max = optional_integer(p, "max_length", 128, location)?;
    if !(0..=4096).contains(&min) || !(0..=4096).contains(&max) || max < min {
        return Err(invalid(
            location,
            "length",
            "requires 0 <= min_length <= max_length <= 4096",
        ));
    }
    if let Some(v) = p.get("sampling") {
        if !matches!(v.as_str(), Some("uniform" | "shuffle_cycle")) {
            return Err(invalid(
                location,
                "sampling",
                "expected `uniform` or `shuffle_cycle`",
            ));
        }
    }
    for key in ["deduplicate", "keep_empty"] {
        if p.get(key).is_some_and(|v| !v.is_boolean()) {
            return Err(invalid(location, key, "must be a boolean"));
        }
    }
    Ok(())
}

fn string<'a>(p: &'a JsonObject, key: &str, loc: &str) -> Result<&'a str, ValidationError> {
    p.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| invalid(loc, key, "is required and must be a string"))
}
fn optional_integer(
    p: &JsonObject,
    key: &str,
    default: i64,
    loc: &str,
) -> Result<i64, ValidationError> {
    match p.get(key) {
        None => Ok(default),
        Some(v) => v
            .as_i64()
            .ok_or_else(|| invalid(loc, key, "must be an integer")),
    }
}
fn length(
    p: &JsonObject,
    key: &str,
    min: i64,
    max: i64,
    required: bool,
    loc: &str,
) -> Result<(), ValidationError> {
    if required && !p.contains_key(key) {
        return Err(invalid(loc, key, "is required"));
    }
    let n = optional_integer(p, key, min, loc)?;
    if !(min..=max).contains(&n) {
        return Err(invalid(loc, key, format!("must be in {min}..={max}")));
    }
    Ok(())
}
fn encoding(p: &JsonObject, loc: &str) -> Result<(), ValidationError> {
    if let Some(v) = p.get("encoding") {
        if !matches!(v.as_str(), Some("base64url" | "hex" | "alphanumeric")) {
            return Err(invalid(loc, "encoding", "unsupported encoding"));
        }
    }
    Ok(())
}
fn reject_unknown(p: &JsonObject, allowed: &[&str], loc: &str) -> Result<(), ValidationError> {
    if let Some(key) = p.keys().find(|k| !allowed.contains(&k.as_str())) {
        return Err(invalid(loc, key, "unknown parameter"));
    }
    Ok(())
}
fn invalid(
    location: impl Into<String>,
    field: impl Into<String>,
    message: impl Into<String>,
) -> ValidationError {
    ValidationError::invalid_value(location, field, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{Context, ContextVariable, GeneratorValue, Scenario};
    use serde_json::Map;
    use std::collections::BTreeMap;

    fn ir(version: &str, kind: &str, params: JsonObject) -> Ir {
        let generator = Generator {
            id: "g".into(),
            generator_type: kind.into(),
            params,
            description: None,
        };
        Ir {
            ir_version: version.into(),
            systems: BTreeMap::new(),
            generators: BTreeMap::from([("g".into(), generator)]),
            context: Context::default(),
            templates: BTreeMap::new(),
            machines: BTreeMap::new(),
            instance_groups: BTreeMap::new(),
            time_profiles: BTreeMap::new(),
            scenario: Scenario {
                id: "s".into(),
                systems: vec![],
                instance_groups: vec![],
                time_profile: "t".into(),
                description: None,
            },
        }
    }
    fn obj(s: &str) -> JsonObject {
        serde_json::from_str::<Map<String, Value>>(s)
            .unwrap()
            .into_iter()
            .collect()
    }
    #[test]
    fn rejects_non_02_ir() {
        assert!(validate_generators(&ir("0.1", "fixed", obj(r#"{"value":"x"}"#))).is_err());
    }
    #[test]
    fn accepts_machine_profile() {
        assert!(
            validate_generators(&ir(
                "0.2",
                "machine_secret",
                obj(r#"{"profile":"hex","byte_length":32}"#)
            ))
            .is_ok()
        );
    }
    #[test]
    fn rejects_corpus_conflicting_source() {
        assert!(
            validate_generators(&ir(
                "0.2",
                "corpus_choice",
                obj(r#"{"source":{"builtin":"human-passwords-v1","path":"x"}}"#)
            ))
            .is_err()
        );
    }
    #[test]
    fn rejects_invalid_length() {
        assert!(
            validate_generators(&ir(
                "0.2",
                "machine_secret",
                obj(r#"{"profile":"pem_shape","line_width":257}"#)
            ))
            .is_err()
        );
    }

    #[test]
    fn validates_context_use_site_after_parameter_override_merge() {
        let mut value = ir(
            "0.2",
            "corpus_choice",
            obj(r#"{"source":{"builtin":"human-passwords-v1"},"min_length":10,"max_length":20}"#),
        );
        value.context.global.insert(
            "password".into(),
            ContextVariable {
                source: ValueSource::Generator(GeneratorValue {
                    generator: "g".into(),
                    params: obj(r#"{"max_length":5}"#),
                }),
                description: None,
            },
        );

        let error = validate_generators(&value).expect_err("merged length range must be rejected");
        assert!(error.to_string().contains("context.global.password"));
        assert!(error.to_string().contains("min_length <= max_length"));
    }
}
