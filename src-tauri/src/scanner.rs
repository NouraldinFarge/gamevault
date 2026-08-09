use crate::models::ScanProgress;
use crate::path_safety;
use crate::storage::DetectedGame;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use walkdir::{DirEntry, WalkDir};

const IGNORED_EXECUTABLE_MARKERS: &[&str] = &[
    "unins",
    "uninstall",
    "setup",
    "installer",
    "crashreport",
    "crash_report",
    "reporter",
    "vc_redist",
    "vcredist",
    "dxsetup",
    "directx",
    "redistributable",
    "configtool",
    "configuration",
    "benchmark",
    "dedicatedserver",
    "servertool",
    "editor",
];

const IGNORED_DIRECTORY_MARKERS: &[&str] = &[
    "_commonredist",
    "redist",
    "redistributable",
    "directx",
    "support",
    "installer",
    "prerequisites",
    "__installer",
];

const MANAGED_WORKSPACE_FOLDERS: &[&str] = &[
    "inbox",
    "staging",
    "archives",
    "dependencies",
    "quarantine",
    "reports",
];

#[derive(Debug)]
struct Candidate {
    path: PathBuf,
    score: i32,
    size: u64,
}

pub struct Discovery {
    pub games: Vec<DetectedGame>,
    pub available_roots: Vec<String>,
    pub unavailable_roots: Vec<String>,
    pub folders_scanned: usize,
}

pub fn discover_games<F>(roots: &[String], max_depth: u32, mut on_progress: F) -> Discovery
where
    F: FnMut(ScanProgress),
{
    let mut games = Vec::new();
    let mut available_roots = Vec::new();
    let mut unavailable_roots = Vec::new();
    let mut folders_scanned = 0;

    for root_text in roots {
        let root = PathBuf::from(root_text);
        if !root.is_dir() {
            unavailable_roots.push(root_text.clone());
            on_progress(ScanProgress {
                root: root_text.clone(),
                current_folder: String::new(),
                folders_scanned,
                folders_total: 0,
                games_detected: games.len(),
                message: "Library root is unavailable.".to_string(),
            });
            continue;
        }
        available_roots.push(root_text.clone());

        let mut folders = match fs::read_dir(&root) {
            Ok(entries) => entries
                .filter_map(Result::ok)
                .filter(|entry| !path_safety::is_link_or_reparse(&entry.path()))
                .filter(|entry| entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false))
                .filter(|entry| {
                    let name = entry.file_name().to_string_lossy().to_lowercase();
                    !MANAGED_WORKSPACE_FOLDERS.contains(&name.as_str())
                })
                .map(|entry| entry.path())
                .collect::<Vec<_>>(),
            Err(_) => {
                unavailable_roots.push(root_text.clone());
                continue;
            }
        };
        folders.sort_by_key(|path| path.to_string_lossy().to_lowercase());
        let total = folders.len();

        for folder in folders {
            folders_scanned += 1;
            if let Some(game) = inspect_game_folder(&folder, max_depth) {
                games.push(game);
            }
            on_progress(ScanProgress {
                root: root_text.clone(),
                current_folder: folder
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("Unreadable folder")
                    .to_string(),
                folders_scanned,
                folders_total: total,
                games_detected: games.len(),
                message: "Scanning local game folders...".to_string(),
            });
        }
    }

    Discovery {
        games,
        available_roots,
        unavailable_roots,
        folders_scanned,
    }
}

pub fn inspect_game_folder(folder: &Path, max_depth: u32) -> Option<DetectedGame> {
    if !folder.is_dir() {
        return None;
    }
    let folder_name = folder.file_name()?.to_string_lossy().to_string();
    let mut candidates = Vec::new();
    let mut observed_size = 0_u64;
    let depth = usize::try_from(max_depth.clamp(1, 8)).unwrap_or(4);

    let walker = WalkDir::new(folder)
        .max_depth(depth)
        .follow_links(false)
        .into_iter()
        .filter_entry(is_allowed_entry);

    for entry in walker.filter_map(Result::ok).take(50_000) {
        if !entry.file_type().is_file() {
            continue;
        }
        let metadata = match entry.metadata() {
            Ok(value) => value,
            Err(_) => continue,
        };
        observed_size = observed_size.saturating_add(metadata.len());
        if !is_executable(entry.path()) {
            continue;
        }
        let score = score_executable(entry.path(), folder, metadata.len());
        if score > 0 {
            candidates.push(Candidate {
                path: entry.path().to_path_buf(),
                score,
                size: metadata.len(),
            });
        }
    }

    candidates.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| right.size.cmp(&left.size))
            .then_with(|| left.path.cmp(&right.path))
    });
    let selected = candidates.first()?;
    let signature = content_signature(&candidates);
    let modified = fs::metadata(folder)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .and_then(|duration| i64::try_from(duration.as_secs()).ok())
        .unwrap_or_default();

    Some(DetectedGame {
        title: display_title(&folder_name),
        install_path: folder.to_string_lossy().to_string(),
        executable_path: selected.path.to_string_lossy().to_string(),
        folder_size_bytes: Some(observed_size),
        folder_modified_at: modified,
        content_signature: signature,
    })
}

