# Security policy

## Supported versions

| Version | Supported |
| --- | --- |
| Latest published release | Yes |
| Older portable releases | No; upgrade before reporting |

## Report privately

Use [GitHub private vulnerability reporting](https://github.com/NouraldinFarge/gamevault/security/advisories/new) for path traversal, links/reparse handling, unsafe extraction, command or argument injection, process-launch containment, metadata allowlist bypass, backup validation, or credential/path exposure.

Include the affected version or commit, expected impact, and a minimal synthetic reproduction. Redact personal paths and do not upload commercial games, private archives, credentials, license keys, or personal library databases. Do not open a public issue until a coordinated fix is available.

For ordinary defects and usage questions, use the routes in [SUPPORT.md](SUPPORT.md).

## Trust boundaries

- ZIP archives and extracted files remain untrusted until validation and explicit approval complete.
- Structural preflight precedes decompression testing and extraction. Traversal, absolute or drive-relative paths, NTFS streams, reserved device names, case collisions, link/reparse metadata, excessive expansion, and modified-platform markers block intake or promotion.
- Extracted trees and every managed-folder boundary are checked again for Windows links/reparse points before files are moved or scanned.
- Nested archives are not recursively expanded, and inspection never executes archive content.
- Promotion cleanup is journaled so failed library registration can restore staging, the Inbox ZIP, separated prerequisites, quarantined extras, and a previous game version.
- Backup restore requires SQLite integrity, compatible GameVault tables/settings, and a supported schema identity before the active transaction begins.
- Filesystem, SQLite, backup, metadata, URL, and process operations remain behind the Rust/Tauri authority.
- Optional metadata is restricted to approved official HTTPS hosts and never becomes a launch dependency.

## Security claims and non-goals

GameVault reduces specific archive, path, process, and recovery risks; it is not an antivirus product, sandbox, ownership validator, or guarantee that an approved archive is safe. It does not download games, bypass DRM or platform controls, patch commercial content, or weaken operating-system security.

The portable executable is not yet Authenticode-signed. Verify the published SHA-256 checksum and GitHub provenance before running a release.
