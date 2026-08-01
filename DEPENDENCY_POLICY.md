# Dependency policy

Dependabot groups compatible minor and patch updates by ecosystem. Every group must pass the complete repository verification gate before merge.

Automated version-update pull requests intentionally exclude semantic-major upgrades. Major upgrades are reviewed deliberately against migration notes, database and archive compatibility, release behavior, and rollback plans. Security updates remain enabled and are evaluated independently of this cadence.

GitHub Actions are pinned to immutable commit identities with their release tag noted in comments. Dependabot may propose monthly action updates; maintainers verify the upstream repository and release notes before merge.
