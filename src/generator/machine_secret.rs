use base64::{
    Engine as _,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};
use rand::{RngExt, rngs::StdRng};
use serde_json::{Map, Value};

use crate::{
    error::RuntimeError,
    ir::{Generator, JsonObject},
};

const BASE64URL: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
const ALPHANUMERIC: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
const UPPER_ALPHANUMERIC: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
const HEX: &[u8] = b"0123456789abcdef";

pub(super) fn generate(
    rng: &mut StdRng,
    generator: &Generator,
    overrides: &JsonObject,
    location: &str,
) -> Result<Value, RuntimeError> {
    let mut params = generator.params.clone();
    params.extend(overrides.clone());
    let profile = string_required(&params, "profile", location)?;
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
        other => {
            return Err(invalid(
                location,
                format!("unsupported machine_secret profile `{other}`"),
            ));
        }
    };
    reject_unknown(&params, allowed, location)?;

    let value = match profile {
        "hex" => Value::String(random_chars(
            rng,
            HEX,
            length(&params, "byte_length", None, 4096, location)? * 2,
        )),
        "base64" => Value::String(STANDARD.encode(random_bytes(
            rng,
            length(&params, "byte_length", None, 4096, location)?,
        ))),
        "base64url" => Value::String(URL_SAFE_NO_PAD.encode(random_bytes(
            rng,
            length(&params, "byte_length", None, 4096, location)?,
        ))),
        "uuid" => Value::String(uuid_v4(rng)),
        "api_key" => prefixed(rng, &params, "sim_sk_", 32, location)?,
        "bearer_token" => prefixed(rng, &params, "sim_bearer_", 40, location)?,
        "private_token" => prefixed(rng, &params, "sim_tok_", 32, location)?,
        "jwt_shape" => jwt(rng, &params, location)?,
        "pem_shape" => pem(rng, &params, location)?,
        "cloud_access_key" => {
            let prefix = string_default(&params, "prefix", "SIMAKIA", location)?;
            let n = length(&params, "body_length", Some(16), 4096, location)?;
            Value::String(format!(
                "{prefix}{}",
                random_chars(rng, UPPER_ALPHANUMERIC, n)
            ))
        }
        "cloud_secret_pair" => {
            let prefix = string_default(&params, "id_prefix", "SIMAKIA", location)?;
            let id_len = length(&params, "id_body_length", Some(16), 4096, location)?;
            let secret_len = length(&params, "secret_length", Some(40), 4096, location)?;
            let mut object = Map::new();
            object.insert(
                "access_key_id".into(),
                Value::String(format!(
                    "{prefix}{}",
                    random_chars(rng, UPPER_ALPHANUMERIC, id_len)
                )),
            );
            object.insert(
                "secret_access_key".into(),
                Value::String(random_chars(rng, BASE64URL, secret_len)),
            );
            Value::Object(object)
        }
        "connection_string" => connection_string(rng, &params, location)?,
        _ => unreachable!(),
    };
    Ok(value)
}

fn prefixed(
    rng: &mut StdRng,
    p: &JsonObject,
    default_prefix: &str,
    default_len: usize,
    loc: &str,
) -> Result<Value, RuntimeError> {
    let prefix = string_default(p, "prefix", default_prefix, loc)?;
    let len = length(p, "body_length", Some(default_len), 4096, loc)?;
    let alphabet = match string_default(p, "encoding", "base64url", loc)? {
        "base64url" => BASE64URL,
        "hex" => HEX,
        "alphanumeric" => ALPHANUMERIC,
        other => {
            return Err(invalid(
                loc,
                format!("unsupported machine_secret encoding `{other}`"),
            ));
        }
    };
    Ok(Value::String(format!(
        "{prefix}{}",
        random_chars(rng, alphabet, len)
    )))
}

fn jwt(rng: &mut StdRng, p: &JsonObject, loc: &str) -> Result<Value, RuntimeError> {
    let issuer = string_default(p, "issuer", "simulator", loc)?;
    let subject = string_default(p, "subject", "simulated-user", loc)?;
    let sig_len = length(p, "signature_length", Some(32), 4096, loc)?;
    let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"none","typ":"JWT"}"#);
    let payload = URL_SAFE_NO_PAD.encode(
        serde_json::to_vec(&serde_json::json!({"iss": issuer, "sub": subject}))
            .expect("JSON serialization cannot fail"),
    );
    Ok(Value::String(format!(
        "{header}.{payload}.{}",
        random_chars(rng, BASE64URL, sig_len)
    )))
}

