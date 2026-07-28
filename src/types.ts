export type ConnectionState = "connected" | "fallback" | "signed-out";

export interface RateLimit {
  label: string;
  remainingPercent: number;
  usedPercent: number;
  windowDurationMinutes: number | null;
  resetsAt: string | null;
}

export interface DailyUsage {
  date: string;
  tokens: number;
}

export interface CodexSnapshot {
  plan: string | null;
  email: string | null;
  connectionState: ConnectionState;
  source: "codex-app-server" | "unavailable";
  statusMessage: string;
  primaryLimit: RateLimit | null;
  weeklyLimit: RateLimit | null;
  lifetimeTokens: number | null;
  peakDailyTokens: number | null;
  longestRunningTurnSeconds: number | null;
  currentStreakDays: number | null;
  longestStreakDays: number | null;
  dailyUsage: DailyUsage[];
  updatedAt: string;
}

export interface SystemSnapshot {
  available: boolean;
  cpuPercent: number;
  memoryPercent: number;
  usedMemoryBytes: number;
  totalMemoryBytes: number;
  cpuFrequencyMhz: number | null;
  cpuHistory: number[];
  memoryHistory: number[];
  updatedAt: string;
}
