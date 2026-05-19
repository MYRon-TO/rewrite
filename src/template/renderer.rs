use serde_json::Value;

use crate::{
    context::{ContextStore, ContextValueGenerator, EventContext},
    error::RuntimeError,
    ir::{JsonObject, Template},
    machine::runner::EventDraft,
    template::value_expr::{ContextRef, ContextValueResolver, eval_template_value_expr},
};

/// Render one template into business fields for a concrete event draft.
///
/// This renderer has one narrow job: evaluate `template.fields` against the
/// runtime context attached to the draft and return the resulting field map. It
/// intentionally does not inspect `state.kind`, `machine.attack`, or
/// `state.attack_label`; mixing label decisions into field rendering is how a
/// clean MVP turns into a haunted spreadsheet.
pub fn render_template<G>(
    template: &Template,
    store: &mut ContextStore<'_>,
    generator: &mut G,
    draft: &EventDraft,
) -> Result<JsonObject, RuntimeError>
where
    G: ContextValueGenerator,
{
    if template.id != draft.template_id {
        return Err(RuntimeError::invalid_runtime_plan(
            format!("event.{}.template", draft.instance_id),
            format!(
                "draft references template `{}`, but renderer received `{}`",
                draft.template_id, template.id
            ),
        ));
    }

    render_template_fields(
        template,
        store,
        generator,
        &draft.event_context,
        &draft.system_id,
    )
}

/// Render template fields using an already-created event context.
///
/// This helper is useful for tests and for future assemblers that already hold
/// an `EventContext` but have not built a full `EventDraft` yet.
pub fn render_template_fields<G>(
    template: &Template,
    store: &mut ContextStore<'_>,
    generator: &mut G,
    event_context: &EventContext,
    system_id: &str,
) -> Result<JsonObject, RuntimeError>
where
    G: ContextValueGenerator,
{
    if template.system != system_id {
        return Err(RuntimeError::invalid_runtime_plan(
            format!("template.{}.system", template.id),
            format!(
                "template system `{}` does not match event system `{}`",
                template.system, system_id
            ),
        ));
    }

    let mut resolver = TemplateRenderResolver::new(store, event_context, generator);
    let mut fields = JsonObject::new();

    for (field_name, expr) in &template.fields {
        let location = format!("template.{}.fields.{field_name}", template.id);
        let value = eval_template_value_expr(expr, &mut resolver, location)?;
        fields.insert(field_name.clone(), value);
    }

    Ok(fields)
}

struct TemplateRenderResolver<'a, 'ir, G>
where
    G: ContextValueGenerator,
{
    store: &'a mut ContextStore<'ir>,
    event: &'a EventContext,
    generator: &'a mut G,
}

impl<'a, 'ir, G> TemplateRenderResolver<'a, 'ir, G>
where
    G: ContextValueGenerator,
{
    fn new(
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

impl<G> ContextValueResolver for TemplateRenderResolver<'_, '_, G>
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

    use chrono::{TimeZone, Utc};
    use serde_json::{Value, json};

    use super::*;
    use crate::{
        context::ContextValueGenerator, error::Result, ir::*, machine::runner::EventDraft,
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
            ir_version: "0.1".to_owned(),
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
                    generator_type: "fake".to_owned(),
                    params: JsonObject::new(),
                    description: None,
                },
            )]),
            context: Context {
                global: BTreeMap::from([(
                    "tenant".to_owned(),
                    const_var(json!({"id":"acme","tier":"prod"})),
                )]),
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

    fn template(id: &str) -> Template {
        Template {
            id: id.to_owned(),
            system: "linux".to_owned(),
            event_type: Some("auth".to_owned()),
            description: None,
            fields: BTreeMap::from([
                (
                    "constant".to_owned(),
                    TemplateValueExpr::Const(ConstValue { value: json!(true) }),
                ),
                (
                    "tenant".to_owned(),
                    TemplateValueExpr::Ref(ContextRefValue {
                        reference: "ctx.global.tenant.id".to_owned(),
                    }),
                ),
                (
                    "message".to_owned(),
                    TemplateValueExpr::Interpolate(InterpolationValue {
                        interpolate:
                            "host=${ctx.system.host} pid=${ctx.event.pid} session=${ctx.instance.session}"
                                .to_owned(),
                    }),
                ),
            ]),
        }
    }

    fn draft(event_context: EventContext, template_id: &str, state_kind: StateKind) -> EventDraft {
        EventDraft {
            scenario_id: "s".to_owned(),
            system_id: "linux".to_owned(),
            instance_id: "i-1".to_owned(),
            machine_id: "login".to_owned(),
            state_id: "start".to_owned(),
            state_kind,
            template_id: template_id.to_owned(),
            timestamp: Utc.with_ymd_and_hms(2026, 5, 17, 0, 0, 0).single().unwrap(),
            event_context,
        }
    }

    #[test]
    fn renders_const_ref_and_interpolated_fields() {
        let ir = test_ir();
        let mut store = ContextStore::new(&ir);
        let mut generator = CountingGenerator::default();
        let event_context = store
            .event_context("linux", "login", "i-1", &mut generator)
            .unwrap();
        let draft = draft(event_context, "ssh_login", StateKind::Benign);

        let fields = render_template(&template("ssh_login"), &mut store, &mut generator, &draft)
            .expect("template should render");

        assert_eq!(fields["constant"], json!(true));
        assert_eq!(fields["tenant"], json!("acme"));
        assert_eq!(
            fields["message"],
            json!("host=host-1 pid=1234 session=session-1")
        );
    }

    #[test]
    fn does_not_change_output_based_on_attack_state() {
        let ir = test_ir();
        let mut store = ContextStore::new(&ir);
        let mut generator = CountingGenerator::default();
        let event_context = store
            .event_context("linux", "login", "i-1", &mut generator)
            .unwrap();
        let benign = draft(event_context.clone(), "ssh_login", StateKind::Benign);
        let attack = draft(event_context, "ssh_login", StateKind::Attack);
        let template = template("ssh_login");

        let benign_fields =
            render_template(&template, &mut store, &mut generator, &benign).unwrap();
        let attack_fields =
            render_template(&template, &mut store, &mut generator, &attack).unwrap();

        assert_eq!(benign_fields, attack_fields);
    }

    #[test]
    fn rejects_template_mismatch_instead_of_rendering_wrong_shape() {
        let ir = test_ir();
        let mut store = ContextStore::new(&ir);
        let mut generator = CountingGenerator::default();
        let event_context = store
            .event_context("linux", "login", "i-1", &mut generator)
            .unwrap();
        let draft = draft(event_context, "expected", StateKind::Benign);

        let err = render_template(&template("other"), &mut store, &mut generator, &draft)
            .expect_err("mismatched template should fail");

        assert!(matches!(err, RuntimeError::InvalidRuntimePlan { .. }));
    }
}
