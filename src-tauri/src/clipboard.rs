use crate::db;
use crate::state::AppState;
use clipboard_master::{CallbackResult, ClipboardHandler, Master};
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn hash_bytes(bytes: &[u8]) -> String {
    use std::collections::hash_map::DefaultHasher;
    let mut h = DefaultHasher::new();
    bytes.hash(&mut h);
    format!("{:x}", h.finish())
}

fn is_link(text: &str) -> bool {
    let t = text.trim();
    (t.starts_with("http://") || t.starts_with("https://")) && !t.contains(char::is_whitespace)
}

fn cleanup(files: Vec<String>) {
    for f in files {
        let _ = std::fs::remove_file(f);
    }
}

fn save_image(img: &arboard::ImageData, orig: &Path, thumb: &Path) -> Result<(), String> {
    let w = img.width as u32;
    let h = img.height as u32;
    let buf = image::RgbaImage::from_raw(w, h, img.bytes.clone().into_owned())
        .ok_or_else(|| "图片数据无效".to_string())?;
    let dynimg = image::DynamicImage::ImageRgba8(buf);
    dynimg.save(orig).map_err(|e| e.to_string())?;
    let tw = 320u32;
    let thumbimg = dynimg.thumbnail(tw, tw * 4);
    thumbimg.save(thumb).map_err(|e| e.to_string())?;
    Ok(())
}

/// 读取当前剪贴板并写入数据库；与上一次内容去重。
fn capture(app: &AppHandle) {
    let state = app.state::<AppState>();
    let mut clipboard = match arboard::Clipboard::new() {
        Ok(c) => c,
        Err(_) => return,
    };

    // 文本优先
    if let Ok(text) = clipboard.get_text() {
        if !text.trim().is_empty() {
            let h = hash_bytes(text.as_bytes());
            {
                let mut last = state.last_hash.lock().unwrap();
                if last.as_deref() == Some(h.as_str()) {
                    return;
                }
                *last = Some(h);
            }
            let kind = if is_link(&text) { "link" } else { "text" };
            let now = now_ms();
            let files = {
                let conn = state.db.lock().unwrap();
                db::insert(&conn, kind, &text, &text, None, now);
                let max = state.settings.lock().unwrap().max_records;
                db::trim(&conn, max)
            };
            cleanup(files);
            let _ = app.emit("records-updated", ());
            return;
        }
    }

    // 图片
    if let Ok(img) = clipboard.get_image() {
        let h = hash_bytes(&img.bytes);
        {
            let mut last = state.last_hash.lock().unwrap();
            if last.as_deref() == Some(h.as_str()) {
                return;
            }
            *last = Some(h);
        }
        let now = now_ms();
        let orig_path = state.images_dir.join(format!("clip_{}.png", now));
        let thumb_path = state.thumbs_dir.join(format!("thumb_{}.png", now));
        if save_image(&img, &orig_path, &thumb_path).is_ok() {
            let files = {
                let conn = state.db.lock().unwrap();
                db::insert(
                    &conn,
                    "image",
                    &orig_path.to_string_lossy(),
                    "[图片]",
                    Some(&thumb_path.to_string_lossy()),
                    now,
                );
                let max = state.settings.lock().unwrap().max_records;
                db::trim(&conn, max)
            };
            cleanup(files);
            let _ = app.emit("records-updated", ());
        }
    }
}

struct Handler {
    app: AppHandle,
}

impl ClipboardHandler for Handler {
    fn on_clipboard_change(&mut self) -> CallbackResult {
        capture(&self.app);
        CallbackResult::Next
    }
    fn on_clipboard_error(&mut self, _error: std::io::Error) -> CallbackResult {
        CallbackResult::Next
    }
}

pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        let mut master = match Master::new(Handler { app }) {
            Ok(m) => m,
            Err(_) => return,
        };
        let _ = master.run();
    });
}
