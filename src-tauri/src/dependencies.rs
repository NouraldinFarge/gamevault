use crate::models::{DependencyAudit, DependencyItem};
use chrono::Utc;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use walkdir::{DirEntry, WalkDir};

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY};
#[cfg(windows)]
use winreg::RegKey;

const MICROSOFT_VC_X64: &str = "https://aka.ms/vs/17/release/vc_redist.x64.exe";
const MICROSOFT_VC_X86: &str = "https://aka.ms/vs/17/release/vc_redist.x86.exe";
const MICROSOFT_VC_GUIDANCE: &str =
    "https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist";
const MICROSOFT_DIRECTX: &str = "https://www.microsoft.com/en-us/download/details.aspx?id=8109";
const MICROSOFT_DOTNET: &str = "https://dotnet.microsoft.com/en-us/download/dotnet";
const NVIDIA_PHYSX: &str = "https://www.nvidia.com/en-gb/drivers/physx-system-software/";
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SignatureDetails {
    status: String,
    publisher: String,
    file_version: String,
}

#[derive(Debug)]
struct Classification {
    name: &'static str,
    architecture: &'static str,
    official_url: Option<&'static str>,
    installed_status: String,
    installed_version: Option<String>,
}

pub fn audit(managed_root: &Path) -> Result<DependencyAudit, String> {
    if !managed_root.is_dir() {
        return Err("The managed GameVault folder is unavailable.".to_string());
    }

    let mut redist_folders = HashSet::new();
    let mut candidate_files = Vec::new();
    let mut files_inspected = 0_usize;

    for entry in WalkDir::new(managed_root)
        .max_depth(10)
        .follow_links(false)
        .into_iter()
        .filter_entry(allowed_workspace_entry)
        .filter_map(Result::ok)
        .take(50_000)
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let Some(redist_root) = containing_redist_folder(entry.path(), managed_root) else {
            continue;
        };
        redist_folders.insert(redist_root);
        files_inspected += 1;
        if is_installer_candidate(entry.path()) {
            candidate_files.push(entry.path().to_path_buf());
        }
    }

    candidate_files.sort();
    let mut sources = HashMap::new();
    let mut items = Vec::new();
    for path in candidate_files {
        let sha256 = hash_file(&path)?;
        let signature = inspect_signature(&path);
        let classification = classify(&path);
        let imitation = contains_imitation_marker(&path);
        let signature_status = if imitation {
            "unsafe imitation".to_string()
        } else {
            signature
                .as_ref()
                .map(|details| normalize_signature_status(&details.status))
                .unwrap_or_else(|| "unavailable".to_string())
        };
        let online_status = match classification.official_url {
            Some(url) => sources
                .entry(url)
                .or_insert_with(|| official_source_status(url))
                .clone(),
            None => "not available".to_string(),
        };
        let recommendation = recommendation(
            &signature_status,
            &classification.installed_status,
            classification.official_url.is_some(),
        );
        items.push(DependencyItem {
            id: format!("{}-{}", &sha256[..12], items.len()),
            name: classification.name.to_string(),
            architecture: classification.architecture.to_string(),
            bundled_path: path.to_string_lossy().to_string(),
            bundled_version: signature
                .as_ref()
                .and_then(|details| optional_text(&details.file_version)),
            sha256,
            signature_status,
            publisher: signature
                .as_ref()
                .and_then(|details| optional_text(&details.publisher)),
            installed_status: classification.installed_status,
            installed_version: classification.installed_version,
            official_source_url: classification.official_url.map(str::to_string),
            online_status,
            recommendation,
        });
    }

    let installed = items
        .iter()
        .filter(|item| item.installed_status == "installed")
        .count();
    let missing = items
        .iter()
        .filter(|item| item.installed_status == "missing")
        .count();
    let suspicious = items
        .iter()
        .filter(|item| !matches!(item.signature_status.as_str(), "valid" | "not applicable"))
        .count();
    let official_sources_reachable = !sources.is_empty()
        && sources
            .values()
            .all(|status| status.as_str() == "reachable");
    let audited_at = Utc::now().to_rfc3339();
    let reports = managed_root.join("Reports");
    fs::create_dir_all(&reports).map_err(|error| error.to_string())?;
    let report_path = reports.join(format!(
        "dependency-audit-{}.json",
        Utc::now().format("%Y%m%d-%H%M%S")
    ));
    let audit = DependencyAudit {
        audited_at,
        managed_root: managed_root.to_string_lossy().to_string(),
        redist_folders: redist_folders.len(),
        files_inspected,
        installed,
        missing,
        suspicious,
        official_sources_reachable,
        report_path: report_path.to_string_lossy().to_string(),
        items,
    };
    let json = serde_json::to_vec_pretty(&audit).map_err(|error| error.to_string())?;
    let mut report = File::create(&report_path).map_err(|error| error.to_string())?;
    report.write_all(&json).map_err(|error| error.to_string())?;
    report.sync_all().map_err(|error| error.to_string())?;
    Ok(audit)
}

