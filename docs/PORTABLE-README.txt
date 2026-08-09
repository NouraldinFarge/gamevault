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
  game is backed up before a staged update is promoted.

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
