import { invoke } from "@tauri-apps/api/core";
import { Menu } from "@tauri-apps/api/menu";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { ThemeMode } from "../hooks/useTheme";

type PlacementMode = "taskbar" | "menubar" | "overlay";

interface WidgetContextMenuOptions {
  detailVisible: boolean;
  placementMode: PlacementMode;
  onDetailVisibilityChange: (visible: boolean) => void;
  onRefresh: () => void | Promise<void>;
  theme: ThemeMode;
  onThemeChange: (theme: ThemeMode) => void;
}

export async function openWidgetContextMenu({
  detailVisible,
  placementMode,
  onDetailVisibilityChange,
  onRefresh,
  theme,
  onThemeChange,
}: WidgetContextMenuOptions) {
  const isWindows = navigator.userAgent.includes("Windows");
  const nextMode: PlacementMode = placementMode === "overlay"
    ? isWindows ? "taskbar" : "menubar"
    : "overlay";
  const placementItems = [
    {
      id: "placement",
      text: nextMode === "overlay"
        ? "자유 배치"
        : nextMode === "taskbar"
          ? "작업표시줄 도킹"
          : "메뉴 막대에 표시",
      action: () => {
        void invoke("change_placement_mode", { mode: nextMode });
      },
    },
  ];
  const menu = await Menu.new({
    items: [
      {
        id: "detail",
        text: detailVisible ? "상세 정보 닫기" : "상세 정보 열기",
        action: () => {
          void invoke<boolean>("toggle_detail").then(onDetailVisibilityChange);
        },
      },
      {
        id: "refresh",
        text: "사용량 새로고침",
        action: () => {
          void onRefresh();
        },
      },
      ...placementItems,
      {
        id: "theme",
        text: theme === "dark" ? "라이트 모드" : "다크 모드",
        action: () => {
          onThemeChange(theme === "dark" ? "light" : "dark");
        },
      },
      {
        id: "quit",
        text: "앱 종료",
        action: () => {
          void invoke("quit_app");
        },
      },
    ],
  });

  await menu.popup(undefined, getCurrentWindow());
}
