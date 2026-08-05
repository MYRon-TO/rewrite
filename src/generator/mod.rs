mod const_str;
mod machine_secret;

use std::{collections::BTreeMap, net::Ipv4Addr};

use rand::{RngExt, SeedableRng, rngs::StdRng};
use serde_json::{Number, Value};

use crate::{
    context::ContextValueGenerator,
    error::RuntimeError,
    ir::{Generator, JsonObject},
};

use const_str::*;

pub const BUILTIN_GENERATOR_TYPES: &[&str] = &[
    "fixed",
    "const",
    "choice",
    "number_range",
    "range",
    "ipv4",
    "username",
    "simple_username",
    "simple_words",
    "simple_filename",
    "uuid",
    "session_id",
    "sequence",
    "machine_secret",
];

pub fn is_builtin_generator_type(generator_type: &str) -> bool {
    BUILTIN_GENERATOR_TYPES.contains(&generator_type)
}

#[derive(Debug)]
pub struct BuiltinGenerator {
    rng: StdRng,
    sequences: BTreeMap<String, i64>,
}

impl BuiltinGenerator {
    pub fn with_seed(seed: u64) -> Self {
        Self {
            rng: StdRng::seed_from_u64(seed),
            sequences: BTreeMap::new(),
        }
    }

    pub fn generate_value(
        &mut self,
        generator: &Generator,
        params: &JsonObject,
        location: &str,
    ) -> Result<Value, RuntimeError> {
        match generator.generator_type.as_str() {
            "fixed" | "const" => Ok(param(generator, params, "value", location)?.clone()),
            "choice" => self.choice(generator, params, location),
            "number_range" | "range" => self.number_range(generator, params, location),
            "ipv4" => self.ipv4(generator, params, location),
            "username" | "simple_username" | "simple_words" => self.simple_username(generator, params, location),
            "uuid" => Ok(Value::String(self.uuid_v4())),
            "session_id" => Ok(Value::String(format!("sess-{}", self.hex_bytes(16)))),
            "sequence" => self.sequence(generator, params, location),
            "simple_filename" => self.simple_filename(generator, params, location),
            "machine_secret" => machine_secret::generate(&mut self.rng, generator, params, location),
            other => Err(invalid(
                location,
                format!(
                    "unsupported builtin generator type `{other}` for generator `{}`",
                    generator.id
                ),
            )),
        }
    }

    fn choice(
        &mut self,
        generator: &Generator,
        params: &JsonObject,
        location: &str,
    ) -> Result<Value, RuntimeError> {
        let values = param(generator, params, "values", location)?
            .as_array()
            .ok_or_else(|| invalid(location, "choice.values must be an array"))?;
        if values.is_empty() {
            return Err(invalid(location, "choice.values must not be empty"));
        }

        let index = self.rng.random_range(0..values.len());
        Ok(values[index].clone())
    }

    fn number_range(
        &mut self,
        generator: &Generator,
        params: &JsonObject,
        location: &str,
    ) -> Result<Value, RuntimeError> {
        let min = integer_param(generator, params, "min", location)?;
        let max = integer_param(generator, params, "max", location)?;
        if min > max {
            return Err(invalid(location, "number_range.min must be <= max"));
        }

        let value = self.rng.random_range(min..=max);
        Ok(Value::Number(Number::from(value)))
    }

    fn ipv4(
        &mut self,
        generator: &Generator,
        params: &JsonObject,
        location: &str,
    ) -> Result<Value, RuntimeError> {
        if let Some(cidr) = optional_string_param(generator, params, "cidr", location)? {
            return Ok(Value::String(self.ipv4_from_cidr(cidr, location)?));
        }

        let start = optional_string_param(generator, params, "start", location)?
            .unwrap_or("10.0.0.1")
            .parse::<Ipv4Addr>()
            .map_err(|err| invalid(location, format!("ipv4.start is invalid: {err}")))?;
        let end = optional_string_param(generator, params, "end", location)?
            .unwrap_or("10.0.0.254")
            .parse::<Ipv4Addr>()
            .map_err(|err| invalid(location, format!("ipv4.end is invalid: {err}")))?;
        let start = u32::from(start);
        let end = u32::from(end);
        if start > end {
            return Err(invalid(location, "ipv4.start must be <= end"));
        }

        Ok(Value::String(
            Ipv4Addr::from(self.rng.random_range(start..=end)).to_string(),
        ))
    }

