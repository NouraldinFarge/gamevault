use crate::path_safety;
use chrono::Utc;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

const MAX_LOG_BYTES: u64 = 1024 * 1024;

pub fn record(root: &Path, enabled: bool, event: &str, outcome: &str) -> Result<(), String> {
    if !enabled {
        return Ok(());
    }
    let logs = path_safety::ensure_managed_directory(root, &["logs"])?;
    let current = logs.join("gamevault.log");
    let previous = logs.join("gamevault.previous.log");
    if fs::metadata(&current)
        .map(|metadata| metadata.len() >= MAX_LOG_BYTES)
        .unwrap_or(false)
    {
        if previous.is_file() {
            fs::remove_file(&previous).map_err(|error| error.to_string())?;
        }
        fs::rename(&current, &previous).map_err(|error| error.to_string())?;
    }
    let event = safe_field(event);
    let outcome = safe_field(outcome);
    let mut log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&current)
        .map_err(|error| error.to_string())?;
    writeln!(
        log,
        "{} event={} outcome={}",
        Utc::now().to_rfc3339(),
        event,
        outcome
    )
    .map_err(|error| error.to_string())
}

fn safe_field(value: &str) -> String {
    value
        .chars()
        .take(64)
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-') {
                character
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_log_uses_bounded_redacted_fields() {
        let directory = tempfile::tempdir().expect("temp directory");
        record(directory.path(), true, "archive ../raw path", "ok\nspoofed")
            .expect("diagnostic record");
        let log =
            fs::read_to_string(directory.path().join("logs").join("gamevault.log")).expect("log");
        assert!(log.contains("event=archive_.._raw_path"));
        assert!(log.contains("outcome=ok_spoofed"));
        assert_eq!(log.lines().count(), 1);
    }

    #[test]
    fn disabled_logging_writes_nothing() {
        let directory = tempfile::tempdir().expect("temp directory");
        record(directory.path(), false, "application.started", "ok").expect("disabled log");
        assert!(!directory.path().join("logs").exists());
    }
}
