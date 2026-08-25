# Dependency policy

Dependabot groups compatible minor and patch updates by ecosystem. Every group must pass the complete repository verification gate before merge.

Automated version-update pull requests intentionally exclude semantic-major upgrades. Major upgrades are reviewed deliberately against migration notes, database and archive compatibility, release behavior, and rollback plans. Security updates remain enabled and are evaluated independently of this cadence.

GitHub Actions are pinned to immutable commit identities with their release tag noted in comments. Dependabot may propose monthly action updates; maintainers verify the upstream repository and release notes before merge.

The complete JavaScript graph is checked with `pnpm audit`, with `pnpm audit --prod` retained as a separate shipped-runtime signal; the complete Rust lockfile is checked with `cargo audit`. Informational maintenance and soundness warnings are reviewed separately from exploitable vulnerability advisories.

The 2026-08-25 locked-graph review reported no known JavaScript or Rust vulnerabilities and 17 RustSec warnings inherited through Tauri. Twelve affect GTK3 or its macro chain and are absent from the compiled Windows target. Five mark legacy Unicode crates in Tauri's `urlpattern` chain as unmaintained; those crates are reachable during the Windows build, but RustSec does not identify an exploitable vulnerability or a compatible patched line. The warnings remain visible and must be re-evaluated on every Tauri update; they are not described as fixed or silently ignored.

Playwright, axe, fast-check, proptest, and libFuzzer support verification only and do not ship in the portable runtime. Browser automation uses the Playwright-managed Chromium version locked by the JavaScript dependency graph. The fuzz harness pins `libfuzzer-sys` and compiles as an isolated crate that includes the production archive-path source directly.
