use crate::models::{
    ArchiveInspection, InboxArchive, InstallStagedInput, StagedArchive, StagedExecutableCandidate,
    StagedPackageAnalysis,
};
use crate::scanner;
use chrono::Utc;
use std::ffi::OsStr;
use std::fs;
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::time::UNIX_EPOCH;
use walkdir::WalkDir;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const MAX_ARCHIVE_ENTRIES: usize = 100_000;
const MAX_EXECUTABLE_CANDIDATES: usize = 20;

#[derive(Debug)]
pub struct PromotionResult {
    pub installed_path: PathBuf,
    pub executable_path: PathBuf,
    pub backup_path: Option<PathBuf>,
    pub dependencies_path: Option<PathBuf>,
    pub extras_path: Option<PathBuf>,
    pub archived_package_path: Option<PathBuf>,
    pub updated: bool,
    pub warnings: Vec<String>,
    pub report_path: PathBuf,
}

#[derive(Debug)]
struct ArchiveEntry {
    path: String,
    size: u64,
}

pub fn inspect(archive: &Path) -> Result<ArchiveInspection, String> {
    validate_archive(archive)?;
    let extractor = find_7zip()?;
    let test = seven_zip(&extractor, ["t", "-bd", "-y", "-bso0", "-bsp0"], archive)?;
    if !test.status.success() {
        return Ok(ArchiveInspection {
            archive_path: display_path(archive),
            archive_name: archive_name(archive),
            archive_size_bytes: fs::metadata(archive)
                .map(|metadata| metadata.len())
                .unwrap_or_default(),
            valid: false,
            extractor: extractor.to_string_lossy().to_string(),
            file_count: 0,
            unpacked_size_bytes: 0,
            executable_candidates: Vec::new(),
            warnings: vec!["7-Zip could not fully decompress and verify this archive.".to_string()],
            can_stage: false,
        });
    }

    let listing = seven_zip(&extractor, ["l", "-slt", "-ba"], archive)?;
    if !listing.status.success() {
        return Err("7-Zip verified the archive but could not list its contents.".to_string());
    }
    let entries = parse_listing(&String::from_utf8_lossy(&listing.stdout));
    if entries.len() > MAX_ARCHIVE_ENTRIES {
        return Err("The archive contains too many entries for safe staging.".to_string());
    }

    let unsafe_paths = entries
        .iter()
        .filter(|entry| unsafe_archive_path(&entry.path))
        .count();
    let nested_archives = entries
        .iter()
        .filter(|entry| has_extension(&entry.path, &["zip", "7z", "rar", "tar", "gz"]))
        .count();
    let package_extras = entries
        .iter()
        .filter(|entry| has_extension(&entry.path, &["url", "bat", "cmd"]))
        .count();
    let redist_files = entries
        .iter()
        .filter(|entry| path_has_redist_component(&entry.path))
        .count();
    let modified_platform_markers = entries
        .iter()
        .filter(|entry| has_modified_platform_marker(&entry.path))
        .count();
    let long_paths = entries
        .iter()
        .filter(|entry| entry.path.encode_utf16().count() > 180)
        .count();
    let executable_candidates = entries
        .iter()
        .filter(|entry| likely_game_executable(&entry.path))
        .take(MAX_EXECUTABLE_CANDIDATES)
        .map(|entry| entry.path.clone())
        .collect::<Vec<_>>();

    let mut warnings = Vec::new();
    if unsafe_paths > 0 {
        warnings.push(format!(
            "Blocked: {unsafe_paths} archive paths could escape the staging folder."
        ));
    }
    if nested_archives > 0 {
        warnings.push(format!(
            "Contains {nested_archives} nested archive(s); GameVault will not unpack them recursively."
        ));
    }
    if package_extras > 0 {
        warnings.push(format!(
            "Contains {package_extras} shortcut or script file(s) that should not enter Games."
        ));
    }
    if redist_files > 0 {
        warnings.push(format!(
            "Contains {redist_files} Redist file(s); audit them before installing anything."
        ));
    }
    if modified_platform_markers > 0 {
        warnings.push(format!(
            "Contains {modified_platform_markers} modified-platform marker(s); verify the game through its official store."
        ));
    }
    if long_paths > 0 {
        warnings.push(format!(
            "Contains {long_paths} long path(s); the short Staging location reduces extraction failures."
        ));
    }
    if executable_candidates.is_empty() {
        warnings.push("No likely primary game executable was found.".to_string());
    }

    Ok(ArchiveInspection {
        archive_path: display_path(archive),
        archive_name: archive_name(archive),
        archive_size_bytes: fs::metadata(archive)
            .map(|metadata| metadata.len())
            .unwrap_or_default(),
        valid: true,
        extractor: extractor.to_string_lossy().to_string(),
        file_count: entries.len(),
        unpacked_size_bytes: entries.iter().map(|entry| entry.size).sum(),
        executable_candidates,
        warnings,
        can_stage: unsafe_paths == 0,
    })
}