pub fn is_approved_official_url(url: &str) -> bool {
    matches!(
        url,
        MICROSOFT_VC_X64
            | MICROSOFT_VC_X86
            | MICROSOFT_VC_GUIDANCE
            | MICROSOFT_DIRECTX
            | MICROSOFT_DOTNET
            | NVIDIA_PHYSX
    )
}

fn allowed_workspace_entry(entry: &DirEntry) -> bool {
    if entry.depth() != 1 || !entry.file_type().is_dir() {
        return !entry.file_type().is_symlink();
    }
    let name = entry.file_name().to_string_lossy();
    !matches!(name.as_ref(), "Archives" | "Inbox" | "Staging")
}

fn containing_redist_folder(path: &Path, root: &Path) -> Option<PathBuf> {
    let relative = path.strip_prefix(root).ok()?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component.as_os_str());
        let name = component.as_os_str().to_string_lossy().to_lowercase();
        if name.contains("redist") || matches!(name.as_str(), "prerequisites" | "support") {
            return Some(current);
        }
    }
    None
}

fn is_installer_candidate(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|extension| {
            extension.eq_ignore_ascii_case("exe") || extension.eq_ignore_ascii_case("msi")
        })
        .unwrap_or(false)
}

fn classify(path: &Path) -> Classification {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_lowercase();
    if name.contains("vc_redist.x64") {
        let (status, version) = visual_cpp_status("x64");
        return Classification {
            name: "Microsoft Visual C++ 2015–2022",
            architecture: "x64",
            official_url: Some(MICROSOFT_VC_X64),
            installed_status: status,
            installed_version: version,
        };
    }
    if name.contains("vc_redist.x86") {
        let (status, version) = visual_cpp_status("x86");
        return Classification {
            name: "Microsoft Visual C++ 2015–2022",
            architecture: "x86",
            official_url: Some(MICROSOFT_VC_X86),
            installed_status: status,
            installed_version: version,
        };
    }
    if name.contains("vcredist") || name.contains("vc_redist") {
        return Classification {
            name: "Microsoft Visual C++ legacy package",
            architecture: architecture_from_name(&name),
            official_url: Some(MICROSOFT_VC_GUIDANCE),
            installed_status: "review required".to_string(),
            installed_version: None,
        };
    }
    if name.contains("dxsetup") || name.contains("dxwebsetup") || name.contains("directx") {
        let (status, version) = directx_status();
        return Classification {
            name: "Microsoft DirectX legacy runtime",
            architecture: "x86 + x64",
            official_url: Some(MICROSOFT_DIRECTX),
            installed_status: status,
            installed_version: version,
        };
    }
    if name.contains("physx") {
        let (status, version) = physx_status();
        return Classification {
            name: "NVIDIA PhysX System Software",
            architecture: "x86 + x64",
            official_url: Some(NVIDIA_PHYSX),
            installed_status: status,
            installed_version: version,
        };
    }
    if name.contains("dotnet") || name.starts_with("ndp") {
        let (status, version) = dotnet_status();
        return Classification {
            name: "Microsoft .NET runtime",
            architecture: architecture_from_name(&name),
            official_url: Some(MICROSOFT_DOTNET),
            installed_status: status,
            installed_version: version,
        };
    }
    Classification {
        name: "Unrecognized bundled installer",
        architecture: architecture_from_name(&name),
        official_url: None,
        installed_status: "unknown".to_string(),
        installed_version: None,
    }
}

fn architecture_from_name(name: &str) -> &'static str {
    if name.contains("x64") || name.contains("amd64") {
        "x64"
    } else if name.contains("x86") {
        "x86"
    } else if name.contains("arm64") {
        "arm64"
    } else {
        "unknown"
    }
}

