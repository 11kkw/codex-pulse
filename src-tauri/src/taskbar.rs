use tauri::WebviewWindow;

const COMPACT_WIDTH: f64 = 306.0;
const COMPACT_HEIGHT: f64 = 64.0;
const DETAIL_WIDTH: f64 = 306.0;
const DETAIL_HEIGHT: f64 = 496.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PlacementMode {
    #[default]
    Taskbar,
    Overlay,
}

impl PlacementMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Taskbar => "taskbar",
            Self::Overlay => "overlay",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Bounds {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[derive(Clone, Copy, Debug)]
pub struct TaskbarWindow {
    pub hwnd: isize,
    pub bounds: Bounds,
}

pub fn place_main(window: &WebviewWindow, mode: PlacementMode) -> Result<(), String> {
    window
        .set_skip_taskbar(true)
        .map_err(|error| error.to_string())?;
    window
        .set_always_on_top(true)
        .map_err(|error| error.to_string())?;
    prepare_popup_window(window)?;

    if mode == PlacementMode::Taskbar {
        return Err("작업표시줄 모드는 네이티브 미니 위젯에서 처리됩니다.".to_string());
    }

    let scale = window.scale_factor().map_err(|error| error.to_string())?;
    let work_area = monitor_work_area(window)
        .ok_or_else(|| "현재 모니터의 작업 영역을 찾을 수 없습니다.".to_string())?;
    let margin = (16.0 * scale).round() as i32;
    let width = (COMPACT_WIDTH * scale).round() as u32;
    let height = (COMPACT_HEIGHT * scale).round() as u32;
    let x = work_area.left + margin;
    let bottom = work_area.bottom - margin;
    let y = (bottom - height as i32).max(work_area.top);

    apply_bounds(window, x, y, width, height)
}

#[cfg(windows)]
fn prepare_popup_window(window: &WebviewWindow) -> Result<(), String> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE, GWL_STYLE, HWND_TOPMOST,
        SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOOWNERZORDER, SWP_NOSIZE, WS_BORDER,
        WS_CAPTION, WS_CHILD, WS_DLGFRAME, WS_EX_APPWINDOW, WS_EX_CLIENTEDGE, WS_EX_DLGMODALFRAME,
        WS_EX_TOOLWINDOW, WS_EX_WINDOWEDGE, WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_POPUP, WS_SYSMENU,
        WS_THICKFRAME,
    };

    let hwnd = window.hwnd().map_err(|error| error.to_string())?.0;
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
        let popup_style = (style | WS_POPUP as isize)
            & !((WS_CHILD
                | WS_BORDER
                | WS_CAPTION
                | WS_DLGFRAME
                | WS_MAXIMIZEBOX
                | WS_MINIMIZEBOX
                | WS_SYSMENU
                | WS_THICKFRAME) as isize);
        let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let popup_ex_style = (ex_style | WS_EX_TOOLWINDOW as isize)
            & !((WS_EX_APPWINDOW | WS_EX_CLIENTEDGE | WS_EX_DLGMODALFRAME | WS_EX_WINDOWEDGE)
                as isize);
        let frame_changed = style != popup_style || ex_style != popup_ex_style;
        if style != popup_style {
            SetWindowLongPtrW(hwnd, GWL_STYLE, popup_style);
        }
        if ex_style != popup_ex_style {
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, popup_ex_style);
        }
        if frame_changed {
            crate::logging::write(format!(
                "popup frame normalized; hwnd={hwnd:?}, style={popup_style:#x}, ex_style={popup_ex_style:#x}"
            ));
        }
        let flags = SWP_NOMOVE
            | SWP_NOSIZE
            | SWP_NOACTIVATE
            | SWP_NOOWNERZORDER
            | if frame_changed { SWP_FRAMECHANGED } else { 0 };
        if SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, flags) == 0 {
            return Err("독립 위젯 창 스타일을 적용하지 못했습니다.".into());
        }
    }
    Ok(())
}

#[cfg(not(windows))]
fn prepare_popup_window(_window: &WebviewWindow) -> Result<(), String> {
    Ok(())
}

