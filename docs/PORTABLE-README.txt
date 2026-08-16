GameVault Portable
==================

1. Extract the entire ZIP into any writable folder.
2. Run GameVault.exe.
3. Open Local files and prepare the default library folder beside the app, or
   choose another dedicated managed folder in Settings.
4. Correct an executable from a game's details page when needed.

ZIP intake:
  Place user-owned ZIP archives in library\Inbox beside the app (or the Inbox
  under your selected managed folder). Local files preflight paths, links,
  expanded size, and free space before testing, extracting, reviewing,
  cleaning, and organizing them without executing archive content.
  Suspicious platform-modification markers block installation, and an existing
  game is backed up before a staged update is promoted. Staging folders and
  operation history remain visible after a restart; interrupted work is not
  resumed automatically. Every promotion requires a current file-change
  preview, which is checked again before files move.

Redistributables:
  Local files can audit installers found under Redist folders. The audit hashes
  each file, reviews its Windows signature and expected publisher, and compares
  version evidence already present on the system. It can open approved official
  Microsoft or NVIDIA sources for your review. It never runs an installer.

Official metadata:
  Link a game to Steam, GOG, or Epic from its details page. Steam accepts an
  App ID or official product URL. GOG and Epic accept official product URLs.

All GameVault-controlled state remains in this folder:
  data\    SQLite library, metadata, playtime, and backups
  config\  portable configuration
  logs\    redacted local diagnostics
  cache\   disposable cached data
  library\ managed Inbox, Staging, Games, Archives, Dependencies, Quarantine,
           and Reports folders (unless the user selects another location)

Portable upgrades preserve data, library, user configuration, and logs. The
new build is health-checked before the previous active build is removed.

Release verification:
  Portable ZIP entries use stable ordering, timestamps, and metadata. Published
  releases include a SHA-256 checksum and GitHub provenance. The current public
  release is unsigned unless its release notes and Windows signature properties
  explicitly show otherwise; verify the published evidence before running it.

Application updates:
  Settings can manually check the official GameVault GitHub release endpoint.
  GameVault can open the exact release page for review but does not download or
  install application updates automatically.

No installer, administrator rights, registry setup, service, scheduled task,
shell extension, or uninstaller is used. Remove GameVault by closing it and
deleting its folder.

Windows prerequisite:
  Microsoft Edge WebView2 Evergreen Runtime. It is normally present on current
  Windows 10 and Windows 11 systems. This release does not bundle a fixed
  WebView2 runtime.

Legal:
  Use GameVault only with games you own or are authorized to run. GameVault
  does not download games or bypass DRM, ownership, authentication, anti-cheat,
  or operating-system security.
