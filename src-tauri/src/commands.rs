use crate::app::{clear_cache, AppCore};
use crate::models::{
    AppSnapshot, ArchiveInspection, CommandError, DependencyAudit, Game, GameMetadata,
    HealthReport, InboxArchive, InstallStagedInput, InstalledPackage, MetadataLookupInput,
    SaveGameMetadataInput, ScanResult, Settings, StagedArchive, StagedPackageAnalysis,
    UpdateGameInput, WorkspaceStatus,
};
use crate::{archives, dependencies, diagnostics, metadata, scanner, storage, workspace};
use chrono::Utc;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Instant;
use tauri::{AppHandle, Emitter, State};

struct ScanInProgressReset {
    core: Arc<AppCore>,
}

impl Drop for ScanInProgressReset {
    fn drop(&mut self) {
        self.core.scan_in_progress.store(false, Ordering::SeqCst);
    }
}

fn lock_error() -> CommandError {
    CommandError::new(
        "state.lock_failed",
        "GameVault is busy. Wait a moment and try again.",
        true,
    )
}

fn record_event(core: &AppCore, event: &str, outcome: &str) {
    let enabled = core
        .database
        .lock()
        .ok()
        .and_then(|connection| storage::get_settings(&connection).ok())
        .is_some_and(|settings| settings.logging_enabled);
    let _ = diagnostics::record(&core.portable_root, enabled, event, outcome);
}

fn validate_executable(executable: &Path, install_path: &Path) -> Result<PathBuf, CommandError> {
    if !install_path.is_dir() {
        return Err(CommandError::new(
            "launch.folder_unavailable",
            "The game folder cannot be accessed.",
            false,
        ));
    }
    if !executable.is_file() {
        return Err(CommandError::new(
            "launch.executable_missing",
            "The selected game executable is missing. Choose another executable.",
            false,
        ));
    }
    let is_exe = executable
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.eq_ignore_ascii_case("exe"))
        .unwrap_or(false);
    if !is_exe {
        return Err(CommandError::new(
            "launch.executable_invalid",
            "GameVault launches Windows .exe files only.",
            false,
        ));
    }
    let canonical_executable = executable.canonicalize().map_err(|_| {
        CommandError::new(
            "launch.executable_unavailable",
            "The selected executable cannot be accessed.",
            false,
        )
    })?;
    let canonical_install = install_path.canonicalize().map_err(|_| {
        CommandError::new(
            "launch.folder_unavailable",
            "The game folder cannot be accessed.",
            false,
        )
    })?;
    if !canonical_executable.starts_with(&canonical_install) {
        return Err(CommandError::new(
            "launch.executable_outside_game",
            "Choose an executable inside the game's installation folder.",
            false,
        ));
    }
    Ok(canonical_executable)
}

fn validate_arguments(arguments: &[String]) -> Result<(), CommandError> {
    if arguments.len() > 64
        || arguments
            .iter()
            .any(|argument| argument.len() > 2048 || argument.contains('\0'))
    {
        return Err(CommandError::new(
            "launch.arguments_invalid",
            "Launch arguments exceed the safe length or count limit.",
            false,
        ));
    }
    Ok(())
}

#[tauri::command]
pub fn get_snapshot(state: State<'_, Arc<AppCore>>) -> Result<AppSnapshot, CommandError> {
    let connection = state.database.lock().map_err(|_| lock_error())?;
    let games = storage::list_games(&connection)
        .map_err(|_| CommandError::internal("storage.library_read_failed"))?;
    let settings = storage::get_settings(&connection)
        .map_err(|_| CommandError::internal("storage.settings_read_failed"))?;
    let stats = storage::stats(&games);
    Ok(AppSnapshot {
        games,
        settings,
        stats,
        portable_root: state.portable_root.to_string_lossy().to_string(),
        sqlite_version: storage::sqlite_version(&connection),
        scan_in_progress: state.scan_in_progress.load(Ordering::SeqCst),
    })
}

