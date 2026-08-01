use crate::models::{Game, GameMetadata, LibraryStats, Settings, UpdateGameInput};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Row};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS games (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  description TEXT NOT NULL DEFAULT '',
  install_path TEXT NOT NULL UNIQUE,
  executable_path TEXT NOT NULL,
  launch_args_json TEXT NOT NULL DEFAULT '[]',
  tags_json TEXT NOT NULL DEFAULT '[]',
  category TEXT NOT NULL DEFAULT 'Uncategorized',
  favorite INTEGER NOT NULL DEFAULT 0,
  detection_status TEXT NOT NULL,
  detection_source TEXT NOT NULL,
  folder_size_bytes INTEGER,
  last_played_at TEXT,
  playtime_seconds INTEGER NOT NULL DEFAULT 0,
  added_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  folder_modified_at INTEGER NOT NULL DEFAULT 0,
  content_signature TEXT NOT NULL DEFAULT '',
  artwork_seed INTEGER NOT NULL DEFAULT 0
  ,metadata_json TEXT NOT NULL DEFAULT '{}'
);
CREATE INDEX IF NOT EXISTS idx_games_status ON games(detection_status);
CREATE INDEX IF NOT EXISTS idx_games_title ON games(title);
CREATE INDEX IF NOT EXISTS idx_games_signature ON games(content_signature);

CREATE TABLE IF NOT EXISTS settings (
  key TEXT PRIMARY KEY,
  value_json TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS scan_history (
  id TEXT PRIMARY KEY,
  started_at TEXT NOT NULL,
  completed_at TEXT NOT NULL,
  folders_scanned INTEGER NOT NULL,
  games_detected INTEGER NOT NULL,
  unavailable_roots_json TEXT NOT NULL
);

PRAGMA user_version = 2;
"#;

#[derive(Debug, Clone)]
pub struct DetectedGame {
    pub title: String,
    pub install_path: String,
    pub executable_path: String,
    pub folder_size_bytes: Option<u64>,
    pub folder_modified_at: i64,
    pub content_signature: String,
}

pub struct ApplyScanResult {
    pub added: usize,
    pub updated: usize,
}

#[cfg(test)]
pub fn open_database(path: &Path) -> Result<Connection, String> {
    open_database_with_settings(path, &Settings::default())
}

pub fn open_portable_database(path: &Path, portable_root: &Path) -> Result<Connection, String> {
    open_database_with_settings(path, &Settings::for_portable_root(portable_root))
}

fn open_database_with_settings(
    path: &Path,
    initial_settings: &Settings,
) -> Result<Connection, String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    match open_and_initialize(path, initial_settings) {
        Ok(connection) => Ok(connection),
        Err(first_error) => {
            let stamp = Utc::now().format("%Y%m%d-%H%M%S");
            let corrupt_path = path.with_file_name(format!("library-corrupt-{stamp}.db"));
            if path.exists() {
                fs::rename(path, &corrupt_path).map_err(|error| {
                    format!("Database recovery failed after {first_error}: {error}")
                })?;
            }
            open_and_initialize(path, initial_settings)
        }
    }
}

fn open_and_initialize(path: &Path, initial_settings: &Settings) -> Result<Connection, String> {
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    connection
        .busy_timeout(std::time::Duration::from_secs(5))
        .map_err(|error| error.to_string())?;
    connection
        .execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA journal_mode = DELETE;
             PRAGMA synchronous = NORMAL;",
        )
        .map_err(|error| error.to_string())?;
    connection
        .execute_batch(SCHEMA)
        .map_err(|error| error.to_string())?;
    ensure_schema_migrations(&connection)?;

    let has_application_settings: i64 = connection
        .query_row(
            "SELECT count(*) FROM settings WHERE key = 'application'",
            [],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if has_application_settings == 0 {
        save_settings(&connection, initial_settings)?;
    } else {
        get_settings(&connection)?;
    }
    Ok(connection)
}