pub fn stage(archive: &Path, managed_root: &Path) -> Result<StagedArchive, String> {
    let inspection = inspect(archive)?;
    if !inspection.valid || !inspection.can_stage {
        return Err("This archive did not pass the safe staging checks.".to_string());
    }
    let staging_root = managed_root.join("Staging");
    fs::create_dir_all(&staging_root).map_err(|error| error.to_string())?;
    let staging_root = staging_root
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let destination = staging_root.join(format!(
        "{}-{}",
        clean_package_name(archive),
        Utc::now().format("%Y%m%d-%H%M%S")
    ));
    if destination.exists() {
        return Err("A staging folder with this name already exists.".to_string());
    }
    if destination.parent() != Some(staging_root.as_path()) {
        return Err("The staging destination escaped the managed root.".to_string());
    }
    fs::create_dir(&destination).map_err(|error| error.to_string())?;

    let extractor = find_7zip()?;
    let output_argument = format!("-o{}", destination.to_string_lossy());
    let extraction = Command::new(&extractor)
        .args(["x", "-bd", "-y", "-bso0", "-bsp0"])
        .arg(output_argument)
        .arg(archive)
        .creation_flags_if_windows(CREATE_NO_WINDOW)
        .output()
        .map_err(|error| error.to_string())?;
    if !extraction.status.success() {
        let _ = fs::remove_dir_all(&destination);
        return Err("7-Zip could not extract the archive into Staging.".to_string());
    }

    let files = WalkDir::new(&destination)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .take(MAX_ARCHIVE_ENTRIES)
        .collect::<Vec<_>>();
    let executable_candidates = files
        .iter()
        .filter_map(|entry| {
            let relative = entry.path().strip_prefix(&destination).ok()?;
            let text = relative.to_string_lossy();
            likely_game_executable(&text).then(|| display_path(entry.path()))
        })
        .take(MAX_EXECUTABLE_CANDIDATES)
        .collect::<Vec<_>>();
    let staged = StagedArchive {
        archive_path: display_path(archive),
        staging_path: display_path(&destination),
        files_extracted: files.len(),
        executable_candidates,
        warnings: inspection.warnings,
        report_path: String::new(),
    };
    write_staging_report(managed_root, staged)
}

pub fn list_inbox(managed_root: &Path) -> Result<Vec<InboxArchive>, String> {
    let inbox = managed_root.join("Inbox");
    fs::create_dir_all(&inbox).map_err(|error| error.to_string())?;
    let mut archives = fs::read_dir(&inbox)
        .map_err(|error| error.to_string())?
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_type()
                .map(|kind| kind.is_file())
                .unwrap_or(false)
        })
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.eq_ignore_ascii_case("zip"))
        })
        .filter_map(|entry| {
            let metadata = entry.metadata().ok()?;
            let modified_at = metadata
                .modified()
                .ok()
                .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
                .and_then(|value| {
                    chrono::DateTime::<Utc>::from_timestamp(value.as_secs() as i64, 0)
                })
                .map(|value| value.to_rfc3339());
            Some(InboxArchive {
                path: display_path(&entry.path()),
                name: entry.file_name().to_string_lossy().to_string(),
                size_bytes: metadata.len(),
                modified_at,
            })
        })
        .collect::<Vec<_>>();
    archives.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
    Ok(archives)
}