fn hash_file(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|error| error.to_string())?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 128 * 1024];
    loop {
        let count = file.read(&mut buffer).map_err(|error| error.to_string())?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn contains_imitation_marker(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    if metadata.len() > 2 * 1024 * 1024 {
        return false;
    }
    let Ok(bytes) = fs::read(path) else {
        return false;
    };
    let text = String::from_utf8_lossy(&bytes).to_lowercase();
    text.contains("imitation of the game's executable file")
        || text.contains("imitation of the game’s executable file")
}

fn normalize_signature_status(status: &str) -> String {
    match status.trim().to_lowercase().as_str() {
        "valid" => "valid".to_string(),
        "notsigned" => "unsigned".to_string(),
        "hashmismatch" => "hash mismatch".to_string(),
        "nottrusted" => "not trusted".to_string(),
        "unknownerror" => "verification error".to_string(),
        _ => "unavailable".to_string(),
    }
}

fn optional_text(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn recommendation(signature: &str, installed: &str, has_official_source: bool) -> String {
    if signature == "unsafe imitation" || signature == "hash mismatch" {
        return "Do not run the bundled file. Keep it quarantined and use the official source."
            .to_string();
    }
    if signature != "valid" {
        return if has_official_source {
            "Do not trust this bundled installer; compare it with the official vendor source."
                .to_string()
        } else {
            "Leave this installer quarantined until its publisher and purpose are verified."
                .to_string()
        };
    }
    if installed == "installed" {
        "Already installed; no bundled installer needs to run.".to_string()
    } else if installed == "missing" {
        "Use the official vendor source if this game requires the dependency.".to_string()
    } else {
        "Verify the exact required version before installing from the official source.".to_string()
    }
}

fn official_source_status(url: &str) -> String {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(4))
        .timeout_read(Duration::from_secs(4))
        .build();
    match agent.head(url).call() {
        Ok(response) if (200..400).contains(&response.status()) => "reachable".to_string(),
        _ => "offline or unavailable".to_string(),
    }
}

#[cfg(windows)]
fn inspect_signature(path: &Path) -> Option<SignatureDetails> {
    let script = r#"& { param([string]$p) $s = Get-AuthenticodeSignature -LiteralPath $p; $v = (Get-Item -LiteralPath $p).VersionInfo.FileVersion; [pscustomobject]@{ status = [string]$s.Status; publisher = if ($s.SignerCertificate) { [string]$s.SignerCertificate.Subject } else { '' }; fileVersion = if ($v) { [string]$v } else { '' } } | ConvertTo-Json -Compress }"#;
    let output = Command::new("powershell.exe")
        .creation_flags(CREATE_NO_WINDOW)
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .arg(path)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let json = String::from_utf8_lossy(&output.stdout)
        .trim_start_matches('\u{feff}')
        .trim()
        .to_string();
    serde_json::from_str(&json).ok()
}

#[cfg(not(windows))]
fn inspect_signature(_path: &Path) -> Option<SignatureDetails> {
    None
}

#[cfg(windows)]
fn visual_cpp_status(architecture: &str) -> (String, Option<String>) {
    let key_path = format!(
        r"SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\{}",
        architecture
    );
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    for view in [KEY_WOW64_64KEY, KEY_WOW64_32KEY] {
        if let Ok(key) = hklm.open_subkey_with_flags(&key_path, KEY_READ | view) {
            let installed = key.get_value::<u32, _>("Installed").unwrap_or_default();
            if installed == 1 {
                let version = key.get_value::<String, _>("Version").ok();
                return ("installed".to_string(), version);
            }
        }
    }
    ("missing".to_string(), None)
}

#[cfg(not(windows))]
fn visual_cpp_status(_architecture: &str) -> (String, Option<String>) {
    ("unknown".to_string(), None)
}

fn directx_status() -> (String, Option<String>) {
    let Some(windows) = std::env::var_os("WINDIR").map(PathBuf::from) else {
        return ("unknown".to_string(), None);
    };
    let required = [
        windows.join("System32").join("d3dx9_43.dll"),
        windows.join("System32").join("xinput1_3.dll"),
    ];
    if required.iter().all(|path| path.is_file()) {
        (
            "installed".to_string(),
            Some("June 2010 components detected".to_string()),
        )
    } else {
        ("missing".to_string(), None)
    }
}

#[cfg(windows)]
fn physx_status() -> (String, Option<String>) {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    for view in [KEY_WOW64_64KEY, KEY_WOW64_32KEY] {
        if let Ok(key) =
            hklm.open_subkey_with_flags(r"SOFTWARE\NVIDIA Corporation\PhysX", KEY_READ | view)
        {
            let version = key
                .get_value::<String, _>("Version")
                .ok()
                .or_else(|| key.get_value::<String, _>("InstalledVersion").ok());
            return ("installed".to_string(), version);
        }
    }
    ("missing".to_string(), None)
}

#[cfg(not(windows))]
fn physx_status() -> (String, Option<String>) {
    ("unknown".to_string(), None)
}

fn dotnet_status() -> (String, Option<String>) {
    let output = Command::new("dotnet").arg("--list-runtimes").output();
    match output {
        Ok(output) if output.status.success() => {
            let runtimes = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let version = runtimes.lines().last().map(str::to_string);
            ("installed".to_string(), version)
        }
        _ => ("missing".to_string(), None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classification_never_trusts_a_familiar_filename() {
        let classification = classify(Path::new("vc_redist.x64.exe"));
        assert_eq!(classification.name, "Microsoft Visual C++ 2015–2022");
        assert_eq!(classification.official_url, Some(MICROSOFT_VC_X64));
    }

    #[test]
    fn imitation_marker_is_detected() {
        let directory = tempfile::tempdir().expect("temp directory");
        let path = directory.path().join("DXWebSetup.exe");
        fs::write(&path, b"This is an imitation of the game's executable file").expect("fixture");
        assert!(contains_imitation_marker(&path));
    }

    #[test]
    fn official_link_allowlist_is_exact() {
        assert!(is_approved_official_url(MICROSOFT_DIRECTX));
        assert!(!is_approved_official_url(
            "https://example.com/vc_redist.x64.exe"
        ));
    }
}
