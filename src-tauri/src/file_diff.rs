use crate::models::StagedUpdatePreview;
use crate::path_safety;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use walkdir::WalkDir;

const MAX_MANIFEST_ENTRIES: usize = 50_000;
const SAMPLE_LIMIT: usize = 24;

#[derive(Debug)]
struct ManifestEntry {
    path: String,
    size: u64,
    sha256: String,
}

pub fn compare(
    current_root: Option<&Path>,
    proposed_root: &Path,
    destination: &Path,
    rollback_root: &Path,
) -> Result<StagedUpdatePreview, String> {
    let proposed = manifest(proposed_root)?;
    let current = match current_root {
        Some(root) => manifest(root)?,
        None => BTreeMap::new(),
    };

    let mut added = Vec::new();
    let mut changed = Vec::new();
    let mut removed = Vec::new();
    let mut unchanged_count = 0;
    for (key, proposed_entry) in &proposed {
        match current.get(key) {
            None => added.push(proposed_entry.path.clone()),
            Some(current_entry)
                if current_entry.size != proposed_entry.size
                    || current_entry.sha256 != proposed_entry.sha256 =>
            {
                changed.push(proposed_entry.path.clone());
            }
            Some(_) => unchanged_count += 1,
        }
    }
    for (key, current_entry) in &current {
        if !proposed.contains_key(key) {
            removed.push(current_entry.path.clone());
        }
    }

    added.sort();
    changed.sort();
    removed.sort();
    let fingerprint = manifest_fingerprint(&current, &proposed, destination);
    Ok(StagedUpdatePreview {
        is_update: current_root.is_some(),
        destination_path: destination.to_string_lossy().to_string(),
        rollback_root: rollback_root.to_string_lossy().to_string(),
        added_count: added.len(),
        changed_count: changed.len(),
        removed_count: removed.len(),
        unchanged_count,
        added_sample: added.into_iter().take(SAMPLE_LIMIT).collect(),
        changed_sample: changed.into_iter().take(SAMPLE_LIMIT).collect(),
        removed_sample: removed.into_iter().take(SAMPLE_LIMIT).collect(),
        current_size_bytes: current.values().map(|entry| entry.size).sum(),
        proposed_size_bytes: proposed.values().map(|entry| entry.size).sum(),
        fingerprint,
    })
}

fn manifest(root: &Path) -> Result<BTreeMap<String, ManifestEntry>, String> {
    if !root.is_dir() || path_safety::is_link_or_reparse(root) {
        return Err("The file-plan root is unavailable or is a link/reparse point.".to_string());
    }
    let canonical_root = root
        .canonicalize()
        .map_err(|_| "The file-plan root could not be resolved.".to_string())?;
    let mut files = BTreeMap::new();
    for (index, item) in WalkDir::new(&canonical_root)
        .follow_links(false)
        .into_iter()
        .enumerate()
    {
        if index > MAX_MANIFEST_ENTRIES {
            return Err(format!(
                "The file plan exceeds the {MAX_MANIFEST_ENTRIES}-entry review limit."
            ));
        }
        let entry =
            item.map_err(|error| format!("A file-plan entry could not be read: {error}"))?;
        if entry.depth() == 0 {
            continue;
        }
        if path_safety::is_link_or_reparse(entry.path()) {
            return Err("The file plan contains a link or Windows reparse point.".to_string());
        }
        if !entry.file_type().is_file() {
            continue;
        }
        let relative = entry
            .path()
            .strip_prefix(&canonical_root)
            .map_err(|_| "A file-plan entry escaped its root.".to_string())?;
        let display = portable_relative_path(relative)?;
        let key = normalized_key(&display);
        let metadata = entry
            .metadata()
            .map_err(|error| format!("File metadata could not be read: {error}"))?;
        let value = ManifestEntry {
            path: display,
            size: metadata.len(),
            sha256: hash_file(entry.path())?,
        };
        if files.insert(key, value).is_some() {
            return Err(
                "The file plan contains paths that collide on Windows case-insensitive storage."
                    .to_string(),
            );
        }
    }
    Ok(files)
}

