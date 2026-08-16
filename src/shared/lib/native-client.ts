import { invoke } from "@tauri-apps/api/core";
import { demoSnapshot } from "./demo-data";
import type {
  AppSnapshot,
  AppUpdateCheck,
  ArchiveInspection,
  DependencyAudit,
  Game,
  GameMetadata,
  HealthReport,
  InboxArchive,
  InstalledPackage,
  InstallStagedInput,
  MetadataLookupInput,
  OperationRecord,
  PreviewStagedUpdateInput,
  ScanResult,
  Settings,
  StagedArchive,
  StagedPackageAnalysis,
  StagedUpdatePreview,
  StagingPackage,
  UpdateGameInput,
  WorkspaceStatus,
} from "./types";
import {
  AppUpdateCheckSchema,
  ArchiveInspectionSchema,
  DependencyAuditSchema,
  GameMetadataSchema,
  GameSchema,
  HealthReportSchema,
  InstalledPackageSchema,
  OperationRecordSchema,
  SettingsSchema,
  SnapshotSchema,
  StagedArchiveSchema,
  StagedPackageAnalysisSchema,
  StagedUpdatePreviewSchema,
  StagingPackageSchema,
  WorkspaceStatusSchema,
} from "./types";

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

export const isDesktopRuntime = () => typeof window !== "undefined" && !!window.__TAURI_INTERNALS__;

const mockSnapshot = structuredClone(demoSnapshot);
const mockStagingPackages: StagingPackage[] = [];
const mockOperations: OperationRecord[] = [
  {
    id: "demo-operation-scan",
    kind: "library-scan",
    label: "Scan local library",
    status: "completed",
    sourcePath: null,
    targetPath: mockSnapshot.settings.managedRoot,
    summary: `Checked ${mockSnapshot.games.length} folders and detected ${mockSnapshot.games.length} games.`,
    errorMessage: null,
    recoveryHint: "Run a new scan; GameVault never resumes a filesystem scan silently.",
    reportPath: null,
    startedAt: new Date(Date.now() - 3_600_000).toISOString(),
    updatedAt: new Date(Date.now() - 3_590_000).toISOString(),
    completedAt: new Date(Date.now() - 3_590_000).toISOString(),
  },
];

const refreshMockStats = () => {
  mockSnapshot.stats = {
    totalGames: mockSnapshot.games.length,
    readyGames: mockSnapshot.games.filter((game) =>
      ["detected", "configured"].includes(game.detectionStatus),
    ).length,
    missingGames: mockSnapshot.games.filter((game) =>
      ["missing", "unavailable"].includes(game.detectionStatus),
    ).length,
    favorites: mockSnapshot.games.filter((game) => game.favorite).length,
    totalPlaytimeSeconds: mockSnapshot.games.reduce(
      (total, game) => total + game.playtimeSeconds,
      0,
    ),
  };
};

const pause = (milliseconds = 250) =>
  new Promise<void>((resolve) => window.setTimeout(resolve, milliseconds));

