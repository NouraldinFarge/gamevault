use crate::models::OperationRecord;
use chrono::Utc;
use rusqlite::{params, Connection};
use uuid::Uuid;

const HISTORY_LIMIT: i64 = 100;

pub fn begin(
    connection: &Connection,
    kind: &str,
    label: &str,
    source_path: Option<&str>,
    target_path: Option<&str>,
    recovery_hint: &str,
) -> Result<String, String> {
    let id = Uuid::now_v7().to_string();
    let now = Utc::now().to_rfc3339();
    connection
        .execute(
            "INSERT INTO operations(
               id, kind, label, status, source_path, target_path, summary,
               error_message, recovery_hint, report_path, started_at, updated_at, completed_at
             ) VALUES(?1, ?2, ?3, 'running', ?4, ?5, 'Operation started.', NULL, ?6, NULL, ?7, ?7, NULL)",
            params![id, kind, label, source_path, target_path, recovery_hint, now],
        )
        .map_err(|error| error.to_string())?;
    Ok(id)
}

pub fn complete(
    connection: &Connection,
    id: &str,
    summary: &str,
    target_path: Option<&str>,
    report_path: Option<&str>,
) -> Result<(), String> {
    let now = Utc::now().to_rfc3339();
    connection
        .execute(
            "UPDATE operations SET
               status = 'completed', summary = ?2, target_path = COALESCE(?3, target_path),
               report_path = COALESCE(?4, report_path), error_message = NULL,
               updated_at = ?5, completed_at = ?5
             WHERE id = ?1 AND status = 'running'",
            params![id, summary, target_path, report_path, now],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

pub fn fail(connection: &Connection, id: &str, message: &str) -> Result<(), String> {
    let now = Utc::now().to_rfc3339();
    connection
        .execute(
            "UPDATE operations SET
               status = 'failed', summary = 'Operation did not complete.', error_message = ?2,
               updated_at = ?3, completed_at = ?3
             WHERE id = ?1 AND status = 'running'",
            params![id, message, now],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

pub fn reconcile_interrupted(connection: &Connection) -> Result<usize, String> {
    let now = Utc::now().to_rfc3339();
    connection
        .execute(
            "UPDATE operations SET
               status = 'interrupted',
               summary = 'GameVault closed before this operation reported completion.',
               error_message = 'The prior process ended while this operation was running.',
               updated_at = ?1, completed_at = ?1
             WHERE status = 'running'",
            [now],
        )
        .map_err(|error| error.to_string())
}

pub fn list(connection: &Connection) -> Result<Vec<OperationRecord>, String> {
    let mut statement = connection
        .prepare(
            "SELECT id, kind, label, status, source_path, target_path, summary,
                    error_message, recovery_hint, report_path, started_at, updated_at, completed_at
             FROM operations
             ORDER BY started_at DESC
             LIMIT ?1",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([HISTORY_LIMIT], |row| {
            Ok(OperationRecord {
                id: row.get(0)?,
                kind: row.get(1)?,
                label: row.get(2)?,
                status: row.get(3)?,
                source_path: row.get(4)?,
                target_path: row.get(5)?,
                summary: row.get(6)?,
                error_message: row.get(7)?,
                recovery_hint: row.get(8)?,
                report_path: row.get(9)?,
                started_at: row.get(10)?,
                updated_at: row.get(11)?,
                completed_at: row.get(12)?,
            })
        })
        .map_err(|error| error.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage;

    #[test]
    fn running_operations_are_reconciled_after_restart() {
        let directory = tempfile::tempdir().expect("temp directory");
        let connection =
            storage::open_database(&directory.path().join("library.db")).expect("database");
        let id = begin(
            &connection,
            "archive-stage",
            "Stage archive",
            Some("Owned.zip"),
            None,
            "Review Staging before retrying.",
        )
        .expect("operation");

        assert_eq!(reconcile_interrupted(&connection).expect("reconcile"), 1);
        let history = list(&connection).expect("history");
        assert_eq!(history[0].id, id);
        assert_eq!(history[0].status, "interrupted");
        assert!(history[0].completed_at.is_some());
    }

    #[test]
    fn completed_operation_keeps_recovery_evidence() {
        let directory = tempfile::tempdir().expect("temp directory");
        let connection =
            storage::open_database(&directory.path().join("library.db")).expect("database");
        let id = begin(
            &connection,
            "dependency-audit",
            "Audit prerequisites",
            None,
            None,
            "Run the audit again if files changed.",
        )
        .expect("operation");
        complete(
            &connection,
            &id,
            "Reviewed 3 installers.",
            None,
            Some("report.json"),
        )
        .expect("complete");

        let history = list(&connection).expect("history");
        assert_eq!(history[0].status, "completed");
        assert_eq!(history[0].report_path.as_deref(), Some("report.json"));
    }
}
