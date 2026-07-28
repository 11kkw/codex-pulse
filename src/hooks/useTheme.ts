import { useCallback, useEffect, useState } from "react";
import { emit, listen } from "@tauri-apps/api/event";
import { isTauriRuntime } from "../lib/runtime";

export type ThemeMode = "dark" | "light";

const THEME_STORAGE_KEY = "codex-pulse-theme";

function readStoredTheme(): ThemeMode {
  const previewTheme = new URLSearchParams(window.location.search).get("theme");
  if (previewTheme === "light" || previewTheme === "dark") {
    return previewTheme;
  }
  try {
    return window.localStorage.getItem(THEME_STORAGE_KEY) === "light" ? "light" : "dark";
  } catch {
    return "dark";
  }
}

export function useTheme() {
  const [theme, setThemeState] = useState<ThemeMode>(readStoredTheme);

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    document.documentElement.style.colorScheme = theme;
    try {
      window.localStorage.setItem(THEME_STORAGE_KEY, theme);
    } catch {
      // The selected theme still applies for the current run.
    }
  }, [theme]);

  useEffect(() => {
    if (!isTauriRuntime()) return;
    const unlisten = listen<ThemeMode>("monitor://set-theme", (event) => {
      setThemeState(event.payload);
    });
    return () => {
      void unlisten.then((dispose) => dispose());
    };
  }, []);

  useEffect(() => {
    const syncTheme = (event: StorageEvent) => {
      if (event.key === THEME_STORAGE_KEY && (event.newValue === "dark" || event.newValue === "light")) {
        setThemeState(event.newValue);
      }
    };
    window.addEventListener("storage", syncTheme);
    return () => window.removeEventListener("storage", syncTheme);
  }, []);

  const setTheme = useCallback((nextTheme: ThemeMode) => {
    setThemeState(nextTheme);
    if (isTauriRuntime()) {
      void emit("monitor://set-theme", nextTheme);
    }
  }, []);

  return { theme, setTheme };
}