fn dock_to_taskbar(window: &WebviewWindow) -> Result<(), String> {
    let bounds = taskbar_dock_bounds(window)?;
    crate::logging::write(format!(
        "docking independent popup over taskbar; x={}, y={}, width={}, height={}",
        bounds.left,
        bounds.top,
        bounds.right - bounds.left,
        bounds.bottom - bounds.top
    ));
    apply_docked_bounds(
        window,
        bounds.left,
        bounds.top,
        (bounds.right - bounds.left).max(1) as u32,
        (bounds.bottom - bounds.top).max(1) as u32,
    )
}

#[cfg(windows)]
fn apply_docked_bounds(
    window: &WebviewWindow,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<(), String> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SetWindowPos, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOOWNERZORDER,
    };

    let hwnd = window.hwnd().map_err(|error| error.to_string())?.0;
    if unsafe {
        SetWindowPos(
            hwnd,
            HWND_TOPMOST,
            x,
            y,
            width as i32,
            height as i32,
            SWP_NOACTIVATE | SWP_NOOWNERZORDER,
        )
    } == 0
    {
        return Err("작업표시줄 도킹 위치를 적용하지 못했습니다.".into());
    }
    Ok(())
}

#[cfg(not(windows))]
fn apply_docked_bounds(
    window: &WebviewWindow,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<(), String> {
    apply_bounds(window, x, y, width, height)
}

#[cfg(windows)]
fn raise_docked_window(window: &WebviewWindow) -> Result<(), String> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SetWindowPos, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOOWNERZORDER, SWP_NOSIZE,
    };

    let hwnd = window.hwnd().map_err(|error| error.to_string())?.0;
    if unsafe {
        SetWindowPos(
            hwnd,
            HWND_TOPMOST,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
        )
    } == 0
    {
        return Err("도킹 위젯을 작업표시줄 위로 복구하지 못했습니다.".into());
    }
    Ok(())
}

#[cfg(not(windows))]
fn raise_docked_window(_window: &WebviewWindow) -> Result<(), String> {
    Ok(())
}

fn taskbar_dock_bounds(window: &WebviewWindow) -> Result<Bounds, String> {
    let taskbars = taskbar_bounds_all();
    if taskbars.is_empty() {
        return Err("Windows 작업표시줄을 찾을 수 없습니다.".into());
    }

    let current = window_screen_bounds(window)?;
    let center_x = current.left + (current.right - current.left) / 2;
    let center_y = current.top + (current.bottom - current.top) / 2;
    let taskbar = taskbars
        .into_iter()
        .min_by_key(|bounds| distance_to_bounds(center_x, center_y, *bounds))
        .ok_or_else(|| "Windows 작업표시줄 위치를 읽을 수 없습니다.".to_string())?;
    let scale = window.scale_factor().map_err(|error| error.to_string())?;
    let offset = (8.0 * scale).round() as i32;
    let taskbar_width = (taskbar.right - taskbar.left).max(1);
    let taskbar_height = (taskbar.bottom - taskbar.top).max(1);

    if taskbar_width >= taskbar_height {
        let width = ((COMPACT_WIDTH * scale).round() as i32).min(taskbar_width);
        let x =
            (taskbar.left + offset).clamp(taskbar.left, (taskbar.right - width).max(taskbar.left));
        Ok(Bounds {
            left: x,
            top: taskbar.top,
            right: x + width,
            bottom: taskbar.bottom,
        })
    } else {
        let height = ((COMPACT_HEIGHT * scale).round() as i32).min(taskbar_height);
        let y =
            (taskbar.top + offset).clamp(taskbar.top, (taskbar.bottom - height).max(taskbar.top));
        Ok(Bounds {
            left: taskbar.left,
            top: y,
            right: taskbar.right,
            bottom: y + height,
        })
    }
}

fn distance_to_bounds(x: i32, y: i32, bounds: Bounds) -> i64 {
    let dx = if x < bounds.left {
        bounds.left - x
    } else if x > bounds.right {
        x - bounds.right
    } else {
        0
    };
    let dy = if y < bounds.top {
        bounds.top - y
    } else if y > bounds.bottom {
        y - bounds.bottom
    } else {
        0
    };
    i64::from(dx) * i64::from(dx) + i64::from(dy) * i64::from(dy)
}

