use crate::clipboard::{hash_bytes, now_ms};
use crate::db;
use crate::state::AppState;
use enigo::{Direction, Enigo, Key, Keyboard, Settings as EnigoSettings};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

/// 把内容写回系统剪贴板。返回写入内容的指纹，用于让监听去重。
fn write_to_clipboard(rec: &db::Record) -> Result<String, String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    if rec.kind == "image" {
        let dynimg = image::open(&rec.content).map_err(|e| e.to_string())?;
        let rgba = dynimg.to_rgba8();
        let (w, h) = (rgba.width() as usize, rgba.height() as usize);
        let bytes = rgba.into_raw();
        let hash = hash_bytes(&bytes);
        clipboard
            .set_image(arboard::ImageData {
                width: w,
                height: h,
                bytes: bytes.into(),
            })
            .map_err(|e| e.to_string())?;
        Ok(hash)
    } else {
        let hash = hash_bytes(rec.content.as_bytes());
        clipboard.set_text(&rec.content).map_err(|e| e.to_string())?;
        Ok(hash)
    }
}

fn send_paste() {
    if let Ok(mut enigo) = Enigo::new(&EnigoSettings::default()) {
        let _ = enigo.key(Key::Control, Direction::Press);
        let _ = enigo.key(Key::Unicode('v'), Direction::Click);
        let _ = enigo.key(Key::Control, Direction::Release);
    }
}

/// 点击记录：写回剪贴板 -> 隐藏窗口 -> 模拟 Ctrl+V -> 置顶该记录。
pub fn paste_record(app: &AppHandle, id: i64) -> Result<(), String> {
    let state = app.state::<AppState>();
    let rec = {
        let conn = state.db.lock().unwrap();
        db::get(&conn, id).ok_or_else(|| "记录不存在".to_string())?
    };

    let hash = write_to_clipboard(&rec)?;
    // 标记为自身写入，避免监听把它当成新复制再插一条
    *state.last_hash.lock().unwrap() = Some(hash);

    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }

    // 让焦点回到目标窗口后再发送粘贴
    std::thread::sleep(Duration::from_millis(120));
    send_paste();

    // 置顶
    {
        let conn = state.db.lock().unwrap();
        db::touch(&conn, id, now_ms());
    }
    let _ = app.emit("records-updated", ());
    Ok(())
}
