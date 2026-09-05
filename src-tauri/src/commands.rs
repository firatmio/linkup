//! Frontend'e açılan tek API yüzeyi (PLAN.md §2.1).
//! Frontend ağ veya dosya sistemiyle doğrudan konuşmaz; her şey buradan geçer.

use serde::Serialize;
use tauri::State;

use crate::db::devices::TrustedDeviceDto;
use crate::db::settings::{self, Settings};
use crate::discovery::{DiscoveredDeviceDto, DiscoveryService};
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
    let devices = state.discovery.list();
    tracing::debug!(count = devices.len(), "keşfedilen cihazlar istendi");
    devices
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

/// Eşleşmiş, güvenilir cihazlar (PLAN.md §2.12).
#[tauri::command]
pub fn trusted_devices(state: State<'_, AppState>) -> Vec<TrustedDeviceDto> {
    state
        .pairing
        .trusted_devices()
        .iter()
        .map(|device| TrustedDeviceDto {
            id: data_encoding::BASE32_NOPAD.encode(&device.device_id),
            fingerprint: crate::identity::format_fingerprint(&device.device_id),
            name: device.display_name().to_string(),
            last_address: device.last_address.clone(),
            paired_at: device.paired_at,
            online: state.connections.presence.is_online(&device.device_id),
        })
        .collect()
}

/// Keşfedilmiş bir cihazla eşleştirmeyi başlatır (PLAN.md §2.5).
///
/// Komut, eşleştirme bitene kadar (en fazla 90 sn) bekler; kullanıcı kararı
/// bu sırada `respond_to_pairing` ile ayrı bir komuttan gelir.
#[tauri::command]
pub async fn start_pairing(state: State<'_, AppState>, id: String) -> AppResult<()> {
    let device_id = DiscoveryService::parse_device_id(&id)
        .ok_or_else(|| AppError::InvalidInput("geçersiz cihaz kimliği".to_string()))?;

    let address = state
        .discovery
        .address_of(&device_id)
        .ok_or_else(|| AppError::Unreachable("cihazın adresi bilinmiyor".to_string()))?;

    let mut connection = state
        .network
        .endpoint()
        .connect(address, Some(device_id))
        .await
        .map_err(|err| AppError::Unreachable(err.to_string()))?;

    let result = crate::pairing::run(std::sync::Arc::clone(&state.pairing), &mut connection, true)
        .await
        .map_err(|err| AppError::Pairing(err.code()));

    match result {
        Ok(()) => {
            // Bağlantı kapatılmıyor: eşleşme biter bitmez kapatmak, karşı
            // tarafın henüz okumadığı onay mesajını kaybettiriyordu.
            let connections = std::sync::Arc::clone(&state.connections);
            connections.supervise(device_id);
            tauri::async_runtime::spawn(async move {
                connections.hold(connection).await;
            });
            Ok(())
        }
        Err(err) => {
            connection.close();
            Err(err)
        }
    }
}

/// Kullanıcının eşleştirme kararını akışa iletir.
#[tauri::command]
pub fn respond_to_pairing(state: State<'_, AppState>, session_id: String, accept: bool) -> bool {
    state.pairing.respond(&session_id, accept)
}

/// Cihazı unutur: kayıt, mesajları ve senkron klasörleriyle birlikte silinir.
#[tauri::command]
pub fn forget_device(state: State<'_, AppState>, id: String) -> AppResult<bool> {
    let device_id = DiscoveryService::parse_device_id(&id)
        .ok_or_else(|| AppError::InvalidInput("geçersiz cihaz kimliği".to_string()))?;
    Ok(state.pairing.forget(&device_id))
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