fn pem(rng: &mut StdRng, p: &JsonObject, loc: &str) -> Result<Value, RuntimeError> {
    let label = string_default(p, "label", "SIMULATED PRIVATE KEY", loc)?;
    let byte_len = length(p, "byte_length", Some(64), 4096, loc)?;
    let width = length(p, "line_width", Some(64), 256, loc)?;
    let body = STANDARD.encode(random_bytes(rng, byte_len));
    let lines = body
        .as_bytes()
        .chunks(width)
        .map(|c| std::str::from_utf8(c).expect("base64 is UTF-8"))
        .collect::<Vec<_>>()
        .join("\n");
    Ok(Value::String(format!(
        "-----BEGIN {label}-----\n{lines}\n-----END {label}-----"
    )))
}

fn connection_string(rng: &mut StdRng, p: &JsonObject, loc: &str) -> Result<Value, RuntimeError> {
    let scheme = nonempty(p, "scheme", "postgresql", loc)?;
    let host = nonempty(p, "host", "db.invalid", loc)?;
    let database = nonempty(p, "database", "simdb", loc)?;
    let username = nonempty(p, "username", "sim_user", loc)?;
    let port = length(p, "port", Some(5432), 65535, loc)?;
    let password_len = length(p, "password_length", Some(32), 4096, loc)?;
    Ok(Value::String(format!(
        "{scheme}://{username}:{}@{host}:{port}/{database}",
        random_chars(rng, BASE64URL, password_len)
    )))
}

