//! Frontend'e açılan tek API yüzeyi (PLAN.md §2.1).
//! Frontend ağ veya dosya sistemiyle doğrudan konuşmaz; her şey buradan geçer.

use serde::Serialize;
use tauri::{Emitter, State};

use crate::db::devices::TrustedDeviceDto;
use crate::db::messages::{self, Message, MessageStatus};
use crate::db::settings::{self, Settings};
use crate::db::transfers::{self, Transfer};
use crate::discovery::{DiscoveredDeviceDto, DiscoveryService};
use crate::error::{AppError, AppResult};
use crate::identity::KeyStorage;
use crate::network::protocol::{ContentType, ControlMessage};
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
    let conn = state.db.get().ok();
    state
        .pairing
        .trusted_devices()
        .iter()
        .map(|device| {
            let last = conn
                .as_ref()
                .and_then(|c| messages::last_message(c, &device.device_id).ok())
                .flatten();
            TrustedDeviceDto {
                id: data_encoding::BASE32_NOPAD.encode(&device.device_id),
                fingerprint: crate::identity::format_fingerprint(&device.device_id),
                name: device.display_name().to_string(),
                last_address: device.last_address.clone(),
                paired_at: device.paired_at,
                online: state.connections.presence.is_online(&device.device_id),
                auto_accept: device.auto_accept,
                last_message: last.as_ref().map(|m| m.content.clone()),
                last_message_at: last.as_ref().map(|m| m.sent_at),
                unread: conn
                    .as_ref()
                    .and_then(|c| messages::unread_count(c, &device.device_id).ok())
                    .unwrap_or(0),
            }
        })
        .collect()
}

/// Keşfedilmiş bir cihazla eşleştirmeyi başlatır (PLAN.md §2.5).
///
/// Komut, eşleştirme bitene kadar (en fazla 90 sn) bekler; kullanıcı kararı
/// bu sırada `respond_to_pairing` ile ayrı bir komuttan gelir.
#[tauri::command]
pub async fn start_pairing(state: State<'_, AppState>, id: String) -> AppResult<()> {
    let device_id = parse_device_id(&id)?;

    let address = state
        .discovery
        .address_of(&device_id)
        .ok_or_else(|| AppError::Unreachable("cihazın adresi bilinmiyor".to_string()))?;

    let mut parts = state
        .network
        .endpoint()
        .connect(address, Some(device_id))
        .await
        .map_err(|err| AppError::Unreachable(err.to_string()))?
        .into_parts();

    let result = crate::pairing::run(std::sync::Arc::clone(&state.pairing), &mut parts, true)
        .await
        .map_err(|err| AppError::Pairing(err.code()));

    match result {
        Ok(()) => {
            // Bağlantı kapatılmıyor: eşleşme biter bitmez kapatmak, karşı
            // tarafın henüz okumadığı onay mesajını kaybettiriyordu.
            let connections = std::sync::Arc::clone(&state.connections);
            connections.supervise(device_id);
            tauri::async_runtime::spawn(async move {
                connections.hold_parts(parts).await;
            });
            Ok(())
        }
        Err(err) => {
            parts.close();
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
    let device_id = parse_device_id(&id)?;
    Ok(state.pairing.forget(&device_id))
}

/// Bir cihazla olan sohbet geçmişi (PLAN.md §2.8).
#[tauri::command]
pub fn chat_history(
    state: State<'_, AppState>,
    id: String,
    limit: Option<u32>,
    before_id: Option<i64>,
) -> AppResult<Vec<Message>> {
    let device_id = parse_device_id(&id)?;
    let conn = state.db.get().map_err(pool_error)?;
    // Varsayılan sayfa küçük tutuldu: sohbet açılışı, geçmişin uzunluğundan
    // bağımsız olarak sabit maliyetli olmalı. Geri kalanı kaydırdıkça gelir.
    messages::list(&conn, &device_id, limit.unwrap_or(60), before_id)
}

/// Metin mesajı gönderir.
///
/// Mesaj her hâlükârda kaydedilir; cihaz bağlı değilse durumu başarısız olur.
/// Böylece kullanıcı yazdığı şeyi kaybetmez ve neden gitmediğini görür.
#[tauri::command]
pub fn send_message(
    state: State<'_, AppState>,
    id: String,
    body: String,
    is_code: Option<bool>,
) -> AppResult<Message> {
    let device_id = parse_device_id(&id)?;
    let content_type = if is_code.unwrap_or(false) {
        ContentType::Code
    } else {
        ContentType::Text
    };

    let (mut stored, frame) =
        crate::chat::prepare_outgoing(&state.db, &device_id, content_type, &body)?;

    // Kuyruğa alınabildiyse durum `sending` kalır; `sent`e geçişi, çerçeveyi
    // gerçekten akışa yazan bağlantı döngüsü yapar. Böylece "Gönderildi"
    // ifadesi karşıya çıktığı anlamına gelir.
    if state.connections.send_to(&device_id, frame) {
        return Ok(stored);
    }

    tracing::info!("cihaz bağlı değil, mesaj gönderilemedi");
    let conn = state.db.get().map_err(pool_error)?;
    messages::advance_status(&conn, &stored.msg_id, MessageStatus::Failed)?;
    stored.status = MessageStatus::Failed.as_str().to_string();
    Ok(stored)
}

/// Sohbet açıldığında gelen mesajları okundu işaretler ve karşı tarafa bildirir.
#[tauri::command]
pub fn mark_conversation_read(state: State<'_, AppState>, id: String) -> AppResult<usize> {
    let device_id = parse_device_id(&id)?;
    let conn = state.db.get().map_err(pool_error)?;
    let msg_ids = messages::mark_incoming_read(&conn, &device_id)?;

    if !msg_ids.is_empty() {
        state.connections.send_to(
            &device_id,
            ControlMessage::ReadReceipt {
                msg_ids: msg_ids.clone(),
            },
        );
    }
    Ok(msg_ids.len())
}

/// Bir cihaza dosya gönderir (PLAN.md §2.7.1).
///
/// Teklif gönderilir; karşı taraf kabul ederse veri akışı bağlantı döngüsü
/// tarafından başlatılır. Cihaz bağlı değilse transfer hiç oluşturulmaz.
#[tauri::command]
pub async fn send_file(state: State<'_, AppState>, id: String, path: String) -> AppResult<String> {
    let device_id = parse_device_id(&id)?;
    if !state.connections.is_connected(&device_id) {
        return Err(AppError::Unreachable("cihaz bağlı değil".to_string()));
    }

    let (transfer_id, offer) =
        crate::transfer::engine::prepare_offer(&state.transfers, &device_id, path.into()).await?;

    if !state.connections.send_to(&device_id, offer) {
        return Err(AppError::Unreachable("cihaz bağlı değil".to_string()));
    }
    Ok(transfer_id)
}

/// Cihazın "güvenli cihaz" işaretini değiştirir.
///
/// Açıkken o cihazdan gelen dosyalar onay sorulmadan kabul edilir.
#[tauri::command]
pub fn set_device_auto_accept(
    state: State<'_, AppState>,
    id: String,
    enabled: bool,
) -> AppResult<()> {
    let device_id = parse_device_id(&id)?;
    let conn = state.db.get().map_err(pool_error)?;
    crate::db::devices::set_auto_accept(&conn, &device_id, enabled)?;
    state.pairing.emit_devices_changed();
    Ok(())
}

/// Sonlanmış aktarımları listeden temizler.
///
/// Tamamlanan ve başarısız kayıtlar birikir; kullanıcı listeyi
/// temizleyebilmeli. Dosyalar silinmez, yalnızca kayıt düşer.
#[tauri::command]
pub fn clear_finished_transfers(state: State<'_, AppState>) -> AppResult<usize> {
    let conn = state.db.get().map_err(pool_error)?;
    let cleared = transfers::clear_finished(&conn)?;
    let _ = state
        .transfers
        .app
        .emit(crate::transfer::engine::EVENT_CHANGED, ());
    Ok(cleared)
}

/// Kullanıcının dosya kabul kararını akışa iletir (PLAN.md §2.13.3).
#[tauri::command]
pub fn respond_to_transfer(state: State<'_, AppState>, transfer_id: String, accept: bool) -> bool {
    state.transfers.approvals.respond(&transfer_id, accept)
}

/// Alınan dosyaların geçmişi (PLAN.md §3.2 "Gelen Dosyalar").
#[tauri::command]
pub fn incoming_files(state: State<'_, AppState>, limit: Option<u32>) -> AppResult<Vec<Transfer>> {
    let conn = state.db.get().map_err(pool_error)?;
    transfers::list_incoming(&conn, limit.unwrap_or(500))
}

/// Sürmekte olan transferler — ilerleme paneli için.
#[tauri::command]
pub fn active_transfers(state: State<'_, AppState>) -> AppResult<Vec<Transfer>> {
    let conn = state.db.get().map_err(pool_error)?;
    transfers::list_active(&conn)
}

/// Alınan bir dosyayı işletim sisteminin varsayılan uygulamasıyla açar.
#[tauri::command]
pub fn open_transfer_file(state: State<'_, AppState>, transfer_id: String) -> AppResult<()> {
    let path = transfer_path(&state, &transfer_id)?;
    tauri_plugin_opener::open_path(path, None::<&str>)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("dosya açılamadı: {e}")))
}