pub fn place_detail(main: &WebviewWindow, detail: &WebviewWindow) -> Result<(), String> {
    let scale = main.scale_factor().map_err(|error| error.to_string())?;
    let work_area = monitor_work_area(main)
        .ok_or_else(|| "현재 모니터의 작업 영역을 찾을 수 없습니다.".to_string())?;
    let main_bounds = window_screen_bounds(main)?;
    place_detail_in_work_area(main_bounds, work_area, scale, detail)
}

pub fn place_detail_at(anchor: Bounds, scale: f64, detail: &WebviewWindow) -> Result<(), String> {
    let work_area = monitor_work_area_for_bounds(anchor)
        .ok_or_else(|| "현재 모니터의 작업 영역을 찾을 수 없습니다.".to_string())?;
    place_detail_in_work_area(anchor, work_area, scale, detail)
}

pub fn prepare_detail_window(detail: &WebviewWindow) -> Result<(), String> {
    remove_non_client_frame(detail)
}

fn place_detail_in_work_area(
    main_bounds: Bounds,
    work_area: Bounds,
    scale: f64,
    detail: &WebviewWindow,
) -> Result<(), String> {
    let (x, y, width, height) = detail_layout(main_bounds, work_area, scale);

    remove_non_client_frame(detail)?;
    apply_bounds(detail, x, y, width, height)
}

fn detail_layout(main_bounds: Bounds, work_area: Bounds, scale: f64) -> (i32, i32, u32, u32) {
    let width = (DETAIL_WIDTH * scale).round() as u32;
    let desired_height = (DETAIL_HEIGHT * scale).round() as u32;
    let height = desired_height.min((work_area.bottom - work_area.top).max(1) as u32);
    let anchor_top = main_bounds.top.clamp(work_area.top, work_area.bottom);
    let anchor_bottom = main_bounds.bottom.clamp(work_area.top, work_area.bottom);
    let available_above = (anchor_top - work_area.top).max(0);
    let available_below = (work_area.bottom - anchor_bottom).max(0);
    let vertical_x = main_bounds.left.clamp(
        work_area.left,
        (work_area.right - width as i32).max(work_area.left),
    );

    if available_below >= height as i32 {
        return (vertical_x, anchor_bottom, width, height);
    }
    if available_above >= height as i32 {
        return (
            vertical_x,
            anchor_top - height as i32,
            width,
            height,
        );
    }

    let available_right = (work_area.right - main_bounds.right).max(0);
    let available_left = (main_bounds.left - work_area.left).max(0);
    let x = if available_right >= width as i32 || available_right >= available_left {
        main_bounds.right
    } else {
        main_bounds.left - width as i32
    };
    let y = main_bounds.top.clamp(
        work_area.top,
        (work_area.bottom - height as i32).max(work_area.top),
    );

    (x, y, width, height)
}

#[cfg(test)]
mod detail_layout_tests {
    use super::{detail_layout, Bounds};

    const WORK_AREA: Bounds = Bounds {
        left: 0,
        top: 0,
        right: 1920,
        bottom: 1000,
    };

    #[test]
    fn attaches_below_without_a_gap_near_the_top() {
        let anchor = Bounds {
            left: 20,
            top: 0,
            right: 326,
            bottom: 64,
        };
        let (_, y, _, height) = detail_layout(anchor, WORK_AREA, 1.0);
        assert_eq!(y, anchor.bottom);
        assert_eq!(height, 496);
    }

    #[test]
    fn attaches_above_without_a_gap_near_the_bottom() {
        let anchor = Bounds {
            left: 20,
            top: 936,
            right: 326,
            bottom: 1000,
        };
        let (_, y, _, height) = detail_layout(anchor, WORK_AREA, 1.0);
        assert_eq!(y + height as i32, anchor.top);
        assert_eq!(height, 496);
    }

    #[test]
    fn attaches_to_the_right_at_full_height_when_vertical_space_is_tight() {
        let anchor = Bounds {
            left: 20,
            top: 470,
            right: 326,
            bottom: 534,
        };
        let (x, y, _, height) = detail_layout(anchor, WORK_AREA, 1.0);
        assert_eq!(x, anchor.right);
        assert_eq!(y, anchor.top);
        assert_eq!(height, 496);
    }

