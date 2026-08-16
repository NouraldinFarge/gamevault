use std::path::{Component, Path};

#[derive(Debug)]
pub(crate) struct ArchiveEntry {
    pub(crate) path: String,
    pub(crate) size: u64,
    pub(crate) link_or_reparse: bool,
}

pub(crate) fn parse_listing(output: &str) -> Vec<ArchiveEntry> {
    let mut entries = Vec::new();
    let mut current_path: Option<String> = None;
    let mut current_size = 0_u64;
    let mut current_link_or_reparse = false;
    for line in output.lines().chain(std::iter::once("")) {
        if let Some(value) = line.strip_prefix("Path = ") {
            if let Some(path) = current_path.take() {
                entries.push(ArchiveEntry {
                    path,
                    size: current_size,
                    link_or_reparse: current_link_or_reparse,
                });
            }
            current_path = Some(value.to_string());
            current_size = 0;
            current_link_or_reparse = false;
        } else if let Some(value) = line.strip_prefix("Size = ") {
            current_size = value.parse().unwrap_or_default();
        } else if let Some(value) = line.strip_prefix("Attributes = ") {
            current_link_or_reparse |= archive_attributes_are_link_or_reparse(value);
        } else if ["Symbolic Link = ", "Hard Link = ", "Reparse Point = "]
            .iter()
            .any(|prefix| {
                line.strip_prefix(prefix)
                    .is_some_and(|value| !value.is_empty())
            })
        {
            current_link_or_reparse = true;
        } else if line.is_empty() {
            if let Some(path) = current_path.take() {
                entries.push(ArchiveEntry {
                    path,
                    size: current_size,
                    link_or_reparse: current_link_or_reparse,
                });
                current_size = 0;
                current_link_or_reparse = false;
            }
        }
    }
    entries
}

pub(crate) fn unsafe_archive_path(value: &str) -> bool {
    if value.is_empty()
        || value.contains('\0')
        || value.starts_with(['\\', '/'])
        || Path::new(value).is_absolute()
    {
        return true;
    }
    let windows_path = value.replace('/', "\\");
    windows_path.split('\\').any(|component| {
        component.is_empty()
            || matches!(component, "." | "..")
            || component.contains(':')
            || component.ends_with([' ', '.'])
            || is_reserved_windows_name(component)
    }) || Path::new(value)
        .components()
        .any(|component| matches!(component, Component::ParentDir | Component::Prefix(_)))
}

pub(crate) fn normalized_archive_key(value: &str) -> String {
    value.replace('/', "\\").to_lowercase()
}

pub(crate) fn is_reserved_windows_name(value: &str) -> bool {
    let stem = value
        .split('.')
        .next()
        .unwrap_or_default()
        .trim_end_matches([' ', '.'])
        .to_ascii_uppercase();
    matches!(
        stem.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CLOCK$" | "CONIN$" | "CONOUT$"
    ) || stem
        .strip_prefix("COM")
        .or_else(|| stem.strip_prefix("LPT"))
        .is_some_and(|suffix| {
            matches!(
                suffix,
                "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
            )
        })
}

pub(crate) fn has_extension(value: &str, extensions: &[&str]) -> bool {
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

pub(crate) fn path_has_redist_component(value: &str) -> bool {
    Path::new(value).components().any(|component| {
        let name = component.as_os_str().to_string_lossy().to_lowercase();
        name.contains("redist") || matches!(name.as_str(), "prerequisites" | "support")
    })
}

pub(crate) fn has_modified_platform_marker(value: &str) -> bool {
    let lower = value.to_lowercase();
    lower.contains("steam_emu")
        || lower.contains("screamapi")
        || lower.ends_with(".rne")
        || lower.ends_with(".valve")
}

pub(crate) fn likely_game_executable(value: &str) -> bool {
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

fn archive_attributes_are_link_or_reparse(value: &str) -> bool {
    let lower = value.trim().to_ascii_lowercase();
    lower.starts_with('l') || lower.contains("reparse")
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn listing_parser_preserves_paths_and_sizes() {
        let entries = parse_listing(
            "Path = Game/Game.exe\nSize = 42\n\nPath = Game/Redist/setup.exe\nSize = 9\n",
        );
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].path, "Game/Game.exe");
        assert_eq!(entries[0].size, 42);
        assert!(!entries[0].link_or_reparse);
    }

    #[test]
    fn listing_parser_marks_links_and_reparse_entries() {
        let entries = parse_listing(
            "Path = Game/link.exe\nSize = 5\nAttributes = lrwxrwxrwx\nSymbolic Link = ../outside.exe\n",
        );
        assert_eq!(entries.len(), 1);
        assert!(entries[0].link_or_reparse);
    }

    #[test]
    fn path_traversal_is_blocked() {
        assert!(unsafe_archive_path("../outside.exe"));
        assert!(unsafe_archive_path(r"C:\outside.exe"));
        assert!(unsafe_archive_path(r"Game\..\outside.exe"));
        assert!(unsafe_archive_path(r"Game\payload.exe:stream"));
        assert!(unsafe_archive_path(r"Game\CON.txt"));
        assert!(unsafe_archive_path(r"Game\name.\payload.exe"));
        assert!(unsafe_archive_path(r"\\server\share\game.exe"));
        assert!(!unsafe_archive_path("Game/Game.exe"));
    }

    #[test]
    fn windows_case_collisions_normalize_to_the_same_key() {
        assert_eq!(
            normalized_archive_key("Game/Bin/Game.exe"),
            normalized_archive_key(r"game\bin\GAME.EXE")
        );
    }

    proptest! {
        #[test]
        fn traversal_components_are_always_rejected(
            prefix in "[A-Za-z0-9_-]{1,24}",
            suffix in "[A-Za-z0-9_-]{1,24}",
        ) {
            let forward = format!("{prefix}/../{suffix}.exe");
            let backward = format!(r"{prefix}\..\{suffix}.exe");
            prop_assert!(unsafe_archive_path(&forward));
            prop_assert!(unsafe_archive_path(&backward));
        }

        #[test]
        fn normalized_keys_ignore_windows_case_and_separator_style(
            components in prop::collection::vec("[A-Za-z0-9_-]{1,16}", 1..8),
        ) {
            let forward = components.join("/");
            let backward_upper = components
                .iter()
                .map(|component| component.to_uppercase())
                .collect::<Vec<_>>()
                .join("\\");
            prop_assert_eq!(
                normalized_archive_key(&forward),
                normalized_archive_key(&backward_upper),
            );
        }

        #[test]
        fn arbitrary_listing_text_never_creates_more_entries_than_lines(
            input in ".{0,4096}",
        ) {
            let line_bound = input.lines().count().saturating_add(1);
            prop_assert!(parse_listing(&input).len() <= line_bound);
        }
    }
}
