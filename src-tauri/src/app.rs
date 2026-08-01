use crate::storage;
use rusqlite::Connection;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;

pub struct AppCore {
    pub database: Mutex<Connection>,
    pub database_path: PathBuf,
    pub portable_root: PathBuf,
    pub running_games: Mutex<HashSet<String>>,
    pub scan_in_progress: AtomicBool,
}

impl AppCore {
    pub fn new(portable_root: PathBuf) -> Result<Self, String> {
        ensure_portable_layout(&portable_root)?;
        let database_path = portable_root.join("data").join("library.db");
        let database = storage::open_database(&database_path)?;
        Ok(Self {
            database: Mutex::new(database),
            database_path,
            portable_root,
            running_games: Mutex::new(HashSet::new()),
            scan_in_progress: AtomicBool::new(false),
        })
    }
}

pub fn resolve_portable_root() -> Result<PathBuf, String> {
    if let Ok(root) = std::env::var("GAMEVAULT_PORTABLE_ROOT") {
        let path = PathBuf::from(root);
        if path.as_os_str().is_empty() {
            return Err("Portable root override is empty.".to_string());
        }
        return Ok(path);
    }
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    executable
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "The application folder could not be resolved.".to_string())
}

pub fn ensure_portable_layout(root: &Path) -> Result<(), String> {
    for directory in [
        "assets", "config", "data", "logs", "cache", "runtime", "licenses",
    ] {
        fs::create_dir_all(root.join(directory)).map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub fn clear_cache(root: &Path) -> Result<(), String> {
    let cache = root.join("cache");
    if cache.parent() != Some(root)
        || cache.file_name().and_then(|value| value.to_str()) != Some("cache")
    {
        return Err("Cache path validation failed.".to_string());
    }
    fs::create_dir_all(&cache).map_err(|error| error.to_string())?;
    for entry in fs::read_dir(&cache).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        if path.parent() != Some(cache.as_path()) {
            return Err("Cache entry escaped the approved root.".to_string());
        }
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        if file_type.is_symlink() {
            fs::remove_file(&path).map_err(|error| error.to_string())?;
        } else if file_type.is_dir() {
            fs::remove_dir_all(&path).map_err(|error| error.to_string())?;
        } else {
            fs::remove_file(&path).map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}
