import { ArrowClockwise } from "@phosphor-icons/react/ArrowClockwise";
import { WarningCircle } from "@phosphor-icons/react/WarningCircle";
import type { CodexSnapshot, SystemSnapshot } from "../types";
import { MetricCell } from "./MetricCell";
import { MiniChart } from "./MiniChart";

const compactNumber = new Intl.NumberFormat("ko-KR", {
  notation: "compact",
  maximumFractionDigits: 2,
});

function formatBytes(bytes: number) {
  return `${(bytes / 1024 ** 3).toFixed(1)} GB`;
}

function formatWindow(minutes: number | null) {
  if (!minutes) return "기간 정보 없음";
  if (minutes % 1_440 === 0) return `${minutes / 1_440}일 기준`;
  if (minutes % 60 === 0) return `${minutes / 60}시간 기준`;
  return `${minutes}분 기준`;
}

function formatReset(resetsAt: string | null) {
  if (!resetsAt) return "초기화 시간 없음";
  const remaining = Math.max(0, new Date(resetsAt).getTime() - Date.now());
  const days = Math.floor(remaining / 86_400_000);
  const hours = Math.floor((remaining % 86_400_000) / 3_600_000);
  const minutes = Math.floor((remaining % 3_600_000) / 60_000);
  return days > 0 ? `${days}일 ${hours}시간 후 초기화` : `${hours}시간 ${minutes}분 후 초기화`;
}

function formatAbsoluteReset(resetsAt: string | null) {
  if (!resetsAt) return "-";
  return new Intl.DateTimeFormat("ko-KR", {
    month: "long",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
    hourCycle: "h23",
  }).format(new Date(resetsAt));
}

function maskEmail(email: string | null) {
  if (!email) return "로그인 정보 없음";
  const [local, domain] = email.split("@");
  if (!local || !domain) return email;
  const visible = Array.from(local).slice(0, 3).join("");
  return `${visible}***@${domain}`;
}

type UsageChange = {
  text: string;
  direction: "up" | "down" | "flat";
};

function previousDate(date: string) {
  const value = new Date(`${date}T00:00:00Z`);
  if (Number.isNaN(value.getTime())) return null;
  value.setUTCDate(value.getUTCDate() - 1);
  return value.toISOString().slice(0, 10);
}

function usageSummary(dailyUsage: { date: string; tokens: number }[]) {
  if (!dailyUsage.length) return { tokens: null, change: null };
  const ordered = [...dailyUsage].sort((left, right) => left.date.localeCompare(right.date));
  const latest = ordered.at(-1)!;
  const previousDateKey = previousDate(latest.date);
  const yesterday = previousDateKey
    ? ordered.find((usage) => usage.date === previousDateKey)?.tokens ?? 0
    : null;

  if (yesterday === null) return { tokens: latest.tokens, change: null };

  const difference = latest.tokens - yesterday;
  if (difference === 0) {
    return {
      tokens: latest.tokens,
      change: { text: "±0%", direction: "flat" } satisfies UsageChange,
    };
  }

  if (yesterday === 0) {
    return {
      tokens: latest.tokens,
      change: {
        text: "어제 0",
        direction: "flat",
      } satisfies UsageChange,
    };
  }

  const percent = Math.round(Math.abs((difference / yesterday) * 100));
  return {
    tokens: latest.tokens,
    change: {
      text: `${difference > 0 ? "+" : "−"}${percent}%`,
      direction: difference > 0 ? "up" : "down",
    } satisfies UsageChange,
  };
}

interface DetailPanelProps {
  codex: CodexSnapshot;
  system: SystemSnapshot;
  isRefreshing: boolean;
  draggable: boolean;
  onRefresh: () => void;
}

