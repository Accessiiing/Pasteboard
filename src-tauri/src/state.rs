use crate::settings::Settings;
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;

pub struct AppState {
    pub db: Mutex<Connection>,
    pub settings: Mutex<Settings>,
    /// 最近一次写入剪贴板内容的指纹，用于去重以及忽略本程序自身写入。
    pub last_hash: Mutex<Option<String>>,
    /// 文件对话框打开期间置真，避免失焦自动隐藏窗口。
    pub dialog_open: AtomicBool,
    pub images_dir: PathBuf,
    pub thumbs_dir: PathBuf,
}