    fn ipv4_from_cidr(&mut self, cidr: &str, location: &str) -> Result<String, RuntimeError> {
        let (base, prefix) = cidr
            .split_once('/')
            .ok_or_else(|| invalid(location, "ipv4.cidr must use a.b.c.d/prefix format"))?;
        let base = base
            .parse::<Ipv4Addr>()
            .map_err(|err| invalid(location, format!("ipv4.cidr address is invalid: {err}")))?;
        let prefix = prefix
            .parse::<u32>()
            .map_err(|err| invalid(location, format!("ipv4.cidr prefix is invalid: {err}")))?;
        if prefix > 32 {
            return Err(invalid(location, "ipv4.cidr prefix must be <= 32"));
        }

        let host_bits = 32 - prefix;
        let host_mask = if host_bits == 32 {
            u32::MAX
        } else {
            (1u32 << host_bits) - 1
        };
        let network = u32::from(base) & !host_mask;
        let host = if host_mask == 0 {
            0
        } else {
            self.rng.random_range(0..=host_mask)
        };

        Ok(Ipv4Addr::from(network | host).to_string())
    }

    fn simple_filename(
        &mut self,
        generator: &Generator,
        params: &JsonObject,
        location: &str,
    ) -> Result<Value, RuntimeError> {
        let a_values = string_array_param(generator, params, "a", location)?.unwrap_or_else(|| {
            DEFAULT_ADJ
                .iter()
                .map(|value| value.to_string())
                .collect()
        });
        let b_values = string_array_param(generator, params, "b", location)?.unwrap_or_else(|| {
            DEFAULT_NAME
                .iter()
                .map(|value| value.to_string())
                .collect()
        });

        let file_suffix = string_array_param(generator, params, "c", location)?.unwrap_or_else(|| {
            DEFAULT_FILE_SUFFIX
                .iter()
                .map(|value| value.to_string())
                .collect()
        });

        if a_values.is_empty() || b_values.is_empty() || file_suffix.is_empty() {
            return Err(invalid(
                location,
                "arrays `a`, `b` and `file_suffix` must not be empty",
            ));
        }

        let a = &a_values[self.rng.random_range(0..a_values.len())];
        let b = &b_values[self.rng.random_range(0..b_values.len())];
        let c = self.rng.random_range(0..100).to_string();
        let s = &file_suffix[self.rng.random_range(0..file_suffix.len())];
        Ok(Value::String(format!("{a}-{b}{c}.{s}")))
    }

    fn simple_username(
        &mut self,
        generator: &Generator,
        params: &JsonObject,
        location: &str,
    ) -> Result<Value, RuntimeError> {
        let a_values = string_array_param(generator, params, "a", location)?.unwrap_or_else(|| {
            DEFAULT_ADJ
                .iter()
                .map(|value| value.to_string())
                .collect()
        });
        let b_values = string_array_param(generator, params, "b", location)?.unwrap_or_else(|| {
            DEFAULT_NAME
                .iter()
                .map(|value| value.to_string())
                .collect()
        });

        if a_values.is_empty() || b_values.is_empty() {
            return Err(invalid(
                location,
                "username arrays `a` and `b` must not be empty",
            ));
        }

        let a = &a_values[self.rng.random_range(0..a_values.len())];
        let b = &b_values[self.rng.random_range(0..b_values.len())];
        let c = self.rng.random_range(0..100).to_string();
        Ok(Value::String(format!("{a}{b}{c}")))
    }

