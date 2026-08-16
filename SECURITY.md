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
- Interrupted scans, staging, promotion, and prerequisite audits are persisted as evidence and changed to `interrupted` on restart. They are never resumed silently.
- Staged installation and update approval is bound to a SHA-256 fingerprint of bounded, link-free current/proposed manifests and the destination. Promotion recomputes it before any move and rejects stale approval.
- Promotion cleanup is journaled so failed library registration can restore staging, the Inbox ZIP, separated prerequisites, quarantined extras, and a previous game version.
- Redist review hashes each bundled candidate, checks Authenticode and the expected vendor identity, and records installed-version evidence from the relevant registry, representative system files, or `dotnet --list-runtimes`. A familiar filename is never sufficient and GameVault never executes the installer during audit.
- Backup restore requires SQLite integrity, compatible GameVault tables/settings, and a supported schema identity before the active transaction begins.
- Filesystem, SQLite, backup, metadata, URL, and process operations remain behind the Rust/Tauri authority.
- Optional metadata is restricted to approved official HTTPS hosts and never becomes a launch dependency.
- Application update checks are manual, response-size and timeout bounded, and restricted to the official GitHub latest-release endpoint. GameVault validates the exact repository release URL before opening it and does not download or install update assets.

## Security claims and non-goals

GameVault reduces specific archive, path, process, and recovery risks; it is not an antivirus product, sandbox, ownership validator, or guarantee that an approved archive is safe. It does not download games, bypass DRM or platform controls, patch commercial content, or weaken operating-system security.

The portable executable is not yet Authenticode-signed. Verify the published SHA-256 checksum and GitHub provenance before running a release.

The release pipeline is signing-ready without embedding a certificate or password. Signing remains disabled unless all three protected inputs are present: a base64 PFX, its password, and an HTTPS RFC 3161 timestamp URL. Partial configuration fails the build, temporary certificate material is removed in a guarded cleanup block, and the signed executable must pass `signtool verify` before packaging. Until a published release is actually signed, the checksum, SBOM, and GitHub provenance remain the applicable verification evidence.
