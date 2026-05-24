use crate::clipboard::now_ms;
use crate::settings::{self, Settings};
use crate::state::AppState;
use crate::{db, hotkey, paste};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Serialize;
use std::path::Path;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager, State};

#[derive(Serialize)]
pub struct RecordView {
    pub id: i64,
    pub kind: String,
    pub preview: String,
    pub thumb: Option<String>,
    pub pinned: bool,
    pub created_at: i64,
    pub last_used_at: i64,
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max).collect::<String>() + "…"
    }
}

fn thumb_data_uri(path: &str) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    Some(format!("data:image/png;base64,{}", STANDARD.encode(bytes)))
}

#[tauri::command]
pub fn list_records(state: State<AppState>) -> Vec<RecordView> {
    let conn = state.db.lock().unwrap();
    db::list(&conn)
        .into_iter()
        .map(|r| {
            let thumb = r.thumb_path.as_deref().and_then(thumb_data_uri);
            RecordView {
                id: r.id,
                kind: r.kind,
                preview: truncate(&r.preview, 600),
                thumb,
                pinned: r.pinned,
                created_at: r.created_at,
                last_used_at: r.last_used_at,
            }
        })
        .collect()
}

#[tauri::command]
pub fn paste_record(app: AppHandle, id: i64) -> Result<(), String> {
    paste::paste_record(&app, id)
}

#[tauri::command]
pub fn delete_record(app: AppHandle, id: i64) {
    let state = app.state::<AppState>();
    let files = {
        let conn = state.db.lock().unwrap();
        db::delete(&conn, id)
    };
    for f in files {
        let _ = std::fs::remove_file(f);
    }
    let _ = app.emit("records-updated", ());
}

#[tauri::command]
pub fn pin_record(app: AppHandle, id: i64, pinned: bool) {
    let state = app.state::<AppState>();
    {
        let conn = state.db.lock().unwrap();
        db::set_pinned(&conn, id, pinned);
    }
    let _ = app.emit("records-updated", ());
}

#[tauri::command]
pub fn save_record(app: AppHandle, id: i64) -> Result<String, String> {
    let state = app.state::<AppState>();
    let rec = {
        let conn = state.db.lock().unwrap();
        db::get(&conn, id).ok_or_else(|| "记录不存在".to_string())?
    };
    let save_path = state.settings.lock().unwrap().save_path.clone();
    std::fs::create_dir_all(&save_path).map_err(|e| e.to_string())?;
    if rec.kind == "image" {
        let dest = Path::new(&save_path).join(format!("clip_{}.png", rec.id));
        std::fs::copy(&rec.content, &dest).map_err(|e| e.to_string())?;
        Ok(dest.to_string_lossy().to_string())
    } else {
        let dest = Path::new(&save_path).join(format!("clip_{}.txt", rec.id));
        std::fs::write(&dest, &rec.content).map_err(|e| e.to_string())?;
        Ok(dest.to_string_lossy().to_string())
    }
}

#[tauri::command]
pub fn clear_all(app: AppHandle) {
    let state = app.state::<AppState>();
    let files = {
        let conn = state.db.lock().unwrap();
        db::clear_all(&conn, false)
    };
    for f in files {
        let _ = std::fs::remove_file(f);
    }
    let _ = app.emit("records-updated", ());
}

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
pub fn set_settings(
    app: AppHandle,
    save_path: String,
    max_records: u32,
    clear_on_shutdown: bool,
    autostart: bool,
) -> Result<(), String> {
    let max = max_records.clamp(100, 300);
    let _ = std::fs::create_dir_all(&save_path);
    let state = app.state::<AppState>();
    {
        let mut s = state.settings.lock().unwrap();
        s.save_path = save_path;
        s.max_records = max;
        s.clear_on_shutdown = clear_on_shutdown;
        s.autostart = autostart;
        let snap = s.clone();
        drop(s);
        settings::save(&app, &snap);
    }
    let files = {
        let conn = state.db.lock().unwrap();
        db::trim(&conn, max)
    };
    for f in files {
        let _ = std::fs::remove_file(f);
    }
    {
        use tauri_plugin_autostart::ManagerExt;
        let am = app.autolaunch();
        if autostart {
            let _ = am.enable();
        } else {
            let _ = am.disable();
        }
    }
    let _ = app.emit("records-updated", ());
    Ok(())
}

#[tauri::command]
pub fn set_hotkey(app: AppHandle, spec: String) -> Result<(), String> {
    hotkey::register(&app, &spec)?;
    let state = app.state::<AppState>();
    let snap = {
        let mut s = state.settings.lock().unwrap();
        s.hotkey = spec;
        s.clone()
    };
    settings::save(&app, &snap);
    Ok(())
}

#[tauri::command]
pub fn choose_folder(app: AppHandle) -> Option<String> {
    use tauri_plugin_dialog::DialogExt;
    let state = app.state::<AppState>();
    state.dialog_open.store(true, Ordering::SeqCst);
    let picked = app.dialog().file().blocking_pick_folder();
    state.dialog_open.store(false, Ordering::SeqCst);
    picked.map(|p| p.to_string())
}

#[tauri::command]
pub fn hide_window(app: AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

// 供前端在“被调用置顶”后获取最新时间戳（保留以便后续扩展）。
#[tauri::command]
pub fn current_ms() -> i64 {
    now_ms()
}
