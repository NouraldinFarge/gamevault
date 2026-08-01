import { type AppSnapshot, emptyGameMetadata, type Game } from "./types";

const now = new Date();
const daysAgo = (days: number) =>
  new Date(now.getTime() - days * 24 * 60 * 60 * 1000).toISOString();

const makeGame = (
  id: string,
  title: string,
  category: string,
  seed: number,
  overrides: Partial<Game> = {},
): Game => ({
  id,
  title,
  description:
    "A locally installed game in your portable library. Add a personal description, tags, and launch preferences from the details view.",
  installPath: `D:\\Portable Apps\\GameVault\\library\\Games\\${title}`,
  executablePath: `D:\\Portable Apps\\GameVault\\library\\Games\\${title}\\${title.replaceAll(" ", "")}.exe`,
  launchArgs: [],
  tags: category === "RPG" ? ["Story", "Single-player"] : ["Local"],
  category,
  favorite: false,
  detectionStatus: "detected",
  detectionSource: "automatic",
  folderSizeBytes: (8 + seed * 3.7) * 1024 * 1024 * 1024,
  lastPlayedAt: null,
  playtimeSeconds: seed * 1843,
  addedAt: daysAgo(seed + 2),
  updatedAt: daysAgo(seed),
  contentSignature: `demo-${id}`,
  artworkSeed: seed,
  metadata: structuredClone(emptyGameMetadata),
  ...overrides,
});

const games: Game[] = [
  makeGame("demo-1", "Neon Divide", "Action", 1, {
    favorite: true,
    lastPlayedAt: daysAgo(1),
    playtimeSeconds: 9 * 3600 + 42 * 60,
    description:
      "Trace the fault line between two radiant cities in a precision action campaign built for quick, focused sessions.",
  }),
  makeGame("demo-2", "Frostline Protocol", "Strategy", 2, {
    lastPlayedAt: daysAgo(3),
    playtimeSeconds: 21 * 3600 + 15 * 60,
  }),
  makeGame("demo-3", "Echoes of Aster", "RPG", 3, {
    favorite: true,
    lastPlayedAt: daysAgo(8),
    playtimeSeconds: 47 * 3600 + 8 * 60,
  }),
  makeGame("demo-4", "Rally North", "Racing", 4),
  makeGame("demo-5", "Small Hours", "Indie", 5, {
    detectionStatus: "configured",
    detectionSource: "manual",
  }),
  makeGame("demo-6", "Signal Coast", "Adventure", 6),
  makeGame("demo-7", "Iron Orchard", "Simulation", 7, {
    detectionStatus: "missing",
    executablePath: "",
  }),
  makeGame("demo-8", "Paper Kingdoms", "Strategy", 8),
];

export const demoSnapshot: AppSnapshot = {
  games,
  settings: {
    managedRoot: "D:\\Portable Apps\\GameVault\\library",
    libraryRoots: ["D:\\Portable Apps\\GameVault\\library\\Games"],
    scanDepth: 4,
    theme: "midnight",
    defaultLaunchArgs: [],
    loggingEnabled: true,
    lastScanAt: daysAgo(0),
  },
  stats: {
    totalGames: games.length,
    readyGames: games.filter((game) => ["detected", "configured"].includes(game.detectionStatus))
      .length,
    missingGames: games.filter((game) => ["missing", "unavailable"].includes(game.detectionStatus))
      .length,
    favorites: games.filter((game) => game.favorite).length,
    totalPlaytimeSeconds: games.reduce((total, game) => total + game.playtimeSeconds, 0),
  },
  portableRoot: "D:\\Portable Apps\\GameVault",
  sqliteVersion: "3.51.3",
  scanInProgress: false,
};
