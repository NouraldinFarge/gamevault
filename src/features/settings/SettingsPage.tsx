import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  ArchiveRestore,
  Brush,
  DatabaseBackup,
  Eraser,
  ExternalLink,
  FileText,
  Gauge,
  RefreshCw,
  Save,
  ShieldCheck,
  Stethoscope,
} from "lucide-react";
import { useState } from "react";
import { snapshotKey, useSnapshot } from "../../app/query";
import { getErrorMessage, nativeClient } from "../../shared/lib/native-client";
import type { Settings } from "../../shared/lib/types";
import styles from "./SettingsPage.module.css";

export function SettingsPage() {
  const snapshot = useSnapshot();
  if (snapshot.isLoading || !snapshot.data) {
    return <div className="page-state">Loading settings...</div>;
  }
  return (
    <SettingsForm
      key={snapshot.data.settings.lastScanAt + snapshot.data.settings.theme}
      initial={snapshot.data.settings}
      portableRoot={snapshot.data.portableRoot}
    />
  );
}

function SettingsForm({ initial, portableRoot }: { initial: Settings; portableRoot: string }) {
  const queryClient = useQueryClient();
  const [form, setForm] = useState(initial);
  const [message, setMessage] = useState<string | null>(null);
  const health = useQuery({
    queryKey: ["application", "health"],
    queryFn: nativeClient.getHealthReport,
  });
  const save = useMutation({
    mutationFn: nativeClient.saveSettings,
    onSuccess: () => {
      setMessage("Settings saved locally.");
      void queryClient.invalidateQueries({ queryKey: snapshotKey });
    },
  });
  const backup = useMutation({
    mutationFn: nativeClient.createDatabaseBackup,
    onSuccess: (path) => setMessage(`Backup created: ${path}`),
  });
  const restore = useMutation({
    mutationFn: nativeClient.restoreDatabaseBackup,
    onSuccess: () => {
      setMessage("Backup restored. Library data has been refreshed.");
      void queryClient.invalidateQueries({ queryKey: snapshotKey });
    },
  });
  const clearCache = useMutation({
    mutationFn: nativeClient.clearCache,
    onSuccess: () => setMessage("Application cache cleared."),
  });
  const logs = useMutation({ mutationFn: nativeClient.openLogsFolder });
  const updateCheck = useMutation({
    mutationFn: nativeClient.checkForAppUpdate,
    onSuccess: (result) =>
      setMessage(
        result.updateAvailable
          ? `GameVault ${result.latestVersion} is available. Review the official release before downloading.`
          : `GameVault ${result.currentVersion} is current.`,
      ),
  });
  const openRelease = useMutation({ mutationFn: nativeClient.openReleasePage });
  const error =
    save.error ??
    backup.error ??
    restore.error ??
    clearCache.error ??
    logs.error ??
    updateCheck.error ??
    openRelease.error;

  const restoreBackup = async () => {
    const path = await nativeClient.chooseBackupFile();
    if (path) restore.mutate(path);
  };

  return (
    <div className={styles.page}>
      <header className={styles.header}>
        <div>
          <p className="eyebrow">Portable preferences</p>
          <h1>Settings</h1>
          <p>Configuration stays beside GameVault and moves with the application folder.</p>
        </div>
        <button
          className="button primary"
          type="button"
          disabled={save.isPending}
          onClick={() => save.mutate(form)}
        >
          <Save aria-hidden="true" size={17} />
          {save.isPending ? "Saving..." : "Save settings"}
        </button>
      </header>

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

      <div className={styles.grid}>
        <section className={styles.panel}>
          <div className={styles.panelHeading}>
            <Brush aria-hidden="true" size={20} />
            <div>
              <h2>Appearance</h2>
              <p>Choose a semantic color-token theme.</p>
            </div>
          </div>
          <label className="field-label">
            Interface theme
            <select
              className="select"
              value={form.theme}
              onChange={(event) =>
                setForm((current) => ({
                  ...current,
                  theme: event.target.value as Settings["theme"],
                }))
              }
            >
              <option value="midnight">Midnight</option>
              <option value="deep-blue">Deep blue</option>
              <option value="high-contrast">High contrast</option>
            </select>
          </label>
        </section>

        <section className={styles.panel}>
          <div className={styles.panelHeading}>
            <Gauge aria-hidden="true" size={20} />
            <div>
              <h2>Scanning</h2>
              <p>Bound recursive executable discovery.</p>
            </div>
          </div>
          <label className="field-label">
            Managed library root
            <input
              className="field"
              value={form.managedRoot}
              onChange={(event) =>
                setForm((current) => ({ ...current, managedRoot: event.target.value }))
              }
              spellCheck={false}
            />
          </label>
          <label className="field-label">
            Maximum folder depth: {form.scanDepth}
            <input
              className={styles.range}
              type="range"
              min={1}
              max={8}
              step={1}
              value={form.scanDepth}
              onChange={(event) =>
                setForm((current) => ({
                  ...current,
                  scanDepth: Number(event.target.value),
                }))
              }
            />
          </label>
          <p className={styles.help}>
            A depth of 4 finds common Win32/Win64 subfolders without traversing an entire drive.
          </p>
        </section>

        <section className={styles.panel}>
          <div className={styles.panelHeading}>
            <FileText aria-hidden="true" size={20} />
            <div>
              <h2>Launch defaults</h2>
              <p>Passed as an argument array, never a shell command.</p>
            </div>
          </div>
          <label className="field-label">
            Default arguments, one per line
            <textarea
              className="textarea"
              value={form.defaultLaunchArgs.join("\n")}
              onChange={(event) =>
                setForm((current) => ({
                  ...current,
                  defaultLaunchArgs: event.target.value
                    .split(/\r?\n/)
                    .map((value) => value.trim())
                    .filter(Boolean),
                }))
              }
              placeholder="Leave blank for most games"
            />
          </label>
        </section>

        <section className={styles.panel}>
          <div className={styles.panelHeading}>
            <ShieldCheck aria-hidden="true" size={20} />
            <div>
              <h2>Privacy and logging</h2>
              <p>
                No telemetry is enabled. Dependency checks contact approved vendor sources only.
              </p>
            </div>
          </div>
          <label className={styles.check}>
            <input
              type="checkbox"
              checked={form.loggingEnabled}
              onChange={(event) =>
                setForm((current) => ({
                  ...current,
                  loggingEnabled: event.target.checked,
                }))
              }
            />
            Keep redacted local diagnostic logs
          </label>
          <button className="button" type="button" onClick={() => logs.mutate()}>
            Open log folder
          </button>
        </section>
      </div>

      <section className={styles.dataPanel}>
        <div>
          <p className="eyebrow">Recovery</p>
          <h2>Portable data</h2>
          <p>
            Backups include the local library index, settings, playtime, favorites, and personal
            metadata. They do not include game files.
          </p>
        </div>
        <div className={styles.dataActions}>
          <button
            className="button"
            type="button"
            disabled={backup.isPending}
            onClick={() => backup.mutate()}
          >
            <DatabaseBackup aria-hidden="true" size={17} />
            Create backup
          </button>
          <button
            className="button"
            type="button"
            disabled={restore.isPending}
            onClick={() => void restoreBackup()}
          >
            <ArchiveRestore aria-hidden="true" size={17} />
            Restore backup
          </button>
          <button
            className="button"
            type="button"
            disabled={clearCache.isPending}
            onClick={() => clearCache.mutate()}
          >
            <Eraser aria-hidden="true" size={17} />
            Clear cache
          </button>
        </div>
      </section>

      <section className={styles.updatePanel}>
        <div>
          <p className="eyebrow">Manual release check</p>
          <h2>Application updates</h2>
          <p>
            Contact the official GameVault GitHub releases endpoint only when you choose. GameVault
            never downloads or installs an application update automatically.
          </p>
        </div>
        <div className={styles.updateActions}>
          <button
            className="button"
            type="button"
            disabled={updateCheck.isPending}
            onClick={() => updateCheck.mutate()}
          >
            <RefreshCw aria-hidden="true" size={17} />
            {updateCheck.isPending ? "Checking GitHub..." : "Check for updates"}
          </button>
          {updateCheck.data ? (
            <button
              className="button"
              type="button"
              disabled={openRelease.isPending}
              onClick={() => openRelease.mutate(updateCheck.data.releaseUrl)}
            >
              Official release
              <ExternalLink aria-hidden="true" size={15} />
            </button>
          ) : null}
        </div>
        {updateCheck.data ? (
          <dl className={styles.updateResult} aria-live="polite">
            <div>
              <dt>Installed</dt>
              <dd>{updateCheck.data.currentVersion}</dd>
            </div>
            <div>
              <dt>Latest stable</dt>
              <dd>{updateCheck.data.latestVersion}</dd>
            </div>
            <div>
              <dt>Status</dt>
              <dd>{updateCheck.data.updateAvailable ? "Update available" : "Current"}</dd>
            </div>
          </dl>
        ) : null}
      </section>

      <section className={styles.diagnostics}>
        <div className={styles.panelHeading}>
          <Stethoscope aria-hidden="true" size={20} />
          <div>
            <h2>Diagnostics</h2>
            <p>Safe local version and storage health information.</p>
          </div>
        </div>
        <dl>
          <div>
            <dt>Portable root</dt>
            <dd>{portableRoot}</dd>
          </div>
          <div>
            <dt>Application</dt>
            <dd>{health.data ? `GameVault ${health.data.appVersion}` : "Checking..."}</dd>
          </div>
          <div>
            <dt>SQLite</dt>
            <dd>{health.data?.sqliteVersion ?? "Checking..."}</dd>
          </div>
          <div>
            <dt>WebView2</dt>
            <dd>{health.data?.webview2Runtime ?? "Checking..."}</dd>
          </div>
        </dl>
      </section>
    </div>
  );
}
