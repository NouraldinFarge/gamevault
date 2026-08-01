mod app;
mod archives;
mod commands;
mod dependencies;
mod metadata;
mod models;
mod scanner;
mod storage;
mod workspace;

use app::{resolve_portable_root, AppCore};
use std::sync::Arc;

pub fn run() -> Result<(), String> {
    let root = resolve_portable_root()?;
    let state = Arc::new(AppCore::new(root)?);
    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::get_snapshot,
            commands::scan_library,
            commands::choose_library_directory,
            commands::choose_game_executable,
            commands::choose_backup_file,
            commands::save_settings,
            commands::add_manual_game,
            commands::update_game,
            commands::toggle_favorite,
            commands::remove_game,
            commands::launch_game,
            commands::open_game_folder,
            commands::open_logs_folder,
            commands::create_database_backup,
            commands::restore_database_backup,
            commands::clear_application_cache,
            commands::get_health_report,
            commands::get_workspace_status,
            commands::prepare_workspace,
            commands::audit_dependencies,
            commands::open_official_dependency_source,
            commands::choose_game_archive,
            commands::inspect_game_archive,
            commands::stage_game_archive,
            commands::list_inbox_archives,
            commands::analyze_staged_package,
            commands::install_staged_package,
            commands::lookup_game_metadata,
            commands::save_game_metadata,
            commands::open_official_store_search
        ])
        .run(tauri::generate_context!())
        .map_err(|error| error.to_string())
}

pub fn health_check() -> Result<(), String> {
    if tauri::is_dev() {
        return Err("The executable was built in development mode.".to_string());
    }

    let context: tauri::Context<tauri::Wry> = tauri::generate_context!();
    let index_key = tauri::utils::assets::AssetKey::from("index.html");
    let index = context
        .assets()
        .get(&index_key)
        .ok_or_else(|| "The production interface is not embedded.".to_string())?;
    let index_html = std::str::from_utf8(index.as_ref())
        .map_err(|_| "The embedded interface is invalid.".to_string())?;
    if !index_html.contains("id=\"root\"") {
        return Err("The embedded interface entry point is invalid.".to_string());
    }

    let root = resolve_portable_root()?;
    let core = AppCore::new(root)?;
    let connection = core.database.lock().map_err(|_| "Database lock failed.")?;
    let _: i64 = connection
        .query_row("SELECT count(*) FROM games", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    Ok(())
}
