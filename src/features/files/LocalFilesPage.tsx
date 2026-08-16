import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  Boxes,
  CircleAlert,
  FileArchive,
  FileClock,
  FilePlus2,
  FolderCheck,
  FolderPlus,
  GitCompareArrows,
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
import { DependencyAuditPanel } from "./DependencyAuditPanel";
import styles from "./LocalFilesPage.module.css";
import { OperationHistoryPanel } from "./OperationHistoryPanel";

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
  const stagingPackages = useQuery({
    queryKey: ["staging-packages"],
    queryFn: nativeClient.listStagingPackages,
    retry: false,
  });
  const operationHistory = useQuery({
    queryKey: ["operation-history"],
    queryFn: nativeClient.getOperationHistory,
    retry: false,
    refetchInterval: (query) =>
      query.state.data?.some((operation) => operation.status === "running") ? 2_000 : false,
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
      void operationHistory.refetch();
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
      void operationHistory.refetch();
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
      void stagingPackages.refetch();
      void operationHistory.refetch();
      analyzePackage.mutate(result.stagingPath);
    },
  });
  const analyzePackage = useMutation({
    mutationFn: nativeClient.analyzeStagedPackage,
    onSuccess: (result) => {
      setInstallTitle(result.suggestedTitle);
      setSelectedExecutable(result.executableCandidates[0]?.executablePath ?? "");
      previewUpdate.reset();
      setMessage(
        result.blocked
          ? "The extracted package is blocked. Review the detected safety markers."
          : "Package analysis complete. Review the proposed title and executable.",
      );
    },
  });
  const previewUpdate = useMutation({
    mutationFn: nativeClient.previewStagedUpdate,
    onSuccess: (result) => {
      setMessage(
        result.isUpdate
          ? `Update plan ready: ${result.addedCount} added, ${result.changedCount} changed, and ${result.removedCount} removed files.`
          : `New installation plan ready: ${result.addedCount} files will enter Games.`,
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
      void stagingPackages.refetch();
      void operationHistory.refetch();
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
    stagingPackages.error ??
    operationHistory.error ??
    archiveInspection.error ??
    stageArchive.error ??
    analyzePackage.error ??
    previewUpdate.error ??
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
    previewUpdate.reset();
    installPackage.reset();
    archiveInspection.mutate(archive);
  };

  const inspectInboxArchive = (archive: string) => {
    setSelectedArchive(archive);
    setMessage(null);
    stageArchive.reset();
    analyzePackage.reset();
    previewUpdate.reset();
    installPackage.reset();
    archiveInspection.mutate(archive);
  };

  const reviewStagedPackage = (stagingPath: string) => {
    setSelectedArchive(null);
    setMessage("Re-analyzing the existing Staging folder. No files have been promoted.");
    archiveInspection.reset();
    stageArchive.reset();
    installPackage.reset();
    previewUpdate.reset();
    analyzePackage.mutate(stagingPath);
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
        <div className={styles.recoveryQueue}>
          <div className={styles.inboxHeading}>
            <span>
              <strong>Staging recovery queue</strong>
              <small>
                Packages left available for explicit review after a restart or interruption
              </small>
            </span>
            <strong>{stagingPackages.data?.length ?? 0}</strong>
          </div>
          {(stagingPackages.data ?? []).map((staged) => (
            <article className={styles.inboxArchive} key={staged.path}>
              <FileClock aria-hidden="true" size={17} />
              <span>
                <strong>{staged.name}</strong>
                <small>
                  {staged.fileCount === null ? "Unreadable file set" : `${staged.fileCount} files`}
                  {staged.modifiedAt ? ` · ${formatRelativeDate(staged.modifiedAt)}` : ""}
                </small>
              </span>
              <button
                className="button ghost"
                type="button"
                disabled={!staged.reviewable || analyzePackage.isPending}
                title={staged.recoveryHint}
                onClick={() => reviewStagedPackage(staged.path)}
              >
                Review again
              </button>
            </article>
          ))}
          {stagingPackages.data?.length === 0 ? (
            <p className={styles.auditHint}>No staged packages are waiting for review.</p>
          ) : null}
        </div>
        {archiveInspection.data || analyzePackage.data ? (
          <div className={styles.archiveResult}>
            {archiveInspection.data ? (
              <>
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
              </>
            ) : (
              <div className="notice" role="status">
                Existing staged content was re-opened for analysis. No ZIP was re-extracted and no
                files were promoted.
              </div>
            )}
            {analyzePackage.data ? (
              <div className={styles.installReview}>
                <div className={styles.archiveHeadline}>
                  <div>
                    <h3>Cleanup and installation plan</h3>
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
                      onChange={(event) => {
                        setInstallTitle(event.target.value);
                        previewUpdate.reset();
                      }}
                    />
                  </label>
                  <label className="field-label">
                    Primary executable
                    <select
                      className="field"
                      value={selectedExecutable}
                      onChange={(event) => {
                        setSelectedExecutable(event.target.value);
                        previewUpdate.reset();
                      }}
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
                <div className={styles.updateReview}>
                  <div>
                    <h4>File-by-file promotion preview</h4>
                    <p>
                      Hash the staged and current files before any move. If either side changes,
                      GameVault requires a fresh review.
                    </p>
                  </div>
                  <button
                    className="button"
                    type="button"
                    disabled={
                      !analyzePackage.data.canInstall ||
                      !installTitle.trim() ||
                      !selectedExecutable ||
                      previewUpdate.isPending
                    }
                    onClick={() =>
                      previewUpdate.mutate({
                        stagingPath: analyzePackage.data.stagingPath,
                        executablePath: selectedExecutable,
                        title: installTitle,
                      })
                    }
                  >
                    <GitCompareArrows aria-hidden="true" size={17} />
                    {previewUpdate.isPending
                      ? "Hashing files..."
                      : previewUpdate.data
                        ? "Refresh file preview"
                        : "Preview file changes"}
                  </button>
                </div>
                {previewUpdate.data ? (
                  <div className={styles.diffPanel} aria-live="polite">
                    <h4>File change checkpoint</h4>
                    <div className={styles.diffSummary}>
                      <span>
                        <strong>{previewUpdate.data.addedCount}</strong> added
                      </span>
                      <span>
                        <strong>{previewUpdate.data.changedCount}</strong> changed
                      </span>
                      <span>
                        <strong>{previewUpdate.data.removedCount}</strong> removed
                      </span>
                      <span>
                        <strong>{previewUpdate.data.unchangedCount}</strong> unchanged
                      </span>
                    </div>
                    <p>
                      {previewUpdate.data.isUpdate
                        ? `The current installation will first move to ${previewUpdate.data.rollbackRoot}.`
                        : `This is a new managed installation at ${previewUpdate.data.destinationPath}.`}
                    </p>
                    {previewUpdate.data.addedSample.length ||
                    previewUpdate.data.changedSample.length ||
                    previewUpdate.data.removedSample.length ? (
                      <details>
                        <summary>Review representative relative paths</summary>
                        {[
                          ["Added", previewUpdate.data.addedSample],
                          ["Changed", previewUpdate.data.changedSample],
                          ["Removed", previewUpdate.data.removedSample],
                        ].map(([label, paths]) =>
                          paths.length ? (
                            <div key={label as string}>
                              <strong>{label as string}</strong>
                              {(paths as string[]).map((path) => (
                                <code key={`${label}-${path}`}>{path}</code>
                              ))}
                            </div>
                          ) : null,
                        )}
                      </details>
                    ) : null}
                  </div>
                ) : null}
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
                      !previewUpdate.data ||
                      installPackage.isPending
                    }
                    onClick={() =>
                      installPackage.mutate({
                        stagingPath: analyzePackage.data.stagingPath,
                        executablePath: selectedExecutable,
                        title: installTitle,
                        archivePath: selectedArchive,
                        updateFingerprint: previewUpdate.data?.fingerprint ?? "",
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

      <DependencyAuditPanel
        audit={dependencyAudit.data}
        pending={dependencyAudit.isPending}
        onAudit={() => dependencyAudit.mutate()}
        onOpenSource={(url) => void nativeClient.openOfficialDependencySource(url)}
      />

      <OperationHistoryPanel
        operations={operationHistory.data}
        fetching={operationHistory.isFetching}
        onRefresh={() => void operationHistory.refetch()}
      />

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