export const nativeClient = {
  async getSnapshot(): Promise<AppSnapshot> {
    if (!isDesktopRuntime()) return SnapshotSchema.parse(structuredClone(mockSnapshot));
    return SnapshotSchema.parse(await invoke("get_snapshot"));
  },

  async scanLibrary(): Promise<ScanResult> {
    if (!isDesktopRuntime()) {
      await pause(700);
      mockSnapshot.settings.lastScanAt = new Date().toISOString();
      return {
        foldersScanned: mockSnapshot.games.length,
        gamesDetected: mockSnapshot.games.length,
        gamesAdded: 0,
        gamesUpdated: mockSnapshot.games.length,
        unavailableRoots: [],
        completedAt: new Date().toISOString(),
      };
    }
    return invoke<ScanResult>("scan_library");
  },

  async chooseLibraryDirectory(): Promise<string | null> {
    if (!isDesktopRuntime()) return "D:\\Games";
    return invoke<string | null>("choose_library_directory");
  },

  async chooseGameExecutable(initialDirectory?: string): Promise<string | null> {
    if (!isDesktopRuntime()) return null;
    return invoke<string | null>("choose_game_executable", {
      initialDirectory: initialDirectory ?? null,
    });
  },

  async chooseBackupFile(): Promise<string | null> {
    if (!isDesktopRuntime()) return null;
    return invoke<string | null>("choose_backup_file");
  },

  async saveSettings(settings: Settings): Promise<Settings> {
    if (!isDesktopRuntime()) {
      mockSnapshot.settings = SettingsSchema.parse(settings);
      return structuredClone(mockSnapshot.settings);
    }
    return SettingsSchema.parse(await invoke("save_settings", { settings }));
  },

  async addManualGame(executablePath: string): Promise<Game> {
    if (!isDesktopRuntime()) throw new Error("Choose an executable in the packaged desktop app.");
    return GameSchema.parse(await invoke("add_manual_game", { executablePath }));
  },

  async updateGame(input: UpdateGameInput): Promise<Game> {
    if (!isDesktopRuntime()) {
      const index = mockSnapshot.games.findIndex((game) => game.id === input.id);
      if (index < 0) throw new Error("Game not found.");
      mockSnapshot.games[index] = {
        ...mockSnapshot.games[index],
        ...input,
        detectionStatus: "configured",
        detectionSource: "manual",
        updatedAt: new Date().toISOString(),
      };
      return structuredClone(mockSnapshot.games[index]);
    }
    return GameSchema.parse(await invoke("update_game", { input }));
  },

  async toggleFavorite(id: string): Promise<Game> {
    if (!isDesktopRuntime()) {
      const game = mockSnapshot.games.find((candidate) => candidate.id === id);
      if (!game) throw new Error("Game not found.");
      game.favorite = !game.favorite;
      refreshMockStats();
      return structuredClone(game);
    }
    return GameSchema.parse(await invoke("toggle_favorite", { id }));
  },

  async launchGame(id: string): Promise<void> {
    if (!isDesktopRuntime()) {
      await pause();
      return;
    }
    return invoke("launch_game", { id });
  },

  async openGameFolder(id: string): Promise<void> {
    if (!isDesktopRuntime()) return;
    return invoke("open_game_folder", { id });
  },

  async createDatabaseBackup(): Promise<string> {
    if (!isDesktopRuntime()) return "D:\\Portable Apps\\GameVault\\data\\backups\\demo.db";
    return invoke<string>("create_database_backup");
  },

  async restoreDatabaseBackup(backupPath: string): Promise<void> {
    if (!isDesktopRuntime()) return;
    return invoke("restore_database_backup", { backupPath });
  },

  async clearCache(): Promise<void> {
    if (!isDesktopRuntime()) return;
    return invoke("clear_application_cache");
  },

  async openLogsFolder(): Promise<void> {
    if (!isDesktopRuntime()) return;
    return invoke("open_logs_folder");
  },

  async getHealthReport(): Promise<HealthReport> {
    if (!isDesktopRuntime()) {
      return {
        ok: true,
        appVersion: "0.3.5",
        portableRoot: mockSnapshot.portableRoot,
        databasePath: `${mockSnapshot.portableRoot}\\data\\library.db`,
        sqliteVersion: mockSnapshot.sqliteVersion,
        webview2Runtime: "Browser preview",
      };
    }
    return HealthReportSchema.parse(await invoke("get_health_report"));
  },

  async getWorkspaceStatus(): Promise<WorkspaceStatus> {
    if (!isDesktopRuntime()) {
      return {
        root: mockSnapshot.settings.managedRoot,
        ready: true,
        folders: [
          "App",
          "Inbox",
          "Staging",
          "Games",
          "Archives",
          "Dependencies",
          "Quarantine",
          "Reports",
        ].map((name) => ({
          name,
          path: `${mockSnapshot.settings.managedRoot}\\${name}`,
          exists: true,
          itemCount: name === "Games" ? mockSnapshot.games.length : 0,
        })),
      };
    }
    return WorkspaceStatusSchema.parse(await invoke("get_workspace_status"));
  },

  async prepareWorkspace(): Promise<WorkspaceStatus> {
    if (!isDesktopRuntime()) {
      return {
        root: mockSnapshot.settings.managedRoot,
        ready: true,
        folders: [
          "App",
          "Inbox",
          "Staging",
          "Games",
          "Archives",
          "Dependencies",
          "Quarantine",
          "Reports",
        ].map((name) => ({
          name,
          path: `${mockSnapshot.settings.managedRoot}\\${name}`,
          exists: true,
          itemCount: name === "Games" ? mockSnapshot.games.length : 0,
        })),
      };
    }
    return WorkspaceStatusSchema.parse(await invoke("prepare_workspace"));
  },

  async auditDependencies(): Promise<DependencyAudit> {
    if (!isDesktopRuntime()) {
      await pause(500);
      return DependencyAuditSchema.parse({
        auditedAt: new Date().toISOString(),
        managedRoot: mockSnapshot.settings.managedRoot,
        redistFolders: 2,
        filesInspected: 14,
        installed: 1,
        missing: 0,
        suspicious: 1,
        officialSourcesReachable: true,
        reportPath: `${mockSnapshot.settings.managedRoot}\\Reports\\dependency-audit-demo.json`,
        items: [
          {
            id: "demo-vc-x64",
            name: "Microsoft Visual C++ 2015–2022",
            architecture: "x64",
            bundledPath: `${mockSnapshot.settings.managedRoot}\\Dependencies\\Bundled\\Example Game\\Redist\\vc_redist.x64.exe`,
            bundledVersion: "14.44.35211.0",
            sha256: "c5e68d3f5a8dd27e5bbf3f22551d36dcdfb32f66f2a4677d9f882999839ef865",
            signatureStatus: "valid",
            publisher: "CN=Microsoft Corporation",
            installedStatus: "installed",
            installedVersion: "v14.44.35211.0",
            officialSourceUrl: "https://aka.ms/vc14/vc_redist.x64.exe",
            onlineStatus: "reachable",
            recommendation: "Already installed; no bundled installer needs to run.",
            detectedBy:
              "Filename matched the official vc_redist.x64 naming pattern; identity still requires signature and publisher verification.",
            installedEvidence: [
              "HKLM\\SOFTWARE\\Microsoft\\VisualStudio\\14.0\\VC\\Runtimes\\x64: Installed=1",
            ],
            confidence: "high",
            publisherMatch: "verified",
            checkedAt: new Date().toISOString(),
          },
          {
            id: "demo-unknown",
            name: "Unrecognized bundled installer",
            architecture: "unknown",
            bundledPath: `${mockSnapshot.settings.managedRoot}\\Dependencies\\Bundled\\Example Game\\Support\\setup.exe`,
            bundledVersion: null,
            sha256: "0a87b1558e3f95409a39dc5ecfd2ea4aa0dd1f6d69a38bcf057fef87072b62d3",
            signatureStatus: "unsigned",
            publisher: null,
            installedStatus: "unknown",
            installedVersion: null,
            officialSourceUrl: null,
            onlineStatus: "not available",
            recommendation:
              "Leave this installer quarantined until its publisher and purpose are verified.",
            detectedBy: "No recognized prerequisite filename pattern matched.",
            installedEvidence: ["No supported installed-state detector matched this file."],
            confidence: "low",
            publisherMatch: "not evaluated",
            checkedAt: new Date().toISOString(),
          },
        ],
      });
    }
    return DependencyAuditSchema.parse(await invoke("audit_dependencies"));
  },

  async openOfficialDependencySource(url: string): Promise<void> {
    if (!isDesktopRuntime()) {
      window.open(url, "_blank", "noopener,noreferrer");
      return;
    }
    return invoke("open_official_dependency_source", { url });
  },

  async chooseGameArchive(): Promise<string | null> {
    if (!isDesktopRuntime())
      return "D:\\Portable Apps\\GameVault\\library\\Inbox\\Example Game.zip";
    return invoke<string | null>("choose_game_archive");
  },

  async inspectGameArchive(archivePath: string): Promise<ArchiveInspection> {
    if (!isDesktopRuntime()) {
      await pause(500);
      return {
        archivePath,
        archiveName: archivePath.split("\\").at(-1) ?? "Example Game.zip",
        archiveSizeBytes: 2_400_000_000,
        valid: true,
        extractor: "7-Zip",
        fileCount: 1284,
        unpackedSizeBytes: 4_800_000_000,
        executableCandidates: ["Example Game\\ExampleGame.exe"],
        warnings: ["Contains a Redist folder; audit it before installing anything."],
        canStage: true,
      };
    }
    return ArchiveInspectionSchema.parse(await invoke("inspect_game_archive", { archivePath }));
  },

  async stageGameArchive(archivePath: string): Promise<StagedArchive> {
    if (!isDesktopRuntime()) {
      await pause(500);
      const staged = {
        archivePath,
        stagingPath: "D:\\Portable Apps\\GameVault\\library\\Staging\\Example Game-20260718-120000",
        filesExtracted: 1284,
        executableCandidates: [
          "D:\\Portable Apps\\GameVault\\library\\Staging\\Example Game-20260718-120000\\Example Game\\ExampleGame.exe",
        ],
        warnings: ["Contains a Redist folder; audit it before installing anything."],
        reportPath: "D:\\Portable Apps\\GameVault\\library\\Reports\\archive-intake-demo.json",
      };
      mockStagingPackages.splice(0, mockStagingPackages.length, {
        path: staged.stagingPath,
        name: staged.stagingPath.split("\\").at(-1) ?? "Example Game",
        fileCount: staged.filesExtracted,
        modifiedAt: new Date().toISOString(),
        reviewable: true,
        recoveryHint: "Review this staged package again before choosing a title and executable.",
      });
      mockOperations.unshift({
        id: `demo-stage-${Date.now()}`,
        kind: "archive-stage",
        label: "Verify and stage ZIP archive",
        status: "completed",
        sourcePath: archivePath,
        targetPath: staged.stagingPath,
        summary: `Staged ${staged.filesExtracted} files for explicit review.`,
        errorMessage: null,
        recoveryHint:
          "Review the Staging recovery queue and re-run analysis; staged content is never resumed silently.",
        reportPath: staged.reportPath,
        startedAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        completedAt: new Date().toISOString(),
      });
      return staged;
    }
    return StagedArchiveSchema.parse(await invoke("stage_game_archive", { archivePath }));
  },

  async listInboxArchives(): Promise<InboxArchive[]> {
    if (!isDesktopRuntime()) return [];
    return invoke<InboxArchive[]>("list_inbox_archives");
  },

  async listStagingPackages(): Promise<StagingPackage[]> {
    if (!isDesktopRuntime())
      return StagingPackageSchema.array().parse(structuredClone(mockStagingPackages));
    return StagingPackageSchema.array().parse(await invoke("list_staging_packages"));
  },

  async getOperationHistory(): Promise<OperationRecord[]> {
    if (!isDesktopRuntime())
      return OperationRecordSchema.array().parse(structuredClone(mockOperations));
    return OperationRecordSchema.array().parse(await invoke("get_operation_history"));
  },

  async analyzeStagedPackage(stagingPath: string): Promise<StagedPackageAnalysis> {
    if (!isDesktopRuntime()) {
      return StagedPackageAnalysisSchema.parse({
        stagingPath,
        suggestedTitle: "Example Game",
        executableCandidates: [
          {
            executablePath: `${stagingPath}\\Example Game\\ExampleGame.exe`,
            installRoot: `${stagingPath}\\Example Game`,
            displayName: "Example Game",
            score: 200,
          },
        ],
        redistFolders: [`${stagingPath}\\Example Game\\Redist`],
        packageExtras: [],
        nestedArchives: [],
        suspiciousMarkers: [],
        blocked: false,
        canInstall: true,
        warnings: ["1 redistributable folder will be separated for the dependency audit."],
      });
    }
    return StagedPackageAnalysisSchema.parse(
      await invoke("analyze_staged_package", { stagingPath }),
    );
  },

  async previewStagedUpdate(input: PreviewStagedUpdateInput): Promise<StagedUpdatePreview> {
    if (!isDesktopRuntime()) {
      await pause(450);
      const isUpdate = input.title.trim().toLowerCase() === "neon divide";
      return StagedUpdatePreviewSchema.parse({
        isUpdate,
        destinationPath: `${mockSnapshot.settings.managedRoot}\\Games\\${input.title.trim()}`,
        rollbackRoot: `${mockSnapshot.settings.managedRoot}\\Archives\\Updates`,
        addedCount: isUpdate ? 12 : 1_172,
        changedCount: isUpdate ? 84 : 0,
        removedCount: isUpdate ? 3 : 0,
        unchangedCount: isUpdate ? 1_073 : 0,
        addedSample: isUpdate
          ? ["content/new-level.pak", "bin/helper.dll"]
          : ["ExampleGame.exe", "content/base.pak"],
        changedSample: isUpdate ? ["ExampleGame.exe", "content/base.pak"] : [],
        removedSample: isUpdate ? ["content/obsolete.pak"] : [],
        currentSizeBytes: isUpdate ? 4_300_000_000 : 0,
        proposedSizeBytes: 4_800_000_000,
        fingerprint: "d".repeat(64),
      });
    }
    return StagedUpdatePreviewSchema.parse(await invoke("preview_staged_update", { input }));
  },

  async installStagedPackage(input: InstallStagedInput): Promise<InstalledPackage> {
    if (!isDesktopRuntime()) {
      const game = structuredClone(mockSnapshot.games[0]);
      return InstalledPackageSchema.parse({
        game: { ...game, title: input.title },
        installedPath: `D:\\Portable Apps\\GameVault\\library\\Games\\${input.title}`,
        backupPath: null,
        dependenciesPath: `D:\\Portable Apps\\GameVault\\library\\Dependencies\\Bundled\\${input.title}`,
        extrasPath: null,
        archivedPackagePath: null,
        updated: false,
        warnings: [],
        reportPath: "D:\\Portable Apps\\GameVault\\library\\Reports\\game-install-demo.json",
        updatePreview: {
          isUpdate: false,
          destinationPath: `D:\\Portable Apps\\GameVault\\library\\Games\\${input.title}`,
          rollbackRoot: "D:\\Portable Apps\\GameVault\\library\\Archives\\Updates",
          addedCount: 1_172,
          changedCount: 0,
          removedCount: 0,
          unchangedCount: 0,
          addedSample: ["ExampleGame.exe", "content/base.pak"],
          changedSample: [],
          removedSample: [],
          currentSizeBytes: 0,
          proposedSizeBytes: 4_800_000_000,
          fingerprint: input.updateFingerprint,
        },
      });
    }
    return InstalledPackageSchema.parse(await invoke("install_staged_package", { input }));
  },

  async lookupGameMetadata(input: MetadataLookupInput): Promise<GameMetadata> {
    if (!isDesktopRuntime()) {
      await pause(400);
      return GameMetadataSchema.parse({
        provider: input.provider,
        externalId: input.identifier,
        storeUrl: "https://store.steampowered.com/app/440/",
        title: "Example Game",
        shortDescription: "Official store description preview.",
        coverUrl: null,
        heroUrl: null,
        developers: ["Example Studio"],
        publishers: [],
        genres: ["Action"],
        releaseDate: null,
        website: null,
        minimumRequirements: null,
        recommendedRequirements: null,
        fetchedAt: new Date().toISOString(),
      });
    }
    return GameMetadataSchema.parse(await invoke("lookup_game_metadata", { input }));
  },

  async saveGameMetadata(gameId: string, metadata: GameMetadata): Promise<Game> {
    if (!isDesktopRuntime()) {
      const game = mockSnapshot.games.find((candidate) => candidate.id === gameId);
      if (!game) throw new Error("Game not found.");
      game.metadata = metadata;
      if (!game.description) game.description = metadata.shortDescription ?? "";
      return GameSchema.parse(structuredClone(game));
    }
    return GameSchema.parse(await invoke("save_game_metadata", { input: { gameId, metadata } }));
  },

  async openOfficialStoreSearch(provider: string, query: string): Promise<void> {
    if (!isDesktopRuntime()) {
      const urls: Record<string, string> = {
        steam: `https://store.steampowered.com/search/?term=${encodeURIComponent(query)}`,
        gog: `https://www.gog.com/en/games?query=${encodeURIComponent(query)}`,
        epic: `https://store.epicgames.com/en-US/browse?q=${encodeURIComponent(query)}`,
      };
      window.open(urls[provider], "_blank", "noopener,noreferrer");
      return;
    }
    return invoke("open_official_store_search", { provider, query });
  },

  async checkForAppUpdate(): Promise<AppUpdateCheck> {
    if (!isDesktopRuntime()) {
      await pause(400);
      return AppUpdateCheckSchema.parse({
        currentVersion: "0.3.5",
        latestVersion: "0.3.5",
        updateAvailable: false,
        releaseUrl: "https://github.com/NouraldinFarge/gamevault/releases/tag/v0.3.5",
        publishedAt: "2026-08-08T00:00:00Z",
        checkedAt: new Date().toISOString(),
      });
    }
    return AppUpdateCheckSchema.parse(await invoke("check_for_app_update"));
  },

  async openReleasePage(url: string): Promise<void> {
    if (!isDesktopRuntime()) {
      window.open(url, "_blank", "noopener,noreferrer");
      return;
    }
    return invoke("open_release_page", { url });
  },
};

export const getErrorMessage = (error: unknown): string => {
  if (typeof error === "object" && error !== null && "message" in error) {
    const message = (error as { message?: unknown }).message;
    if (typeof message === "string") return message;
  }
  if (typeof error === "string") return error;
  return "GameVault could not complete that action.";
};
