import type { ReactNode } from "react";

interface MetricCellProps {
  color: "mint" | "amber" | "violet";
  label: string;
  value: ReactNode;
  className?: string;
}

export function MetricCell({ color, label, value, className = "" }: MetricCellProps) {
  return (
    <div className={`metric-cell ${className}`}>
      <span className={`signal signal-${color}`} aria-hidden="true" />
      <span className="metric-label">{label}</span>
      <strong className="metric-value">{value}</strong>
    </div>
  );
}