pub fn migrate_legacy_portability_defaults(
    connection: &Connection,
    portable_root: &Path,
) -> Result<(), String> {
    let mut settings = get_settings(connection)?;
    let managed_root = portable_root.join("library");
    let games_root = managed_root.join("Games");
    let managed_was_legacy = matches_legacy_default(&settings.managed_root);
    let mut migrated = false;

    if managed_was_legacy {
        settings.managed_root = managed_root.to_string_lossy().to_string();
        migrated = true;
    }

    for root in &mut settings.library_roots {
        if matches_legacy_library_root(root) || (managed_was_legacy && matches_legacy_default(root))
        {
            *root = games_root.to_string_lossy().to_string();
            migrated = true;
        }
    }

    if settings.library_roots.is_empty() {
        settings
            .library_roots
            .push(games_root.to_string_lossy().to_string());
        migrated = true;
    }

    if migrated {
        connection
            .execute(
                "UPDATE games SET detection_status = 'missing', updated_at = ?1
                 WHERE lower(install_path) LIKE lower('E:\\SteamRIPPED%')
                    OR lower(install_path) LIKE lower('E:\\GameVault%')",
                [Utc::now().to_rfc3339()],
            )
            .map_err(|error| error.to_string())?;
        save_settings(connection, &settings)?;
    }

    Ok(())
}

fn matches_legacy_default(value: &str) -> bool {
    value.eq_ignore_ascii_case(r"E:\SteamRIPPED") || value.eq_ignore_ascii_case(r"E:\GameVault")
}

fn matches_legacy_library_root(value: &str) -> bool {
    value.eq_ignore_ascii_case(r"E:\SteamRIPPED\Games")
        || value.eq_ignore_ascii_case(r"E:\GameVault\Games")
}

