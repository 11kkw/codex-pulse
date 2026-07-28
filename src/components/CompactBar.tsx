import { DotsSixVertical } from "@phosphor-icons/react/DotsSixVertical";
import type { MouseEventHandler } from "react";
import type { CodexSnapshot, SystemSnapshot } from "../types";
import { MetricCell } from "./MetricCell";

interface CompactBarProps {
  codex: CodexSnapshot;
  system: SystemSnapshot;
  expanded: boolean;
  overlay: boolean;
  onContextMenu?: MouseEventHandler<HTMLButtonElement>;
  onMoveStart: () => void;
  onToggle: () => void;
}

function countdown(resetsAt: string | null) {
  if (!resetsAt) return "--:--";
  const remaining = Math.max(0, new Date(resetsAt).getTime() - Date.now());
  const totalHours = Math.floor(remaining / 3_600_000);
  const days = Math.floor(totalHours / 24);
  const hours = totalHours % 24;
  const minutes = Math.floor((remaining % 3_600_000) / 60_000);
  if (days > 0) return `${days}일 ${hours}시간`;
  return `${String(hours).padStart(2, "0")}:${String(minutes).padStart(2, "0")}`;
}

export function CompactBar({
  codex,
  system,
  expanded,
  overlay,
  onContextMenu,
  onMoveStart,
  onToggle,
}: CompactBarProps) {
  const primaryLimit = codex.primaryLimit;

  return (
    <button
      className={`compact-bar ${expanded ? "compact-bar-expanded" : ""} ${overlay ? "compact-bar-overlay" : ""}`}
      onClick={onToggle}
      onContextMenu={onContextMenu}
      aria-expanded={expanded}
      aria-label={expanded ? "상세 패널 닫기" : "상세 패널 열기"}
    >
      <div className="compact-column compact-column-primary">
        <MetricCell
          color="mint"
          label="CODEX"
          value={primaryLimit ? `${Math.round(primaryLimit.remainingPercent)}%` : "-"}
        />
        <div className="compact-secondary-row">
          <span>RESET</span>
          <strong>{primaryLimit ? countdown(primaryLimit.resetsAt) : "-"}</strong>
        </div>
      </div>
      <div className="compact-column">
        <MetricCell color="amber" label="CPU" value={system.available ? `${Math.round(system.cpuPercent)}%` : "-"} />
        <MetricCell color="violet" label="MEM" value={system.available ? `${Math.round(system.memoryPercent)}%` : "-"} />
      </div>
      {overlay && (
        <span
          className="drag-handle"
          data-tauri-drag-region
          aria-label="위젯 이동"
          onMouseDown={(event) => {
            if (event.button !== 0) return;
            onMoveStart();
          }}
          onClick={(event) => event.stopPropagation()}
        >
          <DotsSixVertical weight="bold" />
        </span>
      )}
    </button>
  );
}
