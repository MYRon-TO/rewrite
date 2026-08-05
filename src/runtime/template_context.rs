use serde_json::Value;

use crate::{
    context::{ContextStore, ContextValueGenerator, EventContext},
    error::RuntimeError,
    template::value_expr::{ContextRef, ContextValueResolver},
};

/// Adapter that lets template value expressions resolve context references
/// through the runtime context store.
///
/// `ContextStore` and the generator both mutate while resolving values: the
/// store fills scope caches, and generator-backed context values advance their
/// deterministic random stream. Keeping this adapter in `runtime` preserves the
/// template module's narrow dependency boundary.
pub struct RuntimeTemplateResolver<'a, 'ir, G>
where
    G: ContextValueGenerator,
{
    store: &'a mut ContextStore<'ir>,
    event: &'a EventContext,
    generator: &'a mut G,
}

impl<'a, 'ir, G> RuntimeTemplateResolver<'a, 'ir, G>
where
    G: ContextValueGenerator,
{
    pub fn new(
        store: &'a mut ContextStore<'ir>,
        event: &'a EventContext,
        generator: &'a mut G,
    ) -> Self {
        Self {
            store,
            event,
            generator,
        }
    }
}

impl<G> ContextValueResolver for RuntimeTemplateResolver<'_, '_, G>
where
    G: ContextValueGenerator,
{
    fn resolve_context_value(
        &mut self,
        reference: &ContextRef,
        _location: &str,
    ) -> Result<Value, RuntimeError> {
        self.store
            .value_for_ref(reference.raw(), self.event, self.generator)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::{Value, json};

    use super::*;
    use crate::{
        context::ContextValueGenerator,
        error::{Result, RuntimeError},
        ir::*,
        template::value_expr::eval_template_value_expr,
    };

    #[derive(Default)]
    struct CountingGenerator {
        next: usize,
    }

    impl ContextValueGenerator for CountingGenerator {
        fn generate(
            &mut self,
            generator: &Generator,
            _params: &JsonObject,
            _location: &str,
        ) -> Result<Value, RuntimeError> {
            self.next += 1;
            Ok(json!(format!("{}-{}", generator.id, self.next)))
        }
    }

    fn const_var(value: Value) -> ContextVariable {
        ContextVariable {
            source: ValueSource::Const(ConstValue { value }),
            description: None,
        }
    }

    fn generator_var(id: &str) -> ContextVariable {
        ContextVariable {
            source: ValueSource::Generator(GeneratorValue {
                generator: id.to_owned(),
                params: JsonObject::new(),
            }),
            description: None,
        }
    }

    fn test_ir() -> Ir {
        Ir {
            ir_version: "0.2".to_owned(),
            systems: BTreeMap::from([(
                "linux".to_owned(),
                System {
                    id: "linux".to_owned(),
                    kind: None,
                    description: None,
                },
            )]),
            generators: BTreeMap::from([(
                "session".to_owned(),
                Generator {
                    id: "session".to_owned(),
                    generator_type: "uuid".to_owned(),
                    params: JsonObject::new(),
                    description: None,
                },
            )]),
            context: Context {
                global: BTreeMap::from([("actor".to_owned(), const_var(json!({"user_id":"u-1"})))]),
                system: BTreeMap::from([(
                    "linux".to_owned(),
                    BTreeMap::from([("host".to_owned(), const_var(json!("host-1")))]),
                )]),
                instance: BTreeMap::from([(
                    "login".to_owned(),
                    BTreeMap::from([("session".to_owned(), generator_var("session"))]),
                )]),
                event: BTreeMap::from([("pid".to_owned(), const_var(json!(1234)))]),
            },
            templates: ObjectMap::new(),
            machines: ObjectMap::new(),
            instance_groups: ObjectMap::new(),
            time_profiles: ObjectMap::new(),
            scenario: Scenario {
                id: "s".to_owned(),
                systems: vec!["linux".to_owned()],
                instance_groups: vec![],
                time_profile: "t".to_owned(),
                description: None,
            },
        }
    }

    #[test]
    fn resolves_template_expressions_through_context_store() {
        let ir = test_ir();
        let mut store = ContextStore::new(&ir);
        let mut generator = CountingGenerator::default();
        let event = store
            .event_context("linux", "login", "i-1", &mut generator)
            .unwrap();
        let mut resolver = RuntimeTemplateResolver::new(&mut store, &event, &mut generator);

        let message = TemplateValueExpr::Interpolate(InterpolationValue {
            interpolate: "actor=${ctx.global.actor.user_id} host=${ctx.system.host} pid=${ctx.event.pid} session=${ctx.instance.session}".to_owned(),
        });

        assert_eq!(
            eval_template_value_expr(&message, &mut resolver, "template.t.fields.message").unwrap(),
            json!("actor=u-1 host=host-1 pid=1234 session=session-1")
        );
    }
}