fn ensure_schema_migrations(connection: &Connection) -> Result<(), String> {
    let has_metadata: i64 = connection
        .query_row(
            "SELECT count(*) FROM pragma_table_info('games') WHERE name = 'metadata_json'",
            [],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if has_metadata == 0 {
        connection
            .execute(
                "ALTER TABLE games ADD COLUMN metadata_json TEXT NOT NULL DEFAULT '{}'",
                [],
            )
            .map_err(|error| error.to_string())?;
    }
    connection
        .pragma_update(None, "user_version", 2_i64)
        .map_err(|error| error.to_string())?;
    Ok(())
}

pub fn sqlite_version(connection: &Connection) -> String {
    connection
        .query_row("SELECT sqlite_version()", [], |row| row.get(0))
        .unwrap_or_else(|_| "unknown".to_string())
}

pub fn get_settings(connection: &Connection) -> Result<Settings, String> {
    let value: Option<String> = connection
        .query_row(
            "SELECT value_json FROM settings WHERE key = 'application'",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;

    match value {
        Some(json) => {
            let mut settings: Settings =
                serde_json::from_str(&json).map_err(|error| error.to_string())?;
            let mut migrated = false;
            if settings
                .managed_root
                .eq_ignore_ascii_case(r"E:\SteamRIPPED")
            {
                settings.managed_root = r"E:\GameVault".to_string();
                migrated = true;
            }
            for root in &mut settings.library_roots {
                if root.eq_ignore_ascii_case(r"E:\SteamRIPPED")
                    || root.eq_ignore_ascii_case(r"E:\GameVault")
                {
                    *root = r"E:\GameVault\Games".to_string();
                    migrated = true;
                }
            }
            if migrated {
                connection
                    .execute(
                        "UPDATE games SET detection_status = 'missing', updated_at = ?1
                         WHERE lower(install_path) LIKE lower('E:\\SteamRIPPED%')",
                        [Utc::now().to_rfc3339()],
                    )
                    .map_err(|error| error.to_string())?;
                save_settings(connection, &settings)?;
            }
            Ok(settings)
        }
        None => {
            let settings = Settings::default();
            save_settings(connection, &settings)?;
            Ok(settings)
        }
    }
}

pub fn save_settings(connection: &Connection, settings: &Settings) -> Result<(), String> {
    let json = serde_json::to_string(settings).map_err(|error| error.to_string())?;
    connection
        .execute(
            "INSERT INTO settings(key, value_json, updated_at)
             VALUES('application', ?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json, updated_at = excluded.updated_at",
            params![json, Utc::now().to_rfc3339()],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

pub fn list_games(connection: &Connection) -> Result<Vec<Game>, String> {
    let mut statement = connection
        .prepare(
            "SELECT id, title, description, install_path, executable_path,
                    launch_args_json, tags_json, category, favorite,
                    detection_status, detection_source, folder_size_bytes,
                    last_played_at, playtime_seconds, added_at, updated_at,
                    content_signature, artwork_seed, metadata_json
             FROM games
             ORDER BY lower(title), id",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], row_to_game)
        .map_err(|error| error.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

pub fn get_game(connection: &Connection, id: &str) -> Result<Option<Game>, String> {
    connection
        .query_row(
            "SELECT id, title, description, install_path, executable_path,
                    launch_args_json, tags_json, category, favorite,
                    detection_status, detection_source, folder_size_bytes,
                    last_played_at, playtime_seconds, added_at, updated_at,
                    content_signature, artwork_seed, metadata_json
             FROM games WHERE id = ?1",
            [id],
            row_to_game,
        )
        .optional()
        .map_err(|error| error.to_string())
}

fn row_to_game(row: &Row<'_>) -> rusqlite::Result<Game> {
    let launch_args_json: String = row.get(5)?;
    let tags_json: String = row.get(6)?;
    let size: Option<i64> = row.get(11)?;
    let playtime: i64 = row.get(13)?;
    let artwork_seed: i64 = row.get(17)?;
    let metadata_json: String = row.get(18)?;
    Ok(Game {
        id: row.get(0)?,
        title: row.get(1)?,
        description: row.get(2)?,
        install_path: row.get(3)?,
        executable_path: row.get(4)?,
        launch_args: serde_json::from_str(&launch_args_json).unwrap_or_default(),
        tags: serde_json::from_str(&tags_json).unwrap_or_default(),
        category: row.get(7)?,
        favorite: row.get::<_, i64>(8)? != 0,
        detection_status: row.get(9)?,
        detection_source: row.get(10)?,
        folder_size_bytes: size.and_then(|value| u64::try_from(value).ok()),
        last_played_at: row.get(12)?,
        playtime_seconds: u64::try_from(playtime).unwrap_or_default(),
        added_at: row.get(14)?,
        updated_at: row.get(15)?,
        content_signature: row.get(16)?,
        artwork_seed: u32::try_from(artwork_seed).unwrap_or_default(),
        metadata: serde_json::from_str(&metadata_json).unwrap_or_default(),
    })
}

pub fn stats(games: &[Game]) -> LibraryStats {
    LibraryStats {
        total_games: games.len(),
        ready_games: games
            .iter()
            .filter(|game| matches!(game.detection_status.as_str(), "detected" | "configured"))
            .count(),
        missing_games: games
            .iter()
            .filter(|game| matches!(game.detection_status.as_str(), "missing" | "unavailable"))
            .count(),
        favorites: games.iter().filter(|game| game.favorite).count(),
        total_playtime_seconds: games.iter().map(|game| game.playtime_seconds).sum(),
    }
}

pub fn apply_scan(
    connection: &mut Connection,
    detected: &[DetectedGame],
    available_roots: &[String],
    unavailable_roots: &[String],
    started_at: &str,
) -> Result<ApplyScanResult, String> {
    let transaction = connection
        .transaction()
        .map_err(|error| error.to_string())?;

    for root in available_roots {
        transaction
            .execute(
                "UPDATE games SET detection_status = 'missing', updated_at = ?2
                 WHERE lower(install_path) LIKE lower(?1)",
                params![
                    format!("{}%", root.trim_end_matches(['\\', '/'])),
                    Utc::now().to_rfc3339()
                ],
            )
            .map_err(|error| error.to_string())?;
    }
    for root in unavailable_roots {
        transaction
            .execute(
                "UPDATE games SET detection_status = 'unavailable', updated_at = ?2
                 WHERE lower(install_path) LIKE lower(?1)",
                params![
                    format!("{}%", root.trim_end_matches(['\\', '/'])),
                    Utc::now().to_rfc3339()
                ],
            )
            .map_err(|error| error.to_string())?;
    }

    let mut added = 0;
    let mut updated = 0;
    for game in detected {
        let existing_id: Option<String> = transaction
            .query_row(
                "SELECT id FROM games WHERE lower(install_path) = lower(?1)",
                [&game.install_path],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| error.to_string())?;

        let moved_id = if existing_id.is_none() && !game.content_signature.is_empty() {
            let mut statement = transaction
                .prepare(
                    "SELECT id FROM games
                     WHERE content_signature = ?1 AND detection_status IN ('missing', 'unavailable')
                     LIMIT 2",
                )
                .map_err(|error| error.to_string())?;
            let matches = statement
                .query_map([&game.content_signature], |row| row.get::<_, String>(0))
                .map_err(|error| error.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| error.to_string())?;
            (matches.len() == 1).then(|| matches[0].clone())
        } else {
            None
        };

        let now = Utc::now().to_rfc3339();
        if let Some(id) = existing_id.or(moved_id) {
            transaction
                .execute(
                    "UPDATE games SET
                       title = CASE WHEN detection_source = 'manual' THEN title ELSE ?2 END,
                       install_path = ?3,
                       executable_path = CASE WHEN detection_source = 'manual' THEN executable_path ELSE ?4 END,
                       detection_status = CASE WHEN detection_source = 'manual' THEN 'configured' ELSE 'detected' END,
                       folder_size_bytes = ?5,
                       folder_modified_at = ?6,
                       content_signature = ?7,
                       updated_at = ?8
                     WHERE id = ?1",
                    params![
                        id,
                        game.title,
                        game.install_path,
                        game.executable_path,
                        game.folder_size_bytes.and_then(|value| i64::try_from(value).ok()),
                        game.folder_modified_at,
                        game.content_signature,
                        now
                    ],
                )
                .map_err(|error| error.to_string())?;
            updated += 1;
        } else {
            let id = Uuid::now_v7().to_string();
            let seed = artwork_seed(&id);
            transaction
                .execute(
                    "INSERT INTO games(
                       id, title, install_path, executable_path, detection_status,
                       detection_source, folder_size_bytes, folder_modified_at,
                       content_signature, added_at, updated_at, artwork_seed
                     ) VALUES(?1, ?2, ?3, ?4, 'detected', 'automatic', ?5, ?6, ?7, ?8, ?8, ?9)",
                    params![
                        id,
                        game.title,
                        game.install_path,
                        game.executable_path,
                        game.folder_size_bytes
                            .and_then(|value| i64::try_from(value).ok()),
                        game.folder_modified_at,
                        game.content_signature,
                        now,
                        i64::from(seed)
                    ],
                )
                .map_err(|error| error.to_string())?;
            added += 1;
        }
    }

    let completed_at = Utc::now().to_rfc3339();
    transaction
        .execute(
            "INSERT INTO scan_history(
               id, started_at, completed_at, folders_scanned, games_detected, unavailable_roots_json
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                Uuid::now_v7().to_string(),
                started_at,
                completed_at,
                i64::try_from(detected.len()).unwrap_or(i64::MAX),
                i64::try_from(detected.len()).unwrap_or(i64::MAX),
                serde_json::to_string(unavailable_roots).unwrap_or_else(|_| "[]".to_string())
            ],
        )
        .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())?;
    Ok(ApplyScanResult { added, updated })
}

pub fn add_manual_game(connection: &Connection, executable: &Path) -> Result<Game, String> {
    let install_dir = executable
        .parent()
        .ok_or_else(|| "The executable has no parent folder.".to_string())?;
    let title = install_dir
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("Local game")
        .replace(['_', '-'], " ");
    let existing: Option<String> = connection
        .query_row(
            "SELECT id FROM games WHERE lower(install_path) = lower(?1)",
            [install_dir.to_string_lossy().to_string()],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let id = existing.unwrap_or_else(|| Uuid::now_v7().to_string());
    let now = Utc::now().to_rfc3339();
    connection
        .execute(
            "INSERT INTO games(
               id, title, install_path, executable_path, detection_status,
               detection_source, added_at, updated_at, artwork_seed
             ) VALUES(?1, ?2, ?3, ?4, 'configured', 'manual', ?5, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET
               executable_path = excluded.executable_path,
               detection_status = 'configured',
               detection_source = 'manual',
               updated_at = excluded.updated_at",
            params![
                id,
                title,
                install_dir.to_string_lossy().to_string(),
                executable.to_string_lossy().to_string(),
                now,
                i64::from(artwork_seed(&id))
            ],
        )
        .map_err(|error| error.to_string())?;
    get_game(connection, &id)?.ok_or_else(|| "Manual game was not saved.".to_string())
}

pub fn register_imported_game(
    connection: &Connection,
    title: &str,
    install_dir: &Path,
    executable: &Path,
) -> Result<Game, String> {
    if !install_dir.is_dir() || !executable.is_file() || !executable.starts_with(install_dir) {
        return Err("The promoted game files are unavailable.".to_string());
    }
    let detected = crate::scanner::inspect_game_folder(install_dir, 8);
    let existing: Option<String> = connection
        .query_row(
            "SELECT id FROM games WHERE lower(install_path) = lower(?1)",
            [install_dir.to_string_lossy().to_string()],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let id = existing.unwrap_or_else(|| Uuid::now_v7().to_string());
    let now = Utc::now().to_rfc3339();
    let folder_size = detected
        .as_ref()
        .and_then(|value| value.folder_size_bytes)
        .and_then(|value| i64::try_from(value).ok());
    let folder_modified = detected
        .as_ref()
        .map(|value| value.folder_modified_at)
        .unwrap_or_default();
    let signature = detected
        .as_ref()
        .map(|value| value.content_signature.as_str())
        .unwrap_or_default();
    connection
        .execute(
            "INSERT INTO games(
               id, title, install_path, executable_path, detection_status,
               detection_source, folder_size_bytes, folder_modified_at,
               content_signature, added_at, updated_at, artwork_seed
             ) VALUES(?1, ?2, ?3, ?4, 'configured', 'archive', ?5, ?6, ?7, ?8, ?8, ?9)
             ON CONFLICT(id) DO UPDATE SET
               title = excluded.title,
               install_path = excluded.install_path,
               executable_path = excluded.executable_path,
               detection_status = 'configured',
               detection_source = 'archive',
               folder_size_bytes = excluded.folder_size_bytes,
               folder_modified_at = excluded.folder_modified_at,
               content_signature = excluded.content_signature,
               updated_at = excluded.updated_at",
            params![
                id,
                title.trim(),
                install_dir.to_string_lossy().to_string(),
                executable.to_string_lossy().to_string(),
                folder_size,
                folder_modified,
                signature,
                now,
                i64::from(artwork_seed(&id))
            ],
        )
        .map_err(|error| error.to_string())?;
    get_game(connection, &id)?.ok_or_else(|| "Imported game was not saved.".to_string())
}

pub fn save_game_metadata(
    connection: &Connection,
    game_id: &str,
    metadata: &GameMetadata,
) -> Result<Game, String> {
    let metadata_json = serde_json::to_string(metadata).map_err(|error| error.to_string())?;
    let description = metadata.short_description.as_deref().unwrap_or_default();
    let tags_json = serde_json::to_string(&metadata.genres).map_err(|error| error.to_string())?;
    let category = metadata
        .genres
        .first()
        .map(String::as_str)
        .unwrap_or("Uncategorized");
    connection
        .execute(
            "UPDATE games SET
               metadata_json = ?2,
               description = CASE WHEN trim(description) = '' THEN ?3 ELSE description END,
               tags_json = CASE WHEN tags_json = '[]' THEN ?4 ELSE tags_json END,
               category = CASE WHEN category = '' OR category = 'Uncategorized' THEN ?5 ELSE category END,
               updated_at = ?6
             WHERE id = ?1",
            params![
                game_id,
                metadata_json,
                description,
                tags_json,
                category,
                Utc::now().to_rfc3339()
            ],
        )
        .map_err(|error| error.to_string())?;
    get_game(connection, game_id)?.ok_or_else(|| "Game not found.".to_string())
}

pub fn update_game(connection: &Connection, input: &UpdateGameInput) -> Result<Game, String> {
    let title = input.title.trim();
    if title.is_empty() {
        return Err("A game title is required.".to_string());
    }
    connection
        .execute(
            "UPDATE games SET
               title = ?2, description = ?3, executable_path = ?4,
               launch_args_json = ?5, tags_json = ?6, category = ?7,
               detection_status = 'configured', detection_source = 'manual',
               updated_at = ?8
             WHERE id = ?1",
            params![
                input.id,
                title,
                input.description.trim(),
                input.executable_path,
                serde_json::to_string(&input.launch_args).map_err(|error| error.to_string())?,
                serde_json::to_string(&input.tags).map_err(|error| error.to_string())?,
                input.category.trim(),
                Utc::now().to_rfc3339()
            ],
        )
        .map_err(|error| error.to_string())?;
    get_game(connection, &input.id)?.ok_or_else(|| "Game not found.".to_string())
}

pub fn toggle_favorite(connection: &Connection, id: &str) -> Result<Game, String> {
    connection
        .execute(
            "UPDATE games SET favorite = CASE favorite WHEN 0 THEN 1 ELSE 0 END, updated_at = ?2 WHERE id = ?1",
            params![id, Utc::now().to_rfc3339()],
        )
        .map_err(|error| error.to_string())?;
    get_game(connection, id)?.ok_or_else(|| "Game not found.".to_string())
}

pub fn remove_game(connection: &Connection, id: &str) -> Result<(), String> {
    connection
        .execute("DELETE FROM games WHERE id = ?1", [id])
        .map_err(|error| error.to_string())?;
    Ok(())
}

pub fn record_play_session(connection: &Connection, id: &str, seconds: u64) -> Result<(), String> {
    connection
        .execute(
            "UPDATE games SET
               last_played_at = ?2,
               playtime_seconds = playtime_seconds + ?3,
               updated_at = ?2
             WHERE id = ?1",
            params![
                id,
                Utc::now().to_rfc3339(),
                i64::try_from(seconds).unwrap_or(i64::MAX)
            ],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

pub fn create_backup(connection: &Connection, backups_dir: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(backups_dir).map_err(|error| error.to_string())?;
    let path = backups_dir.join(format!(
        "GameVault-library-{}.db",
        Utc::now().format("%Y%m%d-%H%M%S")
    ));
    connection
        .execute("VACUUM INTO ?1", [path.to_string_lossy().to_string()])
        .map_err(|error| error.to_string())?;
    Ok(path)
}

pub fn restore_backup(connection: &mut Connection, backup_path: &Path) -> Result<(), String> {
    let check =
        Connection::open_with_flags(backup_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|error| error.to_string())?;
    let has_games: i64 = check
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='games'",
            [],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if has_games != 1 {
        return Err("The selected file is not a GameVault backup.".to_string());
    }
    drop(check);

    connection
        .execute(
            "ATTACH DATABASE ?1 AS restore_db",
            [backup_path.to_string_lossy().to_string()],
        )
        .map_err(|error| error.to_string())?;
    let backup_has_metadata: i64 = connection
        .query_row(
            "SELECT count(*) FROM pragma_table_info('games', 'restore_db') WHERE name = 'metadata_json'",
            [],
            |row| row.get(0),
        )
        .unwrap_or_default();
    let metadata_columns = if backup_has_metadata == 1 {
        ", metadata_json"
    } else {
        ""
    };
    let restore_sql = format!(
        "BEGIN IMMEDIATE;
         DELETE FROM games;
         INSERT INTO games(
           id, title, description, install_path, executable_path,
           launch_args_json, tags_json, category, favorite,
           detection_status, detection_source, folder_size_bytes,
           last_played_at, playtime_seconds, added_at, updated_at,
           folder_modified_at, content_signature, artwork_seed{metadata_columns}
         ) SELECT
           id, title, description, install_path, executable_path,
           launch_args_json, tags_json, category, favorite,
           detection_status, detection_source, folder_size_bytes,
           last_played_at, playtime_seconds, added_at, updated_at,
           folder_modified_at, content_signature, artwork_seed{metadata_columns}
         FROM restore_db.games;
         DELETE FROM settings;
         INSERT INTO settings SELECT * FROM restore_db.settings;
         COMMIT;"
    );
    let result = connection.execute_batch(&restore_sql);
    let _ = connection.execute_batch("DETACH DATABASE restore_db;");
    result.map_err(|error| error.to_string())
}

fn artwork_seed(value: &str) -> u32 {
    value.bytes().fold(2_166_136_261_u32, |hash, byte| {
        hash.wrapping_mul(16_777_619) ^ u32::from(byte)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn database_round_trip_preserves_manual_metadata() {
        let directory = tempfile::tempdir().expect("temp directory");
        let database = directory.path().join("library.db");
        let connection = open_database(&database).expect("database");
        let game_dir = directory.path().join("Example Game");
        fs::create_dir_all(&game_dir).expect("game dir");
        let executable = game_dir.join("ExampleGame.exe");
        fs::write(&executable, b"fixture").expect("fixture");

        let game = add_manual_game(&connection, &executable).expect("manual game");
        let updated = update_game(
            &connection,
            &UpdateGameInput {
                id: game.id.clone(),
                title: "Example Game".to_string(),
                description: "A local fixture".to_string(),
                executable_path: executable.to_string_lossy().to_string(),
                launch_args: vec!["--windowed".to_string()],
                tags: vec!["Test".to_string()],
                category: "Fixture".to_string(),
            },
        )
        .expect("updated game");

        assert_eq!(updated.description, "A local fixture");
        assert_eq!(updated.launch_args, vec!["--windowed"]);
        assert_eq!(updated.detection_status, "configured");

        let metadata = GameMetadata {
            provider: Some("steam".to_string()),
            external_id: Some("440".to_string()),
            store_url: Some("https://store.steampowered.com/app/440/".to_string()),
            title: Some("Example Game".to_string()),
            genres: vec!["Action".to_string()],
            ..GameMetadata::default()
        };
        let linked = save_game_metadata(&connection, &game.id, &metadata).expect("metadata");
        assert_eq!(linked.metadata.provider.as_deref(), Some("steam"));
        assert_eq!(linked.metadata.external_id.as_deref(), Some("440"));
    }

    #[test]
    fn portable_database_uses_a_relative_managed_library() {
        let directory = tempfile::tempdir().expect("temp directory");
        let portable_root = directory.path().join("GameVault portable");
        let database = portable_root.join("data").join("library.db");
        let connection = open_portable_database(&database, &portable_root).expect("database");
        let settings = get_settings(&connection).expect("settings");

        assert_eq!(
            PathBuf::from(settings.managed_root),
            portable_root.join("library")
        );
        assert_eq!(
            settings.library_roots,
            vec![portable_root
                .join("library")
                .join("Games")
                .to_string_lossy()
                .to_string()]
        );
    }

    #[test]
    fn legacy_e_drive_defaults_migrate_to_the_portable_library() {
        let directory = tempfile::tempdir().expect("temp directory");
        let portable_root = directory.path().join("GameVault portable");
        let database = portable_root.join("data").join("library.db");
        let connection = open_database(&database).expect("database");
        save_settings(
            &connection,
            &Settings {
                managed_root: r"E:\GameVault".to_string(),
                library_roots: vec![r"E:\GameVault\Games".to_string()],
                ..Settings::default()
            },
        )
        .expect("legacy settings");

        migrate_legacy_portability_defaults(&connection, &portable_root).expect("migration");
        let settings = get_settings(&connection).expect("settings");

        assert_eq!(
            PathBuf::from(settings.managed_root),
            portable_root.join("library")
        );
        assert_eq!(
            settings.library_roots,
            vec![portable_root
                .join("library")
                .join("Games")
                .to_string_lossy()
                .to_string()]
        );
    }
}
