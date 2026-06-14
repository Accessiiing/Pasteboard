use tauri::{Manager, PhysicalPosition, WebviewWindow};

#[cfg(windows)]
use windows::Win32::Foundation::HWND;

/// 取出底层 HWND（跨 windows crate 版本安全转换）。
#[cfg(windows)]
fn raw_hwnd(window: &WebviewWindow) -> Option<HWND> {
    window.hwnd().ok().map(|h| HWND(h.0 as _))
}

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

/// 切换窗口的「不抢焦点」扩展样式 WS_EX_NOACTIVATE。
/// on=true：作为剪贴板浮层弹出，点击/显示都不会夺取前台焦点
///          ——保住目标输入框（含资源管理器重命名框）的光标与编辑态。
/// on=false：作为可输入窗口（搜索 / 设置），允许获得键盘焦点。
#[cfg(windows)]
fn set_no_activate(window: &WebviewWindow, on: bool) {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_NOACTIVATE,
    };
    if let Some(hwnd) = raw_hwnd(window) {
        unsafe {
            let cur = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            let flag = WS_EX_NOACTIVATE.0 as isize;
            let next = if on { cur | flag } else { cur & !flag };
            if next != cur {
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, next);
            }
        }
    }
}

/// 把窗口显示出来但「不激活」（不抢焦点），并保持置顶。
#[cfg(windows)]
fn show_inactive(window: &WebviewWindow) {
    use windows::Win32::UI::WindowsAndMessaging::{
        SetWindowPos, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW,
    };
    if let Some(hwnd) = raw_hwnd(window) {
        unsafe {
            let _ = SetWindowPos(
                hwnd,
                HWND_TOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
            );
        }
    }
}

