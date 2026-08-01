import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Link, useParams } from "@tanstack/react-router";
import {
  ArrowLeft,
  Clock3,
  Database,
  ExternalLink,
  FolderOpen,
  Gamepad2,
  HardDrive,
  Heart,
  PencilLine,
  Save,
  Search,
  TerminalSquare,
} from "lucide-react";
import { useState } from "react";
import { snapshotKey, useSnapshot } from "../../app/query";
import { Artwork } from "../../shared/components/Artwork";
import { StatusBadge } from "../../shared/components/StatusBadge";
import { formatBytes, formatDuration, formatRelativeDate } from "../../shared/lib/format";
import { getErrorMessage, nativeClient } from "../../shared/lib/native-client";
import type {
  Game,
  GameMetadata,
  MetadataLookupInput,
  UpdateGameInput,
} from "../../shared/lib/types";
import styles from "./GameDetailsPage.module.css";

export function GameDetailsPage() {
  const { gameId } = useParams({ from: "/library/$gameId" });
  const snapshot = useSnapshot();
  const game = snapshot.data?.games.find((candidate) => candidate.id === gameId);

  if (snapshot.isLoading) return <div className="page-state">Loading game details...</div>;
  if (!game) {
    return (
      <div className="page-state">
        <p className="eyebrow">Game unavailable</p>
        <h1>This library item no longer exists.</h1>
        <Link className="button" to="/library">
          Back to library
        </Link>
      </div>
    );
  }
  return <GameDetails key={game.id + game.updatedAt} game={game} />;
}

