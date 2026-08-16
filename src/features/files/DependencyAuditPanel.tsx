import { ExternalLink, PackageCheck } from "lucide-react";
import { formatRelativeDate } from "../../shared/lib/format";
import type { DependencyAudit } from "../../shared/lib/types";
import styles from "./LocalFilesPage.module.css";

type Props = {
  audit: DependencyAudit | undefined;
  pending: boolean;
  onAudit: () => void;
  onOpenSource: (url: string) => void;
};

export function DependencyAuditPanel({ audit, pending, onAudit, onOpenSource }: Props) {
  return (
    <section className={styles.dependencyPanel}>
      <div className="section-heading">
        <div>
          <p className="eyebrow">Safe prerequisite review</p>
          <h2>Redistributable audit</h2>
          <p>
            Inspect bundled installers, verify signatures, check Windows, and contact only approved
            official vendor sources.
          </p>
        </div>
        <button className="button" type="button" disabled={pending} onClick={onAudit}>
          <PackageCheck aria-hidden="true" size={17} />
          {pending ? "Auditing..." : "Audit dependencies"}
        </button>
      </div>
      {audit ? (
        <>
          <div className={styles.auditSummary}>
            <span>
              <strong>{audit.redistFolders}</strong> Redist folders
            </span>
            <span>
              <strong>{audit.filesInspected}</strong> files inspected
            </span>
            <span>
              <strong>{audit.installed}</strong> already installed
            </span>
            <span className={audit.suspicious ? styles.needsAttention : undefined}>
              <strong>{audit.suspicious}</strong> need attention
            </span>
          </div>
          <div className={styles.dependencyList}>
            {audit.items.map((item) => (
              <article className={styles.dependencyItem} key={item.id}>
                <div>
                  <strong>{item.name}</strong>
                  <small>
                    {item.architecture} · signature {item.signatureStatus} · system{" "}
                    {item.installedStatus}
                  </small>
                  <p>{item.recommendation}</p>
                  <details className={styles.evidenceDetails}>
                    <summary>Evidence and version details</summary>
                    <dl>
                      <div>
                        <dt>Classification confidence</dt>
                        <dd>{item.confidence}</dd>
                      </div>
                      <div>
                        <dt>Publisher match</dt>
                        <dd>{item.publisherMatch}</dd>
                      </div>
                      <div>
                        <dt>Bundled version</dt>
                        <dd>{item.bundledVersion ?? "Not reported"}</dd>
                      </div>
                      <div>
                        <dt>Installed version</dt>
                        <dd>{item.installedVersion ?? "Not detected"}</dd>
                      </div>
                      <div>
                        <dt>Signed publisher</dt>
                        <dd>{item.publisher ?? "Not available"}</dd>
                      </div>
                      <div>
                        <dt>Checked</dt>
                        <dd>{formatRelativeDate(item.checkedAt)}</dd>
                      </div>
                    </dl>
                    <p>{item.detectedBy}</p>
                    <ul>
                      {item.installedEvidence.map((evidence) => (
                        <li key={evidence}>{evidence}</li>
                      ))}
                    </ul>
                    <code title="Bundled file SHA-256">SHA-256 {item.sha256}</code>
                  </details>
                </div>
                {item.officialSourceUrl ? (
                  <button
                    className="button ghost"
                    type="button"
                    onClick={() => onOpenSource(item.officialSourceUrl ?? "")}
                  >
                    Official source
                    <ExternalLink aria-hidden="true" size={15} />
                  </button>
                ) : null}
              </article>
            ))}
            {!audit.items.length ? (
              <div className={styles.emptyRoot}>No bundled installers were found.</div>
            ) : null}
          </div>
          <small className={styles.reportPath}>Report saved to {audit.reportPath}</small>
        </>
      ) : (
        <p className={styles.auditHint}>
          GameVault never runs a bundled installer during this audit. Installation remains a
          separate, user-approved action.
        </p>
      )}
    </section>
  );
}
