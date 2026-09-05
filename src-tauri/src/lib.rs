mod chat;
mod cli;
mod commands;
mod db;
mod discovery;
mod error;
mod identity;
mod logging;
mod network;
mod pairing;
mod paths;
mod state;

use cli::Cli;
use paths::AppPaths;
use state::AppState;

/// Ağda görünecek cihaz adı. Ayarlardan özelleştirilebilir olacak (Faz 11);
/// şimdilik makine adı, profil varsa onunla ayrıştırılır.
fn default_device_name(paths: &AppPaths) -> String {
    let host = hostname().unwrap_or_else(|| "LinkUp".to_string());
    match &paths.profile {
        Some(profile) => format!("{host} ({})", profile.to_uppercase()),
        None => host,
    }
}

fn hostname() -> Option<String> {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .ok()
        .filter(|name| !name.trim().is_empty())
}

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
        .setup(move |app| {
            use tauri::Manager;

            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_title(&window_title);
            }

            let db = db::open(&paths.db_path)?;

            // Uygulama kapanırken yolda kalan mesajlar `sending` durumunda
            // donar; göstergenin sonsuza kadar dönmemesi için işaretlenir.
            if let Ok(conn) = db.get() {
                if let Ok(count) = db::messages::fail_stuck_outgoing(&conn) {
                    if count > 0 {
                        tracing::info!(count, "yolda kalmış mesajlar başarısız işaretlendi");
                    }
                }
            }
            let identity = identity::load_or_create(&paths)?;
            let device_name = default_device_name(&paths);

            // QUIC uç noktası tokio runtime bağlamı ister; `setup` ana thread'de
            // ve runtime'ın DIŞINDA çalıştığı için açılış block_on içine alınır.
            let network = tauri::async_runtime::block_on(async {
                network::service::NetworkService::start(
                    identity.signing_key(),
                    device_name.clone(),
                    paths.quic_port,
                )
            })?;

            let discovery = discovery::DiscoveryService::start(
                app.handle().clone(),
                network.endpoint(),
                device_name,
                network.local_addr().port(),
            );

            let pairing = std::sync::Arc::new(pairing::PairingManager::new(
                app.handle().clone(),
                db.clone(),
            ));

            let connections = network::manager::ConnectionManager::new(
                app.handle().clone(),
                db.clone(),
                network.endpoint(),
                discovery.registry(),
                std::sync::Arc::clone(&pairing),
            );
            connections.start();

            // Kabul döngüsü en sonda: gelen bağlantının nereye yönleneceğini
            // bilmeden kabul etmek onu sessizce düşürmek olurdu.
            network.start_accepting(
                std::sync::Arc::clone(&pairing),
                std::sync::Arc::clone(&connections),
            );

            app.manage(AppState {
                paths,
                db,
                identity,
                network,
                discovery,
                pairing,
                connections,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::identity_info,
            commands::get_settings,
            commands::discovered_devices,
            commands::add_device_manually,
            commands::forget_discovered_device,
            commands::trusted_devices,
            commands::start_pairing,
            commands::respond_to_pairing,
            commands::forget_device,
            commands::chat_history,
            commands::send_message,
            commands::mark_conversation_read,
            commands::set_setting,
            commands::open_log_dir
        ])
        .run(tauri::generate_context!())
        .expect("Tauri uygulaması başlatılamadı");
}
