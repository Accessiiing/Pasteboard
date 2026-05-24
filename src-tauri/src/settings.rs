use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Serialize, Deserialize, Clone)]
pub struct Settings {
    pub save_path: String,
    pub max_records: u32,
    pub clear_on_shutdown: bool,
    pub autostart: bool,
    pub hotkey: String,
}

fn default_save_path() -> String {
    if let Ok(up) = std::env::var("USERPROFILE") {
        format!("{}\\Documents\\ClipNest", up)
    } else {
        "ClipNest".to_string()
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            save_path: default_save_path(),
            max_records: 200,
            clear_on_shutdown: false,
            autostart: true,
            hotkey: "Alt+V".to_string(),
        }
    }
}

fn settings_path(app: &AppHandle) -> PathBuf {
    let dir = app
        .path()
        .app_config_dir()
        .expect("无法获取配置目录");
    let _ = std::fs::create_dir_all(&dir);
    dir.join("settings.json")
}

pub fn load(app: &AppHandle) -> Settings {
    let path = settings_path(app);
    if let Ok(text) = std::fs::read_to_string(&path) {
        if let Ok(s) = serde_json::from_str::<Settings>(&text) {
            return s;
        }
    }
    let def = Settings::default();
    let _ = std::fs::create_dir_all(&def.save_path);
    save(app, &def);
    def
}

pub fn save(app: &AppHandle, settings: &Settings) {
    let path = settings_path(app);
    if let Ok(text) = serde_json::to_string_pretty(settings) {
        let _ = std::fs::write(path, text);
    }
}
