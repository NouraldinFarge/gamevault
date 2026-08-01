import { CircleAlert, CircleCheck, CircleOff, SlidersHorizontal } from "lucide-react";
import styles from "./StatusBadge.module.css";

const statusConfig = {
  detected: { label: "Detected", Icon: CircleCheck },
  configured: { label: "Configured", Icon: SlidersHorizontal },
  missing: { label: "Missing", Icon: CircleAlert },
  unavailable: { label: "Unavailable", Icon: CircleOff },
} as const;

export function StatusBadge({ status }: { status: keyof typeof statusConfig }) {
  const { label, Icon } = statusConfig[status];
  return (
    <span className={styles.badge} data-status={status}>
      <Icon aria-hidden="true" size={13} />
      {label}
    </span>
  );
}
