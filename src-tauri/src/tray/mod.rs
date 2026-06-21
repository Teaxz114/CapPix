use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    App, Emitter, Manager,
};

pub fn setup_tray(app: &App) -> anyhow::Result<()> {
    let capture_region =
        MenuItem::with_id(app, "capture_region", "区域截图", true, None::<&str>)?;
    let capture_fullscreen =
        MenuItem::with_id(app, "capture_fullscreen", "全屏截图", true, None::<&str>)?;
    let capture_window =
        MenuItem::with_id(app, "capture_window", "窗口截图", true, None::<&str>)?;
    let separator1 = PredefinedMenuItem::separator(app)?;
    let pin_clipboard = MenuItem::with_id(app, "pin_clipboard", "贴图", true, None::<&str>)?;
    let color_picker = MenuItem::with_id(app, "color_picker", "取色器", true, None::<&str>)?;
    let separator2 = PredefinedMenuItem::separator(app)?;
    let screen_record =
        MenuItem::with_id(app, "screen_record", "录屏", true, None::<&str>)?;
    let gif_record =
        MenuItem::with_id(app, "gif_record", "GIF 录制", true, None::<&str>)?;
    let separator3 = PredefinedMenuItem::separator(app)?;
    let history = MenuItem::with_id(app, "history", "历史记录", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?;
    let separator4 = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[
            &capture_region,
            &capture_fullscreen,
            &capture_window,
            &separator1,
            &pin_clipboard,
            &color_picker,
            &separator2,
            &screen_record,
            &gif_record,
            &separator3,
            &history,
            &settings,
            &separator4,
            &quit,
        ],
    )?;

    TrayIconBuilder::new()
        .icon(app.default_window_icon().cloned().unwrap())
        .menu(&menu)
        .tooltip("CapPix")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "capture_region" => {
                let _ = app.emit("hotkey", "capture_region");
            }
            "capture_fullscreen" => {
                let _ = app.emit("hotkey", "capture_fullscreen");
            }
            "capture_window" => {
                let _ = app.emit("hotkey", "capture_window");
            }
            "settings" | "history" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                    // Navigate to the right page
                    let route = if event.id.as_ref() == "history" {
                        "history"
                    } else {
                        "settings"
                    };
                    let _ = window.eval(&format!(
                        "window.location.hash = '/{}'",
                        route
                    ));
                }
            }
            "screen_record" => {
                let _ = app.emit("tray-action", "screen_record");
            }
            "gif_record" => {
                let _ = app.emit("tray-action", "gif_record");
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .build(app)?;

    Ok(())
}
