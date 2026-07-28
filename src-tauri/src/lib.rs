mod alerts;
mod codex;
#[cfg(windows)]
mod direct_text;
mod logging;
mod model;
#[cfg(windows)]
mod native_widget;
mod system_monitor;
mod taskbar;

use std::{
    sync::{mpsc, Arc, Mutex},
    time::Duration,
};

use codex::CodexProvider;
use model::{CodexSnapshot, SystemSnapshot};
#[cfg(windows)]
use native_widget::{NativeWidgetController, NativeWidgetEvent, WidgetTheme};
use system_monitor::SystemMonitor;
use taskbar::PlacementMode;
use tauri::{
    menu::{CheckMenuItem, CheckMenuItemBuilder, MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, State, WindowEvent,
};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt as AutostartManagerExt};

struct CodexState(Arc<Mutex<CodexProvider>>);
struct MonitorState(Arc<Mutex<SystemMonitor>>);
struct PlacementState(Mutex<PlacementMode>);
struct OverlayPositionState(Mutex<Option<tauri::PhysicalPosition<i32>>>);
struct TelemetryCache {
    codex: Mutex<Option<CodexSnapshot>>,
    system: Mutex<Option<SystemSnapshot>>,
}
struct TelemetryState(Arc<TelemetryCache>);
#[cfg(windows)]
struct NativeWidgetState(Arc<NativeWidgetController>);
#[cfg(windows)]
struct AutostartMenuState(CheckMenuItem<tauri::Wry>);

#[cfg(windows)]
struct SingleInstanceGuard(windows_sys::Win32::Foundation::HANDLE);

#[cfg(windows)]
impl SingleInstanceGuard {
    fn acquire() -> Option<Self> {
        use windows_sys::Win32::{
            Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS},
            System::Threading::CreateMutexW,
        };

        let name: Vec<u16> = "Local\\CodexPulse.Widget.SingleInstance\0"
            .encode_utf16()
            .collect();
        let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
        if handle.is_null() {
            return None;
        }
        if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            unsafe {
                CloseHandle(handle);
            }
            return None;
        }
        Some(Self(handle))
    }
}

#[cfg(windows)]
impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.0);
        }
    }
}

#[tauri::command]
async fn get_codex_snapshot(
    state: State<'_, CodexState>,
    telemetry: State<'_, TelemetryState>,
    native: State<'_, NativeWidgetState>,
) -> Result<CodexSnapshot, String> {
    if let Some(snapshot) = telemetry
        .0
        .codex
        .lock()
        .map_err(|_| "Codex 캐시 잠금이 손상되었습니다.".to_string())?
        .clone()
    {
        return Ok(snapshot);
    }

    let provider = state.0.clone();
    let snapshot = tauri::async_runtime::spawn_blocking(move || {
        provider
            .lock()
            .map_err(|_| "Codex 데이터 잠금이 손상되었습니다.".to_string())
            .map(|mut provider| provider.snapshot())
    })
    .await
    .map_err(|error| error.to_string())??;
    native.0.update_codex(&snapshot);
    if let Ok(mut cached) = telemetry.0.codex.lock() {
        *cached = Some(snapshot.clone());
    }
    Ok(snapshot)
}

#[tauri::command]
fn get_system_snapshot(
    state: State<'_, MonitorState>,
    telemetry: State<'_, TelemetryState>,
    native: State<'_, NativeWidgetState>,
) -> Result<SystemSnapshot, String> {
    if let Some(snapshot) = telemetry
        .0
        .system
        .lock()
        .map_err(|_| "시스템 캐시 잠금이 손상되었습니다.".to_string())?
        .clone()
    {
        return Ok(snapshot);
    }

    let snapshot = state
        .0
        .lock()
        .map_err(|_| "시스템 데이터 잠금이 손상되었습니다.".to_string())
        .map(|mut monitor| monitor.snapshot())?;
    native.0.update_system(&snapshot);
    if let Ok(mut cached) = telemetry.0.system.lock() {
        *cached = Some(snapshot.clone());
    }
    Ok(snapshot)
}

#[tauri::command]
fn get_placement_mode(placement: State<'_, PlacementState>) -> Result<String, String> {
    placement
        .0
        .lock()
        .map(|mode| mode.as_str().to_string())
        .map_err(|_| "위젯 배치 상태가 손상되었습니다.".to_string())
}

