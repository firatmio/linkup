mod cli;
mod commands;
mod error;
mod logging;
mod paths;
mod state;

use cli::Cli;
use paths::AppPaths;
use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let cli = Cli::parse_lenient();
    let paths =
        AppPaths::resolve(cli.normalized_profile()).expect("uygulama dizinleri hazırlanamadı");

    // Guard, uygulama kapanana kadar yaşamalı — düşerse log yazımı durur.
    let _log_guard = logging::init(&paths, cli.log_level.as_deref());

    let window_title = match &paths.profile {
        Some(p) => format!("LinkUp ({})", p.to_uppercase()),
        None => "LinkUp".to_string(),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::new(paths))
        .setup(move |app| {
            use tauri::Manager;
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_title(&window_title);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::open_log_dir
        ])
        .run(tauri::generate_context!())
        .expect("Tauri uygulaması başlatılamadı");
}