/// 热键唤起：把剪贴板浮层定位到光标处并「不抢焦点」地弹出。
/// 这样按热键时，正在编辑的窗口（如资源管理器重命名框）不会失焦。
pub fn show_at_cursor(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        position_near_cursor(&window);
        #[cfg(windows)]
        {
            set_no_activate(&window, true);
            show_inactive(&window);
            return;
        }
        #[cfg(not(windows))]
        {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

/// 托盘唤起：作为正常可聚焦窗口弹出（设置界面要打字，且避免托盘点击带来的前台竞态）。
pub fn show_focused(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        position_near_cursor(&window);
        #[cfg(windows)]
        {
            set_no_activate(&window, false);
            show_inactive(&window); // 原生显示，与 show_at_cursor 一致，保证 hide 可控
            if let Some(hwnd) = raw_hwnd(&window) {
                use windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow;
                unsafe {
                    let _ = SetForegroundWindow(hwnd);
                }
            }
            let _ = window.set_focus();
            return;
        }
        #[cfg(not(windows))]
        {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

/// 原生隐藏窗口。
/// 显示走的是原生 SetWindowPos（绕过 tao 的可见性状态跟踪），隐藏也必须用原生
/// ShowWindow(SW_HIDE)——否则 tao 仍以为窗口是隐藏的，会短路掉真正的隐藏，
/// 导致点条目粘贴后面板留在桌面上不退回。
pub fn hide(window: &WebviewWindow) {
    #[cfg(windows)]
    {
        use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};
        if let Some(hwnd) = raw_hwnd(window) {
            unsafe {
                let _ = ShowWindow(hwnd, SW_HIDE);
            }
            return;
        }
    }
    let _ = window.hide();
}

/// 让浮层临时获得键盘焦点（点击搜索框时调用）。
/// 去掉 WS_EX_NOACTIVATE 并主动激活窗口，使搜索框能接收键盘输入。
pub fn focus_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        #[cfg(windows)]
        {
            set_no_activate(&window, false);
            if let Some(hwnd) = raw_hwnd(&window) {
                use windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow;
                unsafe {
                    let _ = SetForegroundWindow(hwnd);
                }
            }
        }
        let _ = window.set_focus();
    }
}

// ---------- 自动收起：点别处 / 焦点切走时隐藏浮层 ----------
// no-activate 浮层拿不到焦点，原先依赖 WindowEvent::Focused(false) 的自动隐藏失效。
// 用两个全局钩子补回：
//   1) 前台窗口变化(EVENT_SYSTEM_FOREGROUND)——覆盖 Alt+Tab / 切到别的程序；
//   2) 低级鼠标钩子(WH_MOUSE_LL)——覆盖「在面板之外任意位置点击」
//      （含点当前前台窗口的其它区域，这是前台钩子覆盖不到的）。

#[cfg(windows)]
static HOOK_APP: std::sync::OnceLock<tauri::AppHandle> = std::sync::OnceLock::new();

/// 收起浮层：仅当其可见、且没有打开文件对话框时。
/// 返回是否真的执行了收起——键盘钩子据此决定要不要吞掉这次 Esc。
#[cfg(windows)]
fn dismiss_panel() -> bool {
    use std::sync::atomic::Ordering;
    if let Some(app) = HOOK_APP.get() {
        let st = app.state::<crate::state::AppState>();
        if st.dialog_open.load(Ordering::SeqCst) {
            return false; // 选择文件夹对话框打开时不收起
        }
        if let Some(win) = app.get_webview_window("main") {
            if win.is_visible().unwrap_or(false) {
                hide(&win);
                return true;
            }
        }
    }
    false
}

/// 前台窗口变化：焦点切到其它进程窗口时收起（Alt+Tab、点别的程序标题栏等）。
#[cfg(windows)]
unsafe extern "system" fn foreground_proc(
    _hook: windows::Win32::UI::Accessibility::HWINEVENTHOOK,
    _event: u32,
    _hwnd: HWND,
    _id_object: i32,
    _id_child: i32,
    _id_thread: u32,
    _time: u32,
) {
    dismiss_panel();
}

/// 低级鼠标钩子：在面板矩形之外按下鼠标键时收起。
#[cfg(windows)]
unsafe extern "system" fn mouse_proc(
    code: i32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::Foundation::RECT;
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, GetWindowRect, HHOOK, MSLLHOOKSTRUCT, WM_LBUTTONDOWN, WM_MBUTTONDOWN,
        WM_RBUTTONDOWN,
    };
    // code < 0 时按约定必须原样转交，不做处理
    if code >= 0 {
        let msg = wparam.0 as u32;
        if msg == WM_LBUTTONDOWN || msg == WM_RBUTTONDOWN || msg == WM_MBUTTONDOWN {
            let info = &*(lparam.0 as *const MSLLHOOKSTRUCT);
            let (x, y) = (info.pt.x, info.pt.y);
            if let Some(app) = HOOK_APP.get() {
                if let Some(win) = app.get_webview_window("main") {
                    if win.is_visible().unwrap_or(false) {
                        if let Some(hwnd) = raw_hwnd(&win) {
                            let mut rect = RECT::default();
                            if GetWindowRect(hwnd, &mut rect).is_ok() {
                                let inside = x >= rect.left
                                    && x < rect.right
                                    && y >= rect.top
                                    && y < rect.bottom;
                                if !inside {
                                    dismiss_panel();
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    CallNextHookEx(HHOOK::default(), code, wparam, lparam)
}

/// 低级键盘钩子：浮层可见时按下 Esc 收起，并吞掉这次按键，
/// 避免 Esc 继续下传给底层正在编辑的窗口（如资源管理器重命名框会因此取消重命名）。
#[cfg(windows)]
unsafe extern "system" fn keyboard_proc(
    code: i32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::Foundation::LRESULT;
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, HHOOK, KBDLLHOOKSTRUCT, WM_KEYDOWN, WM_SYSKEYDOWN,
    };
    const VK_ESCAPE: u32 = 0x1B; // Esc 虚拟键码（直接用常量，免引入 KeyboardAndMouse feature）
    // code < 0 时按约定必须原样转交，不做处理
    if code >= 0 {
        let msg = wparam.0 as u32;
        if msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN {
            let info = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
            if info.vkCode == VK_ESCAPE && dismiss_panel() {
                return LRESULT(1); // 已收起：吞掉 Esc，不再下传给底层窗口
            }
        }
    }
    CallNextHookEx(HHOOK::default(), code, wparam, lparam)
}

/// 安装自动收起钩子。需在主线程（有消息循环）调用，钩子随程序生命周期常驻。
#[cfg(windows)]
pub fn install_dismiss_hooks(app: &tauri::AppHandle) {
    use windows::Win32::Foundation::{HINSTANCE, HMODULE};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::Accessibility::SetWinEventHook;
    use windows::Win32::UI::WindowsAndMessaging::{
        SetWindowsHookExW, EVENT_SYSTEM_FOREGROUND, WH_KEYBOARD_LL, WH_MOUSE_LL,
        WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS,
    };
    let _ = HOOK_APP.set(app.clone());
    unsafe {
        // 1) 前台窗口变化钩子
        SetWinEventHook(
            EVENT_SYSTEM_FOREGROUND,
            EVENT_SYSTEM_FOREGROUND,
            HMODULE::default(),
            Some(foreground_proc),
            0,
            0,
            WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
        );
        // 2) 全局低级鼠标钩子
        let hmod = GetModuleHandleW(None).unwrap_or_default();
        let _ = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), HINSTANCE(hmod.0), 0);
        // 3) 全局低级键盘钩子：浮层可见时按 Esc 收起
        let _ = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), HINSTANCE(hmod.0), 0);
    }
}