pub fn score_executable(path: &Path, game_folder: &Path, file_size: u64) -> i32 {
    if !is_executable(path) {
        return -10_000;
    }
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_lowercase();
    if IGNORED_EXECUTABLE_MARKERS
        .iter()
        .any(|marker| stem.contains(marker))
    {
        return -10_000;
    }

    let folder_name = game_folder
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let normalized_stem = normalize_name(&stem);
    let normalized_folder = normalize_name(folder_name);
    let relative_depth = path
        .strip_prefix(game_folder)
        .ok()
        .map(|relative| relative.components().count())
        .unwrap_or(8);

    let mut score = 20;
    if normalized_stem == normalized_folder {
        score += 140;
    } else if normalized_stem.contains(&normalized_folder)
        || normalized_folder.contains(&normalized_stem)
    {
        score += 75;
    }
    if stem.contains("shipping") || stem.ends_with("win64") || stem.ends_with("win32") {
        score += 25;
    }
    if stem.contains("launcher") {
        score -= 18;
    }
    score += match relative_depth {
        1 => 45,
        2 => 30,
        3 => 15,
        _ => 0,
    };
    score += match file_size {
        value if value >= 100 * 1024 * 1024 => 35,
        value if value >= 10 * 1024 * 1024 => 25,
        value if value >= 1024 * 1024 => 10,
        value if value < 128 * 1024 => -15,
        _ => 0,
    };
    score
}

fn is_allowed_entry(entry: &DirEntry) -> bool {
    if entry.depth() == 0 {
        return true;
    }
    if path_safety::is_link_or_reparse(entry.path()) {
        return false;
    }
    if entry.file_type().is_dir() {
        let name = entry.file_name().to_string_lossy().to_lowercase();
        return !IGNORED_DIRECTORY_MARKERS
            .iter()
            .any(|marker| name.contains(marker));
    }
    true
}

fn is_executable(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|extension| extension.eq_ignore_ascii_case("exe"))
        .unwrap_or(false)
}

fn normalize_name(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn display_title(value: &str) -> String {
    let mut words = value
        .replace(['_', '-'], " ")
        .split_whitespace()
        .map(str::to_string)
        .collect::<Vec<_>>();
    while words.last().is_some_and(|word| {
        matches!(
            word.to_ascii_lowercase().as_str(),
            "ankergames" | "steamripped" | "repack"
        )
    }) {
        words.pop();
    }
    let spaced = words.join(" ");
    if spaced.chars().any(char::is_uppercase) {
        return spaced;
    }
    spaced
        .split_whitespace()
        .map(|word| {
            let mut characters = word.chars();
            match characters.next() {
                Some(first) => first.to_uppercase().collect::<String>() + characters.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn content_signature(candidates: &[Candidate]) -> String {
    let mut signatures = candidates
        .iter()
        .map(|candidate| {
            format!(
                "{}:{}",
                candidate
                    .path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or_default()
                    .to_lowercase(),
                candidate.size
            )
        })
        .collect::<Vec<_>>();
    signatures.sort();
    let mut hasher = Sha256::new();
    for signature in signatures {
        hasher.update(signature.as_bytes());
        hasher.update([0]);
    }
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn executable_scoring_prefers_game_binary() {
        let root = PathBuf::from(r"C:\Games\Nova Drift");
        let game = root.join("NovaDrift.exe");
        let setup = root.join("setup.exe");
        let crash = root.join("CrashReporter.exe");
        assert!(score_executable(&game, &root, 12 * 1024 * 1024) > 150);
        assert!(score_executable(&setup, &root, 50 * 1024 * 1024) < 0);
        assert!(score_executable(&crash, &root, 50 * 1024 * 1024) < 0);
    }

    #[test]
    fn inspection_ignores_installers_and_finds_fixture_game() {
        let directory = tempfile::tempdir().expect("temp directory");
        let game_dir = directory.path().join("fixture_game");
        fs::create_dir_all(game_dir.join("_CommonRedist")).expect("redist");
        fs::write(game_dir.join("fixture_game.exe"), vec![0_u8; 1024 * 1024])
            .expect("game fixture");
        fs::write(
            game_dir.join("_CommonRedist").join("setup.exe"),
            vec![0_u8; 10],
        )
        .expect("setup fixture");
        let detected = inspect_game_folder(&game_dir, 4).expect("detected game");
        assert!(detected.executable_path.ends_with("fixture_game.exe"));
        assert_eq!(detected.title, "Fixture Game");
    }

    #[test]
    fn display_title_removes_package_source_suffixes() {
        assert_eq!(
            display_title("Orb-Of-Creation-AnkerGames"),
            "Orb Of Creation"
        );
        assert_eq!(display_title("slime_rancher"), "Slime Rancher");
    }
}