fn random_bytes(rng: &mut StdRng, len: usize) -> Vec<u8> {
    (0..len).map(|_| rng.random()).collect()
}
fn random_chars(rng: &mut StdRng, alphabet: &[u8], len: usize) -> String {
    (0..len)
        .map(|_| alphabet[rng.random_range(0..alphabet.len())] as char)
        .collect()
}
fn uuid_v4(rng: &mut StdRng) -> String {
    let mut b: [u8; 16] = rng.random();
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        b[0],
        b[1],
        b[2],
        b[3],
        b[4],
        b[5],
        b[6],
        b[7],
        b[8],
        b[9],
        b[10],
        b[11],
        b[12],
        b[13],
        b[14],
        b[15]
    )
}
fn string_required<'a>(p: &'a JsonObject, key: &str, loc: &str) -> Result<&'a str, RuntimeError> {
    p.get(key).and_then(Value::as_str).ok_or_else(|| {
        invalid(
            loc,
            format!("machine_secret.{key} is required and must be a string"),
        )
    })
}
fn string_default<'a>(
    p: &'a JsonObject,
    key: &str,
    default: &'a str,
    loc: &str,
) -> Result<&'a str, RuntimeError> {
    match p.get(key) {
        None => Ok(default),
        Some(v) => v
            .as_str()
            .ok_or_else(|| invalid(loc, format!("machine_secret.{key} must be a string"))),
    }
}
fn nonempty<'a>(
    p: &'a JsonObject,
    key: &str,
    default: &'a str,
    loc: &str,
) -> Result<&'a str, RuntimeError> {
    let value = string_default(p, key, default, loc)?;
    if value.is_empty() {
        Err(invalid(
            loc,
            format!("machine_secret.{key} must not be empty"),
        ))
    } else {
        Ok(value)
    }
}
fn length(
    p: &JsonObject,
    key: &str,
    default: Option<usize>,
    max: usize,
    loc: &str,
) -> Result<usize, RuntimeError> {
    let value = match p.get(key) {
        Some(v) => v
            .as_u64()
            .and_then(|n| usize::try_from(n).ok())
            .ok_or_else(|| invalid(loc, format!("machine_secret.{key} must be an integer")))?,
        None => {
            default.ok_or_else(|| invalid(loc, format!("missing machine_secret param `{key}`")))?
        }
    };
    if !(1..=max).contains(&value) {
        return Err(invalid(
            loc,
            format!("machine_secret.{key} must be in 1..={max}"),
        ));
    }
    Ok(value)
}
fn reject_unknown(p: &JsonObject, allowed: &[&str], loc: &str) -> Result<(), RuntimeError> {
    if let Some(key) = p.keys().find(|key| !allowed.contains(&key.as_str())) {
        return Err(invalid(
            loc,
            format!("unknown machine_secret parameter `{key}`"),
        ));
    }
    Ok(())
}
fn invalid(location: &str, message: impl Into<String>) -> RuntimeError {
    RuntimeError::invalid_context_value(location.to_string(), message.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use serde_json::json;

    fn generator(params: Value) -> Generator {
        Generator {
            id: "secret".into(),
            generator_type: "machine_secret".into(),
            params: params.as_object().unwrap().clone().into_iter().collect(),
            description: None,
        }
    }
    fn run(seed: u64, params: Value) -> Result<Value, RuntimeError> {
        generate(
            &mut StdRng::seed_from_u64(seed),
            &generator(params),
            &JsonObject::new(),
            "test",
        )
    }

    #[test]
    fn deterministic_and_encoded_lengths() {
        assert_eq!(
            run(7, json!({"profile":"hex","byte_length":12})).unwrap(),
            run(7, json!({"profile":"hex","byte_length":12})).unwrap()
        );
        assert_eq!(
            run(1, json!({"profile":"hex","byte_length":12}))
                .unwrap()
                .as_str()
                .unwrap()
                .len(),
            24
        );
        assert_eq!(
            run(1, json!({"profile":"base64","byte_length":12}))
                .unwrap()
                .as_str()
                .unwrap()
                .len(),
            16
        );
        assert_eq!(
            run(1, json!({"profile":"base64url","byte_length":13}))
                .unwrap()
                .as_str()
                .unwrap()
                .len(),
            18
        );
    }

    #[test]
    fn uuid_has_v4_bits() {
        let v = run(2, json!({"profile":"uuid"})).unwrap();
        let s = v.as_str().unwrap();
        assert_eq!(s.len(), 36);
        assert_eq!(&s[14..15], "4");
        assert!(matches!(&s[19..20], "8" | "9" | "a" | "b"));
    }

    #[test]
    fn prefixes_encodings_and_cloud_shapes() {
        for profile in ["api_key", "bearer_token", "private_token"] {
            assert!(
                run(
                    3,
                    json!({"profile":profile,"prefix":"x_","body_length":10,"encoding":"hex"})
                )
                .unwrap()
                .as_str()
                .unwrap()
                .starts_with("x_")
            );
        }
        let access = run(3, json!({"profile":"cloud_access_key"})).unwrap();
        assert_eq!(access.as_str().unwrap().len(), 23);
        let pair = run(3, json!({"profile":"cloud_secret_pair"})).unwrap();
        assert_eq!(pair["access_key_id"].as_str().unwrap().len(), 23);
        assert_eq!(pair["secret_access_key"].as_str().unwrap().len(), 40);
    }

    #[test]
    fn jwt_segments_decode_and_signature_length() {
        let value = run(
            4,
            json!({"profile":"jwt_shape","issuer":"i","subject":"s","signature_length":17}),
        )
        .unwrap();
        let parts: Vec<_> = value.as_str().unwrap().split('.').collect();
        assert_eq!(parts.len(), 3);
        assert!(URL_SAFE_NO_PAD.decode(parts[0]).is_ok());
        let payload = URL_SAFE_NO_PAD.decode(parts[1]).unwrap();
        assert_eq!(
            serde_json::from_slice::<Value>(&payload).unwrap(),
            json!({"iss":"i","sub":"s"})
        );
        assert_eq!(parts[2].len(), 17);
    }

    #[test]
    fn pem_has_matching_boundaries_wrapping_and_base64_body() {
        let value = run(
            5,
            json!({"profile":"pem_shape","label":"TEST KEY","byte_length":20,"line_width":8}),
        )
        .unwrap();
        let lines: Vec<_> = value.as_str().unwrap().lines().collect();
        assert_eq!(lines.first(), Some(&"-----BEGIN TEST KEY-----"));
        assert_eq!(lines.last(), Some(&"-----END TEST KEY-----"));
        assert!(lines[1..lines.len() - 1].iter().all(|line| line.len() <= 8));
        assert_eq!(
            STANDARD
                .decode(lines[1..lines.len() - 1].join(""))
                .unwrap()
                .len(),
            20
        );
    }

    #[test]
    fn connection_and_invalid_params() {
        let value = run(
            6,
            json!({"profile":"connection_string","password_length":9}),
        )
        .unwrap();
        assert!(
            value
                .as_str()
                .unwrap()
                .starts_with("postgresql://sim_user:")
        );
        assert!(run(1, json!({"profile":"hex","byte_length":0})).is_err());
        assert!(run(1, json!({"profile":"uuid","extra":true})).is_err());
        assert!(run(1, json!({"profile":"api_key","encoding":"bad"})).is_err());
        assert!(run(1, json!({"profile":"connection_string","host":""})).is_err());
    }

    #[test]
    fn usage_params_override_definition_params() {
        let g = generator(json!({"profile":"hex","byte_length":1}));
        let mut overrides = JsonObject::new();
        overrides.insert("byte_length".into(), json!(3));
        assert_eq!(
            generate(&mut StdRng::seed_from_u64(1), &g, &overrides, "test")
                .unwrap()
                .as_str()
                .unwrap()
                .len(),
            6
        );
    }
}
