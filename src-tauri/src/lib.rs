mod commands;
mod engine;
mod proxy;
mod system_proxy;

use commands::AppState;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, PhysicalPosition, WindowEvent,
};

fn show_dashboard(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let settings = commands::load_settings(app.handle());
            let state = AppState::new(settings.clone());
            let engine = state.engine.clone();
            app.manage(state);

            proxy::spawn(engine.clone());
            if settings.system_proxy_enabled {
                if let Err(error) = system_proxy::enable(app.handle()) {
                    eprintln!("SinkHole could not reconnect the Windows proxy: {error}");
                }
            }
            tauri::async_runtime::spawn(async move {
                if let Err(error) = commands::fetch_and_replace_rules(&engine, &settings).await {
                    eprintln!("SinkHole kept its fallback rules: {error}");
                }
            });

            let toggle_item = MenuItem::with_id(
                app,
                "toggle-protection",
                "Toggle Protection",
                true,
                None::<&str>,
            )?;
            let dashboard_item =
                MenuItem::with_id(app, "open-dashboard", "Open Dashboard", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit SinkHole", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&toggle_item, &dashboard_item, &quit_item])?;

            let mut tray_builder = TrayIconBuilder::new()
                .tooltip("SinkHole — ads, trackers, and telemetry")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "toggle-protection" => {
                        let state = app.state::<AppState>();
                        let enabled = !state.engine.is_enabled();
                        if let Err(error) = commands::set_protection(app, &state, enabled) {
                            eprintln!("Could not persist protection state: {error}");
                        }
                    }
                    "open-dashboard" => show_dashboard(app),
                    "quit" => {
                        if let Err(error) = system_proxy::disable(app) {
                            eprintln!("Could not restore the previous Windows proxy: {error}");
                        }
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        position,
                        ..
                    } = event
                    {
                        if let Some(quick) = tray.app_handle().get_webview_window("quick") {
                            if quick.is_visible().unwrap_or(false) {
                                let _ = quick.hide();
                            } else {
                                let x = (position.x - 370.0) as i32;
                                let y = (position.y - 470.0) as i32;
                                let _ = quick.set_position(PhysicalPosition::new(x, y));
                                let _ = quick.show();
                                let _ = quick.set_focus();
                            }
                        }
                    }
                });

            if let Some(icon) = app.default_window_icon() {
                tray_builder = tray_builder.icon(icon.clone());
            }
            tray_builder.build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "quick" && matches!(event, WindowEvent::Focused(false)) {
                let _ = window.hide();
            }
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::toggle_adblocker,
            commands::get_stats,
            commands::get_settings,
            commands::set_system_proxy,
            commands::set_quick_protection,
            commands::open_dashboard,
            commands::update_settings,
            commands::update_blocklists,
            commands::check_link,
            commands::inspect_link,
        ])
        .run(tauri::generate_context!())
        .expect("error while running SinkHole");
}
