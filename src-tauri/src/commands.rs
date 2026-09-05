//! Frontend'e açılan tek API yüzeyi (PLAN.md §2.1).
//! Frontend ağ veya dosya sistemiyle doğrudan konuşmaz; her şey buradan geçer.

use serde::Serialize;
use tauri::State;

use crate::db::settings::{self, Settings};
use crate::discovery::DiscoveredDeviceDto;
use crate::error::{AppError, AppResult};
use crate::identity::KeyStorage;
use crate::state::AppState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub profile: Option<String>,
    pub data_dir: String,
    pub log_dir: String,
    pub downloads_dir: String,
    pub db_path: String,
    pub quic_port: u16,
    /// Karşı cihazda elle ekleme için kullanılacak adresler.
    pub reachable_addresses: Vec<String>,
    pub os: String,
}

#[tauri::command]
pub fn app_info(state: State<'_, AppState>) -> AppInfo {
    let p = &state.paths;
    tracing::debug!(profile = p.profile_label(), "app_info istendi");
    AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        profile: p.profile.clone(),
        data_dir: p.data_dir.display().to_string(),
        log_dir: p.log_dir.display().to_string(),
        downloads_dir: p.downloads_dir.display().to_string(),
        db_path: p.db_path.display().to_string(),
        quic_port: state.network.local_addr().port(),
        reachable_addresses: state
            .network
            .reachable_addresses()
            .iter()
            .map(ToString::to_string)
            .collect(),
        os: std::env::consts::OS.to_string(),
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityInfo {
    /// Kullanıcıya gösterilen, 4'erli gruplanmış base32 fingerprint.
    pub fingerprint: String,
    /// Anahtarın nerede saklandığı — dosyaya düşüldüyse UI bunu uyarı olarak gösterir.
    pub storage: KeyStorage,
}

#[tauri::command]
pub fn identity_info(state: State<'_, AppState>) -> IdentityInfo {
    IdentityInfo {
        fingerprint: state.identity.fingerprint(),
        storage: state.identity.storage,
    }
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> AppResult<Settings> {
    let conn = state.db.get().map_err(pool_error)?;
    let loaded = settings::load(&conn)?;
    tracing::debug!(theme = %loaded.theme, "ayarlar okundu");
    Ok(loaded)
}

#[tauri::command]
pub fn set_setting(state: State<'_, AppState>, key: String, value: String) -> AppResult<Settings> {
    let conn = state.db.get().map_err(pool_error)?;
    settings::set(&conn, &key, &value)?;
    // Güncel anlık görüntü geri döner: frontend ayrı bir okuma yapmak zorunda kalmaz.
    settings::load(&conn)
}

/// Ağda görünen, henüz eşleşmemiş cihazlar (PLAN.md §3.2 "Bulunanlar").
#[tauri::command]
pub fn discovered_devices(state: State<'_, AppState>) -> Vec<DiscoveredDeviceDto> {
    state.discovery.list()
}

/// Elle adres girerek cihaz ekler (PLAN.md §2.4, §10-K7).
#[tauri::command]
pub async fn add_device_manually(
    state: State<'_, AppState>,
    address: String,
) -> AppResult<DiscoveredDeviceDto> {
    state
        .discovery
        .add_manual(&address)
        .await
        .map_err(|err| match err {
            crate::discovery::DiscoveryError::BadAddress(detail) => {
                AppError::InvalidAddress(detail)
            }
            other => AppError::Unreachable(other.to_string()),
        })
}

#[tauri::command]
pub fn forget_discovered_device(state: State<'_, AppState>, id: String) -> bool {
    state.discovery.remove(&id)
}

/// Ayarlar → Gelişmiş → "Log klasörünü aç" (PLAN.md §2.14).
#[tauri::command]
pub fn open_log_dir(state: State<'_, AppState>) -> AppResult<()> {
    let path = state.paths.log_dir.clone();
    tauri_plugin_opener::open_path(path.to_string_lossy().to_string(), None::<&str>)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("log klasörü açılamadı: {e}")))?;
    Ok(())
}

fn pool_error(err: r2d2::Error) -> AppError {
    AppError::Internal(anyhow::anyhow!("veritabanı bağlantısı alınamadı: {err}"))
}