pub fn analyze_staged(
    staging_path: &Path,
    managed_root: &Path,
) -> Result<StagedPackageAnalysis, String> {
    let canonical_staging = validate_staging_path(staging_path, managed_root)?;
    let mut executable_candidates = Vec::new();
    let mut redist_folders = Vec::new();
    let mut package_extras = Vec::new();
    let mut nested_archives = Vec::new();
    let mut suspicious_markers = Vec::new();

    for entry in WalkDir::new(&canonical_staging)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .take(MAX_ARCHIVE_ENTRIES)
    {
        let path = entry.path();
        let relative = path
            .strip_prefix(&canonical_staging)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();
        if entry.file_type().is_symlink() {
            suspicious_markers.push(format!("{} (link/reparse point)", relative));
            continue;
        }
        if entry.file_type().is_dir()
            && is_redist_dir_name(entry.file_name().to_string_lossy().as_ref())
        {
            redist_folders.push(display_path(path));
            continue;
        }
        if !entry.file_type().is_file() {
            continue;
        }
        if has_modified_platform_marker(&relative) {
            suspicious_markers.push(relative.clone());
        }
        if has_extension(&relative, &["zip", "7z", "rar", "tar", "gz"]) {
            nested_archives.push(display_path(path));
        }
        if is_package_extra(path) {
            package_extras.push(display_path(path));
        }
        if likely_game_executable(&relative) {
            let install_root = candidate_install_root(path, &canonical_staging);
            let size = entry
                .metadata()
                .map(|value| value.len())
                .unwrap_or_default();
            let score = scanner::score_executable(path, &install_root, size);
            if score > 0 {
                executable_candidates.push(StagedExecutableCandidate {
                    executable_path: display_path(path),
                    install_root: display_path(&install_root),
                    display_name: display_title(&install_root),
                    score,
                });
            }
        }
    }

    executable_candidates.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.executable_path.cmp(&right.executable_path))
    });
    executable_candidates.truncate(MAX_EXECUTABLE_CANDIDATES);
    redist_folders = topmost_paths(redist_folders);
    package_extras.sort();
    package_extras.dedup();
    nested_archives.sort();
    nested_archives.dedup();
    suspicious_markers.sort();
    suspicious_markers.dedup();

    let suggested_title = executable_candidates
        .first()
        .map(|candidate| candidate.display_name.clone())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| display_title(&canonical_staging));
    let mut warnings = Vec::new();
    if !redist_folders.is_empty() {
        warnings.push(format!(
            "{} redistributable folder(s) will be separated for the dependency audit.",
            redist_folders.len()
        ));
    }
    if !package_extras.is_empty() {
        warnings.push(format!(
            "{} package shortcut/readme file(s) will be moved to Quarantine.",
            package_extras.len()
        ));
    }
    if !nested_archives.is_empty() {
        warnings.push(format!(
            "{} nested archive(s) will remain sealed and will not be unpacked.",
            nested_archives.len()
        ));
    }
    if !suspicious_markers.is_empty() {
        warnings.push(
            "Installation is blocked because modified-platform markers or links were detected. Verify this game with its official store."
                .to_string(),
        );
    }
    if executable_candidates.is_empty() {
        warnings.push("No safe primary game executable was found.".to_string());
    }
    let blocked = !suspicious_markers.is_empty();
    Ok(StagedPackageAnalysis {
        staging_path: display_path(&canonical_staging),
        suggested_title,
        executable_candidates,
        redist_folders,
        package_extras,
        nested_archives,
        suspicious_markers,
        blocked,
        can_install: !blocked && !warnings.iter().any(|value| value.starts_with("No safe")),
        warnings,
    })
}