fn portable_relative_path(path: &Path) -> Result<String, String> {
    let text = path.to_str().ok_or_else(|| {
        "The file plan contains a path that cannot be represented safely.".to_string()
    })?;
    Ok(text.replace('\\', "/"))
}

fn normalized_key(path: &str) -> String {
    path.replace('\\', "/")
        .split('/')
        .filter(|component| !component.is_empty())
        .collect::<Vec<_>>()
        .join("/")
        .to_lowercase()
}

fn hash_file(path: &Path) -> Result<String, String> {
    let mut file =
        File::open(path).map_err(|error| format!("A file could not be hashed: {error}"))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 128 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|error| format!("A file could not be hashed: {error}"))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn manifest_fingerprint(
    current: &BTreeMap<String, ManifestEntry>,
    proposed: &BTreeMap<String, ManifestEntry>,
    destination: &Path,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"gamevault-update-preview-v1\0");
    hasher.update(destination.to_string_lossy().to_lowercase().as_bytes());
    for (label, manifest) in [
        (b"current\0".as_slice(), current),
        (b"proposed\0".as_slice(), proposed),
    ] {
        hasher.update(label);
        for (key, entry) in manifest {
            hasher.update(key.as_bytes());
            hasher.update(b"\0");
            hasher.update(entry.size.to_le_bytes());
            hasher.update(entry.sha256.as_bytes());
        }
    }
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn preview_reports_added_changed_removed_and_unchanged_files() {
        let directory = tempfile::tempdir().expect("temp directory");
        let current = directory.path().join("current");
        let proposed = directory.path().join("proposed");
        fs::create_dir_all(current.join("bin")).expect("current");
        fs::create_dir_all(proposed.join("bin")).expect("proposed");
        fs::write(current.join("same.dat"), b"same").expect("same current");
        fs::write(proposed.join("same.dat"), b"same").expect("same proposed");
        fs::write(current.join("bin").join("game.exe"), b"old").expect("old");
        fs::write(proposed.join("bin").join("game.exe"), b"new").expect("new");
        fs::write(current.join("removed.ini"), b"old setting").expect("removed");
        fs::write(proposed.join("added.ini"), b"new setting").expect("added");

        let result = compare(
            Some(&current),
            &proposed,
            &directory.path().join("Games").join("Example"),
            &directory.path().join("Archives").join("Updates"),
        )
        .expect("preview");

        assert!(result.is_update);
        assert_eq!(result.added_count, 1);
        assert_eq!(result.changed_count, 1);
        assert_eq!(result.removed_count, 1);
        assert_eq!(result.unchanged_count, 1);
        assert_eq!(result.added_sample, vec!["added.ini"]);
        assert_eq!(result.changed_sample, vec!["bin/game.exe"]);
        assert_eq!(result.removed_sample, vec!["removed.ini"]);
        assert_eq!(result.fingerprint.len(), 64);
    }

    #[test]
    fn new_install_lists_every_proposed_file_as_added() {
        let directory = tempfile::tempdir().expect("temp directory");
        let proposed = directory.path().join("proposed");
        fs::create_dir_all(&proposed).expect("proposed");
        fs::write(proposed.join("game.exe"), b"fixture").expect("file");

        let result = compare(
            None,
            &proposed,
            &directory.path().join("Games").join("Example"),
            &directory.path().join("Archives").join("Updates"),
        )
        .expect("preview");

        assert!(!result.is_update);
        assert_eq!(result.added_count, 1);
        assert_eq!(result.current_size_bytes, 0);
    }

    #[test]
    fn normalized_keys_match_windows_path_rules() {
        assert_eq!(
            normalized_key(r"Bin\\GAME.EXE"),
            normalized_key("bin/game.exe")
        );
    }
}
