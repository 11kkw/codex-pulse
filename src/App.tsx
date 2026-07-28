import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { CompactBar } from "./components/CompactBar";
import { DetailPanel } from "./components/DetailPanel";
import { useMonitorData } from "./hooks/useMonitorData";
import { useTheme } from "./hooks/useTheme";
import { isTauriRuntime } from "./lib/runtime";
import { openWidgetContextMenu } from "./lib/widgetContextMenu";

type PlacementMode = "taskbar" | "overlay";

function usePlacementMode() {
  const [placementMode, setPlacementMode] = useState<PlacementMode>("taskbar");

  useEffect(() => {
    if (!isTauriRuntime()) return;
    void invoke<PlacementMode>("get_placement_mode").then(setPlacementMode);
    const unlisten = listen<PlacementMode>("monitor://placement-mode", (event) => {
      setPlacementMode(event.payload);
    });
    return () => {
      void unlisten.then((dispose) => dispose());
    };
  }, []);

  return placementMode;
}

function CompactWindow() {
  const { codex, system, refreshCodex } = useMonitorData();
  const { theme, setTheme } = useTheme();
  const placementMode = usePlacementMode();
  const [detailVisible, setDetailVisible] = useState(false);

  useEffect(() => {
    const unlisten = listen<boolean>("monitor://detail-visible", (event) => {
      setDetailVisible(event.payload);
    });
    return () => {
      void unlisten.then((dispose) => dispose());
    };
  }, []);

  useEffect(() => {
    const unlisten = listen("monitor://context-menu-request", () => {
      void openWidgetContextMenu({
        detailVisible,
        placementMode,
        onDetailVisibilityChange: setDetailVisible,
        onRefresh: refreshCodex,
        theme,
        onThemeChange: setTheme,
      });
    });
    return () => {
      void unlisten.then((dispose) => dispose());
    };
  }, [detailVisible, placementMode, refreshCodex, setTheme, theme]);

  return (
    <main className={`native-stage native-stage-compact native-stage-${placementMode}`}>
      <CompactBar
        codex={codex}
        system={system}
        expanded={detailVisible}
        overlay={placementMode === "overlay"}
        onContextMenu={(event) => {
          event.preventDefault();
          event.stopPropagation();
          if (placementMode === "taskbar") return;
          void openWidgetContextMenu({
            detailVisible,
            placementMode,
            onDetailVisibilityChange: setDetailVisible,
            onRefresh: refreshCodex,
            theme,
            onThemeChange: setTheme,
          });
        }}
        onMoveStart={() => {
          void invoke("hide_detail");
        }}
        onToggle={() => {
          if (placementMode === "taskbar") return;
          void invoke<boolean>("toggle_detail").then(setDetailVisible);
        }}
      />
    </main>
  );
}

function DetailWindow() {
  const { codex, system, isRefreshing, refreshCodex } = useMonitorData();
  useTheme();

  return (
    <main className="native-stage native-stage-detail">
      <DetailPanel
        codex={codex}
        system={system}
        isRefreshing={isRefreshing}
        draggable={false}
        onRefresh={() => void refreshCodex()}
      />
    </main>
  );
}

function BrowserPreview() {
  const { codex, system, isRefreshing, refreshCodex } = useMonitorData();
  useTheme();
  const [expanded, setExpanded] = useState(true);
  const dockPreview = new URLSearchParams(window.location.search).get("dock") === "1";

  if (dockPreview) {
    return (
      <main className="native-stage native-stage-compact native-stage-taskbar dock-preview-stage">
        <CompactBar
          codex={codex}
          system={system}
          expanded={false}
          overlay={false}
          onMoveStart={() => undefined}
          onToggle={() => undefined}
        />
      </main>
    );
  }

  return (
    <main className="preview-stage">
      <div className="preview-copy">
        <span>CODEX PULSE</span>
        <p>Taskbar telemetry prototype</p>
      </div>
      <div className="preview-widget-slot">
        <div className="widget widget-expanded">
          {expanded && (
            <div className="detail-wrap detail-wrap-open">
              <DetailPanel
                codex={codex}
                system={system}
                isRefreshing={isRefreshing}
                draggable={false}
                onRefresh={() => void refreshCodex()}
              />
            </div>
          )}
          <CompactBar
            codex={codex}
            system={system}
            expanded={expanded}
            overlay
            onMoveStart={() => undefined}
            onToggle={() => setExpanded((value) => !value)}
          />
        </div>
      </div>
      <div className="mock-taskbar" aria-hidden="true">
        <div className="mock-start">⊞</div>
        <div className="mock-search">검색</div>
        <div className="mock-apps"><i /><i /><i /><i /></div>
        <div className="mock-tray">⌃　Wi-Fi　12:20</div>
      </div>
    </main>
  );
}

export function App() {
  if (!isTauriRuntime()) return <BrowserPreview />;
  const view = new URLSearchParams(window.location.search).get("view");
  return view === "detail" ? <DetailWindow /> : <CompactWindow />;
}
