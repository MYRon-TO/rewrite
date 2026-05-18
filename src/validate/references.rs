use std::collections::BTreeSet;

use crate::error::{Result, ValidationError};
use crate::ir::{Ir, TemplateValueExpr, ValueSource};

/// Validate cross-object references and context-variable references in an IR.
///
/// This module intentionally does not execute generators, render templates, or run machines.
pub fn validate_references(ir: &Ir) -> Result<()> {
    let index = ReferenceIndex::new(ir);

    validate_scenario_references(ir, &index)?;
    validate_context_references(ir, &index)?;
    validate_template_references(ir, &index)?;
    validate_machine_references(ir, &index)?;
    validate_instance_group_references(ir, &index)?;

    Ok(())
}

struct ReferenceIndex<'a> {
    systems: BTreeSet<&'a str>,
    generators: BTreeSet<&'a str>,
    templates: BTreeSet<&'a str>,
    machines: BTreeSet<&'a str>,
    instance_groups: BTreeSet<&'a str>,
    time_profiles: BTreeSet<&'a str>,
}

impl<'a> ReferenceIndex<'a> {
    fn new(ir: &'a Ir) -> Self {
        Self {
            systems: ir.systems.keys().map(String::as_str).collect(),
            generators: ir.generators.keys().map(String::as_str).collect(),
            templates: ir.templates.keys().map(String::as_str).collect(),
            machines: ir.machines.keys().map(String::as_str).collect(),
            instance_groups: ir.instance_groups.keys().map(String::as_str).collect(),
            time_profiles: ir.time_profiles.keys().map(String::as_str).collect(),
        }
    }
}

fn validate_scenario_references(ir: &Ir, index: &ReferenceIndex<'_>) -> Result<()> {
    let owner = format!("scenario.{}", ir.scenario.id);

    for (idx, system_id) in ir.scenario.systems.iter().enumerate() {
        ensure_exists(
            index.systems.contains(system_id.as_str()),
            &owner,
            format!("systems[{idx}]"),
            "system",
            system_id,
        )?;
    }

    for (idx, group_id) in ir.scenario.instance_groups.iter().enumerate() {
        ensure_exists(
            index.instance_groups.contains(group_id.as_str()),
            &owner,
            format!("instance_groups[{idx}]"),
            "instance_group",
            group_id,
        )?;
    }

    ensure_exists(
        index
            .time_profiles
            .contains(ir.scenario.time_profile.as_str()),
        &owner,
        "time_profile",
        "time_profile",
        &ir.scenario.time_profile,
    )
}

fn validate_context_references(ir: &Ir, index: &ReferenceIndex<'_>) -> Result<()> {
    for (name, variable) in &ir.context.global {
        let owner = format!("context.global.{name}");
        validate_context_source(
            ir,
            index,
            &owner,
            "source",
            &variable.source,
            ContextBinding::global(),
        )?;
    }

    for (system_id, variables) in &ir.context.system {
        ensure_exists(
            index.systems.contains(system_id.as_str()),
            format!("context.system.{system_id}"),
            "<scope>",
            "system",
            system_id,
        )?;

        for (name, variable) in variables {
            let owner = format!("context.system.{system_id}.{name}");
            validate_context_source(
                ir,
                index,
                &owner,
                "source",
                &variable.source,
                ContextBinding::system(system_id),
            )?;
        }
    }

    for (machine_id, variables) in &ir.context.instance {
        // IR v0.1 examples and contract key instance context by machine id.
        ensure_exists(
            index.machines.contains(machine_id.as_str()),
            format!("context.instance.{machine_id}"),
            "<scope>",
            "machine",
            machine_id,
        )?;

        let system_id = ir
            .machines
            .get(machine_id)
            .map(|machine| machine.system.as_str());
        for (name, variable) in variables {
            let owner = format!("context.instance.{machine_id}.{name}");
            validate_context_source(
                ir,
                index,
                &owner,
                "source",
                &variable.source,
                ContextBinding {
                    system_id,
                    machine_id: Some(machine_id),
                    allow_any_system: system_id.is_none(),
                    allow_any_instance: false,
                    max_scope: ContextScope::Instance,
                },
            )?;
        }
    }

    for (name, variable) in &ir.context.event {
        let owner = format!("context.event.{name}");
        validate_context_source(
            ir,
            index,
            &owner,
            "source",
            &variable.source,
            ContextBinding::event(),
        )?;
    }

    Ok(())
}

