import { invoke } from "@tauri-apps/api/core";
import { demoSnapshot } from "./demo-data";
import type {
  AppSnapshot,
  ArchiveInspection,
  DependencyAudit,
  Game,
  GameMetadata,
  HealthReport,
  InboxArchive,
  InstalledPackage,
  InstallStagedInput,
  MetadataLookupInput,
  ScanResult,
  Settings,
  StagedArchive,
  StagedPackageAnalysis,
  UpdateGameInput,
  WorkspaceStatus,
} from "./types";
import {
  ArchiveInspectionSchema,
  DependencyAuditSchema,
  GameMetadataSchema,
  GameSchema,
  HealthReportSchema,
  InstalledPackageSchema,
  SettingsSchema,
  SnapshotSchema,
  StagedArchiveSchema,
  StagedPackageAnalysisSchema,
  WorkspaceStatusSchema,
} from "./types";

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

export const isDesktopRuntime = () => typeof window !== "undefined" && !!window.__TAURI_INTERNALS__;

const mockSnapshot = structuredClone(demoSnapshot);

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
        appVersion: "0.3.4",
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
        installed: 2,
        missing: 1,
        suspicious: 1,
        officialSourcesReachable: true,
        reportPath: `${mockSnapshot.settings.managedRoot}\\Reports\\dependency-audit-demo.json`,
        items: [],
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
      return {
        archivePath,
        stagingPath: "D:\\Portable Apps\\GameVault\\library\\Staging\\Example Game-20260718-120000",
        filesExtracted: 1284,
        executableCandidates: [
          "D:\\Portable Apps\\GameVault\\library\\Staging\\Example Game-20260718-120000\\Example Game\\ExampleGame.exe",
        ],
        warnings: ["Contains a Redist folder; audit it before installing anything."],
        reportPath: "D:\\Portable Apps\\GameVault\\library\\Reports\\archive-intake-demo.json",
      };
    }
    return StagedArchiveSchema.parse(await invoke("stage_game_archive", { archivePath }));
  },

  async listInboxArchives(): Promise<InboxArchive[]> {
    if (!isDesktopRuntime()) return [];
    return invoke<InboxArchive[]>("list_inbox_archives");
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
};

export const getErrorMessage = (error: unknown): string => {
  if (typeof error === "object" && error !== null && "message" in error) {
    const message = (error as { message?: unknown }).message;
    if (typeof message === "string") return message;
  }
  if (typeof error === "string") return error;
  return "GameVault could not complete that action.";
};
