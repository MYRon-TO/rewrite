use crate::{
    error::ValidationError,
    ir::{Id, Ir},
};

pub fn validate_identity(ir: &Ir) -> Result<(), ValidationError> {
    for (key, system) in &ir.systems {
        ensure_key_matches_id("systems", key, &system.id)?;
    }
    for (key, generator) in &ir.generators {
        ensure_key_matches_id("generators", key, &generator.id)?;
    }
    for (key, template) in &ir.templates {
        ensure_key_matches_id("templates", key, &template.id)?;
    }
    for (key, machine) in &ir.machines {
        ensure_key_matches_id("machines", key, &machine.id)?;
    }
    for (key, group) in &ir.instance_groups {
        ensure_key_matches_id("instance_groups", key, &group.id)?;
    }
    for (key, profile) in &ir.time_profiles {
        ensure_key_matches_id("time_profiles", key, &profile.id)?;
    }

    Ok(())
}

fn ensure_key_matches_id(collection: &str, key: &str, id: &Id) -> Result<(), ValidationError> {
    if key == id {
        Ok(())
    } else {
        Err(ValidationError::invalid_value(
            format!("{collection}.{key}"),
            "id",
            format!("map key `{key}` must match inner id `{id}`"),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::Ir;

    fn generated_ir() -> Ir {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/generated/examples/ssh_bruteforce.json"
        );
        Ir::from_path(path).expect("generated example should deserialize")
    }

    #[test]
    fn generated_ssh_bruteforce_identity_passes() {
        validate_identity(&generated_ir()).unwrap();
    }

    #[test]
    fn rejects_map_key_and_inner_id_mismatch() {
        let mut ir = generated_ir();
        ir.systems.get_mut("linux_auth").unwrap().id = "wrong_id".to_owned();

        let err = validate_identity(&ir).expect_err("id mismatch should fail");

        assert!(err.to_string().contains("map key `linux_auth`"));
        assert!(err.to_string().contains("inner id `wrong_id`"));
    }
}
