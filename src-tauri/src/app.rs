//! Thin Tauri wiring: install the shared state, the hotkey plugin, the command handlers, the
//! tray, and the Win32 worker thread, then run. Each concern lives in its own module.

use tauri::WindowEvent;

use crate::{hotkeys, ipc, state, tray, update};

pub fn run() {
    state::init();

    tauri::Builder::default()
        .plugin(hotkeys::plugin())
        .invoke_handler(tauri::generate_handler![
            ipc::get_state,
            ipc::update_settings,
            ipc::set_hotkey,
            ipc::apply_action,
            ipc::open_log,
            ipc::open_settings_file,
            ipc::open_url,
            ipc::quit_app,
            ipc::request_accessibility,
            update::update_platform,
            update::download_update,
            update::extract_app_zip,
            update::apply_update,
            update::take_update_recovery_error
        ])
        .setup(|app| {
            // Quad is a tray utility — on macOS, drop the Dock icon and nudge the user to grant
            // the Accessibility permission the window-mover needs.
            #[cfg(target_os = "macos")]
            {
                let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);
                crate::winmgr::ensure_accessibility();
            }

            let _ = state::shared().app.set(app.handle().clone());
            state::shared().settings.lock().unwrap().apply_autostart();
            hotkeys::register_all(app.handle());
            tray::build_tray(app)?;
            crate::native::spawn_worker();
            if std::env::args().any(|a| a == "--open") {
                tray::show_main(app.handle());
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running Quad");
}