    fn sequence(
        &mut self,
        generator: &Generator,
        params: &JsonObject,
        location: &str,
    ) -> Result<Value, RuntimeError> {
        let start = optional_integer_param(generator, params, "start", location)?.unwrap_or(1);
        let step = optional_integer_param(generator, params, "step", location)?.unwrap_or(1);
        if step == 0 {
            return Err(invalid(location, "sequence.step must not be 0"));
        }

        let key = optional_string_param(generator, params, "key", location)?
            .unwrap_or(&generator.id)
            .to_owned();
        let current = self.sequences.entry(key).or_insert(start);
        let value = *current;
        *current = current
            .checked_add(step)
            .ok_or_else(|| invalid(location, "sequence value overflow"))?;

        let prefix = optional_string_param(generator, params, "prefix", location)?.unwrap_or("");
        let suffix = optional_string_param(generator, params, "suffix", location)?.unwrap_or("");
        let pad = optional_integer_param(generator, params, "pad", location)?.unwrap_or(0);
        if pad < 0 {
            return Err(invalid(location, "sequence.pad must be >= 0"));
        }

        let number = if pad == 0 {
            value.to_string()
        } else {
            format!("{:0width$}", value, width = pad as usize)
        };
        Ok(Value::String(format!("{prefix}{number}{suffix}")))
    }

    fn uuid_v4(&mut self) -> String {
        let mut bytes: [u8; 16] = self.rng.random();
        bytes[6] = (bytes[6] & 0x0f) | 0x40;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;

        format!(
            "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            bytes[0],
            bytes[1],
            bytes[2],
            bytes[3],
            bytes[4],
            bytes[5],
            bytes[6],
            bytes[7],
            bytes[8],
            bytes[9],
            bytes[10],
            bytes[11],
            bytes[12],
            bytes[13],
            bytes[14],
            bytes[15]
        )
    }

    fn hex_bytes(&mut self, len: usize) -> String {
        (0..len)
            .map(|_| UUID_HEX[self.rng.random_range(0..UUID_HEX.len())] as char)
            .collect()
    }
}

impl ContextValueGenerator for BuiltinGenerator {
    fn generate(
        &mut self,
        generator: &Generator,
        params: &JsonObject,
        location: &str,
    ) -> Result<Value, RuntimeError> {
        self.generate_value(generator, params, location)
    }
}

fn param<'a>(
    generator: &'a Generator,
    params: &'a JsonObject,
    name: &str,
    location: &str,
) -> Result<&'a Value, RuntimeError> {
    params
        .get(name)
        .or_else(|| generator.params.get(name))
        .ok_or_else(|| invalid(location, format!("missing generator param `{name}`")))
}

fn optional_param<'a>(
    generator: &'a Generator,
    params: &'a JsonObject,
    name: &str,
) -> Option<&'a Value> {
    params.get(name).or_else(|| generator.params.get(name))
}

fn integer_param(
    generator: &Generator,
    params: &JsonObject,
    name: &str,
    location: &str,
) -> Result<i64, RuntimeError> {
    param(generator, params, name, location)?
        .as_i64()
        .ok_or_else(|| {
            invalid(
                location,
                format!("generator param `{name}` must be an integer"),
            )
        })
}

fn optional_integer_param(
    generator: &Generator,
    params: &JsonObject,
    name: &str,
    location: &str,
) -> Result<Option<i64>, RuntimeError> {
    optional_param(generator, params, name)
        .map(|value| {
            value.as_i64().ok_or_else(|| {
                invalid(
                    location,
                    format!("generator param `{name}` must be an integer"),
                )
            })
        })
        .transpose()
}

fn optional_string_param<'a>(
    generator: &'a Generator,
    params: &'a JsonObject,
    name: &str,
    location: &str,
) -> Result<Option<&'a str>, RuntimeError> {
    optional_param(generator, params, name)
        .map(|value| {
            value.as_str().ok_or_else(|| {
                invalid(
                    location,
                    format!("generator param `{name}` must be a string"),
                )
            })
        })
        .transpose()
}

