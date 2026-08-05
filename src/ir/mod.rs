use std::{collections::BTreeMap, fs, path::Path};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::IrParseError;

pub type ObjectMap<T> = BTreeMap<String, T>;
pub type JsonObject = BTreeMap<String, Value>;
pub type Id = String;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ir {
    pub ir_version: String,

    #[serde(default)]
    pub systems: ObjectMap<System>,

    #[serde(default)]
    pub generators: ObjectMap<Generator>,

    #[serde(default)]
    pub context: Context,

    #[serde(default)]
    pub templates: ObjectMap<Template>,

    #[serde(default)]
    pub machines: ObjectMap<Machine>,

    #[serde(default)]
    pub instance_groups: ObjectMap<InstanceGroup>,

    #[serde(default)]
    pub time_profiles: ObjectMap<TimeProfile>,

    pub scenario: Scenario,
}

impl Ir {
    pub fn from_json_str(
        input: &str,
        path_for_error: impl Into<String>,
    ) -> Result<Self, IrParseError> {
        let path = path_for_error.into();
        serde_json::from_str(input).map_err(|source| IrParseError::json(path, source))
    }

    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, IrParseError> {
        let path = path.as_ref();
        let path_string = path.display().to_string();

        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            return Err(IrParseError::unsupported_format(
                path_string,
                "JSON IR (*.json)",
            ));
        }

        let content = fs::read_to_string(path)
            .map_err(|source| IrParseError::read(path.display().to_string(), source))?;

        Self::from_json_str(&content, path.display().to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct System {
    pub id: Id,
    pub kind: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Generator {
    pub id: Id,

    #[serde(rename = "type")]
    pub generator_type: String,

    #[serde(default)]
    pub params: JsonObject,

    pub description: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Context {
    #[serde(default)]
    pub global: ObjectMap<ContextVariable>,

    #[serde(default)]
    pub system: ObjectMap<ObjectMap<ContextVariable>>,

    #[serde(default)]
    pub instance: ObjectMap<ObjectMap<ContextVariable>>,

    #[serde(default)]
    pub event: ObjectMap<ContextVariable>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextVariable {
    pub source: ValueSource,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ValueSource {
    Const(ConstValue),
    Generator(GeneratorValue),
    Ref(ContextRefValue),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConstValue {
    #[serde(rename = "const")]
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratorValue {
    pub generator: Id,

    #[serde(default)]
    pub params: JsonObject,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextRefValue {
    #[serde(rename = "ref")]
    pub reference: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Template {
    pub id: Id,
    pub system: Id,
    pub event_type: Option<String>,
    pub description: Option<String>,
    pub fields: ObjectMap<TemplateValueExpr>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TemplateValueExpr {
    Const(ConstValue),
    Ref(ContextRefValue),
    Interpolate(InterpolationValue),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InterpolationValue {
    pub interpolate: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Machine {
    pub id: Id,
    pub system: Id,
    pub start: Id,
    pub attack: AttackMetadata,
    pub states: ObjectMap<State>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttackMetadata {
    pub enabled: bool,
    #[serde(rename = "type")]
    pub attack_type: Option<String>,
    pub technique: Option<String>,
    pub tactic: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct State {
    pub kind: StateKind,
    pub attack_label: Option<AttackLabel>,
    pub emit: Vec<Emit>,
    pub transitions: Vec<Transition>,
    pub terminal: bool,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StateKind {
    Benign,
    Attack,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttackLabel {
    #[serde(rename = "type")]
    pub attack_type: Option<String>,
    pub technique: Option<String>,
    pub tactic: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Emit {
    pub template: Id,
    pub count: EmitCount,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EmitCount {
    Fixed(usize),
    Range(EmitCountRange),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmitCountRange {
    pub min: usize,
    pub max: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Transition {
    pub to: Id,
    pub weight: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstanceGroup {
    pub id: Id,
    pub machine: Id,
    pub enabled: bool,
    pub count: usize,
    pub params: Option<JsonObject>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimeProfile {
    pub id: Id,
    pub start: String,
    pub timezone: String,
    pub granularity: TimeGranularity,
    pub monotonic_per_instance: bool,
    pub system_clock_skew: ObjectMap<ClockSkew>,
    #[serde(default)]
    pub event_time: Option<EventTimeConfig>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventTimeConfig {
    pub default: BehaviorTimeProfile,

    #[serde(default)]
    pub event_types: ObjectMap<BehaviorTimeProfile>,

    #[serde(default)]
    pub states: ObjectMap<ObjectMap<BehaviorTimeProfile>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BehaviorTimeProfile {
    pub base_delay_ms: TimeDelay,
    pub behavior_jitter_ms: TimeDelay,
    pub error_jitter_ms: TimeDelay,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TimeDelay {
    Fixed(FixedTimeDelay),
    Range(RangeTimeDelay),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixedTimeDelay {
    pub milliseconds: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RangeTimeDelay {
    pub min_milliseconds: f64,
    pub max_milliseconds: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeGranularity {
    Second,
    Millisecond,
    Microsecond,
    Nanosecond,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ClockSkew {
    Fixed(FixedClockSkew),
    Range(RangeClockSkew),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixedClockSkew {
    pub milliseconds: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RangeClockSkew {
    pub min_milliseconds: f64,
    pub max_milliseconds: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    pub id: Id,
    pub systems: Vec<Id>,
    pub instance_groups: Vec<Id>,
    pub time_profile: Id,
    pub description: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_generated_ssh_bruteforce_example() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/generated/examples/ssh_bruteforce.json"
        );
        let ir = Ir::from_path(path).expect("generated example should deserialize");

        assert_eq!(ir.ir_version, "0.2");
        assert!(ir.systems.contains_key("linux_auth"));
        assert!(ir.generators.contains_key("username_pool"));
        assert!(ir.templates.contains_key("ssh_login_failed"));
        assert!(ir.machines.contains_key("ssh_login_flow"));
        assert!(ir.instance_groups.contains_key("ssh_login_sessions"));
        assert!(ir.time_profiles.contains_key("default_time"));
        assert_eq!(ir.scenario.id, "ssh_bruteforce_demo");

        let machine = &ir.machines["ssh_login_flow"];
        assert_eq!(machine.start, "start");
        assert!(machine.attack.enabled);
        assert_eq!(machine.states["bruteforce"].kind, StateKind::Attack);
    }

    #[test]
    fn parses_fixed_and_range_emit_count() {
        let fixed: EmitCount = serde_json::from_str("1").expect("fixed count");
        let range: EmitCount = serde_json::from_str(r#"{"min":20,"max":80}"#).expect("range count");

        assert_eq!(fixed, EmitCount::Fixed(1));
        assert_eq!(range, EmitCount::Range(EmitCountRange { min: 20, max: 80 }));
    }

    #[test]
    fn rejects_ambiguous_template_expr_shape() {
        let err = serde_json::from_str::<TemplateValueExpr>(r#"{"const":1,"ref":"ctx.global.x"}"#)
            .expect_err("template expr must have exactly one shape");

        assert!(err.is_data());
    }

    #[test]
    fn rejects_non_json_ir_extension() {
        let err = Ir::from_path("some/path/scenario.yaml").expect_err("only json is accepted");

        assert!(matches!(err, IrParseError::UnsupportedFormat { .. }));
    }
}