#[tauri::command]
pub async fn scan_library(
    app: AppHandle,
    state: State<'_, Arc<AppCore>>,
) -> Result<ScanResult, CommandError> {
    if state
        .scan_in_progress
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err(CommandError::new(
            "scan.already_running",
            "A library scan is already running.",
            true,
        ));
    }

    let core = Arc::clone(state.inner());
    let _scan_in_progress_reset = ScanInProgressReset {
        core: Arc::clone(&core),
    };
    let settings = {
        let connection = core.database.lock().map_err(|_| lock_error())?;
        storage::get_settings(&connection)
            .map_err(|_| CommandError::internal("storage.settings_read_failed"))?
    };
    let started_at = Utc::now().to_rfc3339();
    let app_for_scan = app.clone();
    let roots = settings.library_roots.clone();
    let scan_depth = settings.scan_depth;
    let discovery = tauri::async_runtime::spawn_blocking(move || {
        scanner::discover_games(&roots, scan_depth, |progress| {
            let _ = app_for_scan.emit("scan-progress", progress);
        })
    })
    .await;

    let result = match discovery {
        Ok(discovery) => {
            let mut connection = core.database.lock().map_err(|_| lock_error())?;
            let applied = storage::apply_scan(
                &mut connection,
                &discovery.games,
                &discovery.available_roots,
                &discovery.unavailable_roots,
                &started_at,
            )
            .map_err(|_| CommandError::internal("scan.persist_failed"))?;
            let mut updated_settings = settings;
            let completed_at = Utc::now().to_rfc3339();
            updated_settings.last_scan_at = Some(completed_at.clone());
            storage::save_settings(&connection, &updated_settings)
                .map_err(|_| CommandError::internal("scan.settings_update_failed"))?;
            Ok(ScanResult {
                folders_scanned: discovery.folders_scanned,
                games_detected: discovery.games.len(),
                games_added: applied.added,
                games_updated: applied.updated,
                unavailable_roots: discovery.unavailable_roots,
                completed_at,
            })
        }
        Err(_) => Err(CommandError::internal("scan.worker_failed")),
    };
    let _ = app.emit("library-changed", ());
    result
}

#[tauri::command]
pub async fn choose_library_directory() -> Result<Option<String>, CommandError> {
    Ok(rfd::AsyncFileDialog::new()
        .set_title("Choose a local game library folder")
        .pick_folder()
        .await
        .map(|handle| handle.path().to_string_lossy().to_string()))
}

#[tauri::command]
pub async fn choose_game_executable(
    initial_directory: Option<String>,
) -> Result<Option<String>, CommandError> {
    let mut dialog = rfd::AsyncFileDialog::new()
        .set_title("Choose the game executable")
        .add_filter("Windows application", &["exe"]);
    if let Some(directory) = initial_directory {
        if Path::new(&directory).is_dir() {
            dialog = dialog.set_directory(directory);
        }
    }
    Ok(dialog
        .pick_file()
        .await
        .map(|handle| handle.path().to_string_lossy().to_string()))
}

#[tauri::command]
pub async fn choose_backup_file() -> Result<Option<String>, CommandError> {
    Ok(rfd::AsyncFileDialog::new()
        .set_title("Choose a GameVault database backup")
        .add_filter("GameVault database", &["db"])
        .pick_file()
        .await
        .map(|handle| handle.path().to_string_lossy().to_string()))
}

#[tauri::command]
pub fn save_settings(
    settings: Settings,
    state: State<'_, Arc<AppCore>>,
) -> Result<Settings, CommandError> {
    if !(1..=8).contains(&settings.scan_depth) || settings.library_roots.len() > 32 {
        return Err(CommandError::new(
            "settings.invalid",
            "Scan depth must be between 1 and 8 and no more than 32 roots may be configured.",
            false,
        ));
    }
    if !matches!(
        settings.theme.as_str(),
        "midnight" | "deep-blue" | "high-contrast"
    ) {
        return Err(CommandError::new(
            "settings.theme_invalid",
            "Choose one of the available themes.",
            false,
        ));
    }
    validate_arguments(&settings.default_launch_args)?;
    workspace::validate_managed_root(Path::new(settings.managed_root.trim())).map_err(|_| {
        CommandError::new(
            "settings.managed_root_invalid",
            "Choose a dedicated GameVault folder rather than a drive root.",
            false,
        )
    })?;
    let mut seen = HashSet::new();
    let mut normalized = settings;
    normalized.managed_root = normalized.managed_root.trim().to_string();
    normalized.library_roots = normalized
        .library_roots
        .into_iter()
        .map(|root| root.trim().to_string())
        .filter(|root| !root.is_empty())
        .filter(|root| seen.insert(root.to_lowercase()))
        .collect();
    let connection = state.database.lock().map_err(|_| lock_error())?;
    storage::save_settings(&connection, &normalized)
        .map_err(|_| CommandError::internal("settings.save_failed"))?;
    drop(connection);
    record_event(state.inner(), "settings.saved", "ok");
    Ok(normalized)
}

