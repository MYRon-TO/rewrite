use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

pub(crate) const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;
pub(crate) const MAX_LINES: usize = 1_000_000;
pub(crate) const MAX_LINE_CHARS: usize = 4096;

const BUILTIN_ID: &str = "human-passwords-v1";
const BUILTIN_CORPUS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/data/human_password.txt"
));

#[derive(Debug, Default)]
pub(crate) struct CorpusCache {
    raw: BTreeMap<String, Vec<String>>,
}

impl CorpusCache {
    pub(crate) fn lines(
        &mut self,
        source: &CorpusSource,
        location: &str,
    ) -> Result<Vec<String>, String> {
        let key = source.key();
        if let Some(lines) = self.raw.get(&key) {
            return Ok(lines.clone());
        }

        let lines = match source {
            CorpusSource::Builtin => parse_lines(BUILTIN_CORPUS)?,
            CorpusSource::Path(path) => {
                let metadata = fs::metadata(path).map_err(|error| {
                    format!("failed to inspect corpus `{}`: {error}", path.display())
                })?;
                if metadata.len() > MAX_FILE_BYTES {
                    return Err("corpus file exceeds 16 MiB".into());
                }
                let bytes = fs::read(path).map_err(|error| {
                    format!("failed to read corpus `{}`: {error}", path.display())
                })?;
                let text = std::str::from_utf8(&bytes)
                    .map_err(|_| "corpus must be valid UTF-8".to_string())?;
                parse_lines(text)?
            }
        };
        self.raw.insert(key, lines.clone());
        let _ = location;
        Ok(lines)
    }
}

#[derive(Debug, Clone)]
pub(crate) enum CorpusSource {
    Builtin,
    Path(PathBuf),
}

impl CorpusSource {
    pub(crate) fn from_parts(
        path: Option<&str>,
        builtin: Option<&str>,
        asset_root: &Path,
    ) -> Result<Self, String> {
        match (path, builtin) {
            (None, Some(BUILTIN_ID)) => Ok(Self::Builtin),
            (Some(path), None) => {
                let path = Path::new(path);
                let resolved = if path.is_absolute() {
                    path.to_owned()
                } else {
                    asset_root.join(path)
                };
                let normalized = fs::canonicalize(&resolved).unwrap_or(resolved);
                Ok(Self::Path(normalized))
            }
            (None, Some(id)) => Err(format!("unsupported builtin corpus `{id}`")),
            _ => Err("source must contain exactly one of `builtin` or `path`".into()),
        }
    }

    pub(crate) fn key(&self) -> String {
        match self {
            Self::Builtin => format!("builtin:{BUILTIN_ID}"),
            Self::Path(path) => format!("path:{}", path.display()),
        }
    }
}

fn parse_lines(text: &str) -> Result<Vec<String>, String> {
    let mut lines = Vec::new();
    for raw in text.split_inclusive('\n') {
        let without_lf = raw.strip_suffix('\n').unwrap_or(raw);
        let line = without_lf.strip_suffix('\r').unwrap_or(without_lf);
        if line.chars().count() > MAX_LINE_CHARS {
            return Err("corpus line exceeds 4096 Unicode characters".into());
        }
        lines.push(line.to_owned());
        if lines.len() > MAX_LINES {
            return Err("corpus exceeds 1,000,000 lines".into());
        }
    }
    Ok(lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_only_line_endings() {
        assert_eq!(parse_lines(" a \r\n#x\n\n").unwrap(), vec![" a ", "#x", ""]);
    }

    #[test]
    fn counts_unicode_characters_for_line_limit() {
        assert!(parse_lines(&"密".repeat(MAX_LINE_CHARS)).is_ok());
        assert!(parse_lines(&"密".repeat(MAX_LINE_CHARS + 1)).is_err());
    }
}
