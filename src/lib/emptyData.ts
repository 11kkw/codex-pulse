import type { CodexSnapshot, SystemSnapshot } from "../types";

export function emptyCodexSnapshot(): CodexSnapshot {
  return {
    plan: null,
    email: null,
    connectionState: "fallback",
    source: "unavailable",
    statusMessage: "Codex 데이터를 조회할 수 없습니다.",
    primaryLimit: null,
    weeklyLimit: null,
    lifetimeTokens: null,
    peakDailyTokens: null,
    longestRunningTurnSeconds: null,
    currentStreakDays: null,
    longestStreakDays: null,
    dailyUsage: [],
    updatedAt: new Date().toISOString(),
  };
}

export function emptySystemSnapshot(): SystemSnapshot {
  return {
    available: false,
    cpuPercent: 0,
    memoryPercent: 0,
    usedMemoryBytes: 0,
    totalMemoryBytes: 0,
    cpuFrequencyMhz: null,
    cpuHistory: [],
    memoryHistory: [],
    updatedAt: new Date().toISOString(),
  };
}