#[tauri::command]
pub fn add_manual_game(
    executable_path: String,
    state: State<'_, Arc<AppCore>>,
) -> Result<Game, CommandError> {
    let executable = PathBuf::from(&executable_path);
    let install = executable.parent().unwrap_or(Path::new(""));
    validate_executable(&executable, install)?;
    let connection = state.database.lock().map_err(|_| lock_error())?;
    storage::add_manual_game(&connection, &executable)
        .map_err(|_| CommandError::internal("library.manual_add_failed"))
}

#[tauri::command]
pub fn update_game(
    input: UpdateGameInput,
    state: State<'_, Arc<AppCore>>,
) -> Result<Game, CommandError> {
    validate_arguments(&input.launch_args)?;
    if input.title.trim().is_empty()
        || input.title.len() > 200
        || input.description.len() > 20_000
        || input.category.len() > 100
        || input.tags.len() > 64
        || input.tags.iter().any(|tag| tag.len() > 100)
        || [&input.title, &input.description, &input.category]
            .into_iter()
            .chain(input.tags.iter())
            .any(|value| value.contains('\0'))
    {
        return Err(CommandError::new(
            "library.metadata_invalid",
            "Game details exceed the supported length or item limits.",
            false,
        ));
    }
    let connection = state.database.lock().map_err(|_| lock_error())?;
    let existing = storage::get_game(&connection, &input.id)
        .map_err(|_| CommandError::internal("storage.game_read_failed"))?
        .ok_or_else(|| {
            CommandError::new(
                "library.game_not_found",
                "The game is no longer in the library.",
                false,
            )
        })?;
    validate_executable(
        Path::new(&input.executable_path),
        Path::new(&existing.install_path),
    )?;
    storage::update_game(&connection, &input)
        .map_err(|_| CommandError::internal("library.game_update_failed"))
}

#[tauri::command]
pub fn toggle_favorite(id: String, state: State<'_, Arc<AppCore>>) -> Result<Game, CommandError> {
    let connection = state.database.lock().map_err(|_| lock_error())?;
    storage::toggle_favorite(&connection, &id)
        .map_err(|_| CommandError::internal("library.favorite_failed"))
}

#[tauri::command]
pub fn remove_game(id: String, state: State<'_, Arc<AppCore>>) -> Result<(), CommandError> {
    let connection = state.database.lock().map_err(|_| lock_error())?;
    storage::remove_game(&connection, &id)
        .map_err(|_| CommandError::internal("library.remove_failed"))
}

