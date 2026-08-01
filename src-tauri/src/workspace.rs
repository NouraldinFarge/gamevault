use crate::models::{WorkspaceFolder, WorkspaceStatus};
use std::fs;
use std::path::{Path, PathBuf};

pub const MANAGED_FOLDERS: &[&str] = &[
    "App",
    "Inbox",
    "Staging",
    "Games",
    "Archives",
    "Dependencies",
    "Quarantine",
    "Reports",
];

pub fn validate_managed_root(root: &Path) -> Result<PathBuf, String> {
    if !root.is_absolute() || root.parent().is_none() || root.components().count() < 2 {
        return Err("Choose a dedicated folder rather than a drive root.".to_string());
    }
    Ok(root.to_path_buf())
}

pub fn prepare(root: &Path) -> Result<WorkspaceStatus, String> {
    let root = validate_managed_root(root)?;
    fs::create_dir_all(&root).map_err(|error| error.to_string())?;
    for name in MANAGED_FOLDERS {
        fs::create_dir_all(root.join(name)).map_err(|error| error.to_string())?;
    }
    status(&root)
}

pub fn status(root: &Path) -> Result<WorkspaceStatus, String> {
    let root = validate_managed_root(root)?;
    let folders = MANAGED_FOLDERS
        .iter()
        .map(|name| {
            let path = root.join(name);
            let exists = path.is_dir();
            let item_count = if exists {
                fs::read_dir(&path)
                    .map(|entries| entries.filter_map(Result::ok).count())
                    .unwrap_or_default()
            } else {
                0
            };
            WorkspaceFolder {
                name: (*name).to_string(),
                path: path.to_string_lossy().to_string(),
                exists,
                item_count,
            }
        })
        .collect::<Vec<_>>();
    Ok(WorkspaceStatus {
        root: root.to_string_lossy().to_string(),
        ready: root.is_dir() && folders.iter().all(|folder| folder.exists),
        folders,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepare_creates_only_the_managed_layout() {
        let directory = tempfile::tempdir().expect("temp directory");
        let root = directory.path().join("GameVault");
        let result = prepare(&root).expect("workspace");
        assert!(result.ready);
        assert_eq!(result.folders.len(), MANAGED_FOLDERS.len());
        assert!(result.folders.iter().all(|folder| folder.exists));
    }

    #[test]
    fn drive_or_filesystem_roots_are_rejected() {
        #[cfg(windows)]
        assert!(validate_managed_root(Path::new(r"E:\")).is_err());
        #[cfg(not(windows))]
        assert!(validate_managed_root(Path::new("/")).is_err());
    }
}
