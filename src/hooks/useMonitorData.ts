import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { emptyCodexSnapshot, emptySystemSnapshot } from "../lib/emptyData";
import { isTauriRuntime } from "../lib/runtime";
import type { CodexSnapshot, SystemSnapshot } from "../types";

const SYSTEM_REFRESH_MS = 1_000;
const CODEX_REFRESH_MS = 30_000;

async function readCodex() {
  if (!isTauriRuntime()) return emptyCodexSnapshot();
  return invoke<CodexSnapshot>("get_codex_snapshot");
}

async function readSystem() {
  if (!isTauriRuntime()) return emptySystemSnapshot();
  return invoke<SystemSnapshot>("get_system_snapshot");
}

export function useMonitorData() {
  const [codex, setCodex] = useState<CodexSnapshot>(() => emptyCodexSnapshot());
  const [system, setSystem] = useState<SystemSnapshot>(() => emptySystemSnapshot());
  const [isRefreshing, setIsRefreshing] = useState(false);

  const refreshCodex = useCallback(async () => {
    setIsRefreshing(true);
    try {
      setCodex(await readCodex());
    } finally {
      setIsRefreshing(false);
    }
  }, []);

  const refreshSystem = useCallback(async () => {
    try {
      setSystem(await readSystem());
    } catch {
      // Preserve the last healthy sample during a transient native refresh.
    }
  }, []);

  useEffect(() => {
    void refreshCodex();
    void refreshSystem();
    const systemTimer = window.setInterval(() => void refreshSystem(), SYSTEM_REFRESH_MS);
    const codexTimer = window.setInterval(() => void refreshCodex(), CODEX_REFRESH_MS);
    const unlisten = isTauriRuntime()
      ? listen("monitor://refresh", () => void refreshCodex())
      : Promise.resolve(() => undefined);

    return () => {
      window.clearInterval(systemTimer);
      window.clearInterval(codexTimer);
      void unlisten.then((dispose) => dispose());
    };
  }, [refreshCodex, refreshSystem]);

  return { codex, system, isRefreshing, refreshCodex };
}
