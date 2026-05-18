use serde_json::Value;

use crate::{
    error::RuntimeError,
    ir::{TemplateValueExpr, ValueSource},
};

/// Scope prefix accepted by MVP context references.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ContextScope {
    Global,
    System,
    Instance,
    Event,
}

impl ContextScope {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::System => "system",
            Self::Instance => "instance",
            Self::Event => "event",
        }
    }
}

/// Parsed `ctx.<scope>.<name>` reference.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContextRef {
    raw: String,
    scope: ContextScope,
    name: String,
}

impl ContextRef {
    pub fn raw(&self) -> &str {
        &self.raw
    }

    pub fn scope(&self) -> ContextScope {
        self.scope
    }

    pub fn scope_name(&self) -> &'static str {
        self.scope.as_str()
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Parsed interpolation template, e.g. `user=${ctx.instance.user}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interpolation {
    parts: Vec<InterpolationPart>,
}

impl Interpolation {
    pub fn parse(input: &str, location: impl Into<String>) -> Result<Self, RuntimeError> {
        let location = location.into();
        let mut parts = Vec::new();
        let mut cursor = 0;

        while let Some(relative_start) = input[cursor..].find("${") {
            let start = cursor + relative_start;
            if start > cursor {
                parts.push(InterpolationPart::Text(input[cursor..start].to_owned()));
            }

            let expr_start = start + 2;
            let Some(relative_end) = input[expr_start..].find('}') else {
                return Err(invalid_expr(
                    &location,
                    "interpolate",
                    "unclosed interpolation placeholder",
                ));
            };
            let end = expr_start + relative_end;
            let expr = input[expr_start..end].trim();
            if expr.is_empty() {
                return Err(invalid_expr(
                    &location,
                    "interpolate",
                    "empty interpolation placeholder",
                ));
            }

            parts.push(InterpolationPart::Ref(parse_context_ref(
                expr,
                &location,
                "interpolate",
            )?));
            cursor = end + 1;
        }

        if cursor < input.len() {
            parts.push(InterpolationPart::Text(input[cursor..].to_owned()));
        }

        Ok(Self { parts })
    }

    pub fn parts(&self) -> &[InterpolationPart] {
        &self.parts
    }

    pub fn refs(&self) -> impl Iterator<Item = &ContextRef> {
        self.parts.iter().filter_map(|part| match part {
            InterpolationPart::Text(_) => None,
            InterpolationPart::Ref(reference) => Some(reference),
        })
    }

    pub fn render(
        &self,
        resolver: &impl ContextValueResolver,
        location: impl Into<String>,
    ) -> Result<String, RuntimeError> {
        let location = location.into();
        let mut output = String::new();

        for part in &self.parts {
            match part {
                InterpolationPart::Text(text) => output.push_str(text),
                InterpolationPart::Ref(reference) => {
                    let value = resolver.resolve_context_value(reference, &location)?;
                    output.push_str(&value_to_interpolation_string(&value));
                }
            }
        }

        Ok(output)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InterpolationPart {
    Text(String),
    Ref(ContextRef),
}

/// Minimal dependency boundary used by value expression evaluation.
///
/// The template expression module deliberately talks to a trait instead of the
/// future context store implementation, so it stays independent of machine and
/// runtime orchestration.
pub trait ContextValueResolver {
    fn resolve_context_value(
        &self,
        reference: &ContextRef,
        location: &str,
    ) -> Result<Value, RuntimeError>;
}

/// Parse a raw `ctx.<scope>.<name>` reference.
pub fn parse_context_ref(
    raw: &str,
    location: impl Into<String>,
    field: impl Into<String>,
) -> Result<ContextRef, RuntimeError> {
    let location = location.into();
    let field = field.into();
    let parts = raw.split('.').collect::<Vec<_>>();
    if parts.len() != 3 || parts[0] != "ctx" || parts[2].is_empty() {
        return Err(invalid_expr(
            location,
            field,
            format!("expected context reference in form `ctx.<scope>.<name>`, got `{raw}`"),
        ));
    }

    let scope = match parts[1] {
        "global" => ContextScope::Global,
        "system" => ContextScope::System,
        "instance" => ContextScope::Instance,
        "event" => ContextScope::Event,
        other => {
            return Err(invalid_expr(
                location,
                field,
                format!("unknown context scope `{other}` in `{raw}`"),
            ));
        }
    };

    Ok(ContextRef {
        raw: raw.to_owned(),
        scope,
        name: parts[2].to_owned(),
    })
}

/// Parse interpolation text and return all referenced context variables.
pub fn parse_interpolation_refs(
    input: &str,
    location: impl Into<String>,
) -> Result<Vec<ContextRef>, RuntimeError> {
    Ok(Interpolation::parse(input, location)?
        .refs()
        .cloned()
        .collect())
}

/// Evaluate one template field expression into a JSON value.
pub fn eval_template_value_expr(
    expr: &TemplateValueExpr,
    resolver: &impl ContextValueResolver,
    location: impl Into<String>,
) -> Result<Value, RuntimeError> {
    let location = location.into();
    match expr {
        TemplateValueExpr::Const(const_value) => Ok(const_value.value.clone()),
        TemplateValueExpr::Ref(ref_value) => {
            let reference = parse_context_ref(&ref_value.reference, &location, "ref")?;
            resolver.resolve_context_value(&reference, &location)
        }
        TemplateValueExpr::Interpolate(interpolation_value) => {
            let interpolation = Interpolation::parse(&interpolation_value.interpolate, &location)?;
            Ok(Value::String(interpolation.render(resolver, &location)?))
        }
    }
}

/// Evaluate a context source that is already resolvable without generator execution.
///
/// Generator-backed variables belong to the context/generator layer; returning
/// `None` here keeps this module focused on constants and references only.
pub fn eval_static_or_ref_source(
    source: &ValueSource,
    resolver: &impl ContextValueResolver,
    location: impl Into<String>,
) -> Result<Option<Value>, RuntimeError> {
    let location = location.into();
    match source {
        ValueSource::Const(const_value) => Ok(Some(const_value.value.clone())),
        ValueSource::Ref(ref_value) => {
            let reference = parse_context_ref(&ref_value.reference, &location, "ref")?;
            resolver
                .resolve_context_value(&reference, &location)
                .map(Some)
        }
        ValueSource::Generator(_) => Ok(None),
    }
}

fn value_to_interpolation_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        Value::Array(_) | Value::Object(_) => value.to_string(),
    }
}

fn invalid_expr(
    location: impl Into<String>,
    field: impl Into<String>,
    message: impl Into<String>,
) -> RuntimeError {
    RuntimeError::invalid_value_expression(location, field, message)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::json;

    use super::*;
    use crate::ir::{ConstValue, ContextRefValue, InterpolationValue};

    #[derive(Default)]
    struct MapResolver {
        values: BTreeMap<String, Value>,
    }

    impl MapResolver {
        fn with(mut self, key: &str, value: Value) -> Self {
            self.values.insert(key.to_owned(), value);
            self
        }
    }

    impl ContextValueResolver for MapResolver {
        fn resolve_context_value(
            &self,
            reference: &ContextRef,
            location: &str,
        ) -> Result<Value, RuntimeError> {
            self.values.get(reference.raw()).cloned().ok_or_else(|| {
                RuntimeError::missing_context_variable(
                    reference.scope_name(),
                    reference.name(),
                    location,
                )
            })
        }
    }

    #[test]
    fn parses_context_ref() {
        let reference = parse_context_ref("ctx.instance.user", "template.t.fields.user", "ref")
            .expect("valid ref");

        assert_eq!(reference.raw(), "ctx.instance.user");
        assert_eq!(reference.scope(), ContextScope::Instance);
        assert_eq!(reference.name(), "user");
    }

    #[test]
    fn rejects_non_context_ref() {
        let err =
            parse_context_ref("exec.rm_rf", "template.t.fields.x", "ref").expect_err("invalid ref");

        assert!(matches!(err, RuntimeError::InvalidValueExpression { .. }));
    }

    #[test]
    fn parses_interpolation_segments_and_refs() {
        let interpolation = Interpolation::parse(
            "Failed password for ${ctx.instance.user} from ${ ctx.event.src_ip }",
            "template.login.message",
        )
        .expect("valid interpolation");

        let refs = interpolation
            .refs()
            .map(ContextRef::raw)
            .collect::<Vec<_>>();
        assert_eq!(refs, vec!["ctx.instance.user", "ctx.event.src_ip"]);
        assert_eq!(interpolation.parts().len(), 4);
    }

    #[test]
    fn rejects_unclosed_interpolation() {
        let err = Interpolation::parse("user=${ctx.instance.user", "template.login.message")
            .expect_err("invalid interpolation");

        assert!(matches!(err, RuntimeError::InvalidValueExpression { .. }));
    }

    #[test]
    fn evaluates_const_ref_and_interpolation() {
        let resolver = MapResolver::default()
            .with("ctx.instance.user", json!("alice"))
            .with("ctx.event.src_ip", json!("10.0.0.5"))
            .with("ctx.event.port", json!(22));

        let const_expr = TemplateValueExpr::Const(ConstValue { value: json!(true) });
        let ref_expr = TemplateValueExpr::Ref(ContextRefValue {
            reference: "ctx.instance.user".to_owned(),
        });
        let interpolation_expr = TemplateValueExpr::Interpolate(InterpolationValue {
            interpolate: "user=${ctx.instance.user} src=${ctx.event.src_ip} port=${ctx.event.port}"
                .to_owned(),
        });

        assert_eq!(
            eval_template_value_expr(&const_expr, &resolver, "template.t.fields.flag").unwrap(),
            json!(true)
        );
        assert_eq!(
            eval_template_value_expr(&ref_expr, &resolver, "template.t.fields.user").unwrap(),
            json!("alice")
        );
        assert_eq!(
            eval_template_value_expr(&interpolation_expr, &resolver, "template.t.fields.message")
                .unwrap(),
            json!("user=alice src=10.0.0.5 port=22")
        );
    }

    #[test]
    fn missing_ref_comes_from_resolver() {
        let resolver = MapResolver::default();
        let expr = TemplateValueExpr::Ref(ContextRefValue {
            reference: "ctx.global.tenant".to_owned(),
        });

        let err = eval_template_value_expr(&expr, &resolver, "template.t.fields.tenant")
            .expect_err("missing context value");

        assert!(matches!(err, RuntimeError::MissingContextVariable { .. }));
    }
}
