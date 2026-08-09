# Dependency policy

Dependabot groups compatible minor and patch updates by ecosystem. Every group must pass the complete repository verification gate before merge.

Automated version-update pull requests intentionally exclude semantic-major upgrades. Major upgrades are reviewed deliberately against migration notes, database and archive compatibility, release behavior, and rollback plans. Security updates remain enabled and are evaluated independently of this cadence.

GitHub Actions are pinned to immutable commit identities with their release tag noted in comments. Dependabot may propose monthly action updates; maintainers verify the upstream repository and release notes before merge.

The complete JavaScript graph is checked with `pnpm audit`, with `pnpm audit --prod` retained as a separate shipped-runtime signal; the complete Rust lockfile is checked with `cargo audit`. Informational maintenance warnings are reviewed separately from exploitable advisories. GameVault is Windows-only, so warnings confined to Tauri's non-shipped GTK/Linux graph are documented and revisited during Tauri upgrades rather than presented as Windows release vulnerabilities.
