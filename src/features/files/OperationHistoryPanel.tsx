import { History } from "lucide-react";
import { formatRelativeDate } from "../../shared/lib/format";
import type { OperationRecord } from "../../shared/lib/types";
import styles from "./LocalFilesPage.module.css";

type Props = {
  operations: OperationRecord[] | undefined;
  fetching: boolean;
  onRefresh: () => void;
};

export function OperationHistoryPanel({ operations, fetching, onRefresh }: Props) {
  return (
    <section className={styles.operationPanel}>
      <div className="section-heading">
        <div>
          <p className="eyebrow">Persistent recovery evidence</p>
          <h2>Operation history</h2>
          <p>
            Long-running scans, staging, promotion, and prerequisite audits remain visible after a
            restart. Interrupted work is never resumed silently.
          </p>
        </div>
        <button className="button" type="button" disabled={fetching} onClick={onRefresh}>
          <History aria-hidden="true" size={17} />
          {fetching ? "Refreshing..." : "Refresh history"}
        </button>
      </div>
      <div className={styles.operationList} aria-live="polite">
        {(operations ?? []).slice(0, 12).map((operation) => (
          <article className={styles.operationItem} key={operation.id}>
            <span className={styles.operationStatus} data-status={operation.status}>
              {operation.status}
            </span>
            <div>
              <strong>{operation.label}</strong>
              <small>
                <time dateTime={operation.startedAt}>
                  {formatRelativeDate(operation.startedAt)}
                </time>
                {` · ${operation.kind}`}
              </small>
              <p>{operation.summary}</p>
              {operation.errorMessage ? (
                <p className={styles.operationError}>{operation.errorMessage}</p>
              ) : null}
              {operation.status === "failed" || operation.status === "interrupted" ? (
                <p className={styles.recoveryHint}>{operation.recoveryHint}</p>
              ) : null}
              {operation.sourcePath || operation.targetPath || operation.reportPath ? (
                <details>
                  <summary>Paths and report evidence</summary>
                  {operation.sourcePath ? <code>Source: {operation.sourcePath}</code> : null}
                  {operation.targetPath ? <code>Target: {operation.targetPath}</code> : null}
                  {operation.reportPath ? <code>Report: {operation.reportPath}</code> : null}
                </details>
              ) : null}
            </div>
          </article>
        ))}
        {operations?.length === 0 ? (
          <div className={styles.emptyRoot}>No persisted operations have run yet.</div>
        ) : null}
      </div>
    </section>
  );
}
