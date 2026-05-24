mod clipboard;
mod commands;
mod db;
mod hotkey;
mod paste;
mod settings;
mod state;
mod tray;
mod window;

use state::AppState;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{Manager, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_global_shortcut::ShortcutState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        hotkey::on_trigger(app);
                    }
                })
                .build(),
        )
        .setup(|app| {
            let handle = app.handle().clone();

            let data_dir = handle.path().app_data_dir().expect("无法获取数据目录");
            let images_dir = data_dir.join("images");
            let thumbs_dir = data_dir.join("thumbs");
            std::fs::create_dir_all(&images_dir).ok();
            std::fs::create_dir_all(&thumbs_dir).ok();

            let settings = settings::load(&handle);
            let conn = db::init(&data_dir.join("pasteboard.db"));

            app.manage(AppState {
                db: Mutex::new(conn),
                settings: Mutex::new(settings.clone()),
                last_hash: Mutex::new(None),
                dialog_open: AtomicBool::new(false),
                images_dir,
                thumbs_dir,
            });

            tray::build(&handle)?;
            let _ = hotkey::register(&handle, &settings.hotkey);

            {
                use tauri_plugin_autostart::ManagerExt;
                let am = handle.autolaunch();
                if settings.autostart {
                    let _ = am.enable();
                } else {
                    let _ = am.disable();
                }
            }

            clipboard::start(handle.clone());

            if let Some(window) = app.get_webview_window("main") {
                let w = window.clone();
                window.on_window_event(move |event| match event {
                    WindowEvent::Focused(false) => {
                        let st = w.state::<AppState>();
                        if !st.dialog_open.load(Ordering::SeqCst) {
                            let _ = w.hide();
                        }
                    }
                    WindowEvent::CloseRequested { api, .. } => {
                        api.prevent_close();
                        let _ = w.hide();
                    }
                    _ => {}
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_records,
            commands::paste_record,
            commands::delete_record,
            commands::pin_record,
            commands::save_record,
            commands::clear_all,
            commands::get_settings,
            commands::set_settings,
            commands::set_hotkey,
            commands::choose_folder,
            commands::hide_window,
            commands::current_ms,
        ])
        .build(tauri::generate_context!())
        .expect("应用启动失败")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested { .. } = event {
                let st = app.state::<AppState>();
                let clear = st.settings.lock().unwrap().clear_on_shutdown;
                if clear {
                    let files = {
                        let conn = st.db.lock().unwrap();
                        db::clear_all(&conn, true)
                    };
                    for f in files {
                        let _ = std::fs::remove_file(f);
                    }
                }
            }
        });
}