#[tauri::command]
pub fn launch_game(
    id: String,
    app: AppHandle,
    state: State<'_, Arc<AppCore>>,
) -> Result<(), CommandError> {
    let (game, settings) = {
        let connection = state.database.lock().map_err(|_| lock_error())?;
        let game = storage::get_game(&connection, &id)
            .map_err(|_| CommandError::internal("storage.game_read_failed"))?
            .ok_or_else(|| {
                CommandError::new(
                    "library.game_not_found",
                    "The game is no longer in the library.",
                    false,
                )
            })?;
        let settings = storage::get_settings(&connection)
            .map_err(|_| CommandError::internal("storage.settings_read_failed"))?;
        (game, settings)
    };

    let executable = validate_executable(
        Path::new(&game.executable_path),
        Path::new(&game.install_path),
    )?;
    let mut arguments = settings.default_launch_args;
    arguments.extend(game.launch_args.clone());
    validate_arguments(&arguments)?;

    {
        let mut running = state.running_games.lock().map_err(|_| lock_error())?;
        if !running.insert(id.clone()) {
            return Err(CommandError::new(
                "launch.already_running",
                "This game is already running from GameVault.",
                true,
            ));
        }
    }

    let working_directory = executable.parent().unwrap_or(Path::new("."));
    let child = Command::new(&executable)
        .args(&arguments)
        .current_dir(working_directory)
        .spawn();
    let mut child = match child {
        Ok(value) => value,
        Err(_) => {
            if let Ok(mut running) = state.running_games.lock() {
                running.remove(&id);
            }
            return Err(CommandError::new(
                "launch.process_failed",
                "Windows could not start this game. Check the executable and permissions.",
                true,
            ));
        }
    };

    record_event(state.inner(), "game.launch", "started");

    let core = Arc::clone(state.inner());
    std::thread::spawn(move || {
        let started = Instant::now();
        let _ = child.wait();
        let elapsed = started.elapsed().as_secs();
        if let Ok(connection) = core.database.lock() {
            let _ = storage::record_play_session(&connection, &id, elapsed);
        }
        if let Ok(mut running) = core.running_games.lock() {
            running.remove(&id);
        }
        record_event(&core, "game.launch", "completed");
        let _ = app.emit("library-changed", ());
    });
    Ok(())
}

#[tauri::command]
pub fn open_game_folder(id: String, state: State<'_, Arc<AppCore>>) -> Result<(), CommandError> {
    let connection = state.database.lock().map_err(|_| lock_error())?;
    let game = storage::get_game(&connection, &id)
        .map_err(|_| CommandError::internal("storage.game_read_failed"))?
        .ok_or_else(|| {
            CommandError::new(
                "library.game_not_found",
                "The game is no longer in the library.",
                false,
            )
        })?;
    let folder = PathBuf::from(game.install_path);
    if !folder.is_dir() {
        return Err(CommandError::new(
            "library.folder_unavailable",
            "The game folder is missing or unavailable.",
            false,
        ));
    }
    Command::new("explorer.exe")
        .arg(&folder)
        .spawn()
        .map_err(|_| {
            CommandError::new(
                "platform.explorer_failed",
                "Windows Explorer could not open this folder.",
                true,
            )
        })?;
    Ok(())
}

#[tauri::command]
pub fn open_logs_folder(state: State<'_, Arc<AppCore>>) -> Result<(), CommandError> {
    let logs = state.portable_root.join("logs");
    Command::new("explorer.exe")
        .arg(&logs)
        .spawn()
        .map_err(|_| {
            CommandError::new(
                "platform.explorer_failed",
                "Windows Explorer could not open the log folder.",
                true,
            )
        })?;
    Ok(())
}

#[tauri::command]
pub fn create_database_backup(state: State<'_, Arc<AppCore>>) -> Result<String, CommandError> {
    let connection = state.database.lock().map_err(|_| lock_error())?;
    let path = storage::create_backup(
        &connection,
        &state.portable_root.join("data").join("backups"),
    )
    .map_err(|_| CommandError::internal("storage.backup_failed"))?;
    drop(connection);
    record_event(state.inner(), "database.backup", "created");
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn restore_database_backup(
    backup_path: String,
    state: State<'_, Arc<AppCore>>,
) -> Result<(), CommandError> {
    let path = PathBuf::from(backup_path);
    if !path.is_file()
        || path
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| !value.eq_ignore_ascii_case("db"))
            .unwrap_or(true)
    {
        return Err(CommandError::new(
            "storage.backup_invalid",
            "Choose a valid GameVault .db backup.",
            false,
        ));
    }
    let mut connection = state.database.lock().map_err(|_| lock_error())?;
    storage::create_backup(
        &connection,
        &state.portable_root.join("data").join("backups"),
    )
    .map_err(|_| CommandError::internal("storage.pre_restore_backup_failed"))?;
    storage::restore_backup(&mut connection, &path)
        .map_err(|_| CommandError::internal("storage.restore_failed"))?;
    storage::migrate_legacy_portability_defaults(&connection, &state.portable_root)
        .map_err(|_| CommandError::internal("storage.restore_migration_failed"))?;
    drop(connection);
    record_event(state.inner(), "database.backup", "restored");
    Ok(())
}

