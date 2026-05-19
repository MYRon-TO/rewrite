use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    path::Path,
};

use serde::Serialize;

use crate::{error::OutputError, runtime::event::EventRecord};

/// Streaming JSON Lines writer.
///
/// The writer serializes one record and immediately appends `\n`; it never
/// builds one giant JSON array in memory. Doing that for logs would be peak
/// raccoon engineering: shiny at first, then your process eats the whole bin.
pub struct JsonlWriter<W>
where
    W: Write,
{
    writer: W,
}

impl<W> JsonlWriter<W>
where
    W: Write,
{
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    /// Write one serializable record as exactly one JSONL line.
    pub fn write_record<T>(
        &mut self,
        record: &T,
        location: impl Into<String>,
    ) -> Result<(), OutputError>
    where
        T: Serialize + ?Sized,
    {
        let location = location.into();
        serde_json::to_writer(&mut self.writer, record)
            .map_err(|source| OutputError::serialize(location, source))?;
        self.writer
            .write_all(b"\n")
            .map_err(|source| OutputError::write("<jsonl stream>", source))?;
        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), OutputError> {
        self.writer
            .flush()
            .map_err(|source| OutputError::write("<jsonl stream>", source))
    }

    pub fn into_inner(self) -> W {
        self.writer
    }
}

/// Write final event records to a `.jsonl` file.
///
/// Parent directories are created automatically. Records are streamed through a
/// `BufWriter`, so large outputs do not need to sit in memory as a single blob.
pub fn write_events_jsonl<'a, P, I>(path: P, events: I) -> Result<(), OutputError>
where
    P: AsRef<Path>,
    I: IntoIterator<Item = &'a EventRecord>,
{
    write_jsonl_file(path, events)
}

/// Generic JSONL file writer used by tests and future output records.
pub fn write_jsonl_file<P, I, T>(path: P, records: I) -> Result<(), OutputError>
where
    P: AsRef<Path>,
    I: IntoIterator<Item = T>,
    T: Serialize,
{
    let path = path.as_ref();
    create_parent_dir(path)?;

    let file = File::create(path)
        .map_err(|source| OutputError::write(path.display().to_string(), source))?;
    let mut writer = JsonlWriter::new(BufWriter::new(file));

    for (index, record) in records.into_iter().enumerate() {
        writer.write_record(&record, format!("event[{index}]"))?;
    }

    writer
        .writer
        .flush()
        .map_err(|source| OutputError::write(path.display().to_string(), source))
}

fn create_parent_dir(path: &Path) -> Result<(), OutputError> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .map_err(|source| OutputError::create_dir(parent.display().to_string(), source))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicUsize, Ordering},
    };

    use chrono::{TimeZone, Utc};
    use serde::{Serialize, Serializer, ser::Error as _};
    use serde_json::{Value, json};

    use super::*;
    use crate::{
        ir::JsonObject, label::attack::ResolvedAttackLabel, runtime::event::EventMetadata,
    };

    static NEXT_TEMP_ID: AtomicUsize = AtomicUsize::new(0);

    fn temp_path(name: &str) -> PathBuf {
        let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "rewrite-jsonl-{}-{}-{name}",
            std::process::id(),
            id
        ))
    }

    fn event(index: usize) -> EventRecord {
        let timestamp = Utc
            .with_ymd_and_hms(2026, 5, 17, 0, 0, index as u32)
            .single()
            .unwrap();
        let fields = JsonObject::from([("message".to_owned(), json!(format!("event-{index}")))]);
        let metadata = EventMetadata::new(
            "scenario",
            "linux_auth",
            format!("session-{index:04}"),
            "ssh_login_flow",
            "start",
            "ssh_login_success",
        );

        crate::runtime::event::assemble_event(
            fields,
            ResolvedAttackLabel::benign(),
            metadata,
            timestamp,
        )
    }

    #[test]
    fn writes_one_valid_json_object_per_line() {
        let path = temp_path("events.jsonl");
        let events = vec![event(0), event(1), event(2)];

        write_events_jsonl(&path, events.iter()).expect("jsonl write should succeed");

        let content = fs::read_to_string(&path).expect("jsonl should be readable");
        let _ = fs::remove_file(&path);
        let lines = content.lines().collect::<Vec<_>>();

        assert_eq!(lines.len(), 3);
        for (index, line) in lines.iter().enumerate() {
            let value: Value = serde_json::from_str(line).expect("each line must be valid JSON");
            assert_eq!(value["scenario"], json!("scenario"));
            assert_eq!(value["fields"]["message"], json!(format!("event-{index}")));
            assert_eq!(value["label"]["is_attack"], json!(false));
        }
        assert!(content.ends_with('\n'));
    }

    #[test]
    fn creates_parent_directories_for_output_file() {
        let dir = temp_path("nested");
        let path = dir.join("a").join("b").join("events.jsonl");
        let events = vec![event(0)];

        write_events_jsonl(&path, events.iter()).expect("jsonl write should create parents");

        assert!(path.exists());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn supports_streaming_writer_without_file_path() {
        let mut writer = JsonlWriter::new(Vec::new());

        writer.write_record(&event(0), "event[0]").unwrap();
        writer.write_record(&event(1), "event[1]").unwrap();
        writer.flush().unwrap();

        let bytes = writer.into_inner();
        let content = String::from_utf8(bytes).unwrap();
        assert_eq!(content.lines().count(), 2);
    }

    struct BrokenRecord;

    impl Serialize for BrokenRecord {
        fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            Err(S::Error::custom("intentional serialization failure"))
        }
    }

    #[test]
    fn reports_serialization_location() {
        let mut writer = JsonlWriter::new(Vec::new());

        let err = writer
            .write_record(&BrokenRecord, "event[99]")
            .expect_err("broken record should fail");

        assert!(matches!(err, OutputError::Serialize { location, .. } if location == "event[99]"));
    }
}
