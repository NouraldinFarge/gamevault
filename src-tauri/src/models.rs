use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Game {
    pub id: String,
    pub title: String,
    pub description: String,
    pub install_path: String,
    pub executable_path: String,
    pub launch_args: Vec<String>,
    pub tags: Vec<String>,
    pub category: String,
    pub favorite: bool,
    pub detection_status: String,
    pub detection_source: String,
    pub folder_size_bytes: Option<u64>,
    pub last_played_at: Option<String>,
    pub playtime_seconds: u64,
    pub added_at: String,
    pub updated_at: String,
    pub content_signature: String,
    pub artwork_seed: u32,
    #[serde(default)]
    pub metadata: GameMetadata,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GameMetadata {
    pub provider: Option<String>,
    pub external_id: Option<String>,
    pub store_url: Option<String>,
    pub title: Option<String>,
    pub short_description: Option<String>,
    pub cover_url: Option<String>,
    pub hero_url: Option<String>,
    #[serde(default)]
    pub developers: Vec<String>,
    #[serde(default)]
    pub publishers: Vec<String>,
    #[serde(default)]
    pub genres: Vec<String>,
    pub release_date: Option<String>,
    pub website: Option<String>,
    pub minimum_requirements: Option<String>,
    pub recommended_requirements: Option<String>,
    pub fetched_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataLookupInput {
    pub provider: String,
    pub identifier: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveGameMetadataInput {
    pub game_id: String,
    pub metadata: GameMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(default = "default_managed_root")]
    pub managed_root: String,
    pub library_roots: Vec<String>,
    pub scan_depth: u32,
    pub theme: String,
    pub default_launch_args: Vec<String>,
    pub logging_enabled: bool,
    pub last_scan_at: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            managed_root: default_managed_root(),
            library_roots: vec![r"E:\GameVault\Games".to_string()],
            scan_depth: 4,
            theme: "midnight".to_string(),
            default_launch_args: Vec::new(),
            logging_enabled: true,
            last_scan_at: None,
        }
    }
}

fn default_managed_root() -> String {
    r"E:\GameVault".to_string()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryStats {
    pub total_games: usize,
    pub ready_games: usize,
    pub missing_games: usize,
    pub favorites: usize,
    pub total_playtime_seconds: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSnapshot {
    pub games: Vec<Game>,
    pub settings: Settings,
    pub stats: LibraryStats,
    pub portable_root: String,
    pub sqlite_version: String,
    pub scan_in_progress: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgress {
    pub root: String,
    pub current_folder: String,
    pub folders_scanned: usize,
    pub folders_total: usize,
    pub games_detected: usize,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub folders_scanned: usize,
    pub games_detected: usize,
    pub games_added: usize,
    pub games_updated: usize,
    pub unavailable_roots: Vec<String>,
    pub completed_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateGameInput {
    pub id: String,
    pub title: String,
    pub description: String,
    pub executable_path: String,
    pub launch_args: Vec<String>,
    pub tags: Vec<String>,
    pub category: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthReport {
    pub ok: bool,
    pub app_version: String,
    pub portable_root: String,
    pub database_path: String,
    pub sqlite_version: String,
    pub webview2_runtime: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceStatus {
    pub root: String,
    pub ready: bool,
    pub folders: Vec<WorkspaceFolder>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceFolder {
    pub name: String,
    pub path: String,
    pub exists: bool,
    pub item_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyAudit {
    pub audited_at: String,
    pub managed_root: String,
    pub redist_folders: usize,
    pub files_inspected: usize,
    pub installed: usize,
    pub missing: usize,
    pub suspicious: usize,
    pub official_sources_reachable: bool,
    pub report_path: String,
    pub items: Vec<DependencyItem>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyItem {
    pub id: String,
    pub name: String,
    pub architecture: String,
    pub bundled_path: String,
    pub bundled_version: Option<String>,
    pub sha256: String,
    pub signature_status: String,
    pub publisher: Option<String>,
    pub installed_status: String,
    pub installed_version: Option<String>,
    pub official_source_url: Option<String>,
    pub online_status: String,
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveInspection {
    pub archive_path: String,
    pub archive_name: String,
    pub archive_size_bytes: u64,
    pub valid: bool,
    pub extractor: String,
    pub file_count: usize,
    pub unpacked_size_bytes: u64,
    pub executable_candidates: Vec<String>,
    pub warnings: Vec<String>,
    pub can_stage: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StagedArchive {
    pub archive_path: String,
    pub staging_path: String,
    pub files_extracted: usize,
    pub executable_candidates: Vec<String>,
    pub warnings: Vec<String>,
    pub report_path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InboxArchive {
    pub path: String,
    pub name: String,
    pub size_bytes: u64,
    pub modified_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StagedExecutableCandidate {
    pub executable_path: String,
    pub install_root: String,
    pub display_name: String,
    pub score: i32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StagedPackageAnalysis {
    pub staging_path: String,
    pub suggested_title: String,
    pub executable_candidates: Vec<StagedExecutableCandidate>,
    pub redist_folders: Vec<String>,
    pub package_extras: Vec<String>,
    pub nested_archives: Vec<String>,
    pub suspicious_markers: Vec<String>,
    pub blocked: bool,
    pub can_install: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallStagedInput {
    pub staging_path: String,
    pub executable_path: String,
    pub title: String,
    pub archive_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledPackage {
    pub game: Game,
    pub installed_path: String,
    pub backup_path: Option<String>,
    pub dependencies_path: Option<String>,
    pub extras_path: Option<String>,
    pub archived_package_path: Option<String>,
    pub updated: bool,
    pub warnings: Vec<String>,
    pub report_path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    pub diagnostic_id: String,
}

impl CommandError {
    pub fn new(code: &str, message: &str, retryable: bool) -> Self {
        Self {
            code: code.to_string(),
            message: message.to_string(),
            retryable,
            diagnostic_id: uuid::Uuid::now_v7().to_string(),
        }
    }

    pub fn internal(code: &str) -> Self {
        Self::new(
            code,
            "GameVault could not complete that action. Try again or check Diagnostics.",
            false,
        )
    }
}