#[tauri::command]
pub fn clear_application_cache(state: State<'_, Arc<AppCore>>) -> Result<(), CommandError> {
    clear_cache(&state.portable_root)
        .map_err(|_| CommandError::internal("storage.cache_clear_failed"))?;
    record_event(state.inner(), "cache.cleared", "ok");
    Ok(())
}

#[tauri::command]
pub fn get_health_report(state: State<'_, Arc<AppCore>>) -> Result<HealthReport, CommandError> {
    let connection = state.database.lock().map_err(|_| lock_error())?;
    Ok(HealthReport {
        ok: true,
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        portable_root: state.portable_root.to_string_lossy().to_string(),
        database_path: state.database_path.to_string_lossy().to_string(),
        sqlite_version: storage::sqlite_version(&connection),
        webview2_runtime: "Windows-provided Evergreen runtime".to_string(),
    })
}

#[tauri::command]
pub fn get_workspace_status(
    state: State<'_, Arc<AppCore>>,
) -> Result<WorkspaceStatus, CommandError> {
    let connection = state.database.lock().map_err(|_| lock_error())?;
    let settings = storage::get_settings(&connection)
        .map_err(|_| CommandError::internal("storage.settings_read_failed"))?;
    workspace::status(Path::new(&settings.managed_root)).map_err(|_| {
        CommandError::new(
            "workspace.status_failed",
            "The managed GameVault folder is unavailable or invalid.",
            true,
        )
    })
}

#[tauri::command]
pub fn prepare_workspace(state: State<'_, Arc<AppCore>>) -> Result<WorkspaceStatus, CommandError> {
    let connection = state.database.lock().map_err(|_| lock_error())?;
    let mut settings = storage::get_settings(&connection)
        .map_err(|_| CommandError::internal("storage.settings_read_failed"))?;
    let status = workspace::prepare(Path::new(&settings.managed_root)).map_err(|_| {
        CommandError::new(
            "workspace.prepare_failed",
            "GameVault could not create the managed folder layout.",
            true,
        )
    })?;
    let games_root = Path::new(&settings.managed_root)
        .join("Games")
        .to_string_lossy()
        .to_string();
    if !settings
        .library_roots
        .iter()
        .any(|root| root.eq_ignore_ascii_case(&games_root))
    {
        settings.library_roots.insert(0, games_root);
        storage::save_settings(&connection, &settings)
            .map_err(|_| CommandError::internal("settings.save_failed"))?;
    }
    Ok(status)
}

#[tauri::command]
pub async fn audit_dependencies(
    state: State<'_, Arc<AppCore>>,
) -> Result<DependencyAudit, CommandError> {
    let managed_root = {
        let connection = state.database.lock().map_err(|_| lock_error())?;
        storage::get_settings(&connection)
            .map_err(|_| CommandError::internal("storage.settings_read_failed"))?
            .managed_root
    };
    tauri::async_runtime::spawn_blocking(move || dependencies::audit(Path::new(&managed_root)))
        .await
        .map_err(|_| CommandError::internal("dependencies.worker_failed"))?
        .map_err(|_| {
            CommandError::new(
                "dependencies.audit_failed",
                "The dependency audit could not inspect the managed Redist folders.",
                true,
            )
        })
}

#[tauri::command]
pub fn open_official_dependency_source(url: String) -> Result<(), CommandError> {
    if !dependencies::is_approved_official_url(&url) {
        return Err(CommandError::new(
            "dependencies.source_not_allowed",
            "GameVault opens only approved official dependency sources.",
            false,
        ));
    }
    Command::new("explorer.exe").arg(url).spawn().map_err(|_| {
        CommandError::new(
            "platform.browser_failed",
            "Windows could not open the official dependency source.",
            true,
        )
    })?;
    Ok(())
}

#[tauri::command]
pub async fn choose_game_archive(
    state: State<'_, Arc<AppCore>>,
) -> Result<Option<String>, CommandError> {
    let inbox = {
        let connection = state.database.lock().map_err(|_| lock_error())?;
        let settings = storage::get_settings(&connection)
            .map_err(|_| CommandError::internal("storage.settings_read_failed"))?;
        Path::new(&settings.managed_root).join("Inbox")
    };
    let mut dialog = rfd::AsyncFileDialog::new()
        .set_title("Choose a game ZIP for safe inspection")
        .add_filter("ZIP archive", &["zip"]);
    if inbox.is_dir() {
        dialog = dialog.set_directory(inbox);
    }
    Ok(dialog
        .pick_file()
        .await
        .map(|handle| handle.path().to_string_lossy().to_string()))
}

