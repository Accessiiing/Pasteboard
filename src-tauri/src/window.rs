use tauri::{Manager, PhysicalPosition, WebviewWindow};

#[cfg(windows)]
fn cursor_and_workarea() -> Option<(i32, i32, i32, i32, i32, i32)> {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

    unsafe {
        let mut pt = POINT::default();
        if GetCursorPos(&mut pt).is_err() {
            return None;
        }
        let hmon = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
        let mut mi = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(hmon, &mut mi).as_bool() {
            return None;
        }
        let work = mi.rcWork;
        Some((pt.x, pt.y, work.left, work.top, work.right, work.bottom))
    }
}

/// 把窗口定位到光标右下 45°，并做屏幕边界检测，保证完整可见。
pub fn position_near_cursor(window: &WebviewWindow) {
    let size = window.outer_size().unwrap_or(tauri::PhysicalSize {
        width: 383,
        height: 509,
    });
    let (w, h) = (size.width as i32, size.height as i32);
    let offset = 14;

    #[cfg(windows)]
    {
        if let Some((cx, cy, wl, wt, wr, wb)) = cursor_and_workarea() {
            // 默认出现在光标右下方 45°
            let mut x = cx + offset;
            let mut y = cy + offset;
            // 右边放不下 -> 翻到光标左侧
            if x + w > wr {
                x = cx - w - offset;
            }
            // 下边放不下 -> 翻到光标上方
            if y + h > wb {
                y = cy - h - offset;
            }
            // 最终夹取到工作区内
            x = x.clamp(wl, (wr - w).max(wl));
            y = y.clamp(wt, (wb - h).max(wt));
            let _ = window.set_position(PhysicalPosition::new(x, y));
            return;
        }
    }

    let _ = window.set_position(PhysicalPosition::new(100, 100));
}

pub fn show_at_cursor(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        position_near_cursor(&window);
        let _ = window.show();
        let _ = window.set_focus();
    }
}
