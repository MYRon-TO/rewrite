use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::{
    error::{Result, RuntimeError},
    ir::{ContextVariable, Generator, Id, Ir, JsonObject, ValueSource},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextScope {
    Global,
    System,
    Instance,
    Event,
}

impl ContextScope {
    fn as_str(self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::System => "system",
            Self::Instance => "instance",
            Self::Event => "event",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct InstanceKey {
    machine_id: Id,
    instance_id: Id,
}

impl InstanceKey {
    fn new(machine_id: impl Into<Id>, instance_id: impl Into<Id>) -> Self {
        Self {
            machine_id: machine_id.into(),
            instance_id: instance_id.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ContextRef {
    scope: ContextScope,
    name: String,
}

/// Pluggable value generator used by the context store.
///
/// The store owns caching and reference resolution. Generator implementations only
/// need to turn a generator definition plus per-use params into one JSON value.
pub trait ContextValueGenerator {
    fn generate(
        &mut self,
        generator: &Generator,
        params: &JsonObject,
        location: &str,
    ) -> Result<Value, RuntimeError>;
}

#[derive(Debug)]
pub struct ContextStore<'ir> {
    ir: &'ir Ir,
    global_values: BTreeMap<String, Value>,
    system_values: BTreeMap<Id, BTreeMap<String, Value>>,
    instance_values: BTreeMap<InstanceKey, BTreeMap<String, Value>>,
}

impl<'ir> ContextStore<'ir> {
    pub fn new(ir: &'ir Ir) -> Self {
        Self {
            ir,
            global_values: BTreeMap::new(),
            system_values: BTreeMap::new(),
            instance_values: BTreeMap::new(),
        }
    }

    pub fn global_value(
        &mut self,
        name: &str,
        generator: &mut impl ContextValueGenerator,
    ) -> Result<Value, RuntimeError> {
        self.resolve_global(name, generator, &mut BTreeSet::new())
    }

    pub fn system_value(
        &mut self,
        system_id: &str,
        name: &str,
        generator: &mut impl ContextValueGenerator,
    ) -> Result<Value, RuntimeError> {
        self.resolve_system(system_id, name, generator, &mut BTreeSet::new())
    }

    pub fn instance_value(
        &mut self,
        system_id: &str,
        machine_id: &str,
        instance_id: &str,
        name: &str,
        generator: &mut impl ContextValueGenerator,
    ) -> Result<Value, RuntimeError> {
        self.resolve_instance(
            system_id,
            machine_id,
            instance_id,
            name,
            generator,
            &mut BTreeSet::new(),
        )
    }

    /// Builds a fresh event context. Unlike global/system/instance scopes, event
    /// values are deliberately not cached by the store and should be rebuilt for
    /// every emitted event.
    pub fn event_context(
        &mut self,
        system_id: impl Into<Id>,
        machine_id: impl Into<Id>,
        instance_id: impl Into<Id>,
        generator: &mut impl ContextValueGenerator,
    ) -> Result<EventContext, RuntimeError> {
        let system_id = system_id.into();
        let machine_id = machine_id.into();
        let instance_id = instance_id.into();
        let mut values = BTreeMap::new();

        for name in self.ir.context.event.keys() {
            let value = self.resolve_event(
                &system_id,
                &machine_id,
                &instance_id,
                name,
                &mut values,
                generator,
                &mut BTreeSet::new(),
            )?;
            values.insert(name.clone(), value);
        }

        Ok(EventContext {
            system_id,
            machine_id,
            instance_id,
            event_values: values,
        })
    }

    pub fn cached_instance_values(
        &self,
        machine_id: &str,
        instance_id: &str,
    ) -> Option<&BTreeMap<String, Value>> {
        self.instance_values
            .get(&InstanceKey::new(machine_id, instance_id))
    }

    /// Resolves a `ctx.<scope>.<name>` reference for an already-created event.
    /// This is the public read path intended for later template rendering.
    pub fn value_for_ref(
        &mut self,
        reference: &str,
        event: &EventContext,
        generator: &mut impl ContextValueGenerator,
    ) -> Result<Value, RuntimeError> {
        let reference = parse_ref(reference)?;
        match reference.scope {
            ContextScope::Global => self.global_value(&reference.name, generator),
            ContextScope::System => self.system_value(&event.system_id, &reference.name, generator),
            ContextScope::Instance => self.instance_value(
                &event.system_id,
                &event.machine_id,
                &event.instance_id,
                &reference.name,
                generator,
            ),
            ContextScope::Event => event.event_value(&reference.name).cloned().ok_or_else(|| {
                missing(
                    ContextScope::Event,
                    &reference.name,
                    format!("event.{}", event.instance_id),
                )
            }),
        }
    }

    fn resolve_global(
        &mut self,
        name: &str,
        generator: &mut impl ContextValueGenerator,
        stack: &mut BTreeSet<String>,
    ) -> Result<Value, RuntimeError> {
        if let Some(value) = self.global_values.get(name) {
            return Ok(value.clone());
        }

        let var = self
            .ir
            .context
            .global
            .get(name)
            .cloned()
            .ok_or_else(|| missing(ContextScope::Global, name, "ctx.global"))?;
        let location = format!("context.global.{name}");
        enter_stack(stack, &location)?;
        let value = self.eval_source(None, None, None, &var, &location, None, generator, stack)?;
        stack.remove(&location);
        self.global_values.insert(name.to_string(), value.clone());
        Ok(value)
    }

    fn resolve_system(
        &mut self,
        system_id: &str,
        name: &str,
        generator: &mut impl ContextValueGenerator,
        stack: &mut BTreeSet<String>,
    ) -> Result<Value, RuntimeError> {
        if let Some(value) = self
            .system_values
            .get(system_id)
            .and_then(|values| values.get(name))
        {
            return Ok(value.clone());
        }

        let var = self
            .ir
            .context
            .system
            .get(system_id)
            .and_then(|vars| vars.get(name))
            .cloned()
            .ok_or_else(|| {
                missing(
                    ContextScope::System,
                    name,
                    format!("ctx.system@{system_id}"),
                )
            })?;
        let location = format!("context.system.{system_id}.{name}");
        enter_stack(stack, &location)?;
        let value = self.eval_source(
            Some(system_id),
            None,
            None,
            &var,
            &location,
            None,
            generator,
            stack,
        )?;
        stack.remove(&location);
        self.system_values
            .entry(system_id.to_string())
            .or_default()
            .insert(name.to_string(), value.clone());
        Ok(value)
    }

    fn resolve_instance(
        &mut self,
        system_id: &str,
        machine_id: &str,
        instance_id: &str,
        name: &str,
        generator: &mut impl ContextValueGenerator,
        stack: &mut BTreeSet<String>,
    ) -> Result<Value, RuntimeError> {
        let key = InstanceKey::new(machine_id, instance_id);
        if let Some(value) = self
            .instance_values
            .get(&key)
            .and_then(|values| values.get(name))
        {
            return Ok(value.clone());
        }

        let var = self
            .ir
            .context
            .instance
            .get(machine_id)
            .and_then(|vars| vars.get(name))
            .cloned()
            .ok_or_else(|| {
                missing(
                    ContextScope::Instance,
                    name,
                    format!("ctx.instance@{machine_id}/{instance_id}"),
                )
            })?;
        let location = format!("context.instance.{machine_id}.{instance_id}.{name}");
        enter_stack(stack, &location)?;
        let value = self.eval_source(
            Some(system_id),
            Some((machine_id, instance_id)),
            None,
            &var,
            &location,
            None,
            generator,
            stack,
        )?;
        stack.remove(&location);
        self.instance_values
            .entry(key)
            .or_default()
            .insert(name.to_string(), value.clone());
        Ok(value)
    }

    #[allow(clippy::too_many_arguments)]
    fn resolve_event(
        &mut self,
        system_id: &str,
        machine_id: &str,
        instance_id: &str,
        name: &str,
        event_values: &mut BTreeMap<String, Value>,
        generator: &mut impl ContextValueGenerator,
        stack: &mut BTreeSet<String>,
    ) -> Result<Value, RuntimeError> {
        if let Some(value) = event_values.get(name) {
            return Ok(value.clone());
        }

        let var = self
            .ir
            .context
            .event
            .get(name)
            .cloned()
            .ok_or_else(|| missing(ContextScope::Event, name, "ctx.event"))?;
        let location = format!("context.event.{name}");
        enter_stack(stack, &location)?;
        let value = self.eval_source(
            Some(system_id),
            Some((machine_id, instance_id)),
            Some(event_values),
            &var,
            &location,
            None,
            generator,
            stack,
        )?;
        stack.remove(&location);
        event_values.insert(name.to_string(), value.clone());
        Ok(value)
    }

    #[allow(clippy::too_many_arguments)]
    fn eval_source(
        &mut self,
        system_id: Option<&str>,
        instance: Option<(&str, &str)>,
        event_values: Option<&mut BTreeMap<String, Value>>,
        var: &ContextVariable,
        location: &str,
        explicit_ref: Option<&ContextRef>,
        generator: &mut impl ContextValueGenerator,
        stack: &mut BTreeSet<String>,
    ) -> Result<Value, RuntimeError> {
        match &var.source {
            ValueSource::Const(value) => Ok(value.value.clone()),
            ValueSource::Generator(source) => {
                let definition = self.ir.generators.get(&source.generator).ok_or_else(|| {
                    RuntimeError::unknown_generator(&source.generator, location.to_string())
                })?;
                generator.generate(definition, &source.params, location)
            }
            ValueSource::Ref(source) => {
                let reference = explicit_ref
                    .cloned()
                    .map(Ok)
                    .unwrap_or_else(|| parse_ref(&source.reference))?;
                self.resolve_ref(
                    system_id,
                    instance,
                    event_values,
                    &reference,
                    generator,
                    stack,
                )
            }
        }
    }

    fn resolve_ref(
        &mut self,
        system_id: Option<&str>,
        instance: Option<(&str, &str)>,
        event_values: Option<&mut BTreeMap<String, Value>>,
        reference: &ContextRef,
        generator: &mut impl ContextValueGenerator,
        stack: &mut BTreeSet<String>,
    ) -> Result<Value, RuntimeError> {
        match reference.scope {
            ContextScope::Global => self.resolve_global(&reference.name, generator, stack),
            ContextScope::System => {
                let system_id = system_id.ok_or_else(|| {
                    missing(
                        reference.scope,
                        &reference.name,
                        "ctx.system without system",
                    )
                })?;
                self.resolve_system(system_id, &reference.name, generator, stack)
            }
            ContextScope::Instance => {
                let system_id = system_id.ok_or_else(|| {
                    missing(
                        reference.scope,
                        &reference.name,
                        "ctx.instance without system",
                    )
                })?;
                let (machine_id, instance_id) = instance.ok_or_else(|| {
                    missing(
                        reference.scope,
                        &reference.name,
                        "ctx.instance without instance",
                    )
                })?;
                self.resolve_instance(
                    system_id,
                    machine_id,
                    instance_id,
                    &reference.name,
                    generator,
                    stack,
                )
            }
            ContextScope::Event => {
                let system_id = system_id.ok_or_else(|| {
                    missing(reference.scope, &reference.name, "ctx.event without system")
                })?;
                let (machine_id, instance_id) = instance.ok_or_else(|| {
                    missing(
                        reference.scope,
                        &reference.name,
                        "ctx.event without instance",
                    )
                })?;
                let event_values = event_values.ok_or_else(|| {
                    missing(reference.scope, &reference.name, "ctx.event without event")
                })?;
                self.resolve_event(
                    system_id,
                    machine_id,
                    instance_id,
                    &reference.name,
                    event_values,
                    generator,
                    stack,
                )
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EventContext {
    pub system_id: Id,
    pub machine_id: Id,
    pub instance_id: Id,
    event_values: BTreeMap<String, Value>,
}

impl EventContext {
    pub fn event_values(&self) -> &BTreeMap<String, Value> {
        &self.event_values
    }

    pub fn event_value(&self, name: &str) -> Option<&Value> {
        self.event_values.get(name)
    }
}

fn parse_ref(reference: &str) -> Result<ContextRef, RuntimeError> {
    let mut parts = reference.split('.');
    let prefix = parts.next();
    let scope = parts.next();
    let name = parts.next();

    if prefix != Some("ctx") || parts.next().is_some() {
        return Err(RuntimeError::invalid_context_value(
            reference,
            "context ref must use ctx.<global|system|instance|event>.<name>",
        ));
    }

    let scope = match scope {
        Some("global") => ContextScope::Global,
        Some("system") => ContextScope::System,
        Some("instance") => ContextScope::Instance,
        Some("event") => ContextScope::Event,
        _ => {
            return Err(RuntimeError::invalid_context_value(
                reference,
                "unknown context scope",
            ));
        }
    };
    let name = name.filter(|value| !value.is_empty()).ok_or_else(|| {
        RuntimeError::invalid_context_value(reference, "context variable name is empty")
    })?;

    Ok(ContextRef {
        scope,
        name: name.to_string(),
    })
}

fn enter_stack(stack: &mut BTreeSet<String>, location: &str) -> Result<(), RuntimeError> {
    if !stack.insert(location.to_string()) {
        return Err(RuntimeError::invalid_context_value(
            location,
            "cyclic context reference detected",
        ));
    }
    Ok(())
}

fn missing(
    scope: ContextScope,
    name: impl AsRef<str>,
    location: impl Into<String>,
) -> RuntimeError {
    RuntimeError::missing_context_variable(scope.as_str(), name.as_ref(), location)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::*;
    use serde_json::json;

    #[derive(Default)]
    struct CountingGenerator {
        next: usize,
        calls: Vec<String>,
    }

    impl ContextValueGenerator for CountingGenerator {
        fn generate(
            &mut self,
            generator: &Generator,
            _params: &JsonObject,
            location: &str,
        ) -> Result<Value, RuntimeError> {
            self.next += 1;
            self.calls.push(format!("{}@{}", generator.id, location));
            Ok(json!(format!("{}-{}", generator.id, self.next)))
        }
    }

    fn var(source: ValueSource) -> ContextVariable {
        ContextVariable {
            source,
            description: None,
        }
    }

    fn generator_var(id: &str) -> ContextVariable {
        var(ValueSource::Generator(GeneratorValue {
            generator: id.to_string(),
            params: JsonObject::new(),
        }))
    }

    fn const_var(value: Value) -> ContextVariable {
        var(ValueSource::Const(ConstValue { value }))
    }

    fn ref_var(reference: &str) -> ContextVariable {
        var(ValueSource::Ref(ContextRefValue {
            reference: reference.to_string(),
        }))
    }

    fn test_ir() -> Ir {
        Ir {
            ir_version: "0.1".to_string(),
            systems: ObjectMap::new(),
            generators: BTreeMap::from([
                (
                    "stable".to_string(),
                    Generator {
                        id: "stable".to_string(),
                        generator_type: "fake".to_string(),
                        params: JsonObject::new(),
                        description: None,
                    },
                ),
                (
                    "event".to_string(),
                    Generator {
                        id: "event".to_string(),
                        generator_type: "fake".to_string(),
                        params: JsonObject::new(),
                        description: None,
                    },
                ),
            ]),
            context: Context {
                global: BTreeMap::from([("tenant".to_string(), const_var(json!("acme")))]),
                system: BTreeMap::from([(
                    "linux".to_string(),
                    BTreeMap::from([
                        ("host".to_string(), generator_var("stable")),
                        ("tenant_copy".to_string(), ref_var("ctx.global.tenant")),
                    ]),
                )]),
                instance: BTreeMap::from([(
                    "login".to_string(),
                    BTreeMap::from([
                        ("session".to_string(), generator_var("stable")),
                        ("host_copy".to_string(), ref_var("ctx.system.host")),
                    ]),
                )]),
                event: BTreeMap::from([
                    ("pid".to_string(), generator_var("event")),
                    ("session_copy".to_string(), ref_var("ctx.instance.session")),
                ]),
            },
            templates: ObjectMap::new(),
            machines: ObjectMap::new(),
            instance_groups: ObjectMap::new(),
            time_profiles: ObjectMap::new(),
            scenario: Scenario {
                id: "s".to_string(),
                systems: vec![],
                instance_groups: vec![],
                time_profile: "t".to_string(),
                description: None,
            },
        }
    }

    #[test]
    fn instance_scope_is_stable_inside_one_instance() {
        let ir = test_ir();
        let mut store = ContextStore::new(&ir);
        let mut value_gen = CountingGenerator::default();

        let first = store
            .instance_value("linux", "login", "i-1", "session", &mut value_gen)
            .unwrap();
        let second = store
            .instance_value("linux", "login", "i-1", "session", &mut value_gen)
            .unwrap();

        assert_eq!(first, second);
        assert_eq!(value_gen.calls.len(), 1);
    }

    #[test]
    fn instance_scope_is_isolated_between_instances() {
        let ir = test_ir();
        let mut store = ContextStore::new(&ir);
        let mut value_gen = CountingGenerator::default();

        let left = store
            .instance_value("linux", "login", "i-1", "session", &mut value_gen)
            .unwrap();
        let right = store
            .instance_value("linux", "login", "i-2", "session", &mut value_gen)
            .unwrap();

        assert_ne!(left, right);
        assert_eq!(value_gen.calls.len(), 2);
    }

    #[test]
    fn event_scope_is_regenerated_for_every_event() {
        let ir = test_ir();
        let mut store = ContextStore::new(&ir);
        let mut value_gen = CountingGenerator::default();

        let first = store
            .event_context("linux", "login", "i-1", &mut value_gen)
            .unwrap();
        let second = store
            .event_context("linux", "login", "i-1", &mut value_gen)
            .unwrap();

        assert_ne!(first.event_value("pid"), second.event_value("pid"));
        assert_eq!(
            first.event_value("session_copy"),
            second.event_value("session_copy")
        );
    }

    #[test]
    fn public_ref_resolution_reads_all_four_scopes() {
        let ir = test_ir();
        let mut store = ContextStore::new(&ir);
        let mut value_gen = CountingGenerator::default();
        let event = store
            .event_context("linux", "login", "i-1", &mut value_gen)
            .unwrap();

        assert_eq!(
            store
                .value_for_ref("ctx.global.tenant", &event, &mut value_gen)
                .unwrap(),
            json!("acme")
        );
        assert_eq!(
            store
                .value_for_ref("ctx.event.pid", &event, &mut value_gen)
                .unwrap(),
            event.event_value("pid").cloned().unwrap()
        );
    }

    #[test]
    fn system_and_global_refs_are_cached() {
        let ir = test_ir();
        let mut store = ContextStore::new(&ir);
        let mut value_gen = CountingGenerator::default();

        let first = store
            .instance_value("linux", "login", "i-1", "host_copy", &mut value_gen)
            .unwrap();
        let second = store.system_value("linux", "host", &mut value_gen).unwrap();
        let tenant = store
            .system_value("linux", "tenant_copy", &mut value_gen)
            .unwrap();

        assert_eq!(first, second);
        assert_eq!(tenant, json!("acme"));
        assert_eq!(value_gen.calls.len(), 1);
    }
}