pub fn promote_staged(
    input: &InstallStagedInput,
    managed_root: &Path,
) -> Result<PromotionResult, String> {
    let analysis = analyze_staged(Path::new(&input.staging_path), managed_root)?;
    if analysis.blocked || !analysis.can_install {
        return Err("This staged package did not pass the installation checks.".to_string());
    }
    let selected = analysis
        .executable_candidates
        .iter()
        .find(|candidate| {
            candidate
                .executable_path
                .eq_ignore_ascii_case(&input.executable_path)
        })
        .ok_or_else(|| "Choose one of the analyzed game executables.".to_string())?;
    let title = safe_folder_name(&input.title)?;
    let staging_path = PathBuf::from(&analysis.staging_path);
    let install_root = PathBuf::from(&selected.install_root)
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let selected_executable = PathBuf::from(&selected.executable_path)
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let executable_relative = selected_executable
        .strip_prefix(&install_root)
        .map_err(|_| "The selected executable is outside the selected game root.".to_string())?
        .to_path_buf();

    let games_root = managed_root.join("Games");
    let updates_root = managed_root.join("Archives").join("Updates");
    fs::create_dir_all(&games_root).map_err(|error| error.to_string())?;
    fs::create_dir_all(&updates_root).map_err(|error| error.to_string())?;
    let destination = games_root.join(&title);
    let stamp = Utc::now().format("%Y%m%d-%H%M%S").to_string();
    let mut backup_path = None;
    if destination.exists() {
        let backup = unique_destination(&updates_root.join(format!("{title}-{stamp}")));
        fs::rename(&destination, &backup).map_err(|error| {
            format!("The existing game could not be moved into the update backup: {error}")
        })?;
        backup_path = Some(backup);
    }
    if let Err(error) = fs::rename(&install_root, &destination) {
        if let Some(backup) = &backup_path {
            let _ = fs::rename(backup, &destination);
        }
        return Err(format!(
            "The staged game could not be promoted into Games: {error}"
        ));
    }

    let installed_executable = destination.join(executable_relative);
    let mut warnings = analysis.warnings.clone();
    let dependencies_path =
        clean_redist_folders(&destination, managed_root, &title, &stamp, &mut warnings);
    let extras_path = clean_package_extras(
        &destination,
        &staging_path,
        managed_root,
        &title,
        &stamp,
        &mut warnings,
    );
    let archived_package_path = archive_inbox_package(
        input.archive_path.as_deref().map(Path::new),
        managed_root,
        &title,
        &stamp,
        &mut warnings,
    );
    let report_path = write_install_report(
        managed_root,
        &title,
        &destination,
        &installed_executable,
        backup_path.as_deref(),
        dependencies_path.as_deref(),
        extras_path.as_deref(),
        archived_package_path.as_deref(),
        &warnings,
    )?;

    Ok(PromotionResult {
        installed_path: destination,
        executable_path: installed_executable,
        updated: backup_path.is_some(),
        backup_path,
        dependencies_path,
        extras_path,
        archived_package_path,
        warnings,
        report_path,
    })
}

pub fn rollback_promotion(result: &PromotionResult) {
    if result.installed_path.is_dir() {
        let _ = fs::remove_dir_all(&result.installed_path);
    } else if result.installed_path.is_file() {
        let _ = fs::remove_file(&result.installed_path);
    }
    if let Some(backup) = &result.backup_path {
        let _ = fs::rename(backup, &result.installed_path);
    }
}

fn validate_staging_path(staging_path: &Path, managed_root: &Path) -> Result<PathBuf, String> {
    let staging_root = managed_root.join("Staging");
    fs::create_dir_all(&staging_root).map_err(|error| error.to_string())?;
    let staging_root = staging_root
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let candidate = staging_path
        .canonicalize()
        .map_err(|_| "The selected staging folder is unavailable.".to_string())?;
    if !candidate.is_dir() || candidate.parent() != Some(staging_root.as_path()) {
        return Err("Choose a package created directly inside GameVault\\Staging.".to_string());
    }
    Ok(candidate)
}

fn candidate_install_root(executable: &Path, staging_root: &Path) -> PathBuf {
    let relative = executable.strip_prefix(staging_root).unwrap_or(executable);
    let components = relative.components().collect::<Vec<_>>();
    if let Some(index) = components.iter().position(|component| {
        matches!(
            component
                .as_os_str()
                .to_string_lossy()
                .to_ascii_lowercase()
                .as_str(),
            "bin" | "binaries" | "win64" | "win32" | "x64" | "x86"
        )
    }) {
        if index > 0 {
            let mut root = staging_root.to_path_buf();
            for component in &components[..index] {
                root.push(component.as_os_str());
            }
            return root;
        }
    }
    executable
        .parent()
        .filter(|path| path.starts_with(staging_root))
        .unwrap_or(staging_root)
        .to_path_buf()
}

fn display_title(path: &Path) -> String {
    let value = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("Local Game")
        .replace(['_', '-'], " ");
    let mut words = value
        .split_whitespace()
        .filter(|word| {
            !matches!(
                word.to_ascii_lowercase().as_str(),
                "ankergames" | "steamripped" | "repack"
            )
        })
        .map(str::to_string)
        .collect::<Vec<_>>();
    if words.is_empty() {
        words.push("Local Game".to_string());
    }
    words.join(" ")
}

