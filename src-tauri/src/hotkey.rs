use crate::window;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};

/// 把 "Ctrl+Shift+V" / "Alt+V" 这类字符串解析为 Shortcut。
pub fn parse(spec: &str) -> Option<Shortcut> {
    let mut mods = Modifiers::empty();
    let mut code: Option<Code> = None;
    for part in spec.split('+') {
        let p = part.trim();
        match p.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => mods |= Modifiers::CONTROL,
            "alt" => mods |= Modifiers::ALT,
            "shift" => mods |= Modifiers::SHIFT,
            "win" | "super" | "meta" | "cmd" => mods |= Modifiers::SUPER,
            other => code = key_to_code(other),
        }
    }
    code.map(|c| {
        let m = if mods.is_empty() { None } else { Some(mods) };
        Shortcut::new(m, c)
    })
}

fn key_to_code(key: &str) -> Option<Code> {
    let k = key.to_ascii_uppercase();
    let bytes = k.as_bytes();
    if bytes.len() == 1 {
        let c = bytes[0];
        if c.is_ascii_alphabetic() {
            return Some(match c {
                b'A' => Code::KeyA, b'B' => Code::KeyB, b'C' => Code::KeyC,
                b'D' => Code::KeyD, b'E' => Code::KeyE, b'F' => Code::KeyF,
                b'G' => Code::KeyG, b'H' => Code::KeyH, b'I' => Code::KeyI,
                b'J' => Code::KeyJ, b'K' => Code::KeyK, b'L' => Code::KeyL,
                b'M' => Code::KeyM, b'N' => Code::KeyN, b'O' => Code::KeyO,
                b'P' => Code::KeyP, b'Q' => Code::KeyQ, b'R' => Code::KeyR,
                b'S' => Code::KeyS, b'T' => Code::KeyT, b'U' => Code::KeyU,
                b'V' => Code::KeyV, b'W' => Code::KeyW, b'X' => Code::KeyX,
                b'Y' => Code::KeyY, _ => Code::KeyZ,
            });
        }
        if c.is_ascii_digit() {
            return Some(match c {
                b'0' => Code::Digit0, b'1' => Code::Digit1, b'2' => Code::Digit2,
                b'3' => Code::Digit3, b'4' => Code::Digit4, b'5' => Code::Digit5,
                b'6' => Code::Digit6, b'7' => Code::Digit7, b'8' => Code::Digit8,
                _ => Code::Digit9,
            });
        }
    }
    None
}

/// 切换主窗口显示：已显示则隐藏，否则在光标处弹出。
pub fn on_trigger(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            let _ = app.emit_to("main", "navigate", "clipboard");
            window::show_at_cursor(app);
        }
    }
}

pub fn register(app: &AppHandle, spec: &str) -> Result<(), String> {
    let shortcut = parse(spec).ok_or_else(|| format!("无法解析快捷键: {}", spec))?;
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    gs.register(shortcut).map_err(|e| e.to_string())
}