#[tauri::command]
pub async fn inspect_game_archive(archive_path: String) -> Result<ArchiveInspection, CommandError> {
    tauri::async_runtime::spawn_blocking(move || archives::inspect(Path::new(&archive_path)))
        .await
        .map_err(|_| CommandError::internal("archives.worker_failed"))?
        .map_err(|_| {
            CommandError::new(
                "archives.inspect_failed",
                "GameVault could not test this ZIP. Confirm that 7-Zip is installed and the file is readable.",
                true,
            )
        })
}

#[tauri::command]
pub async fn stage_game_archive(
    archive_path: String,
    state: State<'_, Arc<AppCore>>,
) -> Result<StagedArchive, CommandError> {
    let managed_root = {
        let connection = state.database.lock().map_err(|_| lock_error())?;
        storage::get_settings(&connection)
            .map_err(|_| CommandError::internal("storage.settings_read_failed"))?
            .managed_root
    };
    tauri::async_runtime::spawn_blocking(move || {
        archives::stage(Path::new(&archive_path), Path::new(&managed_root))
    })
    .await
    .map_err(|_| CommandError::internal("archives.worker_failed"))?
    .map_err(|_| {
        CommandError::new(
            "archives.stage_failed",
            "The ZIP could not be extracted safely into Staging. No game files were launched.",
            true,
        )
    })
}

#[tauri::command]
pub fn list_inbox_archives(
    state: State<'_, Arc<AppCore>>,
) -> Result<Vec<InboxArchive>, CommandError> {
    let connection = state.database.lock().map_err(|_| lock_error())?;
    let settings = storage::get_settings(&connection)
        .map_err(|_| CommandError::internal("storage.settings_read_failed"))?;
    archives::list_inbox(Path::new(&settings.managed_root)).map_err(|_| {
        CommandError::new(
            "archives.inbox_read_failed",
            "GameVault could not read ZIP archives from Inbox.",
            true,
        )
    })
}

#[tauri::command]
pub async fn analyze_staged_package(
    staging_path: String,
    state: State<'_, Arc<AppCore>>,
) -> Result<StagedPackageAnalysis, CommandError> {
    let managed_root = {
        let connection = state.database.lock().map_err(|_| lock_error())?;
        storage::get_settings(&connection)
            .map_err(|_| CommandError::internal("storage.settings_read_failed"))?
            .managed_root
    };
    tauri::async_runtime::spawn_blocking(move || {
        archives::analyze_staged(Path::new(&staging_path), Path::new(&managed_root))
    })
    .await
    .map_err(|_| CommandError::internal("archives.worker_failed"))?
    .map_err(|_| {
        CommandError::new(
            "archives.analysis_failed",
            "The staged package could not be analyzed safely.",
            false,
        )
    })
}