export function DetailPanel({ codex, system, isRefreshing, draggable, onRefresh }: DetailPanelProps) {
  const isConnected = codex.connectionState === "connected";
  const primaryLimit = codex.primaryLimit;
  const todayUsage = usageSummary(codex.dailyUsage);

  return (
    <section className="detail-panel" aria-label="Codex 및 시스템 상세 사용량">
      <header
        className={`panel-header ${draggable ? "panel-header-draggable" : ""}`}
        data-tauri-drag-region={draggable ? "" : undefined}
      >
        <div>
          <div className="account-row">
            <span className="plan-badge">{codex.plan ?? "CODEX"}</span>
            <span className="account-label" title={codex.email ?? undefined}>{maskEmail(codex.email)}</span>
          </div>
          {!isConnected && (
            <div className="connection-state connection-fallback">
              <WarningCircle weight="fill" />
              <span>{codex.statusMessage}</span>
            </div>
          )}
        </div>
        <button className="refresh-button" onClick={onRefresh} aria-label="Codex 사용량 새로고침">
          <ArrowClockwise className={isRefreshing ? "spinning" : ""} />
        </button>
      </header>

      <div className="summary-grid">
        <MetricCell
          color="mint"
          label="CODEX"
          value={primaryLimit ? `${Math.round(primaryLimit.remainingPercent)}%` : "-"}
        />
        <MetricCell color="amber" label="CPU" value={system.available ? `${Math.round(system.cpuPercent)}%` : "-"} />
        <div className="reset-summary">
          <span>RESET</span>
          <strong>{primaryLimit ? formatAbsoluteReset(primaryLimit.resetsAt) : "-"}</strong>
        </div>
        <MetricCell color="violet" label="MEM" value={system.available ? `${Math.round(system.memoryPercent)}%` : "-"} />
      </div>

      <section className="panel-section limit-section">
        <div className="section-heading">
          <span>{primaryLimit?.label ?? "Codex 사용 한도"}</span>
          <strong>{primaryLimit ? `${Math.round(primaryLimit.remainingPercent)}% 남음` : "-"}</strong>
        </div>
        <div className="progress-track">
          <span style={{ width: `${primaryLimit?.remainingPercent ?? 0}%` }} />
        </div>
        <div className="section-meta">
          <span>{formatWindow(primaryLimit?.windowDurationMinutes ?? null)}</span>
          <span>{primaryLimit ? formatReset(primaryLimit.resetsAt) : "-"}</span>
        </div>
      </section>

      <section className="token-section">
        <div className="token-row">
          <div className="token-label">
            <span>오늘 토큰</span>
            {todayUsage.change && (
              <small className={`usage-change usage-change-${todayUsage.change.direction}`}>
                {todayUsage.change.text}
              </small>
            )}
          </div>
          <strong>{todayUsage.tokens === null ? "—" : compactNumber.format(todayUsage.tokens)}</strong>
        </div>
        <div className="token-row">
          <span>누적 토큰</span>
          <strong>{codex.lifetimeTokens ? compactNumber.format(codex.lifetimeTokens) : "—"}</strong>
        </div>
        {codex.weeklyLimit && (
          <div className="token-row">
            <span>{codex.weeklyLimit.label}</span>
            <strong className="violet-text">{Math.round(codex.weeklyLimit.remainingPercent)}%</strong>
          </div>
        )}
      </section>

      <section className="resource-section">
        <div className="resource-heading">
          <MetricCell color="amber" label="CPU" value={system.available ? `${Math.round(system.cpuPercent)}%` : "-"} />
          <span>{system.available && system.cpuFrequencyMhz ? `${(system.cpuFrequencyMhz / 1000).toFixed(1)} GHz` : "-"}</span>
        </div>
        <MiniChart values={system.cpuHistory} color="#f5b91e" />
      </section>

      <section className="resource-section">
        <div className="resource-heading">
          <MetricCell color="violet" label="MEM" value={system.available ? `${Math.round(system.memoryPercent)}%` : "-"} />
          <span>{system.available ? `${formatBytes(system.usedMemoryBytes)} / ${formatBytes(system.totalMemoryBytes)}` : "-"}</span>
        </div>
        <MiniChart values={system.memoryHistory} color="#9b68e8" />
      </section>

      <footer className="panel-footer">
        <span>{codex.currentStreakDays ? `${codex.currentStreakDays}일 연속 사용` : "-"}</span>
        <span>{new Date(codex.updatedAt).toLocaleTimeString("ko-KR", { hour: "2-digit", minute: "2-digit" })} 업데이트</span>
      </footer>
    </section>
  );
}