function GameDetails({ game }: { game: Game }) {
  const queryClient = useQueryClient();
  const [isEditing, setIsEditing] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [metadataProvider, setMetadataProvider] = useState<MetadataLookupInput["provider"]>(
    game.metadata.provider ?? "steam",
  );
  const [metadataIdentifier, setMetadataIdentifier] = useState(
    game.metadata.externalId ?? game.metadata.storeUrl ?? "",
  );
  const [form, setForm] = useState<UpdateGameInput>({
    id: game.id,
    title: game.title,
    description: game.description,
    executablePath: game.executablePath,
    launchArgs: game.launchArgs,
    tags: game.tags,
    category: game.category,
  });

  const update = useMutation({
    mutationFn: nativeClient.updateGame,
    onSuccess: () => {
      setIsEditing(false);
      setMessage("Game metadata saved.");
      void queryClient.invalidateQueries({ queryKey: snapshotKey });
    },
  });
  const favorite = useMutation({
    mutationFn: nativeClient.toggleFavorite,
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: snapshotKey }),
  });
  const launch = useMutation({
    mutationFn: nativeClient.launchGame,
    onSuccess: () => setMessage("Launch request sent to Windows."),
  });
  const openFolder = useMutation({ mutationFn: nativeClient.openGameFolder });
  const metadataLookup = useMutation({
    mutationFn: nativeClient.lookupGameMetadata,
    onSuccess: (metadata) =>
      setMessage(`${metadata.title ?? game.title} was found on the official store.`),
  });
  const saveOfficialMetadata = useMutation({
    mutationFn: (metadata: GameMetadata) => nativeClient.saveGameMetadata(game.id, metadata),
    onSuccess: () => {
      setMessage("Official store metadata saved to this library item.");
      void queryClient.invalidateQueries({ queryKey: snapshotKey });
    },
  });
  const isReady = ["detected", "configured"].includes(game.detectionStatus);
  const error =
    update.error ??
    favorite.error ??
    launch.error ??
    openFolder.error ??
    metadataLookup.error ??
    saveOfficialMetadata.error;

  const chooseExecutable = async () => {
    const selected = await nativeClient.chooseGameExecutable(game.installPath);
    if (selected) setForm((current) => ({ ...current, executablePath: selected }));
  };

  return (
    <div className={styles.page}>
      <Link to="/library" className={styles.back}>
        <ArrowLeft aria-hidden="true" size={17} />
        Library
      </Link>

      <section className={styles.hero}>
        <Artwork
          seed={game.artworkSeed}
          title={game.title}
          imageUrl={game.metadata.heroUrl ?? game.metadata.coverUrl}
          className={styles.heroArt}
        />
        <div className={styles.heroShade} />
        <div className={styles.heroContent}>
          <div className={styles.statusLine}>
            <StatusBadge status={game.detectionStatus} />
            <span>{game.category || "Uncategorized"}</span>
          </div>
          <h1>{game.title}</h1>
          <p>{game.description || "Add a personal description for this local game."}</p>
          <div className={styles.actions}>
            <button
              className="button primary"
              type="button"
              disabled={!isReady || launch.isPending}
              onClick={() => launch.mutate(game.id)}
            >
              <Gamepad2 aria-hidden="true" size={18} />
              {launch.isPending ? "Starting..." : "Play"}
            </button>
            <button
              className="button"
              type="button"
              disabled={openFolder.isPending}
              onClick={() => openFolder.mutate(game.id)}
            >
              <FolderOpen aria-hidden="true" size={18} />
              Open folder
            </button>
            <button
              className="icon-button"
              type="button"
              aria-label={`${game.favorite ? "Remove from" : "Add to"} favorites`}
              aria-pressed={game.favorite}
              disabled={favorite.isPending}
              onClick={() => favorite.mutate(game.id)}
            >
              <Heart aria-hidden="true" size={18} fill={game.favorite ? "currentColor" : "none"} />
            </button>
          </div>
        </div>
      </section>

      {message ? (
        <div className="notice" role="status">
          {message}
        </div>
      ) : null}
      {error ? (
        <div className="notice error" role="alert">
          {getErrorMessage(error)}
        </div>
      ) : null}
      {!isReady ? (
        <div className="notice error">
          The configured file is not currently available. Reconnect the drive or choose another
          executable.
        </div>
      ) : null}

      <div className={styles.content}>
        <section className={styles.details}>
          <div className="section-heading">
            <div>
              <p className="eyebrow">Personal metadata</p>
              <h2>Game details</h2>
            </div>
            <button
              className="button"
              type="button"
              onClick={() => setIsEditing((current) => !current)}
            >
              <PencilLine aria-hidden="true" size={16} />
              {isEditing ? "Cancel" : "Edit"}
            </button>
          </div>

          {isEditing ? (
            <form
              className={styles.form}
              onSubmit={(event) => {
                event.preventDefault();
                update.mutate(form);
              }}
            >
              <label className="field-label">
                Title
                <input
                  className="field"
                  value={form.title}
                  onChange={(event) =>
                    setForm((current) => ({ ...current, title: event.target.value }))
                  }
                  required
                />
              </label>
              <label className="field-label">
                Description
                <textarea
                  className="textarea"
                  value={form.description}
                  onChange={(event) =>
                    setForm((current) => ({ ...current, description: event.target.value }))
                  }
                />
              </label>
              <div className={styles.formColumns}>
                <label className="field-label">
                  Category
                  <input
                    className="field"
                    value={form.category}
                    onChange={(event) =>
                      setForm((current) => ({ ...current, category: event.target.value }))
                    }
                  />
                </label>
                <label className="field-label">
                  Tags, comma separated
                  <input
                    className="field"
                    value={form.tags.join(", ")}
                    onChange={(event) =>
                      setForm((current) => ({
                        ...current,
                        tags: event.target.value
                          .split(",")
                          .map((tag) => tag.trim())
                          .filter(Boolean),
                      }))
                    }
                  />
                </label>
              </div>
              <label className="field-label">
                Executable
                <span className={styles.inlineField}>
                  <input className="field" value={form.executablePath} readOnly />
                  <button className="button" type="button" onClick={() => void chooseExecutable()}>
                    Choose
                  </button>
                </span>
              </label>
              <label className="field-label">
                Launch arguments, one argument per line
                <textarea
                  className="textarea"
                  value={form.launchArgs.join("\n")}
                  onChange={(event) =>
                    setForm((current) => ({
                      ...current,
                      launchArgs: event.target.value
                        .split(/\r?\n/)
                        .map((argument) => argument.trim())
                        .filter(Boolean),
                    }))
                  }
                  placeholder={"--windowed\n--language=en"}
                />
              </label>
              <div>
                <button className="button primary" type="submit" disabled={update.isPending}>
                  <Save aria-hidden="true" size={17} />
                  {update.isPending ? "Saving..." : "Save metadata"}
                </button>
              </div>
            </form>
          ) : (
            <dl className={styles.definitionList}>
              <div>
                <dt>Description</dt>
                <dd>{game.description || "No personal description yet."}</dd>
              </div>
              <div>
                <dt>Tags</dt>
                <dd>{game.tags.length ? game.tags.join(" · ") : "No tags"}</dd>
              </div>
              <div>
                <dt>Detection source</dt>
                <dd>{game.detectionSource}</dd>
              </div>
            </dl>
          )}
        </section>

        <aside className={styles.sidebar}>
          <div className={styles.metric}>
            <Clock3 aria-hidden="true" size={18} />
            <span>
              <strong>{formatDuration(game.playtimeSeconds)}</strong>
              <small>Tracked playtime</small>
            </span>
          </div>
          <div className={styles.metric}>
            <HardDrive aria-hidden="true" size={18} />
            <span>
              <strong>{formatBytes(game.folderSizeBytes)}</strong>
              <small>Scanned folder size</small>
            </span>
          </div>
          <div className={styles.metric}>
            <Gamepad2 aria-hidden="true" size={18} />
            <span>
              <strong>{formatRelativeDate(game.lastPlayedAt)}</strong>
              <small>Last played</small>
            </span>
          </div>
          <div className={styles.pathBlock}>
            <span>
              <FolderOpen aria-hidden="true" size={15} />
              Install folder
            </span>
            <code>{game.installPath}</code>
          </div>
          <div className={styles.pathBlock}>
            <span>
              <TerminalSquare aria-hidden="true" size={15} />
              Executable
            </span>
            <code>{game.executablePath || "Not configured"}</code>
          </div>
        </aside>
      </div>

      <section className={styles.metadataPanel}>
        <div className="section-heading">
          <div>
            <p className="eyebrow">Official catalog</p>
            <h2>Steam, GOG, or Epic metadata</h2>
            <p>
              Fetch public product details directly from an official store. Personal edits remain
              intact when metadata is saved.
            </p>
          </div>
          {game.metadata.provider ? (
            <span className={styles.sourceBadge}>
              <Database aria-hidden="true" size={15} />
              {game.metadata.provider.toUpperCase()} linked
            </span>
          ) : null}
        </div>
        <form
          className={styles.metadataForm}
          onSubmit={(event) => {
            event.preventDefault();
            metadataLookup.mutate({
              provider: metadataProvider,
              identifier: metadataIdentifier.trim(),
            });
          }}
        >
          <label className="field-label">
            Official store
            <select
              className="field"
              value={metadataProvider}
              onChange={(event) => {
                setMetadataProvider(event.target.value as MetadataLookupInput["provider"]);
                metadataLookup.reset();
              }}
            >
              <option value="steam">Steam</option>
              <option value="gog">GOG</option>
              <option value="epic">Epic Games Store</option>
            </select>
          </label>
          <label className="field-label">
            {metadataProvider === "steam" ? "Steam App ID or official URL" : "Official product URL"}
            <input
              className="field"
              value={metadataIdentifier}
              onChange={(event) => setMetadataIdentifier(event.target.value)}
              placeholder={
                metadataProvider === "steam"
                  ? "Example: 440"
                  : metadataProvider === "gog"
                    ? "https://www.gog.com/en/game/..."
                    : "https://store.epicgames.com/..."
              }
              required
            />
          </label>
          <div className={styles.metadataActions}>
            <button
              className="button"
              type="button"
              onClick={() =>
                void nativeClient.openOfficialStoreSearch(metadataProvider, game.title)
              }
            >
              <ExternalLink aria-hidden="true" size={16} />
              Search official store
            </button>
            <button className="button primary" type="submit" disabled={metadataLookup.isPending}>
              <Search aria-hidden="true" size={16} />
              {metadataLookup.isPending ? "Fetching..." : "Fetch metadata"}
            </button>
          </div>
        </form>
        {metadataLookup.data ? (
          <div className={styles.metadataPreview}>
            <Artwork
              seed={game.artworkSeed}
              title={metadataLookup.data.title ?? game.title}
              imageUrl={metadataLookup.data.coverUrl ?? metadataLookup.data.heroUrl}
              className={styles.metadataArtwork}
            />
            <div>
              <p className="eyebrow">Preview from {metadataLookup.data.provider}</p>
              <h3>{metadataLookup.data.title ?? game.title}</h3>
              <p>
                {metadataLookup.data.shortDescription ||
                  "The official page did not publish a short description."}
              </p>
              <small>
                {metadataLookup.data.developers.join(" · ") || "Developer not published"}
                {metadataLookup.data.releaseDate
                  ? ` · Released ${metadataLookup.data.releaseDate}`
                  : ""}
              </small>
              <div className={styles.metadataPreviewActions}>
                <button
                  className="button primary"
                  type="button"
                  disabled={saveOfficialMetadata.isPending}
                  onClick={() => saveOfficialMetadata.mutate(metadataLookup.data)}
                >
                  <Save aria-hidden="true" size={16} />
                  {saveOfficialMetadata.isPending ? "Saving..." : "Save official metadata"}
                </button>
              </div>
            </div>
          </div>
        ) : game.metadata.provider ? (
          <dl className={styles.metadataSummary}>
            <div>
              <dt>Official source</dt>
              <dd>{game.metadata.storeUrl}</dd>
            </div>
            <div>
              <dt>Developers</dt>
              <dd>{game.metadata.developers.join(" · ") || "Not published"}</dd>
            </div>
            <div>
              <dt>Genres</dt>
              <dd>{game.metadata.genres.join(" · ") || "Not published"}</dd>
            </div>
          </dl>
        ) : (
          <p className={styles.metadataHint}>
            Use the official search to find the product page, then paste its URL here. Steam also
            accepts the numeric App ID.
          </p>
        )}
      </section>
    </div>
  );
}
