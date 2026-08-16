# Changelog

## Unreleased

- Persist scans, archive staging, promotion, and prerequisite audits; reconcile unfinished records after restart; and expose operation history plus recoverable Staging folders without silently resuming work.
- Require a bounded file-level install/update preview and recompute its SHA-256 content fingerprint immediately before promotion, preventing a changed staged package from reusing stale approval.
- Expand Redist review with SHA-256, Authenticode, expected-publisher matching, installed-version evidence, detection method, and confidence while keeping installer execution outside the audit.
- Add an explicit manual application-update check against the official latest stable GitHub release and validate the exact release page without downloading or installing an asset.
- Add desktop/mobile end-to-end journeys, keyboard and axe accessibility gates, improved narrow-window navigation, higher text contrast, and a bounded 10,000-game library test.
- Extract archive-path authority, add property-generated cases and a production-source fuzz harness, and compile that harness in CI with warnings denied.
- Make identical staged inputs produce byte-identical portable ZIPs through ordinal entry order, fixed timestamps, and normalized entry metadata; add a fail-closed, opt-in Authenticode signing hook.
- Add a zero-authority public demo, a guided synthetic tour, a 45-second product walkthrough, and a visibly labeled browser-preview boundary.
- Make repository entry points clearer with direct demo and Windows-download actions plus explicit portable, installer-free, and unsigned status.
- Patch development-only PostCSS and Nano ID advisories and expand the required dependency workflow to audit the complete JavaScript graph.
- Run the required dependency check on every pull request and describe hosted release builds as verified rather than bit-for-bit reproducible.
- Refresh the GitHub presentation with current synthetic screenshots, a branded social preview, visitor-first product and safety evidence, clearer support routes, and stronger issue and pull-request forms.

## 0.3.5 - 2026-08-08

- Preflight ZIP structure before decompression testing; reject unsafe Windows paths, NTFS streams, device names, case collisions, link/reparse metadata, excessive expansion, and insufficient staging space.
- Recheck extracted and managed trees for Windows reparse points and fail closed on unreadable or excessive entry sets.
- Journal promotion cleanup so database-registration failure restores staged files, prerequisites, quarantined extras, source ZIPs, and previous installed versions.
- Validate database backups with SQLite integrity, application/schema identity, compatible columns, and parseable settings before transactional restore.
- Preserve `data`, `library`, user configuration, and logs during portable upgrades; migrate recorded default paths when the portable folder moves; create a pre-restore snapshot; and verify renamed executables and paths containing spaces.
- Add bounded redacted local event logging, metadata/input limits, compact-navigation accessible names, and expanded hostile-input/rollback/backup/launch regression tests.
- Update `rfd` and `wayland-scanner`, removing the vulnerable `quick-xml 0.39.4` and obsolete portal dependency chain; adopt the reviewed React type updates.

## 0.3.4 - 2026-08-01

- Publish the verified dependency-maintenance build under a fresh immutable release tag.
- Include the portable Windows ZIP, SHA-256 checksum, SPDX SBOM, and provenance attestation.

## 0.3.2 - 2026-08-01

- Replace the forced `E:\GameVault` default with a portable-relative `library/` folder beside the executable.
- Migrate the former `E:\GameVault` and `E:\SteamRIPPED` defaults without requiring an `E:` drive.
- Add regression coverage for fresh portable settings and the legacy-path migration.
- Update portable guidance and browser-preview fixtures to avoid presenting a fixed drive as required.

## 0.3.1 - 2026-07-18

- Add GitHub CI, CodeQL, dependency updates, private security reporting guidance, and contributor workflows.
- Hide Windows verbatim-path prefixes such as `\\?\` from archive and staging UI text.
- Shorten executable choices to game-relative paths so the selector remains readable.
- Show the exact package-relative item that triggered an installation block.
- Make blocked installation actions visually and textually unambiguous.
- Keep warning-list markers inside their bordered panel.

## 0.3.0 - 2026-07-18

- Detect user-owned ZIP archives placed in the managed `Inbox` folder.
- Analyze extracted packages, rank likely game executables, and block modified-platform markers or links.
- Promote approved packages transactionally into `Games`, keeping update backups for rollback.
- Separate bundled redistributables into `Dependencies`, package wrappers into `Quarantine`, and completed Inbox ZIPs into `Archives`.
- Persist official Steam, GOG, and Epic product metadata with strict official-host allowlists.
- Show official cover/hero artwork with the existing local abstract-art fallback.
- Migrate the portable SQLite database to schema version 2 while preserving personal metadata and play history.

## 0.2.1 - 2026-07-18

- Added safe ZIP intake with full 7-Zip testing before extraction.
- Added path-traversal rejection and warnings for long paths, nested archives, package scripts, Redist content, and modified-platform markers.
- Added extraction into a unique short Staging folder only, without recursive nested extraction or execution.
- Added JSON archive-intake reports and likely game-executable detection.

## 0.2.0 - 2026-07-18

- Added the managed `E:\GameVault` layout with simple App, Inbox, Staging, Games, Archives, Dependencies, Quarantine, and Reports folders.
- Changed the default playable scan root to `E:\GameVault\Games` and added migration from the former SteamRIPPED root.
- Added Redist discovery, SHA-256 hashing, Authenticode inspection, installed-runtime checks, official-source reachability checks, and JSON audit reports.
- Added an exact allowlist for opening official Microsoft, .NET, and NVIDIA dependency sources.
- Added managed-folder status and dependency-audit controls to Local Files.
- Removed common package-source suffixes from automatically detected game titles.

## 0.1.1 - 2026-07-18

- Fixed portable releases loading the development server URL instead of the embedded interface.
- Added a packaged health check that rejects development-mode executables and missing `index.html` assets.
- Kept the portable build installer-free while enabling Tauri's production custom protocol.

## 0.1.0 - 2026-07-18

- Initial portable GameVault application.
- Single-window Home, Library, Game Details, Local Files, and Settings views.
- Local executable discovery and scoring with installer/tool exclusions.
- SQLite persistence, favorites, tags, categories, paths, playtime, backup, and restore.
- Safe executable launching with bounded argument arrays and duplicate-launch prevention.
- Portable-only ZIP, health verification, checksum, release metadata, and transactional active deployment.