#[tauri::command]
fn toggle_detail(app: tauri::AppHandle) -> Result<bool, String> {
    let detail = app
        .get_webview_window("detail")
        .ok_or_else(|| "상세 패널 창을 찾을 수 없습니다.".to_string())?;
    if detail.is_visible().map_err(|error| error.to_string())? {
        hide_detail_window(&app);
        return Ok(false);
    }

    let mode = app
        .state::<PlacementState>()
        .0
        .lock()
        .map(|mode| *mode)
        .unwrap_or_default();
    if mode == PlacementMode::Taskbar {
        let native = app.state::<NativeWidgetState>();
        let bounds = native
            .0
            .bounds()
            .ok_or_else(|| "네이티브 미니 위젯의 위치를 찾을 수 없습니다.".to_string())?;
        taskbar::place_detail_at(bounds, native.0.scale_factor(), &detail)?;
    } else {
        let main = app
            .get_webview_window("main")
            .ok_or_else(|| "미니 위젯 창을 찾을 수 없습니다.".to_string())?;
        taskbar::place_detail(&main, &detail)?;
    }
    detail
        .set_always_on_top(true)
        .map_err(|error| error.to_string())?;
    detail.show().map_err(|error| error.to_string())?;
    let _ = detail.set_focus();
    let _ = app.emit("monitor://detail-visible", true);
    app.state::<NativeWidgetState>().0.set_detail_visible(true);
    Ok(true)
}

#[tauri::command]
fn hide_detail(app: tauri::AppHandle) {
    hide_detail_window(&app);
}

#[tauri::command]
fn change_placement_mode(app: tauri::AppHandle, mode: String) -> Result<(), String> {
    let mode = match mode.as_str() {
        "taskbar" => PlacementMode::Taskbar,
        "overlay" => PlacementMode::Overlay,
        _ => return Err("지원하지 않는 위젯 배치 모드입니다.".to_string()),
    };
    set_placement_mode(&app, mode);
    Ok(())
}

#[tauri::command]
fn quit_app(app: tauri::AppHandle) {
    app.state::<NativeWidgetState>().0.shutdown();
    app.exit(0);
}

fn hide_detail_window(app: &tauri::AppHandle) {
    if let Some(detail) = app.get_webview_window("detail") {
        let _ = detail.hide();
    }
    let _ = app.emit("monitor://detail-visible", false);
    if let Some(native) = app.try_state::<NativeWidgetState>() {
        native.0.set_detail_visible(false);
    }
}