    #[test]
    fn attaches_to_the_left_when_the_widget_is_near_the_right_edge() {
        let anchor = Bounds {
            left: 1500,
            top: 470,
            right: 1806,
            bottom: 534,
        };
        let (x, _, width, height) = detail_layout(anchor, WORK_AREA, 1.0);
        assert_eq!(x + width as i32, anchor.left);
        assert_eq!(height, 496);
    }
}

#[cfg(windows)]
fn remove_non_client_frame(window: &WebviewWindow) -> Result<(), String> {
    use windows_sys::Win32::{
        Graphics::Dwm::{
            DwmSetWindowAttribute, DWMWA_BORDER_COLOR, DWMWA_COLOR_NONE,
            DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND,
        },
        UI::WindowsAndMessaging::{
            GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE, GWL_STYLE,
            SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOOWNERZORDER, SWP_NOSIZE,
            SWP_NOZORDER, WS_BORDER, WS_CAPTION, WS_DLGFRAME, WS_EX_CLIENTEDGE,
            WS_EX_DLGMODALFRAME, WS_EX_WINDOWEDGE, WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_SYSMENU,
            WS_THICKFRAME,
        },
    };

    let hwnd = window.hwnd().map_err(|error| error.to_string())?.0;
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
        let normalized_style = style
            & !((WS_BORDER
                | WS_CAPTION
                | WS_DLGFRAME
                | WS_MAXIMIZEBOX
                | WS_MINIMIZEBOX
                | WS_SYSMENU
                | WS_THICKFRAME) as isize);
        let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let normalized_ex_style =
            ex_style & !((WS_EX_CLIENTEDGE | WS_EX_DLGMODALFRAME | WS_EX_WINDOWEDGE) as isize);
        if normalized_style != style || normalized_ex_style != ex_style {
            SetWindowLongPtrW(hwnd, GWL_STYLE, normalized_style);
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, normalized_ex_style);
            SetWindowPos(
                hwnd,
                std::ptr::null_mut(),
                0,
                0,
                0,
                0,
                SWP_FRAMECHANGED
                    | SWP_NOMOVE
                    | SWP_NOSIZE
                    | SWP_NOACTIVATE
                    | SWP_NOOWNERZORDER
                    | SWP_NOZORDER,
            );
        }

        let corner_preference = DWMWCP_DONOTROUND;
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE as u32,
            (&corner_preference as *const i32).cast(),
            std::mem::size_of_val(&corner_preference) as u32,
        );
        let border_color = DWMWA_COLOR_NONE;
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_BORDER_COLOR as u32,
            (&border_color as *const u32).cast(),
            std::mem::size_of_val(&border_color) as u32,
        );
    }
    Ok(())
}

#[cfg(not(windows))]
fn remove_non_client_frame(_window: &WebviewWindow) -> Result<(), String> {
    Ok(())
}

#[cfg(windows)]
fn window_screen_bounds(window: &WebviewWindow) -> Result<Bounds, String> {
    use windows_sys::Win32::{Foundation::RECT, UI::WindowsAndMessaging::GetWindowRect};

    let hwnd = window.hwnd().map_err(|error| error.to_string())?.0;
    let mut rect = RECT::default();
    if unsafe { GetWindowRect(hwnd, &mut rect) } == 0 {
        return Err("미니 위젯의 화면 좌표를 읽을 수 없습니다.".into());
    }
    Ok(Bounds {
        left: rect.left,
        top: rect.top,
        right: rect.right,
        bottom: rect.bottom,
    })
}

#[cfg(not(windows))]
fn window_screen_bounds(window: &WebviewWindow) -> Result<Bounds, String> {
    let position = window.outer_position().map_err(|error| error.to_string())?;
    let size = window.outer_size().map_err(|error| error.to_string())?;
    Ok(Bounds {
        left: position.x,
        top: position.y,
        right: position.x + size.width as i32,
        bottom: position.y + size.height as i32,
    })
}

fn monitor_work_area(window: &WebviewWindow) -> Option<Bounds> {
    let monitor = window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| window.primary_monitor().ok().flatten())?;
    let area = monitor.work_area();
    Some(Bounds {
        left: area.position.x,
        top: area.position.y,
        right: area.position.x + area.size.width as i32,
        bottom: area.position.y + area.size.height as i32,
    })
}

