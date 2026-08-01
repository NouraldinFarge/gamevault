import { z } from "zod";

export const emptyGameMetadata = {
  provider: null,
  externalId: null,
  storeUrl: null,
  title: null,
  shortDescription: null,
  coverUrl: null,
  heroUrl: null,
  developers: [],
  publishers: [],
  genres: [],
  releaseDate: null,
  website: null,
  minimumRequirements: null,
  recommendedRequirements: null,
  fetchedAt: null,
};

export const GameMetadataSchema = z.object({
  provider: z.enum(["steam", "gog", "epic"]).nullable(),
  externalId: z.string().nullable(),
  storeUrl: z.string().nullable(),
  title: z.string().nullable(),
  shortDescription: z.string().nullable(),
  coverUrl: z.string().url().nullable(),
  heroUrl: z.string().url().nullable(),
  developers: z.array(z.string()),
  publishers: z.array(z.string()),
  genres: z.array(z.string()),
  releaseDate: z.string().nullable(),
  website: z.string().nullable(),
  minimumRequirements: z.string().nullable(),
  recommendedRequirements: z.string().nullable(),
  fetchedAt: z.string().nullable(),
});

export const GameSchema = z.object({
  id: z.string(),
  title: z.string(),
  description: z.string(),
  installPath: z.string(),
  executablePath: z.string(),
  launchArgs: z.array(z.string()),
  tags: z.array(z.string()),
  category: z.string(),
  favorite: z.boolean(),
  detectionStatus: z.enum(["detected", "configured", "missing", "unavailable"]),
  detectionSource: z.string(),
  folderSizeBytes: z.number().nullable(),
  lastPlayedAt: z.string().nullable(),
  playtimeSeconds: z.number(),
  addedAt: z.string(),
  updatedAt: z.string(),
  contentSignature: z.string(),
  artworkSeed: z.number(),
  metadata: GameMetadataSchema.default(emptyGameMetadata),
});

export const SettingsSchema = z.object({
  managedRoot: z.string(),
  libraryRoots: z.array(z.string()),
  scanDepth: z.number().int().min(1).max(8),
  theme: z.enum(["midnight", "deep-blue", "high-contrast"]),
  defaultLaunchArgs: z.array(z.string()),
  loggingEnabled: z.boolean(),
  lastScanAt: z.string().nullable(),
});

export const SnapshotSchema = z.object({
  games: z.array(GameSchema),
  settings: SettingsSchema,
  stats: z.object({
    totalGames: z.number(),
    readyGames: z.number(),
    missingGames: z.number(),
    favorites: z.number(),
    totalPlaytimeSeconds: z.number(),
  }),
  portableRoot: z.string(),
  sqliteVersion: z.string(),
  scanInProgress: z.boolean(),
});

export const ScanProgressSchema = z.object({
  root: z.string(),
  currentFolder: z.string(),
  foldersScanned: z.number(),
  foldersTotal: z.number(),
  gamesDetected: z.number(),
  message: z.string(),
});

export const HealthReportSchema = z.object({
  ok: z.boolean(),
  appVersion: z.string(),
  portableRoot: z.string(),
  databasePath: z.string(),
  sqliteVersion: z.string(),
  webview2Runtime: z.string(),
});

export const WorkspaceStatusSchema = z.object({
  root: z.string(),
  ready: z.boolean(),
  folders: z.array(
    z.object({
      name: z.string(),
      path: z.string(),
      exists: z.boolean(),
      itemCount: z.number(),
    }),
  ),
});

export const DependencyAuditSchema = z.object({
  auditedAt: z.string(),
  managedRoot: z.string(),
  redistFolders: z.number(),
  filesInspected: z.number(),
  installed: z.number(),
  missing: z.number(),
  suspicious: z.number(),
  officialSourcesReachable: z.boolean(),
  reportPath: z.string(),
  items: z.array(
    z.object({
      id: z.string(),
      name: z.string(),
      architecture: z.string(),
      bundledPath: z.string(),
      bundledVersion: z.string().nullable(),
      sha256: z.string(),
      signatureStatus: z.string(),
      publisher: z.string().nullable(),
      installedStatus: z.string(),
      installedVersion: z.string().nullable(),
      officialSourceUrl: z.string().nullable(),
      onlineStatus: z.string(),
      recommendation: z.string(),
    }),
  ),
});