/// Dosyayı içinde bulunduğu klasörde gösterir.
#[tauri::command]
pub fn reveal_transfer_file(state: State<'_, AppState>, transfer_id: String) -> AppResult<()> {
    let path = transfer_path(&state, &transfer_id)?;
    tauri_plugin_opener::reveal_item_in_dir(path)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("klasör açılamadı: {e}")))
}

fn transfer_path(state: &State<'_, AppState>, transfer_id: &str) -> AppResult<String> {
    let conn = state.db.get().map_err(pool_error)?;
    let record = transfers::get(&conn, transfer_id)?
        .ok_or_else(|| AppError::InvalidInput("transfer bulunamadı".to_string()))?;

    let path = record
        .save_path
        .ok_or_else(|| AppError::InvalidInput("dosya yolu yok".to_string()))?;

    // Kayıt eskiyse dosya taşınmış veya silinmiş olabilir; kullanıcıya
    // "bulunamadı" demek, boş bir pencere açmaktan iyidir.
    if !std::path::Path::new(&path).exists() {
        return Err(AppError::InvalidInput("dosya bulunamadı".to_string()));
    }
    Ok(path)
}

/// Ayarlar → Gelişmiş → "Log klasörünü aç" (PLAN.md §2.14).
#[tauri::command]
pub fn open_log_dir(state: State<'_, AppState>) -> AppResult<()> {
    let path = state.paths.log_dir.clone();
    tauri_plugin_opener::open_path(path.to_string_lossy().to_string(), None::<&str>)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("log klasörü açılamadı: {e}")))?;
    Ok(())
}

fn parse_device_id(id: &str) -> AppResult<[u8; 32]> {
    DiscoveryService::parse_device_id(id)
        .ok_or_else(|| AppError::InvalidInput("geçersiz cihaz kimliği".to_string()))
}

fn pool_error(err: r2d2::Error) -> AppError {
    AppError::Internal(anyhow::anyhow!("veritabanı bağlantısı alınamadı: {err}"))
}
