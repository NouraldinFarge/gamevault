use std::fs;
use std::path::{Path, PathBuf};

#[cfg(windows)]
use std::os::windows::fs::MetadataExt;

#[cfg(windows)]
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;

pub fn is_link_or_reparse(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|metadata| metadata_is_link_or_reparse(&metadata))
        .unwrap_or(true)
}

pub fn metadata_is_link_or_reparse(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    false
}

pub fn ensure_managed_directory(root: &Path, components: &[&str]) -> Result<PathBuf, String> {
    fs::create_dir_all(root).map_err(|error| error.to_string())?;
    if is_link_or_reparse(root) {
        return Err("The managed root must not be a link or Windows reparse point.".to_string());
    }
    let mut current = root.canonicalize().map_err(|error| error.to_string())?;
    for component in components {
        if component.is_empty()
            || component
                .chars()
                .any(|value| matches!(value, '/' | '\\' | ':'))
            || Path::new(component).components().count() != 1
            || matches!(*component, "." | "..")
        {
            return Err("A managed folder component is invalid.".to_string());
        }
        let child = current.join(component);
        if child.exists() && is_link_or_reparse(&child) {
            return Err(format!(
                "The managed folder '{component}' must not be a link or Windows reparse point."
            ));
        }
        fs::create_dir(&child)
            .or_else(|error| {
                if error.kind() == std::io::ErrorKind::AlreadyExists {
                    Ok(())
                } else {
                    Err(error)
                }
            })
            .map_err(|error| error.to_string())?;
        let canonical = child.canonicalize().map_err(|error| error.to_string())?;
        if !canonical.is_dir()
            || canonical.parent() != Some(current.as_path())
            || is_link_or_reparse(&child)
        {
            return Err(format!(
                "The managed folder '{component}' escaped its approved parent."
            ));
        }
        current = canonical;
    }
    Ok(current)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn managed_directory_is_created_under_the_canonical_root() {
        let directory = tempfile::tempdir().expect("temp directory");
        let root = directory.path().join("Managed root");
        let reports =
            ensure_managed_directory(&root, &["Reports", "Archive"]).expect("managed directory");
        assert_eq!(
            reports,
            root.join("Reports")
                .join("Archive")
                .canonicalize()
                .expect("canonical path")
        );
    }

    #[test]
    fn managed_directory_rejects_parent_components() {
        let directory = tempfile::tempdir().expect("temp directory");
        assert!(ensure_managed_directory(directory.path(), &[".."]).is_err());
        assert!(ensure_managed_directory(directory.path(), &["nested\\escape"]).is_err());
    }
}