#[cfg(windows)]
fn monitor_work_area_for_bounds(bounds: Bounds) -> Option<Bounds> {
    use windows_sys::Win32::{
        Foundation::{RECT, TRUE},
        Graphics::Gdi::{GetMonitorInfoW, MonitorFromRect, MONITORINFO, MONITOR_DEFAULTTONEAREST},
    };

    let rect = RECT {
        left: bounds.left,
        top: bounds.top,
        right: bounds.right,
        bottom: bounds.bottom,
    };
    let monitor = unsafe { MonitorFromRect(&rect, MONITOR_DEFAULTTONEAREST) };
    if monitor.is_null() {
        return None;
    }
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        rcMonitor: RECT::default(),
        rcWork: RECT::default(),
        dwFlags: 0,
    };
    if unsafe { GetMonitorInfoW(monitor, &mut info) } != TRUE {
        return None;
    }
    Some(Bounds {
        left: info.rcWork.left,
        top: info.rcWork.top,
        right: info.rcWork.right,
        bottom: info.rcWork.bottom,
    })
}

#[cfg(not(windows))]
fn monitor_work_area_for_bounds(bounds: Bounds) -> Option<Bounds> {
    Some(bounds)
}

#[cfg(windows)]
fn apply_bounds(
    window: &WebviewWindow,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<(), String> {
    use std::ptr;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SetWindowPos, SWP_NOACTIVATE, SWP_NOOWNERZORDER, SWP_NOZORDER,
    };

    let hwnd = window.hwnd().map_err(|error| error.to_string())?;
    let result = unsafe {
        SetWindowPos(
            hwnd.0,
            ptr::null_mut(),
            x,
            y,
            width as i32,
            height as i32,
            SWP_NOACTIVATE | SWP_NOOWNERZORDER | SWP_NOZORDER,
        )
    };
    if result == 0 {
        return Err("Windows가 위젯 위치 변경을 거부했습니다.".into());
    }
    Ok(())
}

#[cfg(not(windows))]
fn apply_bounds(
    window: &WebviewWindow,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<(), String> {
    use tauri::{PhysicalPosition, PhysicalSize};

    window
        .set_size(PhysicalSize::new(width, height))
        .map_err(|error| error.to_string())?;
    window
        .set_position(PhysicalPosition::new(x, y))
        .map_err(|error| error.to_string())
}

#[cfg(windows)]
pub fn taskbar_windows_all() -> Vec<TaskbarWindow> {
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::{
        Foundation::{HWND, RECT},
        UI::WindowsAndMessaging::{FindWindowExW, FindWindowW, GetWindowRect},
    };

    fn read_window(window: HWND) -> Option<TaskbarWindow> {
        let mut rect = RECT::default();
        if unsafe { GetWindowRect(window, &mut rect) } == 0 {
            return None;
        }
        Some(TaskbarWindow {
            hwnd: window as isize,
            bounds: Bounds {
                left: rect.left,
                top: rect.top,
                right: rect.right,
                bottom: rect.bottom,
            },
        })
    }

    let mut result = Vec::new();
    let primary_class: Vec<u16> = "Shell_TrayWnd\0".encode_utf16().collect();
    let primary = unsafe { FindWindowW(primary_class.as_ptr(), null()) };
    if !primary.is_null() {
        if let Some(window) = read_window(primary) {
            result.push(window);
        }
    }

    let secondary_class: Vec<u16> = "Shell_SecondaryTrayWnd\0".encode_utf16().collect();
    let mut after: HWND = null_mut();
    loop {
        let window = unsafe { FindWindowExW(null_mut(), after, secondary_class.as_ptr(), null()) };
        if window.is_null() {
            break;
        }
        if let Some(window) = read_window(window) {
            result.push(window);
        }
        after = window;
    }
    result
}

#[cfg(not(windows))]
pub fn taskbar_windows_all() -> Vec<TaskbarWindow> {
    Vec::new()
}

pub fn taskbar_bounds_all() -> Vec<Bounds> {
    taskbar_windows_all()
        .into_iter()
        .map(|window| window.bounds)
        .collect()
}
