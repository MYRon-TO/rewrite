use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use rand::{RngExt, rngs::StdRng, seq::SliceRandom};
use serde_json::Value;

use crate::{
    error::RuntimeError,
    ir::{Generator, JsonObject},
};

use super::{
    corpus_cache::{CorpusCache, CorpusSource},
    optional_param,
};

#[derive(Debug, Default)]
pub(crate) struct SamplingStates {
    states: BTreeMap<String, ShuffleState>,
}

#[derive(Debug, Default)]
struct ShuffleState {
    order: Vec<usize>,
    next: usize,
}

struct Config {
    source: CorpusSource,
    min: usize,
    max: usize,
    sampling: Sampling,
    deduplicate: bool,
    keep_empty: bool,
}

#[derive(Clone, Copy)]
enum Sampling {
    Uniform,
    ShuffleCycle,
}

pub(crate) fn validate_and_load(
    cache: &mut CorpusCache,
    asset_root: &Path,
    generator: &Generator,
    params: &JsonObject,
    location: &str,
) -> Result<(), String> {
    let config = Config::parse(generator, params, asset_root)?;
    filtered(cache.lines(&config.source, location)?, &config)?;
    Ok(())
}

pub(crate) fn generate(
    rng: &mut StdRng,
    cache: &mut CorpusCache,
    states: &mut SamplingStates,
    asset_root: &Path,
    generator: &Generator,
    params: &JsonObject,
    location: &str,
) -> Result<Value, RuntimeError> {
    let config = Config::parse(generator, params, asset_root)
        .map_err(|message| invalid(location, message))?;
    let values = filtered(
        cache
            .lines(&config.source, location)
            .map_err(|message| invalid(location, message))?,
        &config,
    )
    .map_err(|message| invalid(location, message))?;

    let index = match config.sampling {
        Sampling::Uniform => rng.random_range(0..values.len()),
        Sampling::ShuffleCycle => {
            let key = config.key();
            let state = states.states.entry(key).or_default();
            if state.next >= state.order.len() || state.order.len() != values.len() {
                state.order = (0..values.len()).collect();
                state.order.shuffle(rng);
                state.next = 0;
            }
            let index = state.order[state.next];
            state.next += 1;
            index
        }
    };
    Ok(Value::String(values[index].clone()))
}

impl Config {
    fn parse(
        generator: &Generator,
        params: &JsonObject,
        asset_root: &Path,
    ) -> Result<Self, String> {
        let source = value(generator, params, "source")
            .ok_or_else(|| "missing generator param `source`".to_string())?
            .as_object()
            .ok_or_else(|| "corpus_choice.source must be an object".to_string())?;
        let path = source.get("path").and_then(Value::as_str);
        let builtin = source.get("builtin").and_then(Value::as_str);
        let source = CorpusSource::from_parts(path, builtin, asset_root)?;
        let min = integer(generator, params, "min_length", 1)?;
        let max = integer(generator, params, "max_length", 128)?;
        if !(0..=4096).contains(&min) || !(0..=4096).contains(&max) || max < min {
            return Err("requires 0 <= min_length <= max_length <= 4096".into());
        }
        let sampling = match string(generator, params, "sampling", "uniform")? {
            "uniform" => Sampling::Uniform,
            "shuffle_cycle" => Sampling::ShuffleCycle,
            _ => return Err("expected sampling `uniform` or `shuffle_cycle`".into()),
        };
        Ok(Self {
            source,
            min: min as usize,
            max: max as usize,
            sampling,
            deduplicate: boolean(generator, params, "deduplicate", true)?,
            keep_empty: boolean(generator, params, "keep_empty", false)?,
        })
    }

    fn key(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}|shuffle_cycle",
            self.source.key(),
            self.min,
            self.max,
            self.deduplicate,
            self.keep_empty
        )
    }
}

fn filtered(raw: Vec<String>, config: &Config) -> Result<Vec<String>, String> {
    let mut seen = BTreeSet::new();
    let values = raw
        .into_iter()
        .filter(|line| config.keep_empty || !line.is_empty())
        .filter(|line| (config.min..=config.max).contains(&line.chars().count()))
        .filter(|line| !config.deduplicate || seen.insert(line.clone()))
        .collect::<Vec<_>>();
    if values.is_empty() {
        Err("corpus is empty after filtering".into())
    } else {
        Ok(values)
    }
}

