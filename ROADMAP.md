# Roadmap

GameVault prioritizes trustworthy local-library workflows over automated acquisition or opaque convenience. Roadmap items describe direction, not delivery promises.

## Shipped foundation

- Portable-relative SQLite, configuration, logs, and managed `library/` state
- Searchable offline library, launch history, favorites, status, and metadata correction
- Review-gated ZIP preflight, integrity testing, isolated staging, promotion, quarantine, and rollback
- Windows path, NTFS stream, case-collision, link/reparse, expansion, and modified-platform-marker defenses
- Official Steam, GOG, and Epic metadata allowlists with failure-tolerant local launch
- Verified Windows releases with checksum, SPDX SBOM, SLSA provenance, CodeQL, and locked dependency audits

## Next

- Add property-based and fuzz coverage for archive-listing and Windows-path normalization helpers.
- Improve backup, recovery, and library-migration observability in the interface.
- Expand keyboard-first flows and screen-reader regression coverage for archive review.
- Normalize portable ZIP timestamps and build inputs before making a byte-for-byte reproducibility claim.

## Later

- Add more user-controlled organization, metadata correction, and bulk-library tools.
- Evaluate additional official catalog sources only with strict host and schema allowlists.
- Explore Authenticode signing when a sustainable certificate and release-key process are available.

## Non-goals

- Downloading games, bypassing ownership or platform protections, or redistributing content
- Silent archive promotion or process execution
- Cross-game runtime DLL deduplication
- Cloud accounts, remote telemetry, or a mandatory online launch dependency
- Antivirus, sandbox, or ownership-verification claims
