use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    path::Path,
};

use serde::Serialize;

use crate::{error::OutputError, runtime::event::EventRecord};

/// CSV row containing only the emitted log payload and attack label.
///
/// Runtime routing metadata (`scenario`, `system`, `instance`, `machine`,
/// `state`, `template`) is intentionally omitted. Cramming every internal
/// breadcrumb into this format would turn a compact CSV into a spreadsheet
/// goblin with delusions of being a data warehouse.
#[derive(Debug, Serialize)]
struct CsvEventRow {
    timestamp: String,
    log: String,
    is_attack: bool,
    attack_type: Option<String>,
    technique: Option<String>,
    tactic: Option<String>,
    name: Option<String>,
    attack_state: Option<String>,
    attack_machine: Option<String>,
}

impl CsvEventRow {
    fn from_event(event: &EventRecord, location: impl Into<String>) -> Result<Self, OutputError> {
        let location = location.into();
        let log = serde_json::to_string(&event.fields)
            .map_err(|source| OutputError::serialize(location, source))?;

        Ok(Self {
            timestamp: event.timestamp.to_rfc3339(),
            log,
            is_attack: event.label.is_attack,
            attack_type: event.label.attack_type.clone(),
            technique: event.label.technique.clone(),
            tactic: event.label.tactic.clone(),
            name: event.label.name.clone(),
            attack_state: event.label.attack_state.clone(),
            attack_machine: event.label.attack_machine.clone(),
        })
    }
}

pub struct CsvWriter<W>
where
    W: Write,
{
    writer: csv::Writer<W>,
}

impl<W> CsvWriter<W>
where
    W: Write,
{
    pub fn new(writer: W) -> Self {
        Self {
            writer: csv::Writer::from_writer(writer),
        }
    }

    pub fn write_event(
        &mut self,
        event: &EventRecord,
        location: impl Into<String>,
    ) -> Result<(), OutputError> {
        let location = location.into();
        let row = CsvEventRow::from_event(event, location.clone())?;
        self.writer
            .serialize(row)
            .map_err(|source| OutputError::csv(location, source))
    }

    pub fn flush(&mut self) -> Result<(), OutputError> {
        self.writer
            .flush()
            .map_err(|source| OutputError::write("<csv stream>", source))
    }
}

/// Write final event records to a compact `.csv` file.
///
/// Columns are: `timestamp`, `log` (rendered fields as JSON text), and attack
/// label columns. Internal runtime identity fields are not written.
pub fn write_events_csv<'a, P, I>(path: P, events: I) -> Result<(), OutputError>
where
    P: AsRef<Path>,
    I: IntoIterator<Item = &'a EventRecord>,
{
    let path = path.as_ref();
    create_parent_dir(path)?;

    let file = File::create(path)
        .map_err(|source| OutputError::write(path.display().to_string(), source))?;
    let mut writer = CsvWriter::new(BufWriter::new(file));

    for (index, event) in events.into_iter().enumerate() {
        writer.write_event(event, format!("event[{index}]"))?;
    }

    writer.flush()
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
    use serde_json::json;

    use super::*;
    use crate::{
        ir::JsonObject,
        label::attack::ResolvedAttackLabel,
        runtime::event::{EventMetadata, assemble_event},
    };

    static NEXT_TEMP_ID: AtomicUsize = AtomicUsize::new(0);

    fn temp_path(name: &str) -> PathBuf {
        let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("rewrite-csv-{}-{}-{name}", std::process::id(), id))
    }

    fn event(is_attack: bool) -> EventRecord {
        let timestamp = Utc.with_ymd_and_hms(2026, 5, 17, 0, 0, 0).single().unwrap();
        let fields = JsonObject::from([
            ("message".to_owned(), json!("login failed")),
            ("user".to_owned(), json!("root")),
        ]);
        let metadata = EventMetadata::new(
            "scenario",
            "linux_auth",
            "session-0001",
            "ssh_login_flow",
            "bruteforce",
            "ssh_login_failed",
        );
        let label = if is_attack {
            ResolvedAttackLabel {
                is_attack: true,
                attack_type: Some("credential_access".to_owned()),
                technique: Some("T1110".to_owned()),
                tactic: Some("Credential Access".to_owned()),
                name: Some("Brute Force".to_owned()),
                attack_state: Some("bruteforce".to_owned()),
                attack_machine: Some("ssh_login_flow".to_owned()),
            }
        } else {
            ResolvedAttackLabel::benign()
        };

        assemble_event(fields, label, metadata, timestamp)
    }

    #[test]
    fn writes_log_and_attack_label_columns_only() {
        let path = temp_path("events.csv");
        let events = vec![event(false), event(true)];

        write_events_csv(&path, events.iter()).expect("csv write should succeed");

        let content = fs::read_to_string(&path).expect("csv should be readable");
        let _ = fs::remove_file(path);

        let mut reader = csv::Reader::from_reader(content.as_bytes());
        let headers = reader.headers().unwrap().clone();
        assert_eq!(
            headers.iter().collect::<Vec<_>>(),
            vec![
                "timestamp",
                "log",
                "is_attack",
                "attack_type",
                "technique",
                "tactic",
                "name",
                "attack_state",
                "attack_machine",
            ]
        );
        assert!(!headers.iter().any(|header| header == "scenario"));

        let rows = reader.records().collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(&rows[0][2], "false");
        assert_eq!(&rows[1][2], "true");
        assert_eq!(&rows[1][4], "T1110");
        let log: serde_json::Value = serde_json::from_str(&rows[0][1]).unwrap();
        assert_eq!(log["message"], json!("login failed"));
    }

    #[test]
    fn creates_parent_directories_for_output_file() {
        let dir = temp_path("nested");
        let path = dir.join("a").join("b").join("events.csv");
        let events = vec![event(false)];

        write_events_csv(&path, events.iter()).expect("csv write should create parents");

        assert!(path.exists());
        let _ = fs::remove_dir_all(dir);
    }
}