#[tauri::command]
pub async fn install_staged_package(
    input: InstallStagedInput,
    app: AppHandle,
    state: State<'_, Arc<AppCore>>,
) -> Result<InstalledPackage, CommandError> {
    let managed_root = {
        let connection = state.database.lock().map_err(|_| lock_error())?;
        storage::get_settings(&connection)
            .map_err(|_| CommandError::internal("storage.settings_read_failed"))?
            .managed_root
    };
    let input_for_worker = input.clone();
    let root_for_worker = managed_root.clone();
    let promotion = tauri::async_runtime::spawn_blocking(move || {
        archives::promote_staged(&input_for_worker, Path::new(&root_for_worker))
    })
    .await
    .map_err(|_| CommandError::internal("archives.worker_failed"))?
    .map_err(|_| {
        CommandError::new(
            "archives.install_failed",
            "The package could not be organized into Games. Existing game files were preserved or restored.",
            false,
        )
    })?;
    let title = promotion
        .installed_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(input.title.trim());
    let game = {
        let connection = state.database.lock().map_err(|_| lock_error())?;
        match storage::register_imported_game(
            &connection,
            title,
            &promotion.installed_path,
            &promotion.executable_path,
        ) {
            Ok(game) => game,
            Err(_) => {
                return match archives::rollback_promotion(&promotion) {
                    Ok(()) => Err(CommandError::internal(
                        "archives.library_registration_failed",
                    )),
                    Err(_) => Err(CommandError::new(
                        "archives.rollback_incomplete",
                        "The library registration failed and rollback was incomplete. Review the managed folders and Diagnostics before retrying.",
                        false,
                    )),
                };
            }
        }
    };
    let _ = app.emit("library-changed", ());
    record_event(state.inner(), "archive.install", "completed");
    Ok(InstalledPackage {
        game,
        installed_path: promotion.installed_path.to_string_lossy().to_string(),
        backup_path: promotion
            .backup_path
            .map(|path| path.to_string_lossy().to_string()),
        dependencies_path: promotion
            .dependencies_path
            .map(|path| path.to_string_lossy().to_string()),
        extras_path: promotion
            .extras_path
            .map(|path| path.to_string_lossy().to_string()),
        archived_package_path: promotion
            .archived_package_path
            .map(|path| path.to_string_lossy().to_string()),
        updated: promotion.updated,
        warnings: promotion.warnings,
        report_path: promotion.report_path.to_string_lossy().to_string(),
    })
}

#[tauri::command]
pub async fn lookup_game_metadata(
    input: MetadataLookupInput,
) -> Result<GameMetadata, CommandError> {
    tauri::async_runtime::spawn_blocking(move || metadata::lookup(&input))
        .await
        .map_err(|_| CommandError::internal("metadata.worker_failed"))?
        .map_err(|error| CommandError::new("metadata.lookup_failed", &error, true))
}

#[tauri::command]
pub fn save_game_metadata(
    input: SaveGameMetadataInput,
    app: AppHandle,
    state: State<'_, Arc<AppCore>>,
) -> Result<Game, CommandError> {
    metadata::validate_metadata(&input.metadata)
        .map_err(|error| CommandError::new("metadata.source_not_allowed", &error, false))?;
    let connection = state.database.lock().map_err(|_| lock_error())?;
    let game = storage::save_game_metadata(&connection, &input.game_id, &input.metadata)
        .map_err(|_| CommandError::internal("metadata.save_failed"))?;
    let _ = app.emit("library-changed", ());
    Ok(game)
}

#[tauri::command]
pub fn open_official_store_search(provider: String, query: String) -> Result<(), CommandError> {
    let url = metadata::official_search_url(&provider, &query)
        .map_err(|error| CommandError::new("metadata.provider_invalid", &error, false))?;
    Command::new("explorer.exe").arg(url).spawn().map_err(|_| {
        CommandError::new(
            "platform.browser_failed",
            "Windows could not open the official store search.",
            true,
        )
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn launch_arguments_preserve_array_boundaries_and_enforce_limits() {
        assert!(validate_arguments(&[
            "--window title=Game Vault".to_string(),
            "& whoami".to_string(),
        ])
        .is_ok());
        assert!(validate_arguments(&vec!["argument".to_string(); 65]).is_err());
        assert!(validate_arguments(&["bad\0argument".to_string()]).is_err());
        assert!(validate_arguments(&["x".repeat(2049)]).is_err());
    }

    #[test]
    fn executable_must_be_a_windows_binary_inside_a_real_game_folder() {
        let directory = tempfile::tempdir().expect("temp directory");
        let game = directory.path().join("Game with spaces");
        let outside = directory.path().join("Outside");
        fs::create_dir_all(&game).expect("game folder");
        fs::create_dir_all(&outside).expect("outside folder");
        let executable = game.join("Game.exe");
        let text = game.join("Game.txt");
        let outside_executable = outside.join("Outside.exe");
        fs::write(&executable, b"fixture").expect("game executable");
        fs::write(&text, b"fixture").expect("text file");
        fs::write(&outside_executable, b"fixture").expect("outside executable");

        assert!(validate_executable(&executable, &game).is_ok());
        assert!(validate_executable(&text, &game).is_err());
        assert!(validate_executable(&outside_executable, &game).is_err());
        assert!(validate_executable(&executable, &executable).is_err());
    }
}
