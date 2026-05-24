use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter,
};

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let settings_i = MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?;
    let clipboard_i = MenuItem::with_id(app, "clipboard", "剪贴板", true, None::<&str>)?;
    let quit_i = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&settings_i, &clipboard_i, &quit_i])?;

    let _tray = TrayIconBuilder::with_id("main-tray")
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("Pasteboard 剪贴板")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "settings" => {
                let _ = app.emit_to("main", "navigate", "settings");
                crate::window::show_at_cursor(app);
            }
            "clipboard" => {
                let _ = app.emit_to("main", "navigate", "clipboard");
                crate::window::show_at_cursor(app);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                let _ = app.emit_to("main", "navigate", "clipboard");
                crate::window::show_at_cursor(app);
            }
        })
        .build(app)?;
    Ok(())
}