export const ArchiveInspectionSchema = z.object({
  archivePath: z.string(),
  archiveName: z.string(),
  archiveSizeBytes: z.number(),
  valid: z.boolean(),
  extractor: z.string(),
  fileCount: z.number(),
  unpackedSizeBytes: z.number(),
  executableCandidates: z.array(z.string()),
  warnings: z.array(z.string()),
  canStage: z.boolean(),
});

export const StagedArchiveSchema = z.object({
  archivePath: z.string(),
  stagingPath: z.string(),
  filesExtracted: z.number(),
  executableCandidates: z.array(z.string()),
  warnings: z.array(z.string()),
  reportPath: z.string(),
});

export const InboxArchiveSchema = z.object({
  path: z.string(),
  name: z.string(),
  sizeBytes: z.number(),
  modifiedAt: z.string().nullable(),
});

export const StagedPackageAnalysisSchema = z.object({
  stagingPath: z.string(),
  suggestedTitle: z.string(),
  executableCandidates: z.array(
    z.object({
      executablePath: z.string(),
      installRoot: z.string(),
      displayName: z.string(),
      score: z.number(),
    }),
  ),
  redistFolders: z.array(z.string()),
  packageExtras: z.array(z.string()),
  nestedArchives: z.array(z.string()),
  suspiciousMarkers: z.array(z.string()),
  blocked: z.boolean(),
  canInstall: z.boolean(),
  warnings: z.array(z.string()),
});

export const InstalledPackageSchema = z.object({
  game: GameSchema,
  installedPath: z.string(),
  backupPath: z.string().nullable(),
  dependenciesPath: z.string().nullable(),
  extrasPath: z.string().nullable(),
  archivedPackagePath: z.string().nullable(),
  updated: z.boolean(),
  warnings: z.array(z.string()),
  reportPath: z.string(),
});

export type Game = z.infer<typeof GameSchema>;
export type Settings = z.infer<typeof SettingsSchema>;
export type AppSnapshot = z.infer<typeof SnapshotSchema>;
export type ScanProgress = z.infer<typeof ScanProgressSchema>;
export type HealthReport = z.infer<typeof HealthReportSchema>;
export type WorkspaceStatus = z.infer<typeof WorkspaceStatusSchema>;
export type DependencyAudit = z.infer<typeof DependencyAuditSchema>;
export type ArchiveInspection = z.infer<typeof ArchiveInspectionSchema>;
export type StagedArchive = z.infer<typeof StagedArchiveSchema>;
export type InboxArchive = z.infer<typeof InboxArchiveSchema>;
export type StagedPackageAnalysis = z.infer<typeof StagedPackageAnalysisSchema>;
export type InstalledPackage = z.infer<typeof InstalledPackageSchema>;
export type GameMetadata = z.infer<typeof GameMetadataSchema>;

export type ScanResult = {
  foldersScanned: number;
  gamesDetected: number;
  gamesAdded: number;
  gamesUpdated: number;
  unavailableRoots: string[];
  completedAt: string;
};

export type UpdateGameInput = {
  id: string;
  title: string;
  description: string;
  executablePath: string;
  launchArgs: string[];
  tags: string[];
  category: string;
};

export type InstallStagedInput = {
  stagingPath: string;
  executablePath: string;
  title: string;
  archivePath: string | null;
};

export type MetadataLookupInput = {
  provider: "steam" | "gog" | "epic";
  identifier: string;
};

export type NativeError = {
  code: string;
  message: string;
  retryable: boolean;
  diagnosticId: string;
};