fn value<'a>(generator: &'a Generator, params: &'a JsonObject, key: &str) -> Option<&'a Value> {
    optional_param(generator, params, key)
}

fn integer(
    generator: &Generator,
    params: &JsonObject,
    key: &str,
    default: i64,
) -> Result<i64, String> {
    value(generator, params, key)
        .map(|value| {
            value
                .as_i64()
                .ok_or_else(|| format!("`{key}` must be an integer"))
        })
        .transpose()
        .map(|value| value.unwrap_or(default))
}

fn boolean(
    generator: &Generator,
    params: &JsonObject,
    key: &str,
    default: bool,
) -> Result<bool, String> {
    value(generator, params, key)
        .map(|value| {
            value
                .as_bool()
                .ok_or_else(|| format!("`{key}` must be a boolean"))
        })
        .transpose()
        .map(|value| value.unwrap_or(default))
}

fn string<'a>(
    generator: &'a Generator,
    params: &'a JsonObject,
    key: &str,
    default: &'a str,
) -> Result<&'a str, String> {
    value(generator, params, key)
        .map(|value| {
            value
                .as_str()
                .ok_or_else(|| format!("`{key}` must be a string"))
        })
        .transpose()
        .map(|value| value.unwrap_or(default))
}

fn invalid(location: &str, message: impl Into<String>) -> RuntimeError {
    RuntimeError::invalid_context_value(location.to_owned(), message.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use serde_json::json;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn generator(params: Value) -> Generator {
        Generator {
            id: "corpus".into(),
            generator_type: "corpus_choice".into(),
            params: params.as_object().unwrap().clone().into_iter().collect(),
            description: None,
        }
    }

    fn temp_corpus(bytes: &[u8]) -> (std::path::PathBuf, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "rewrite-corpus-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("corpus.txt"), bytes).unwrap();
        (dir.clone(), dir.join("corpus.txt"))
    }

    fn samples(
        seed: u64,
        root: &Path,
        generator: &Generator,
        count: usize,
    ) -> Result<Vec<String>, RuntimeError> {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut cache = CorpusCache::default();
        let mut states = SamplingStates::default();
        (0..count)
            .map(|_| {
                generate(
                    &mut rng,
                    &mut cache,
                    &mut states,
                    root,
                    generator,
                    &JsonObject::new(),
                    "test",
                )
                .map(|v| v.as_str().unwrap().to_owned())
            })
            .collect()
    }

    #[test]
    fn builtin_is_packaged_and_deterministic() {
        let g = generator(json!({"source":{"builtin":"human-passwords-v1"}}));
        assert_eq!(
            samples(42, Path::new("."), &g, 5).unwrap(),
            samples(42, Path::new("."), &g, 5).unwrap()
        );
    }

    #[test]
    fn external_processing_preserves_spaces_hash_and_empty_and_deduplicates() {
        let (dir, _) = temp_corpus(b" a \r\n#x\n\n a \n");
        let g = generator(
            json!({"source":{"path":"corpus.txt"},"min_length":0,"keep_empty":true,"sampling":"shuffle_cycle"}),
        );
        let mut got = samples(2, &dir, &g, 3).unwrap();
        got.sort();
        assert_eq!(got, vec!["", " a ", "#x"]);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn shuffle_cycle_has_no_repeat_and_state_isolated_by_configuration() {
        let (dir, _) = temp_corpus(b"a\nb\nc\n");
        let g = generator(json!({"source":{"path":"corpus.txt"},"sampling":"shuffle_cycle"}));
        let values = samples(7, &dir, &g, 3).unwrap();
        assert_eq!(values.iter().collect::<BTreeSet<_>>().len(), 3);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn rejects_invalid_utf8_and_empty_filter_result() {
        let (dir, path) = temp_corpus(&[0xff]);
        let g = generator(json!({"source":{"path":path}}));
        assert!(
            samples(1, &dir, &g, 1)
                .unwrap_err()
                .to_string()
                .contains("UTF-8")
        );
        fs::write(&path, "short\n").unwrap();
        let g = generator(json!({"source":{"path":path},"min_length":10}));
        assert!(
            samples(1, &dir, &g, 1)
                .unwrap_err()
                .to_string()
                .contains("empty after filtering")
        );
        fs::remove_dir_all(dir).unwrap();
    }
}