fn show_compact(app: &tauri::AppHandle) {
    hide_detail_window(app);
    let mode = app
        .state::<PlacementState>()
        .0
        .lock()
        .map(|mode| *mode)
        .unwrap_or_default();
    let native = app.state::<NativeWidgetState>();
    if mode == PlacementMode::Taskbar {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.hide();
        }
        native.0.show_taskbar();
    } else {
        native.0.hide();
        if let Some(window) = app.get_webview_window("main") {
            let saved_position = app
                .state::<OverlayPositionState>()
                .0
                .lock()
                .ok()
                .and_then(|position| *position);
            if let Some(position) = saved_position {
                let _ = window.set_position(position);
            } else {
                let _ = taskbar::place_main(&window, PlacementMode::Overlay);
            }
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

fn set_placement_mode(app: &tauri::AppHandle, mode: PlacementMode) {
    logging::write(format!("placement mode changed: {}", mode.as_str()));
    let previous = app
        .state::<PlacementState>()
        .0
        .lock()
        .map(|current| *current)
        .unwrap_or_default();
    if previous == PlacementMode::Overlay && mode != PlacementMode::Overlay {
        if let Some(window) = app.get_webview_window("main") {
            if let Ok(position) = window.outer_position() {
                if let Ok(mut saved) = app.state::<OverlayPositionState>().0.lock() {
                    *saved = Some(position);
                }
            }
        }
    }
    if let Ok(mut current) = app.state::<PlacementState>().0.lock() {
        *current = mode;
    }
    hide_detail_window(app);
    let _ = app.emit("monitor://placement-mode", mode.as_str());
    show_compact(app);
}

fn set_theme(app: &tauri::AppHandle, theme: WidgetTheme) {
    app.state::<NativeWidgetState>().0.set_theme(theme);
    let _ = app.emit("monitor://set-theme", theme.as_str());
}

fn sync_autostart_state(app: &tauri::AppHandle, enabled: bool) {
    app.state::<NativeWidgetState>()
        .0
        .set_autostart_enabled(enabled);
    if let Some(menu) = app.try_state::<AutostartMenuState>() {
        let _ = menu.0.set_checked(enabled);
    }
}

fn toggle_autostart(app: &tauri::AppHandle) {
    let manager = app.autolaunch();
    let enabled = manager.is_enabled().unwrap_or(false);
    let result = if enabled {
        manager.disable()
    } else {
        manager.enable()
    };
    match result {
        Ok(()) => {
            let current = manager.is_enabled().unwrap_or(!enabled);
            sync_autostart_state(app, current);
            logging::write(format!("autostart changed: {current}"));
        }
        Err(error) => logging::write(format!("autostart change failed: {error}")),
    }
}

fn refresh_codex_now(app: &tauri::AppHandle) {
    let provider = app.state::<CodexState>().0.clone();
    let telemetry = app.state::<TelemetryState>().0.clone();
    let native = app.state::<NativeWidgetState>().0.clone();
    std::thread::spawn(move || {
        let snapshot = provider.lock().ok().map(|mut provider| provider.snapshot());
        if let Some(snapshot) = snapshot {
            native.update_codex(&snapshot);
            if let Ok(mut cached) = telemetry.codex.lock() {
                *cached = Some(snapshot);
            }
        }
    });
}

fn handle_native_widget_event(app: &tauri::AppHandle, event: NativeWidgetEvent) {
    logging::write(format!("native widget event: {event:?}"));
    match event {
        NativeWidgetEvent::ToggleDetail => {
            let _ = toggle_detail(app.clone());
        }
        NativeWidgetEvent::Refresh => {
            let _ = app.emit("monitor://refresh", ());
            refresh_codex_now(app);
        }
        NativeWidgetEvent::SwitchToOverlay => {
            set_placement_mode(app, PlacementMode::Overlay);
        }
        NativeWidgetEvent::SetTheme(theme) => set_theme(app, theme),
        NativeWidgetEvent::ToggleAutostart => toggle_autostart(app),
        NativeWidgetEvent::Hide => {
            app.state::<NativeWidgetState>().0.hide();
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.hide();
            }
            hide_detail_window(app);
        }
        NativeWidgetEvent::Quit => {
            app.state::<NativeWidgetState>().0.shutdown();
            app.exit(0);
        }
    }
}

fn start_telemetry_workers(app: tauri::AppHandle) {
    let system_app = app.clone();
    std::thread::Builder::new()
        .name("codex-pulse-system-monitor".to_string())
        .spawn(move || loop {
            let monitor = system_app.state::<MonitorState>().0.clone();
            let telemetry = system_app.state::<TelemetryState>().0.clone();
            let native = system_app.state::<NativeWidgetState>().0.clone();
            if let Ok(mut monitor) = monitor.lock() {
                let snapshot = monitor.snapshot();
                native.update_system(&snapshot);
                if let Ok(mut cached) = telemetry.system.lock() {
                    *cached = Some(snapshot);
                }
            }
            std::thread::sleep(Duration::from_secs(1));
        })
        .ok();

    std::thread::Builder::new()
        .name("codex-pulse-codex-monitor".to_string())
        .spawn(move || {
            let mut alerts = alerts::AlertTracker::load(&app);
            loop {
                let provider = app.state::<CodexState>().0.clone();
                let telemetry = app.state::<TelemetryState>().0.clone();
                let native = app.state::<NativeWidgetState>().0.clone();
                if let Ok(mut provider) = provider.lock() {
                    let snapshot = provider.snapshot();
                    alerts.evaluate(&app, &snapshot);
                    native.update_codex(&snapshot);
                    if let Ok(mut cached) = telemetry.codex.lock() {
                        *cached = Some(snapshot);
                    }
                }
                std::thread::sleep(Duration::from_secs(30));
            }
        })
        .ok();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(windows)]
    let _single_instance = match SingleInstanceGuard::acquire() {
        Some(guard) => guard,
        None => {
            logging::write("second instance ignored");
            return;
        }
    };

    logging::write(format!(
        "application starting; log={}",
        logging::log_path()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "unavailable".to_string())
    ));
    std::panic::set_hook(Box::new(|panic_info| {
        logging::write(format!("panic: {panic_info}"));
    }));

    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .manage(CodexState(Arc::new(Mutex::new(CodexProvider::new()))))
        .manage(MonitorState(Arc::new(Mutex::new(SystemMonitor::new()))))
        .manage(PlacementState(Mutex::new(PlacementMode::Taskbar)))
        .manage(OverlayPositionState(Mutex::new(None)))
        .manage(TelemetryState(Arc::new(TelemetryCache {
            codex: Mutex::new(None),
            system: Mutex::new(None),
        })))
        .invoke_handler(tauri::generate_handler![
            get_codex_snapshot,
            get_system_snapshot,
            get_placement_mode,
            toggle_detail,
            hide_detail,
            change_placement_mode,
            quit_app
        ])
        .setup(|app| {
            alerts::ensure_permission(app.handle());
            if let Some(detail) = app.get_webview_window("detail") {
                taskbar::prepare_detail_window(&detail).map_err(std::io::Error::other)?;
            }
            let (widget_events_tx, widget_events_rx) = mpsc::channel();
            let native_widget =
                NativeWidgetController::start(widget_events_tx).map_err(std::io::Error::other)?;
            let autostart_enabled = app.autolaunch().is_enabled().unwrap_or(false);
            native_widget.set_autostart_enabled(autostart_enabled);
            app.manage(NativeWidgetState(native_widget));

            let event_app = app.handle().clone();
            std::thread::Builder::new()
                .name("codex-pulse-native-events".to_string())
                .spawn(move || {
                    while let Ok(event) = widget_events_rx.recv() {
                        let app_for_event = event_app.clone();
                        let _ = event_app.run_on_main_thread(move || {
                            handle_native_widget_event(&app_for_event, event);
                        });
                    }
                })?;

            let show = MenuItemBuilder::with_id("show", "위젯 열기").build(app)?;
            let taskbar_mode =
                MenuItemBuilder::with_id("taskbar_mode", "작업표시줄 도킹").build(app)?;
            let overlay_mode = MenuItemBuilder::with_id("overlay_mode", "자유 배치").build(app)?;
            let light_theme = MenuItemBuilder::with_id("light_theme", "라이트 모드").build(app)?;
            let dark_theme = MenuItemBuilder::with_id("dark_theme", "다크 모드").build(app)?;
            let autostart = CheckMenuItemBuilder::with_id("autostart", "Windows 시작 시 자동 실행")
                .checked(autostart_enabled)
                .build(app)?;
            let refresh = MenuItemBuilder::with_id("refresh", "사용량 새로고침").build(app)?;
            let hide = MenuItemBuilder::with_id("hide", "위젯 숨기기").build(app)?;
            let quit = MenuItemBuilder::with_id("quit", "앱 종료").build(app)?;
            let menu = MenuBuilder::new(app)
                .items(&[
                    &show,
                    &taskbar_mode,
                    &overlay_mode,
                    &light_theme,
                    &dark_theme,
                    &autostart,
                    &refresh,
                    &hide,
                    &quit,
                ])
                .build()?;
            app.manage(AutostartMenuState(autostart));

            let mut tray = TrayIconBuilder::new()
                .tooltip("Codex Pulse")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "show" => show_compact(app),
                    "taskbar_mode" => set_placement_mode(app, PlacementMode::Taskbar),
                    "overlay_mode" => set_placement_mode(app, PlacementMode::Overlay),
                    "light_theme" => set_theme(app, WidgetTheme::Light),
                    "dark_theme" => set_theme(app, WidgetTheme::Dark),
                    "autostart" => toggle_autostart(app),
                    "refresh" => {
                        let _ = app.emit("monitor://refresh", ());
                        refresh_codex_now(app);
                    }
                    "hide" => {
                        app.state::<NativeWidgetState>().0.hide();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.hide();
                        }
                        hide_detail_window(app);
                    }
                    "quit" => {
                        app.state::<NativeWidgetState>().0.shutdown();
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_compact(tray.app_handle());
                    }
                });

            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.build(app)?;

            if let Some(window) = app.get_webview_window("main") {
                let _ = window.hide();
            }
            app.state::<NativeWidgetState>().0.show_taskbar();
            start_telemetry_workers(app.handle().clone());
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let WindowEvent::Moved(position) = event {
                    let is_overlay = window
                        .app_handle()
                        .state::<PlacementState>()
                        .0
                        .lock()
                        .map(|mode| *mode == PlacementMode::Overlay)
                        .unwrap_or(false);
                    if is_overlay {
                        if let Ok(mut saved) =
                            window.app_handle().state::<OverlayPositionState>().0.lock()
                        {
                            *saved = Some(*position);
                        }
                    }
                }
            }
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
                hide_detail_window(window.app_handle());
            }
            if window.label() == "detail" && matches!(event, WindowEvent::Focused(false)) {
                let app = window.app_handle().clone();
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_millis(90));
                    let app_for_check = app.clone();
                    let _ = app.run_on_main_thread(move || {
                        let detail_focused = app_for_check
                            .get_webview_window("detail")
                            .and_then(|window| window.is_focused().ok())
                            .unwrap_or(false);
                        let native_interaction = app_for_check
                            .try_state::<NativeWidgetState>()
                            .map(|native| native.0.should_suppress_detail_blur())
                            .unwrap_or(false);
                        if !detail_focused && !native_interaction {
                            hide_detail_window(&app_for_check);
                        }
                    });
                });
            }
        })
        .run(tauri::generate_context!())
        .expect("Codex Pulse 실행 중 오류가 발생했습니다.");
}
