import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  Boxes,
  CircleAlert,
  ExternalLink,
  FileArchive,
  FilePlus2,
  FolderCheck,
  FolderPlus,
  HardDrive,
  PackageCheck,
  PackageOpen,
  RefreshCw,
  ShieldCheck,
} from "lucide-react";
import { useState } from "react";
import { snapshotKey, useNativeEvents, useSnapshot } from "../../app/query";
import { formatBytes, formatRelativeDate } from "../../shared/lib/format";
import { getErrorMessage, nativeClient } from "../../shared/lib/native-client";
import styles from "./LocalFilesPage.module.css";

const compactPackagePath = (value: string, root: string) => {
  const normalizedValue = value.replaceAll("/", "\\");
  const normalizedRoot = root.replaceAll("/", "\\").replace(/[\\]+$/, "");
  if (normalizedValue.toLowerCase().startsWith(`${normalizedRoot.toLowerCase()}\\`)) {
    return normalizedValue.slice(normalizedRoot.length + 1);
  }
  return normalizedValue.split("\\").at(-1) ?? normalizedValue;
};

export function LocalFilesPage() {
  const snapshot = useSnapshot();
  const queryClient = useQueryClient();
  const { scanProgress, setScanProgress } = useNativeEvents();
  const [message, setMessage] = useState<string | null>(null);
  const [selectedArchive, setSelectedArchive] = useState<string | null>(null);
  const [installTitle, setInstallTitle] = useState("");
  const [selectedExecutable, setSelectedExecutable] = useState("");
  const workspaceStatus = useQuery({
    queryKey: ["managed-workspace"],
    queryFn: nativeClient.getWorkspaceStatus,
    retry: false,
  });
  const inboxArchives = useQuery({
    queryKey: ["inbox-archives"],
    queryFn: nativeClient.listInboxArchives,
    retry: false,
  });
  const scan = useMutation({
    mutationFn: nativeClient.scanLibrary,
    onSuccess: (result) => {
      setScanProgress(null);
      setMessage(
        "Scan complete: " +
          result.gamesDetected +
          " games detected, " +
          result.gamesAdded +
          " added.",
      );
      void queryClient.invalidateQueries({ queryKey: snapshotKey });
    },
    onError: () => setScanProgress(null),
  });
  const save = useMutation({
    mutationFn: nativeClient.saveSettings,
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: snapshotKey }),
  });
  const addManual = useMutation({
    mutationFn: nativeClient.addManualGame,
    onSuccess: (game) => {
      setMessage(`${game.title} was added to the local library.`);
      void queryClient.invalidateQueries({ queryKey: snapshotKey });
    },
  });
  const prepareWorkspace = useMutation({
    mutationFn: nativeClient.prepareWorkspace,
    onSuccess: () => {
      setMessage("The managed GameVault folders are ready.");
      void workspaceStatus.refetch();
      void queryClient.invalidateQueries({ queryKey: snapshotKey });
    },
  });
  const dependencyAudit = useMutation({
    mutationFn: nativeClient.auditDependencies,
    onSuccess: (result) => {
      setMessage(
        `Dependency audit complete: ${result.filesInspected} files checked, ${result.suspicious} need attention.`,
      );
    },
  });
  const archiveInspection = useMutation({
    mutationFn: nativeClient.inspectGameArchive,
    onSuccess: (result) => {
      setSelectedArchive(result.archivePath);
      setMessage(
        result.valid
          ? `ZIP verified: ${result.fileCount} entries inspected without extraction.`
          : "The ZIP failed full decompression verification.",
      );
    },
  });
  const stageArchive = useMutation({
    mutationFn: nativeClient.stageGameArchive,
    onSuccess: (result) => {
      setMessage(`ZIP extracted safely into ${result.stagingPath}. Analyzing the package now.`);
      void workspaceStatus.refetch();
      analyzePackage.mutate(result.stagingPath);
    },
  });
  const analyzePackage = useMutation({
    mutationFn: nativeClient.analyzeStagedPackage,
    onSuccess: (result) => {
      setInstallTitle(result.suggestedTitle);
      setSelectedExecutable(result.executableCandidates[0]?.executablePath ?? "");
      setMessage(
        result.blocked
          ? "The extracted package is blocked. Review the detected safety markers."
          : "Package analysis complete. Review the proposed title and executable.",
      );
    },
  });
  const installPackage = useMutation({
    mutationFn: nativeClient.installStagedPackage,
    onSuccess: (result) => {
      setMessage(
        `${result.game.title} was ${result.updated ? "updated" : "organized and added"}. ` +
          "A rollback backup was kept when an older installation existed.",
      );
      void inboxArchives.refetch();
      void workspaceStatus.refetch();
      void queryClient.invalidateQueries({ queryKey: snapshotKey });
    },
  });

  if (snapshot.isLoading || !snapshot.data) {
    return <div className="page-state">Loading local file status...</div>;
  }

  const { settings, games } = snapshot.data;
  const error =
    scan.error ??
    save.error ??
    addManual.error ??
    prepareWorkspace.error ??
    dependencyAudit.error ??
    archiveInspection.error ??
    stageArchive.error ??
    analyzePackage.error ??
    installPackage.error;

  const addRoot = async () => {
    const root = await nativeClient.chooseLibraryDirectory();
    if (
      !root ||
      settings.libraryRoots.some((current) => current.toLowerCase() === root.toLowerCase())
    ) {
      return;
    }
    save.mutate({ ...settings, libraryRoots: [...settings.libraryRoots, root] });
  };

  const removeRoot = (root: string) => {
    save.mutate({
      ...settings,
      libraryRoots: settings.libraryRoots.filter((candidate) => candidate !== root),
    });
  };

  const addGame = async () => {
    const executable = await nativeClient.chooseGameExecutable();
    if (executable) addManual.mutate(executable);
  };

  const chooseArchive = async () => {
    const archive = await nativeClient.chooseGameArchive();
    if (!archive) return;
    setSelectedArchive(archive);
    setMessage(null);
    stageArchive.reset();
    analyzePackage.reset();
    installPackage.reset();
    archiveInspection.mutate(archive);
  };

  const inspectInboxArchive = (archive: string) => {
    setSelectedArchive(archive);
    setMessage(null);
    stageArchive.reset();
    analyzePackage.reset();
    installPackage.reset();
    archiveInspection.mutate(archive);
  };

  return (
    <div className={styles.page}>
      <header className={styles.header}>
        <div>
          <p className="eyebrow">User-authorized operations</p>
          <h1>Local files</h1>
          <p>Detect, verify, and organize games already stored on this computer.</p>
        </div>
        <button
          className="button primary"
          type="button"
          disabled={scan.isPending}
          onClick={() => {
            setMessage(null);
            scan.mutate();
          }}
        >
          <RefreshCw
            aria-hidden="true"
            size={17}
            className={scan.isPending ? styles.spinning : ""}
          />
          {scan.isPending ? "Scanning..." : "Scan now"}
        </button>
      </header>

      <div className={styles.legalNotice}>
        <ShieldCheck aria-hidden="true" size={20} />
        <p>
          GameVault never downloads games or bypasses ownership, DRM, authentication, anti-cheat, or
          operating-system security. Scanning does not modify game files.
        </p>
      </div>

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

      {scan.isPending ? (
        <section className={styles.progressPanel} aria-live="polite">
          <div>
            <span>
              <RefreshCw aria-hidden="true" size={18} />
              {scanProgress?.message ?? "Preparing the local scan..."}
            </span>
            <strong>{scanProgress?.currentFolder || "Reading library roots"}</strong>
          </div>
          <div
            className={styles.progressTrack}
            role="progressbar"
            aria-valuemin={0}
            aria-valuemax={scanProgress?.foldersTotal || 1}
            aria-valuenow={scanProgress?.foldersScanned || 0}
          >
            <span
              style={{
                inlineSize:
                  scanProgress && scanProgress.foldersTotal > 0
                    ? `${Math.min(
                        100,
                        (scanProgress.foldersScanned / scanProgress.foldersTotal) * 100,
                      )}%`
                    : "8%",
              }}
            />
          </div>
          <small>
            {scanProgress
              ? scanProgress.foldersScanned +
                " folders checked · " +
                scanProgress.gamesDetected +
                " games detected"
              : "The interface remains available while Rust scans in the background."}
          </small>
        </section>
      ) : null}

      <section className={styles.workspacePanel}>
        <div className="section-heading">
          <div>
            <p className="eyebrow">Managed root</p>
            <h2>{settings.managedRoot}</h2>
            <p>Inbox packages, staging, live games, dependencies, and reports remain separated.</p>
          </div>
          <button
            className="button"
            type="button"
            disabled={prepareWorkspace.isPending}
            onClick={() => prepareWorkspace.mutate()}
          >
            <Boxes aria-hidden="true" size={17} />
            {prepareWorkspace.isPending ? "Preparing..." : "Prepare folders"}
          </button>
        </div>
        <div className={styles.workspaceFolders}>
          {(workspaceStatus.data?.folders ?? []).map((folder) => (
            <article className={styles.workspaceFolder} key={folder.name}>
              <span className={folder.exists ? styles.readyIcon : styles.warningIcon}>
                {folder.exists ? (
                  <FolderCheck aria-hidden="true" size={17} />
                ) : (
                  <CircleAlert aria-hidden="true" size={17} />
                )}
              </span>
              <div>
                <strong>{folder.name}</strong>
                <small>{folder.exists ? `${folder.itemCount} top-level items` : "Missing"}</small>
              </div>
            </article>
          ))}
          {!workspaceStatus.data ? (
            <div className={styles.emptyRoot}>Prepare the managed folder layout to begin.</div>
          ) : null}
        </div>
      </section>

      <section className={styles.archivePanel}>
        <div className="section-heading">
          <div>
            <p className="eyebrow">ZIP intake</p>
            <h2>Test before extraction</h2>
            <p>
              Review ZIP paths, link metadata, and expanded size, then fully test with 7-Zip before
              extracting only into the short Staging folder.
            </p>
          </div>
          <button
            className="button"
            type="button"
            disabled={archiveInspection.isPending || stageArchive.isPending}
            onClick={() => void chooseArchive()}
          >
            <FileArchive aria-hidden="true" size={17} />
            {archiveInspection.isPending ? "Testing ZIP..." : "Choose ZIP"}
          </button>
        </div>
        <div className={styles.inboxQueue}>
          <div className={styles.inboxHeading}>
            <span>
              <strong>Inbox</strong>
              <small>Detected user-owned ZIP archives</small>
            </span>
            <strong>{inboxArchives.data?.length ?? 0}</strong>
          </div>
          {(inboxArchives.data ?? []).map((archive) => (
            <article className={styles.inboxArchive} key={archive.path}>
              <FileArchive aria-hidden="true" size={17} />
              <span>
                <strong>{archive.name}</strong>
                <small>{formatBytes(archive.sizeBytes)}</small>
              </span>
              <button
                className="button ghost"
                type="button"
                disabled={archiveInspection.isPending || stageArchive.isPending}
                onClick={() => inspectInboxArchive(archive.path)}
              >
                Inspect
              </button>
            </article>
          ))}
          {inboxArchives.data?.length === 0 ? (
            <p className={styles.auditHint}>Place owned game ZIPs in Inbox to detect them here.</p>
          ) : null}
        </div>
        {archiveInspection.data ? (
          <div className={styles.archiveResult}>
            <div className={styles.archiveHeadline}>
              <div>
                <strong>{archiveInspection.data.archiveName}</strong>
                <small>
                  {formatBytes(archiveInspection.data.archiveSizeBytes)} compressed ·{" "}
                  {formatBytes(archiveInspection.data.unpackedSizeBytes)} unpacked ·{" "}
                  {archiveInspection.data.fileCount} entries
                </small>
              </div>
              <span
                className={
                  archiveInspection.data.valid && archiveInspection.data.canStage
                    ? styles.archiveReady
                    : styles.archiveBlocked
                }
              >
                {archiveInspection.data.valid && archiveInspection.data.canStage
                  ? "Verified"
                  : "Blocked"}
              </span>
            </div>
            {archiveInspection.data.warnings.length ? (
              <ul className={styles.warningList}>
                {archiveInspection.data.warnings.map((warning) => (
                  <li key={warning}>{warning}</li>
                ))}
              </ul>
            ) : (
              <p className={styles.auditHint}>No package-structure warnings were detected.</p>
            )}
            <div className={styles.archiveActions}>
              <span>
                {archiveInspection.data.executableCandidates.length} likely executable
                {archiveInspection.data.executableCandidates.length === 1 ? "" : "s"} found
              </span>
              <button
                className="button primary"
                type="button"
                disabled={!archiveInspection.data.canStage || stageArchive.isPending}
                onClick={() => selectedArchive && stageArchive.mutate(selectedArchive)}
              >
                <PackageOpen aria-hidden="true" size={17} />
                {stageArchive.isPending ? "Extracting..." : "Extract & analyze"}
              </button>
            </div>
            {stageArchive.data ? (
              <div className="notice" role="status">
                Extracted {stageArchive.data.filesExtracted} files to{" "}
                {stageArchive.data.stagingPath}
              </div>
            ) : null}
            {analyzePackage.data ? (
              <div className={styles.installReview}>
                <div className={styles.archiveHeadline}>
                  <div>
                    <strong>Cleanup and installation plan</strong>
                    <small>
                      {analyzePackage.data.redistFolders.length} Redist folders ·{" "}
                      {analyzePackage.data.packageExtras.length} package extras ·{" "}
                      {analyzePackage.data.nestedArchives.length} sealed nested archives
                    </small>
                  </div>
                  <span
                    className={
                      analyzePackage.data.canInstall ? styles.archiveReady : styles.archiveBlocked
                    }
                  >
                    {analyzePackage.data.canInstall ? "Ready" : "Blocked"}
                  </span>
                </div>
                {analyzePackage.data.warnings.length ? (
                  <ul className={styles.warningList}>
                    {analyzePackage.data.warnings.map((warning) => (
                      <li key={warning}>{warning}</li>
                    ))}
                  </ul>
                ) : null}
                {analyzePackage.data.blocked && analyzePackage.data.suspiciousMarkers.length ? (
                  <details className={styles.blockedDetails}>
                    <summary>Detected blocking item</summary>
                    {analyzePackage.data.suspiciousMarkers.map((marker) => (
                      <code key={marker}>
                        {compactPackagePath(marker, analyzePackage.data.stagingPath)}
                      </code>
                    ))}
                  </details>
                ) : null}
                <div className={styles.installFields}>
                  <label className="field-label">
                    Game folder name
                    <input
                      className="field"
                      value={installTitle}
                      maxLength={80}
                      onChange={(event) => setInstallTitle(event.target.value)}
                    />
                  </label>
                  <label className="field-label">
                    Primary executable
                    <select
                      className="field"
                      value={selectedExecutable}
                      onChange={(event) => setSelectedExecutable(event.target.value)}
                    >
                      {analyzePackage.data.executableCandidates.map((candidate) => (
                        <option value={candidate.executablePath} key={candidate.executablePath}>
                          {candidate.displayName} —{" "}
                          {compactPackagePath(candidate.executablePath, candidate.installRoot)}
                        </option>
                      ))}
                    </select>
                  </label>
                </div>
                <div className={styles.installActions}>
                  <p>
                    Game files move to Games. Redist folders move to Dependencies, wrapper extras to
                    Quarantine, and an Inbox ZIP moves to Archives after success.
                  </p>
                  <button
                    className="button primary"
                    type="button"
                    disabled={
                      !analyzePackage.data.canInstall ||
                      !installTitle.trim() ||
                      !selectedExecutable ||
                      installPackage.isPending
                    }
                    onClick={() =>
                      installPackage.mutate({
                        stagingPath: analyzePackage.data.stagingPath,
                        executablePath: selectedExecutable,
                        title: installTitle,
                        archivePath: selectedArchive,
                      })
                    }
                  >
                    <PackageCheck aria-hidden="true" size={17} />
                    {analyzePackage.data.blocked
                      ? "Installation blocked"
                      : installPackage.isPending
                        ? "Organizing..."
                        : "Organize & add game"}
                  </button>
                </div>
                {installPackage.data ? (
                  <small className={styles.reportPath}>
                    Install report saved to {installPackage.data.reportPath}
                  </small>
                ) : null}
              </div>
            ) : null}
          </div>
        ) : (
          <p className={styles.auditHint}>
            Nested archives are never unpacked recursively, and no archive content is executed.
          </p>
        )}
      </section>

      <section className={styles.dependencyPanel}>
        <div className="section-heading">
          <div>
            <p className="eyebrow">Safe prerequisite review</p>
            <h2>Redistributable audit</h2>
            <p>
              Inspect bundled installers, verify signatures, check Windows, and contact only
              approved official vendor sources.
            </p>
          </div>
          <button
            className="button"
            type="button"
            disabled={dependencyAudit.isPending}
            onClick={() => dependencyAudit.mutate()}
          >
            <PackageCheck aria-hidden="true" size={17} />
            {dependencyAudit.isPending ? "Auditing..." : "Audit dependencies"}
          </button>
        </div>
        {dependencyAudit.data ? (
          <>
            <div className={styles.auditSummary}>
              <span>
                <strong>{dependencyAudit.data.redistFolders}</strong> Redist folders
              </span>
              <span>
                <strong>{dependencyAudit.data.filesInspected}</strong> files inspected
              </span>
              <span>
                <strong>{dependencyAudit.data.installed}</strong> already installed
              </span>
              <span className={dependencyAudit.data.suspicious ? styles.needsAttention : undefined}>
                <strong>{dependencyAudit.data.suspicious}</strong> need attention
              </span>
            </div>
            <div className={styles.dependencyList}>
              {dependencyAudit.data.items.map((item) => (
                <article className={styles.dependencyItem} key={item.id}>
                  <div>
                    <strong>{item.name}</strong>
                    <small>
                      {item.architecture} · signature {item.signatureStatus} · system{" "}
                      {item.installedStatus}
                    </small>
                    <p>{item.recommendation}</p>
                  </div>
                  {item.officialSourceUrl ? (
                    <button
                      className="button ghost"
                      type="button"
                      onClick={() =>
                        void nativeClient.openOfficialDependencySource(item.officialSourceUrl ?? "")
                      }
                    >
                      Official source
                      <ExternalLink aria-hidden="true" size={15} />
                    </button>
                  ) : null}
                </article>
              ))}
              {!dependencyAudit.data.items.length ? (
                <div className={styles.emptyRoot}>No bundled installers were found.</div>
              ) : null}
            </div>
            <small className={styles.reportPath}>
              Report saved to {dependencyAudit.data.reportPath}
            </small>
          </>
        ) : (
          <p className={styles.auditHint}>
            GameVault never runs a bundled installer during this audit. Installation remains a
            separate, user-approved action.
          </p>
        )}
      </section>

      <section>
        <div className="section-heading">
          <div>
            <p className="eyebrow">Scan locations</p>
            <h2>Library directories</h2>
          </div>
          <button className="button" type="button" onClick={() => void addRoot()}>
            <FolderPlus aria-hidden="true" size={17} />
            Add directory
          </button>
        </div>
        <div className={styles.rootList}>
          {settings.libraryRoots.map((root) => {
            const related = games.filter((game) =>
              game.installPath.toLowerCase().startsWith(root.toLowerCase()),
            );
            const unavailable = related.some((game) => game.detectionStatus === "unavailable");
            return (
              <article className={styles.rootCard} key={root}>
                <span className={unavailable ? styles.warningIcon : styles.readyIcon}>
                  {unavailable ? (
                    <CircleAlert aria-hidden="true" size={19} />
                  ) : (
                    <FolderCheck aria-hidden="true" size={19} />
                  )}
                </span>
                <div>
                  <strong>{root}</strong>
                  <small>
                    {unavailable
                      ? "Drive or folder unavailable"
                      : `${related.length} indexed game${related.length === 1 ? "" : "s"}`}
                  </small>
                </div>
                <button
                  className="button ghost"
                  type="button"
                  disabled={save.isPending}
                  onClick={() => removeRoot(root)}
                >
                  Remove
                </button>
              </article>
            );
          })}
          {!settings.libraryRoots.length ? (
            <div className={styles.emptyRoot}>Add at least one local library directory.</div>
          ) : null}
        </div>
      </section>

      <div className={styles.cards}>
        <section className={styles.actionCard}>
          <FilePlus2 aria-hidden="true" size={23} />
          <div>
            <h2>Add a game manually</h2>
            <p>Choose a known executable when automatic detection selects the wrong file.</p>
          </div>
          <button
            className="button"
            type="button"
            disabled={addManual.isPending}
            onClick={() => void addGame()}
          >
            Choose executable
          </button>
        </section>
        <section className={styles.actionCard}>
          <HardDrive aria-hidden="true" size={23} />
          <div>
            <h2>Last scan</h2>
            <p>
              {settings.lastScanAt
                ? formatRelativeDate(settings.lastScanAt) +
                  ". Folder fingerprints avoid unnecessary rescans."
                : "No scan has completed yet."}
            </p>
          </div>
          <span className={styles.depth}>Depth {settings.scanDepth}</span>
        </section>
      </div>
    </div>
  );
}