fn safe_folder_name(value: &str) -> Result<String, String> {
    let cleaned = value
        .trim()
        .chars()
        .filter(|character| {
            !matches!(
                character,
                '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
            )
        })
        .collect::<String>()
        .trim_matches([' ', '.'])
        .chars()
        .take(80)
        .collect::<String>();
    if cleaned.is_empty()
        || matches!(
            cleaned.to_ascii_uppercase().as_str(),
            "CON" | "PRN" | "AUX" | "NUL"
        )
    {
        return Err(
            "Enter a simple game title that is valid as a Windows folder name.".to_string(),
        );
    }
    Ok(cleaned)
}

fn is_redist_dir_name(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains("redist")
        || matches!(
            lower.as_str(),
            "prerequisites" | "support" | "__installer" | "directx"
        )
}

fn is_package_extra(path: &Path) -> bool {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if ["url", "bat", "cmd"]
        .iter()
        .any(|value| extension.eq_ignore_ascii_case(value))
    {
        return true;
    }
    let lower = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    (lower.contains("readme") || lower.contains("website"))
        && (lower.contains("steamrip") || lower.contains("ankergames") || lower.contains("repack"))
}

fn topmost_paths(values: Vec<String>) -> Vec<String> {
    let mut paths = values.into_iter().map(PathBuf::from).collect::<Vec<_>>();
    paths.sort_by_key(|path| path.components().count());
    let mut selected: Vec<PathBuf> = Vec::new();
    for path in paths {
        if !selected.iter().any(|parent| path.starts_with(parent)) {
            selected.push(path);
        }
    }
    selected
        .into_iter()
        .map(|path| path.to_string_lossy().to_string())
        .collect()
}

fn unique_destination(path: &Path) -> PathBuf {
    if !path.exists() {
        return path.to_path_buf();
    }
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("item");
    let extension = path.extension().and_then(|value| value.to_str());
    for index in 2..10_000 {
        let name = match extension {
            Some(extension) => format!("{stem}-{index}.{extension}"),
            None => format!("{stem}-{index}"),
        };
        let candidate = parent.join(name);
        if !candidate.exists() {
            return candidate;
        }
    }
    parent.join(format!("{stem}-{}", uuid::Uuid::now_v7()))
}

fn clean_redist_folders(
    installed_root: &Path,
    managed_root: &Path,
    title: &str,
    stamp: &str,
    warnings: &mut Vec<String>,
) -> Option<PathBuf> {
    let paths = WalkDir::new(installed_root)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_dir() && entry.depth() > 0)
        .filter(|entry| is_redist_dir_name(entry.file_name().to_string_lossy().as_ref()))
        .map(|entry| entry.path().to_string_lossy().to_string())
        .collect::<Vec<_>>();
    let paths = topmost_paths(paths);
    if paths.is_empty() {
        return None;
    }
    let destination_root = managed_root
        .join("Dependencies")
        .join("Bundled")
        .join(title)
        .join(stamp);
    if let Err(error) = fs::create_dir_all(&destination_root) {
        warnings.push(format!("Redistributables could not be separated: {error}"));
        return None;
    }
    let mut moved = 0;
    for value in paths {
        let source = PathBuf::from(value);
        let name = source.file_name().unwrap_or_else(|| OsStr::new("Redist"));
        let target = unique_destination(&destination_root.join(name));
        match fs::rename(&source, &target) {
            Ok(()) => moved += 1,
            Err(error) => warnings.push(format!(
                "A redistributable folder stayed with the game because it could not be moved: {error}"
            )),
        }
    }
    (moved > 0).then_some(destination_root)
}

