import { Link } from "@tanstack/react-router";
import { FolderSearch2, Gamepad2, House, LibraryBig, ScanSearch, Settings2 } from "lucide-react";
import type { ReactNode } from "react";
import styles from "./AppShell.module.css";
import { useNativeEvents, useSnapshot } from "./query";

const navigation = [
  { to: "/", label: "Home", Icon: House },
  { to: "/library", label: "Library", Icon: LibraryBig },
  { to: "/files", label: "Local files", Icon: FolderSearch2 },
  { to: "/settings", label: "Settings", Icon: Settings2 },
] as const;

export function AppShell({ children }: { children: ReactNode }) {
  const snapshot = useSnapshot();
  const { scanProgress } = useNativeEvents();
  const theme = snapshot.data?.settings.theme ?? "midnight";

  return (
    <div className={styles.shell} data-theme={theme}>
      <a className={styles.skipLink} href="#main-content">
        Skip to content
      </a>
      <aside className={styles.sidebar} aria-label="Primary">
        <Link to="/" className={styles.brand} aria-label="GameVault home">
          <span className={styles.brandMark}>
            <Gamepad2 aria-hidden="true" size={23} />
          </span>
          <span>
            <strong>GameVault</strong>
            <small>Portable library</small>
          </span>
        </Link>

        <nav className={styles.navigation}>
          {navigation.map(({ to, label, Icon }) => (
            <Link
              key={to}
              to={to}
              className={styles.navLink}
              activeProps={{ className: styles.active }}
              activeOptions={{ exact: to === "/" }}
            >
              <Icon aria-hidden="true" size={19} />
              <span>{label}</span>
            </Link>
          ))}
        </nav>

        <div className={styles.sidebarFooter}>
          <div className={styles.libraryPulse}>
            <span className={styles.pulseIcon}>
              {scanProgress ? (
                <ScanSearch aria-hidden="true" size={18} />
              ) : (
                <LibraryBig aria-hidden="true" size={18} />
              )}
            </span>
            <span>
              <strong>{snapshot.data?.stats.totalGames ?? 0} games</strong>
              <small>
                {scanProgress
                  ? scanProgress.currentFolder || "Scanning..."
                  : snapshot.data?.settings.lastScanAt
                    ? "Library indexed"
                    : "Ready for first scan"}
              </small>
            </span>
          </div>
          {scanProgress && scanProgress.foldersTotal > 0 ? (
            <div
              className={styles.progressTrack}
              role="progressbar"
              aria-valuemin={0}
              aria-valuemax={scanProgress.foldersTotal}
              aria-valuenow={Math.min(scanProgress.foldersScanned, scanProgress.foldersTotal)}
            >
              <span
                style={{
                  inlineSize: `${Math.min(
                    100,
                    (scanProgress.foldersScanned / scanProgress.foldersTotal) * 100,
                  )}%`,
                }}
              />
            </div>
          ) : null}
        </div>
      </aside>

      <main className={styles.main} id="main-content">
        {children}
      </main>
    </div>
  );
}
