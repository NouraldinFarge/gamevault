# Security policy

## Supported version

Security fixes are applied to the latest published GameVault release. Upgrade older portable builds before reporting an issue.

## Reporting a vulnerability

Use [GitHub private vulnerability reporting](https://github.com/NouraldinFarge/gamevault/security/advisories/new). Do not file a public issue for path traversal, link or reparse handling, unsafe extraction, command injection, process-launch behavior, allowlist bypass, or credential exposure.

Include the affected version, a minimal reproduction using synthetic content, expected impact, and non-sensitive logs. Do not upload commercial games, private archives, credentials, or personal library databases.

## Trust boundaries

- ZIP archives and extracted files are untrusted until validation and explicit approval complete.
- Structural preflight precedes decompression testing and extraction. Traversal, absolute/drive-relative paths, NTFS streams, reserved device names, case collisions, link/reparse metadata, excessive expansion, and modified-platform markers block intake or promotion.
- Extracted trees and every managed folder boundary are checked again for Windows links/reparse points before files are moved or scanned.
- Nested archives are not recursively expanded.
- Promotion cleanup is journaled so failed library registration can restore staging, the Inbox ZIP, separated prerequisites, quarantined extras, and the previous game version.
- Backup restore requires SQLite integrity, compatible GameVault tables/settings, and a supported schema identity before the active transaction begins.
- Filesystem, SQLite, metadata, URL, and process operations remain behind the Rust/Tauri authority.
- Optional metadata is restricted to approved official hosts and never becomes a launch dependency.
- GameVault does not download games or establish ownership rights.