fn clean_package_extras(
    installed_root: &Path,
    staging_path: &Path,
    managed_root: &Path,
    title: &str,
    stamp: &str,
    warnings: &mut Vec<String>,
) -> Option<PathBuf> {
    let destination_root = managed_root
        .join("Quarantine")
        .join("Package Extras")
        .join(format!("{title}-{stamp}"));
    let mut moved = 0;
    let files = WalkDir::new(installed_root)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file() && is_package_extra(entry.path()))
        .map(|entry| entry.path().to_path_buf())
        .collect::<Vec<_>>();
    for source in files {
        let relative = source.strip_prefix(installed_root).unwrap_or(&source);
        let target = destination_root.join("From Game").join(relative);
        if let Some(parent) = target.parent() {
            if fs::create_dir_all(parent).is_err() {
                continue;
            }
        }
        if fs::rename(&source, &target).is_ok() {
            moved += 1;
        }
    }
    if staging_path.exists() {
        if fs::create_dir_all(&destination_root).is_ok() {
            let target = unique_destination(&destination_root.join("Package Wrapper"));
            match fs::rename(staging_path, target) {
                Ok(()) => moved += 1,
                Err(error) if error.kind() == ErrorKind::NotFound => {}
                Err(error) => {
                    warnings.push(format!("Package wrapper cleanup was incomplete: {error}"))
                }
            }
        }
    }
    (moved > 0).then_some(destination_root)
}

fn archive_inbox_package(
    archive: Option<&Path>,
    managed_root: &Path,
    title: &str,
    stamp: &str,
    warnings: &mut Vec<String>,
) -> Option<PathBuf> {
    let archive = archive.filter(|path| path.is_file())?;
    let inbox = managed_root.join("Inbox").canonicalize().ok()?;
    let canonical_archive = archive.canonicalize().ok()?;
    if !canonical_archive.starts_with(&inbox) {
        return None;
    }
    let extension = canonical_archive
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("zip");
    let root = managed_root.join("Archives").join("Imported");
    if let Err(error) = fs::create_dir_all(&root) {
        warnings.push(format!("The source ZIP stayed in Inbox: {error}"));
        return None;
    }
    let target = unique_destination(&root.join(format!("{title}-{stamp}.{extension}")));
    match fs::rename(&canonical_archive, &target) {
        Ok(()) => Some(target),
        Err(error) => {
            warnings.push(format!("The source ZIP stayed in Inbox: {error}"));
            None
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn write_install_report(
    managed_root: &Path,
    title: &str,
    installed_path: &Path,
    executable_path: &Path,
    backup_path: Option<&Path>,
    dependencies_path: Option<&Path>,
    extras_path: Option<&Path>,
    archived_package_path: Option<&Path>,
    warnings: &[String],
) -> Result<PathBuf, String> {
    let reports = managed_root.join("Reports");
    fs::create_dir_all(&reports).map_err(|error| error.to_string())?;
    let path = reports.join(format!(
        "game-install-{}.json",
        Utc::now().format("%Y%m%d-%H%M%S")
    ));
    let report = serde_json::json!({
        "installedAt": Utc::now().to_rfc3339(),
        "title": title,
        "installedPath": installed_path,
        "executablePath": executable_path,
        "backupPath": backup_path,
        "dependenciesPath": dependencies_path,
        "extrasPath": extras_path,
        "archivedPackagePath": archived_package_path,
        "warnings": warnings,
    });
    fs::write(
        &path,
        serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    Ok(path)
}

fn write_staging_report(
    managed_root: &Path,
    mut staged: StagedArchive,
) -> Result<StagedArchive, String> {
    let reports = managed_root.join("Reports");
    fs::create_dir_all(&reports).map_err(|error| error.to_string())?;
    let path = reports.join(format!(
        "archive-intake-{}.json",
        Utc::now().format("%Y%m%d-%H%M%S")
    ));
    staged.report_path = path.to_string_lossy().to_string();
    let json = serde_json::to_vec_pretty(&staged).map_err(|error| error.to_string())?;
    fs::write(&path, json).map_err(|error| error.to_string())?;
    Ok(staged)
}

fn validate_archive(archive: &Path) -> Result<(), String> {
    if !archive.is_file()
        || !archive
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| value.eq_ignore_ascii_case("zip"))
            .unwrap_or(false)
    {
        return Err("Choose an existing ZIP archive.".to_string());
    }
    Ok(())
}

fn find_7zip() -> Result<PathBuf, String> {
    let mut candidates = Vec::new();
    if let Some(program_files) = std::env::var_os("ProgramFiles") {
        candidates.push(PathBuf::from(program_files).join("7-Zip").join("7z.exe"));
    }
    if let Some(program_files_x86) = std::env::var_os("ProgramFiles(x86)") {
        candidates.push(
            PathBuf::from(program_files_x86)
                .join("7-Zip")
                .join("7z.exe"),
        );
    }
    candidates
        .into_iter()
        .find(|path| path.is_file())
        .ok_or_else(|| "Install 7-Zip from 7-zip.org before testing ZIP packages.".to_string())
}

fn seven_zip<const N: usize>(
    extractor: &Path,
    arguments: [&str; N],
    archive: &Path,
) -> Result<std::process::Output, String> {
    Command::new(extractor)
        .args(arguments)
        .arg(archive)
        .creation_flags_if_windows(CREATE_NO_WINDOW)
        .output()
        .map_err(|error| error.to_string())
}

trait CommandWindowExt {
    fn creation_flags_if_windows(&mut self, flags: u32) -> &mut Self;
}

impl CommandWindowExt for Command {
    fn creation_flags_if_windows(&mut self, flags: u32) -> &mut Self {
        #[cfg(windows)]
        self.creation_flags(flags);
        #[cfg(not(windows))]
        let _ = flags;
        self
    }
}

fn parse_listing(output: &str) -> Vec<ArchiveEntry> {
    let mut entries = Vec::new();
    let mut current_path: Option<String> = None;
    let mut current_size = 0_u64;
    for line in output.lines().chain(std::iter::once("")) {
        if let Some(value) = line.strip_prefix("Path = ") {
            if let Some(path) = current_path.take() {
                entries.push(ArchiveEntry {
                    path,
                    size: current_size,
                });
            }
            current_path = Some(value.to_string());
            current_size = 0;
        } else if let Some(value) = line.strip_prefix("Size = ") {
            current_size = value.parse().unwrap_or_default();
        } else if line.is_empty() {
            if let Some(path) = current_path.take() {
                entries.push(ArchiveEntry {
                    path,
                    size: current_size,
                });
                current_size = 0;
            }
        }
    }
    entries
}

fn unsafe_archive_path(value: &str) -> bool {
    let path = Path::new(value);
    path.is_absolute()
        || value.starts_with(['\\', '/'])
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::Prefix(_)))
}

fn has_extension(value: &str, extensions: &[&str]) -> bool {
    Path::new(value)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            extensions
                .iter()
                .any(|expected| extension.eq_ignore_ascii_case(expected))
        })
        .unwrap_or(false)
}