fn string_array_param(
    generator: &Generator,
    params: &JsonObject,
    name: &str,
    location: &str,
) -> Result<Option<Vec<String>>, RuntimeError> {
    optional_param(generator, params, name)
        .map(|value| {
            let values = value.as_array().ok_or_else(|| {
                invalid(
                    location,
                    format!("generator param `{name}` must be a string array"),
                )
            })?;
            values
                .iter()
                .map(|value| {
                    value.as_str().map(str::to_string).ok_or_else(|| {
                        invalid(
                            location,
                            format!("generator param `{name}` must contain only strings"),
                        )
                    })
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()
}

fn invalid(location: &str, message: impl Into<String>) -> RuntimeError {
    RuntimeError::invalid_context_value(location.to_string(), message.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_generator(generator_type: &str, params: Value) -> Generator {
        Generator {
            id: format!("{generator_type}_gen"),
            generator_type: generator_type.to_string(),
            params: params.as_object().unwrap().clone().into_iter().collect(),
            description: None,
        }
    }

    #[test]
    fn fixed_returns_configured_value() {
        let generator = make_generator("fixed", json!({"value":"hello"}));
        let mut builtins = BuiltinGenerator::with_seed(1);

        assert_eq!(
            builtins
                .generate_value(&generator, &JsonObject::new(), "test")
                .unwrap(),
            json!("hello")
        );
    }

    #[test]
    fn choice_picks_value_from_list() {
        let generator = make_generator("choice", json!({"values":["alice","bob","carol"]}));
        let mut builtins = BuiltinGenerator::with_seed(7);

        let value = builtins
            .generate_value(&generator, &JsonObject::new(), "test")
            .unwrap();

        assert!(
            generator.params["values"]
                .as_array()
                .unwrap()
                .contains(&value)
        );
    }

    #[test]
    fn number_range_returns_integer_inside_bounds() {
        let generator = make_generator("number_range", json!({"min":1000,"max":1002}));
        let mut builtins = BuiltinGenerator::with_seed(11);

        for _ in 0..16 {
            let value = builtins
                .generate_value(&generator, &JsonObject::new(), "test")
                .unwrap();
            let value = value.as_i64().unwrap();
            assert!((1000..=1002).contains(&value));
        }
    }

    #[test]
    fn ipv4_generates_address_inside_cidr() {
        let generator = make_generator("ipv4", json!({"cidr":"192.0.2.0/24"}));
        let mut builtins = BuiltinGenerator::with_seed(13);

        let value = builtins
            .generate_value(&generator, &JsonObject::new(), "test")
            .unwrap();
        let ip: Ipv4Addr = value.as_str().unwrap().parse().unwrap();
        let octets = ip.octets();
        assert_eq!(&octets[..3], &[192, 0, 2]);
    }

    #[test]
    fn username_uses_a_plus_b_plus_number_shape() {
        let generator = make_generator("username", json!({"a":["red"],"b":["fox"]}));
        let mut builtins = BuiltinGenerator::with_seed(17);

        let value = builtins
            .generate_value(&generator, &JsonObject::new(), "test")
            .unwrap();
        let value = value.as_str().unwrap();

        assert!(
            value.starts_with("redfox"),
            "username should keep the configured a+b prefix"
        );
        let suffix = &value["redfox".len()..];
        assert!(
            suffix.parse::<u8>().is_ok(),
            "username should append a numeric suffix, got `{suffix}`"
        );
    }

    #[test]
    fn uuid_has_v4_shape() {
        let generator = make_generator("uuid", json!({}));
        let mut builtins = BuiltinGenerator::with_seed(19);

        let value = builtins
            .generate_value(&generator, &JsonObject::new(), "test")
            .unwrap();
        let value = value.as_str().unwrap();

        assert_eq!(value.len(), 36);
        assert_eq!(&value[14..15], "4");
        assert!(matches!(&value[19..20], "8" | "9" | "a" | "b"));
    }

    #[test]
    fn session_id_has_stable_prefix() {
        let generator = make_generator("session_id", json!({}));
        let mut builtins = BuiltinGenerator::with_seed(23);

        let value = builtins
            .generate_value(&generator, &JsonObject::new(), "test")
            .unwrap();

        assert!(value.as_str().unwrap().starts_with("sess-"));
    }

    #[test]
    fn sequence_increments_with_prefix_and_padding() {
        let generator = make_generator(
            "sequence",
            json!({"prefix":"host-","start":1,"step":1,"pad":2}),
        );
        let mut builtins = BuiltinGenerator::with_seed(29);

        let first = builtins
            .generate_value(&generator, &JsonObject::new(), "test")
            .unwrap();
        let second = builtins
            .generate_value(&generator, &JsonObject::new(), "test")
            .unwrap();

        assert_eq!(first, json!("host-01"));
        assert_eq!(second, json!("host-02"));
    }
}
