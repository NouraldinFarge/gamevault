# Roadmap

GameVault prioritizes trustworthy local-library workflows over automated acquisition or opaque convenience. Roadmap items describe direction, not delivery promises.

## Shipped foundation

- Portable-relative SQLite, configuration, logs, and managed `library/` state
- Searchable offline library, launch history, favorites, status, and metadata correction
- Review-gated ZIP preflight, integrity testing, isolated staging, promotion, quarantine, and rollback
- Windows path, NTFS stream, case-collision, link/reparse, expansion, and modified-platform-marker defenses
- Official Steam, GOG, and Epic metadata allowlists with failure-tolerant local launch
- Persistent operation history, startup interruption reconciliation, and a visible Staging recovery queue
- File-level install/update previews with a promotion-time SHA-256 freshness gate and rollback backups
- Evidence-rich Redist audits with signature, publisher, installed-version, and official-source checks
- Manual, official GitHub release checks without automatic download or installation
- Deterministic portable ZIP entry order, metadata, and timestamps with an identical-input byte check
- Signing-ready release hook that fails closed unless certificate, password, and HTTPS timestamp configuration are all present
- Verified Windows releases with checksum, SPDX SBOM, SLSA provenance, CodeQL, and locked dependency audits

## Next

- Expand manual screen-reader coverage beyond the automated keyboard and axe regression suite.
- Normalize compiler-produced build inputs before making a full source-to-release byte-for-byte reproducibility claim.

## Later

- Add more user-controlled organization, metadata correction, and bulk-library tools.
- Evaluate additional official catalog sources only with strict host and schema allowlists.
- Activate the existing signing hook when a sustainable Authenticode certificate and protected key process are available.

## Non-goals

- Downloading games, bypassing ownership or platform protections, or redistributing content
- Silent archive promotion or process execution
- Cross-game runtime DLL deduplication
- Cloud accounts, remote telemetry, or a mandatory online launch dependency
- Antivirus, sandbox, or ownership-verification claims