fn validate_template_references(ir: &Ir, index: &ReferenceIndex<'_>) -> Result<()> {
    for (template_id, template) in &ir.templates {
        let owner = format!("template.{template_id}");
        ensure_exists(
            index.systems.contains(template.system.as_str()),
            &owner,
            "system",
            "system",
            &template.system,
        )?;

        for (field_name, expr) in &template.fields {
            let field_owner = format!("template.{template_id}.fields.{field_name}");
            for context_ref in context_refs_from_template_expr(expr, &field_owner)? {
                validate_context_ref(
                    ir,
                    &context_ref,
                    &field_owner,
                    context_ref.field_name(),
                    ContextBinding {
                        system_id: Some(&template.system),
                        machine_id: None,
                        allow_any_system: false,
                        allow_any_instance: true,
                        max_scope: ContextScope::Event,
                    },
                )?;
            }
        }
    }

    Ok(())
}

fn validate_machine_references(ir: &Ir, index: &ReferenceIndex<'_>) -> Result<()> {
    for (machine_id, machine) in &ir.machines {
        let owner = format!("machine.{machine_id}");
        ensure_exists(
            index.systems.contains(machine.system.as_str()),
            &owner,
            "system",
            "system",
            &machine.system,
        )?;

        for (state_id, state) in &machine.states {
            for (emit_idx, emit) in state.emit.iter().enumerate() {
                let emit_owner = format!("machine.{machine_id}.states.{state_id}.emit[{emit_idx}]");
                ensure_exists(
                    index.templates.contains(emit.template.as_str()),
                    &emit_owner,
                    "template",
                    "template",
                    &emit.template,
                )?;

                if let Some(template) = ir.templates.get(&emit.template) {
                    if template.system != machine.system {
                        return Err(ValidationError::invalid_value(
                            &emit_owner,
                            "template",
                            format!(
                                "template `{}` belongs to system `{}`, but machine `{machine_id}` belongs to system `{}`",
                                emit.template, template.system, machine.system
                            ),
                        )
                        .into());
                    }

                    for (field_name, expr) in &template.fields {
                        let field_owner = format!(
                            "machine.{machine_id}.states.{state_id}.emit[{emit_idx}].template.{}.fields.{field_name}",
                            emit.template
                        );
                        for context_ref in context_refs_from_template_expr(expr, &field_owner)? {
                            validate_context_ref(
                                ir,
                                &context_ref,
                                &field_owner,
                                context_ref.field_name(),
                                ContextBinding {
                                    system_id: Some(&machine.system),
                                    machine_id: Some(machine_id),
                                    allow_any_system: false,
                                    allow_any_instance: false,
                                    max_scope: ContextScope::Event,
                                },
                            )?;
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn validate_instance_group_references(ir: &Ir, index: &ReferenceIndex<'_>) -> Result<()> {
    for (group_id, group) in &ir.instance_groups {
        let owner = format!("instance_group.{group_id}");
        ensure_exists(
            index.machines.contains(group.machine.as_str()),
            &owner,
            "machine",
            "machine",
            &group.machine,
        )?;
    }

    Ok(())
}

fn validate_context_source(
    ir: &Ir,
    index: &ReferenceIndex<'_>,
    owner: &str,
    field: &'static str,
    source: &ValueSource,
    binding: ContextBinding<'_>,
) -> Result<()> {
    match source {
        ValueSource::Const(_) => Ok(()),
        ValueSource::Generator(generator_value) => ensure_exists(
            index
                .generators
                .contains(generator_value.generator.as_str()),
            owner,
            field,
            "generator",
            &generator_value.generator,
        ),
        ValueSource::Ref(context_ref_value) => {
            let context_ref = parse_context_ref(&context_ref_value.reference, owner, field)?;
            if context_ref.scope > binding.max_scope {
                return Err(ValidationError::invalid_value(
                    owner,
                    field,
                    format!(
                        "context lifecycle violation: {max:?} scoped variable cannot reference `{raw}`",
                        max = binding.max_scope,
                        raw = context_ref.raw
                    ),
                )
                .into());
            }
            validate_context_ref(ir, &context_ref, owner, field, binding)
        }
    }
}

fn context_refs_from_template_expr(
    expr: &TemplateValueExpr,
    owner: &str,
) -> Result<Vec<ContextRef>> {
    match expr {
        TemplateValueExpr::Const(_) => Ok(Vec::new()),
        TemplateValueExpr::Ref(context_ref_value) => Ok(vec![parse_context_ref(
            &context_ref_value.reference,
            owner,
            "ref",
        )?]),
        TemplateValueExpr::Interpolate(interpolation_value) => {
            extract_interpolation_refs(&interpolation_value.interpolate, owner)
        }
    }
}

fn extract_interpolation_refs(input: &str, owner: &str) -> Result<Vec<ContextRef>> {
    let mut refs = Vec::new();
    let mut rest = input;

    while let Some(start) = rest.find("${") {
        let after_start = &rest[start + 2..];
        let Some(end) = after_start.find('}') else {
            return Err(ValidationError::invalid_value(
                owner,
                "interpolate",
                "unclosed interpolation placeholder",
            )
            .into());
        };

        let expr = after_start[..end].trim();
        refs.push(parse_context_ref(expr, owner, "interpolate")?);
        rest = &after_start[end + 1..];
    }

    Ok(refs)
}

fn validate_context_ref(
    ir: &Ir,
    context_ref: &ContextRef,
    owner: &str,
    field: &str,
    binding: ContextBinding<'_>,
) -> Result<()> {
    let exists = match context_ref.scope {
        ContextScope::Global => ir.context.global.contains_key(&context_ref.name),
        ContextScope::System => match binding.system_id {
            Some(system_id) => ir
                .context
                .system
                .get(system_id)
                .is_some_and(|variables| variables.contains_key(&context_ref.name)),
            None if binding.allow_any_system => ir
                .context
                .system
                .values()
                .any(|variables| variables.contains_key(&context_ref.name)),
            None => false,
        },
        ContextScope::Instance => match binding.machine_id {
            Some(machine_id) => ir
                .context
                .instance
                .get(machine_id)
                .is_some_and(|variables| variables.contains_key(&context_ref.name)),
            None if binding.allow_any_instance => ir
                .context
                .instance
                .values()
                .any(|variables| variables.contains_key(&context_ref.name)),
            None => false,
        },
        ContextScope::Event => ir.context.event.contains_key(&context_ref.name),
    };

    ensure_exists(
        exists,
        owner,
        field,
        "context variable",
        context_ref.raw.as_str(),
    )
}

fn parse_context_ref(raw: &str, owner: &str, field: &'static str) -> Result<ContextRef> {
    let parts = raw.split('.').collect::<Vec<_>>();
    if parts.len() != 3 || parts[0] != "ctx" || parts[2].is_empty() {
        return Err(ValidationError::invalid_value(
            owner,
            field,
            format!("expected context reference in form `ctx.<scope>.<name>`, got `{raw}`"),
        )
        .into());
    }

    let scope = match parts[1] {
        "global" => ContextScope::Global,
        "system" => ContextScope::System,
        "instance" => ContextScope::Instance,
        "event" => ContextScope::Event,
        other => {
            return Err(ValidationError::invalid_value(
                owner,
                field,
                format!("unknown context scope `{other}` in `{raw}`"),
            )
            .into());
        }
    };

    Ok(ContextRef {
        raw: raw.to_owned(),
        scope,
        name: parts[2].to_owned(),
        source_field: field,
    })
}

fn ensure_exists(
    exists: bool,
    owner: impl Into<String>,
    field: impl Into<String>,
    target_kind: impl Into<String>,
    target_id: impl Into<String>,
) -> Result<()> {
    if exists {
        Ok(())
    } else {
        Err(ValidationError::missing_reference(owner, field, target_kind, target_id).into())
    }
}

#[derive(Debug, Clone)]
struct ContextRef {
    raw: String,
    scope: ContextScope,
    name: String,
    source_field: &'static str,
}

impl ContextRef {
    fn field_name(&self) -> &'static str {
        self.source_field
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ContextScope {
    Global,
    System,
    Instance,
    Event,
}

#[derive(Debug, Clone, Copy)]
struct ContextBinding<'a> {
    system_id: Option<&'a str>,
    machine_id: Option<&'a str>,
    allow_any_system: bool,
    allow_any_instance: bool,
    max_scope: ContextScope,
}

impl ContextBinding<'_> {
    fn global() -> Self {
        Self {
            system_id: None,
            machine_id: None,
            allow_any_system: false,
            allow_any_instance: false,
            max_scope: ContextScope::Global,
        }
    }

    fn system(system_id: &str) -> ContextBinding<'_> {
        ContextBinding {
            system_id: Some(system_id),
            machine_id: None,
            allow_any_system: false,
            allow_any_instance: false,
            max_scope: ContextScope::System,
        }
    }

    fn event() -> Self {
        Self {
            system_id: None,
            machine_id: None,
            allow_any_system: true,
            allow_any_instance: true,
            max_scope: ContextScope::Event,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{EmitCountRange, StateKind};

    fn generated_ir() -> Ir {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/generated/examples/ssh_bruteforce.json"
        );
        Ir::from_path(path).expect("generated example should deserialize")
    }

    #[test]
    fn generated_ssh_bruteforce_references_pass() {
        validate_references(&generated_ir()).unwrap();
    }

    #[test]
    fn missing_scenario_system_fails() {
        let mut ir = generated_ir();
        ir.scenario.systems = vec!["missing_system".to_owned()];

        let err = validate_references(&ir).unwrap_err();
        assert!(
            err.to_string()
                .contains("missing system reference `missing_system`")
        );
    }

    #[test]
    fn missing_instance_group_machine_fails() {
        let mut ir = generated_ir();
        ir.instance_groups
            .get_mut("ssh_login_sessions")
            .unwrap()
            .machine = "missing_machine".to_owned();

        let err = validate_references(&ir).unwrap_err();
        assert!(
            err.to_string()
                .contains("missing machine reference `missing_machine`")
        );
    }

    #[test]
    fn missing_machine_system_fails() {
        let mut ir = generated_ir();
        ir.machines.get_mut("ssh_login_flow").unwrap().system = "missing_system".to_owned();

        let err = validate_references(&ir).unwrap_err();
        assert!(
            err.to_string()
                .contains("missing system reference `missing_system`")
        );
    }

    #[test]
    fn missing_emit_template_fails() {
        let mut ir = generated_ir();
        ir.machines
            .get_mut("ssh_login_flow")
            .unwrap()
            .states
            .get_mut("normal_login")
            .unwrap()
            .emit[0]
            .template = "missing_template".to_owned();

        let err = validate_references(&ir).unwrap_err();
        assert!(
            err.to_string()
                .contains("missing template reference `missing_template`")
        );
    }

    #[test]
    fn missing_template_context_ref_fails() {
        let mut ir = generated_ir();
        ir.context
            .instance
            .get_mut("ssh_login_flow")
            .unwrap()
            .remove("user");

        let err = validate_references(&ir).unwrap_err();
        assert!(
            err.to_string()
                .contains("missing context variable reference `ctx.instance.user`")
        );
    }

    #[test]
    fn context_generator_reference_is_checked() {
        let mut ir = generated_ir();
        if let ValueSource::Generator(generator_value) = &mut ir
            .context
            .instance
            .get_mut("ssh_login_flow")
            .unwrap()
            .get_mut("user")
            .unwrap()
            .source
        {
            generator_value.generator = "missing_generator".to_owned();
        }

        let err = validate_references(&ir).unwrap_err();
        assert!(
            err.to_string()
                .contains("missing generator reference `missing_generator`")
        );
    }

    #[test]
    fn keeps_ir_types_usable_for_machine_phase() {
        let ir = generated_ir();
        let machine = &ir.machines["ssh_login_flow"];
        assert_eq!(machine.states["bruteforce"].kind, StateKind::Attack);
        assert_eq!(
            machine.states["bruteforce"].emit[0].count,
            crate::ir::EmitCount::Range(EmitCountRange { min: 20, max: 80 })
        );
    }
}