fn path_has_redist_component(value: &str) -> bool {
    Path::new(value).components().any(|component| {
        let name = component.as_os_str().to_string_lossy().to_lowercase();
        name.contains("redist") || matches!(name.as_str(), "prerequisites" | "support")
    })
}

fn has_modified_platform_marker(value: &str) -> bool {
    let lower = value.to_lowercase();
    lower.contains("steam_emu")
        || lower.contains("screamapi")
        || lower.ends_with(".rne")
        || lower.ends_with(".valve")
}

fn likely_game_executable(value: &str) -> bool {
    if !has_extension(value, &["exe"]) {
        return false;
    }
    let lower = Path::new(value)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default()
        .to_lowercase();
    ![
        "setup",
        "install",
        "unins",
        "crash",
        "reporter",
        "vcredist",
        "vc_redist",
        "dxsetup",
        "dxwebsetup",
        "physx",
        "dotnet",
        "unitycrashhandler",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn clean_package_name(archive: &Path) -> String {
    let stem = archive
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("Package");
    let mut words = stem
        .split(['-', '_', ' '])
        .filter(|word| !word.is_empty())
        .filter(|word| {
            !matches!(
                word.to_ascii_lowercase().as_str(),
                "ankergames" | "steamripped" | "repack"
            )
        })
        .collect::<Vec<_>>();
    if words.is_empty() {
        words.push("Package");
    }
    let cleaned = words.join(" ");
    cleaned.chars().take(48).collect()
}

fn archive_name(archive: &Path) -> String {
    archive
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("Archive.zip")
        .to_string()
}

fn display_path(path: &Path) -> String {
    let value = path.to_string_lossy();
    if let Some(unc) = value.strip_prefix(r"\\?\UNC\") {
        return format!(r"\\{unc}");
    }
    value
        .strip_prefix(r"\\?\")
        .unwrap_or(value.as_ref())
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listing_parser_preserves_paths_and_sizes() {
        let entries = parse_listing(
            "Path = Game/Game.exe\nSize = 42\n\nPath = Game/Redist/setup.exe\nSize = 9\n",
        );
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].path, "Game/Game.exe");
        assert_eq!(entries[0].size, 42);
    }

    #[test]
    fn path_traversal_is_blocked() {
        assert!(unsafe_archive_path("../outside.exe"));
        assert!(unsafe_archive_path(r"C:\outside.exe"));
        assert!(!unsafe_archive_path("Game/Game.exe"));
    }

    #[test]
    fn clean_name_removes_package_source_suffix() {
        assert_eq!(
            clean_package_name(Path::new("Orb-Of-Creation-AnkerGames.zip")),
            "Orb Of Creation"
        );
    }

    #[test]
    fn display_path_hides_windows_verbatim_prefix() {
        assert_eq!(
            display_path(Path::new(r"\\?\E:\GameVault\Staging\Example")),
            r"E:\GameVault\Staging\Example"
        );
        assert_eq!(
            display_path(Path::new(r"\\?\UNC\server\share\Example")),
            r"\\server\share\Example"
        );
    }

    #[test]
    fn installed_7zip_can_test_and_list_a_fixture_archive() {
        let Ok(extractor) = find_7zip() else {
            return;
        };
        let directory = tempfile::tempdir().expect("temp directory");
        let game = directory.path().join("Fixture Game");
        fs::create_dir_all(&game).expect("game directory");
        fs::write(game.join("FixtureGame.exe"), b"fixture").expect("game fixture");
        let archive = directory.path().join("Fixture-Game.zip");
        let output = Command::new(extractor)
            .args(["a", "-tzip", "-bd", "-y"])
            .arg(&archive)
            .arg(&game)
            .output()
            .expect("7-Zip fixture creation");
        assert!(output.status.success());

        let result = inspect(&archive).expect("archive inspection");
        assert!(result.valid);
        assert!(result.can_stage);
        assert!(result
            .executable_candidates
            .iter()
            .any(|path| path.ends_with("FixtureGame.exe")));
    }

    #[test]
    fn staged_package_is_cleaned_and_promoted_transactionally() {
        let directory = tempfile::tempdir().expect("temp directory");
        let managed = directory.path().join("GameVault");
        let staging = managed.join("Staging").join("package-1");
        let game = staging.join("Example Game");
        fs::create_dir_all(game.join("Redist")).expect("redist");
        fs::create_dir_all(managed.join("Inbox")).expect("inbox");
        fs::write(game.join("ExampleGame.exe"), vec![0_u8; 1024]).expect("game");
        fs::write(game.join("Redist").join("setup.exe"), b"fixture").expect("setup");
        fs::write(game.join("source.url"), b"fixture").expect("shortcut");
        let archive = managed.join("Inbox").join("Example Game.zip");
        fs::write(&archive, b"fixture").expect("archive");

        let analysis = analyze_staged(&staging, &managed).expect("analysis");
        assert!(analysis.can_install);
        assert_eq!(analysis.redist_folders.len(), 1);
        let selected = analysis.executable_candidates.first().expect("candidate");
        let result = promote_staged(
            &InstallStagedInput {
                staging_path: analysis.staging_path.clone(),
                executable_path: selected.executable_path.clone(),
                title: "Example Game".to_string(),
                archive_path: Some(archive.to_string_lossy().to_string()),
            },
            &managed,
        )
        .expect("promotion");

        assert!(result.installed_path.join("ExampleGame.exe").is_file());
        assert!(!result.installed_path.join("Redist").exists());
        assert!(result.dependencies_path.is_some());
        assert!(result.extras_path.is_some());
        assert!(result.archived_package_path.is_some());
    }

    #[test]
    fn modified_platform_marker_blocks_staged_installation() {
        let directory = tempfile::tempdir().expect("temp directory");
        let managed = directory.path().join("GameVault");
        let staging = managed.join("Staging").join("package-2");
        fs::create_dir_all(staging.join("Game")).expect("game");
        fs::write(staging.join("Game").join("Game.exe"), b"fixture").expect("exe");
        fs::write(staging.join("Game").join("steam_emu.ini"), b"fixture").expect("marker");

        let analysis = analyze_staged(&staging, &managed).expect("analysis");
        assert!(analysis.blocked);
        assert!(!analysis.can_install);
        assert_eq!(analysis.suspicious_markers.len(), 1);
    }
}
