//! Sohbet: mesaj gönderme, alma ve durum bildirimleri (PLAN.md §2.8).
//!
//! Kalıcılık ve ağ ayrı tutulur: mesaj ÖNCE veritabanına yazılır, sonra
//! gönderilir. Böylece uygulama gönderim sırasında kapansa bile mesaj
//! kaybolmaz — yalnızca durumu `sending` kalır ve açılışta `failed`e çevrilir.

use data_encoding::BASE32_NOPAD;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::db::messages::{self, Message, MessageStatus, NewMessage};
use crate::db::DbPool;
use crate::error::{AppError, AppResult};
use crate::network::protocol::{ChatMessage, ContentType, ControlMessage};

pub const EVENT_MESSAGE: &str = "chat:message";
pub const EVENT_STATUS: &str = "chat:status";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IncomingEvent {
    /// Base32 kodlu device_id.
    pub device_id: String,
    pub message: Message,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusEvent {
    pub device_id: String,
    pub msg_id: String,
    pub status: String,
}

pub fn encode_device_id(device_id: &[u8; 32]) -> String {
    BASE32_NOPAD.encode(device_id)
}

/// Giden mesajı kaydeder ve ağ çerçevesini üretir.
///
/// Gönderimin kendisi çağırana bırakılır: bağlantı yoksa mesaj yine de
/// kaydedilmiş olur ve arayüzde başarısız olarak görünür.
pub fn prepare_outgoing(
    db: &DbPool,
    device_id: &[u8; 32],
    content_type: ContentType,
    body: &str,
) -> AppResult<(Message, ControlMessage)> {
    let body = body.trim();
    if body.is_empty() {
        return Err(AppError::InvalidInput("boş mesaj".to_string()));
    }

    let msg_id = new_message_id();
    let sent_at = crate::db::devices::now();

    let conn = db.get().map_err(pool_error)?;
    let id = messages::insert(
        &conn,
        NewMessage {
            msg_id: &msg_id,
            device_id,
            outgoing: true,
            content_type: content_type.as_str(),
            content: body,
            transfer_id: None,
            sent_at,
            status: MessageStatus::Sending,
        },
    )?
    .ok_or_else(|| AppError::Internal(anyhow::anyhow!("mesaj kimliği çakıştı")))?;

    let stored = Message {
        id,
        msg_id: msg_id.clone(),
        direction: "out".to_string(),
        content_type: content_type.as_str().to_string(),
        content: body.to_string(),
        transfer_id: None,
        transfer: None,
        sent_at,
        status: MessageStatus::Sending.as_str().to_string(),
    };

    let frame = ControlMessage::Chat(ChatMessage {
        msg_id,
        content_type,
        body: body.to_string(),
        sent_at,
    });

    Ok((stored, frame))
}

/// Bir dosya aktarımını sohbet akışına yerleştirir.
///
/// Aktarımlar sohbetin altındaki ayrı bir şeritte gösteriliyordu; dosya
/// bittiğinde oradan kayboluyor ve konuşmada hiçbir izi kalmıyordu. Artık
/// her aktarımın sohbette bir baloncuğu var.
///
/// Bu kayıt ağ üzerinden GİTMEZ: iki taraf da aktarımı zaten `FileOffer`
/// üzerinden biliyor, ayrıca bir sohbet çerçevesi göndermek protokole
/// gereksiz bir tekrar eklerdi. `msg_id` bu yüzden aktarımdan türetiliyor —
/// aynı aktarım için ikinci bir baloncuk oluşmaz.
///
/// Durum kopyalanmıyor: baloncuğun ilerlemesi ve sonucu okunurken
/// `transfers` tablosundan iliştirilir (bkz. `messages::list`).
pub fn record_transfer(
    db: &DbPool,
    app: &AppHandle,
    device_id: &[u8; 32],
    outgoing: bool,
    transfer_id: &str,
    file_name: &str,
) -> AppResult<()> {
    let msg_id = format!("transfer-{transfer_id}");
    let sent_at = crate::db::devices::now();

    let conn = db.get().map_err(pool_error)?;
    let Some(id) = messages::insert(
        &conn,
        NewMessage {
            msg_id: &msg_id,
            device_id,
            outgoing,
            content_type: CONTENT_TYPE_FILE,
            content: file_name,
            transfer_id: Some(transfer_id),
            sent_at,
            // Dosyanın teslim durumu aktarımın kendi durumudur; baloncuğun
            // tik göstergesi bu yüzden `sent`te sabit kalır.
            status: MessageStatus::Sent,
        },
    )?
    else {
        return Ok(());
    };

    let transfer = crate::db::transfers::get(&conn, transfer_id)?;
    let _ = app.emit(
        EVENT_MESSAGE,
        IncomingEvent {
            device_id: encode_device_id(device_id),
            message: Message {
                id,
                msg_id,
                direction: if outgoing { "out" } else { "in" }.to_string(),
                content_type: CONTENT_TYPE_FILE.to_string(),
                content: file_name.to_string(),
                transfer_id: Some(transfer_id.to_string()),
                transfer,
                sent_at,
                status: MessageStatus::Sent.as_str().to_string(),
            },
        },
    );
    Ok(())
}

/// Şemadaki `content_type` değeri (001_initial.sql).
const CONTENT_TYPE_FILE: &str = "file_ref";

/// Gelen mesajı kaydeder, arayüze bildirir ve gönderilecek `ChatAck`i döndürür.
///
/// Aynı mesaj ikinci kez gelirse (yeniden bağlanma sonrası tekrar gönderim)
/// kaydedilmez ama ack yine döner: karşı taraf ilk onayı almamış olabilir.
pub fn handle_incoming(
    db: &DbPool,
    app: &AppHandle,
    device_id: &[u8; 32],
    device_name: &str,
    incoming: ChatMessage,
) -> AppResult<ControlMessage> {
    // Kayıt zamanı bizim saatimize göre: karşı tarafın saati yanlışsa
    // mesajlar geçmişte veya gelecekte görünmemeli.
    let received_at = crate::db::devices::now();

    let conn = db.get().map_err(pool_error)?;
    let inserted = messages::insert(
        &conn,
        NewMessage {
            msg_id: &incoming.msg_id,
            device_id,
            outgoing: false,
            content_type: incoming.content_type.as_str(),
            content: &incoming.body,
            transfer_id: None,
            sent_at: received_at,
            status: MessageStatus::Delivered,
        },
    )?;

    if let Some(id) = inserted {
        crate::notifications::message_received(
            app,
            &encode_device_id(device_id),
            device_name,
            &incoming.body,
        );
        let _ = app.emit(
            EVENT_MESSAGE,
            IncomingEvent {
                device_id: encode_device_id(device_id),
                message: Message {
                    id,
                    msg_id: incoming.msg_id.clone(),
                    direction: "in".to_string(),
                    content_type: incoming.content_type.as_str().to_string(),
                    content: incoming.body,
                    transfer_id: None,
                    transfer: None,
                    sent_at: received_at,
                    status: MessageStatus::Delivered.as_str().to_string(),
                },
            },
        );
    }

    Ok(ControlMessage::ChatAck {
        msg_id: incoming.msg_id,
    })
}

/// Karşı taraftan gelen durum bildirimini işler ve arayüze yansıtır.
pub fn apply_status(
    db: &DbPool,
    app: &AppHandle,
    device_id: &[u8; 32],
    msg_ids: &[String],
    status: MessageStatus,
) -> AppResult<()> {
    let conn = db.get().map_err(pool_error)?;
    for msg_id in msg_ids {
        if messages::advance_status(&conn, msg_id, status)? {
            let _ = app.emit(
                EVENT_STATUS,
                StatusEvent {
                    device_id: encode_device_id(device_id),
                    msg_id: msg_id.clone(),
                    status: status.as_str().to_string(),
                },
            );
        }
    }
    Ok(())
}

fn new_message_id() -> String {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).expect("işletim sistemi entropisi");
    data_encoding::HEXLOWER.encode(&bytes)
}

fn pool_error(err: r2d2::Error) -> AppError {
    AppError::Internal(anyhow::anyhow!("veritabanı bağlantısı alınamadı: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    #[test]
    fn mesaj_kimlikleri_benzersiz() {
        let ids: std::collections::HashSet<_> = (0..200).map(|_| new_message_id()).collect();
        assert_eq!(ids.len(), 200);
    }

    #[test]
    fn giden_mesaj_once_kaydedilir() {
        let pool = db::open_in_memory().unwrap();
        {
            let conn = pool.get().unwrap();
            db::devices::upsert(&conn, &[1; 32], "Cihaz", None).unwrap();
        }

        let (stored, frame) =
            prepare_outgoing(&pool, &[1; 32], ContentType::Text, "  merhaba  ").unwrap();

        assert_eq!(
            stored.content, "merhaba",
            "baştaki/sondaki boşluk kırpılmalı"
        );
        assert_eq!(stored.status, "sending");
        match frame {
            ControlMessage::Chat(chat) => {
                assert_eq!(chat.msg_id, stored.msg_id, "kimlik iki uçta aynı olmalı");
                assert_eq!(chat.body, "merhaba");
            }
            other => panic!("beklenmeyen çerçeve: {other:?}"),
        }

        // Ağa çıkmadan önce kaydedilmiş olmalı.
        let conn = pool.get().unwrap();
        assert_eq!(
            db::messages::list(&conn, &[1; 32], 10, None).unwrap().len(),
            1
        );
    }

    #[test]
    fn bos_mesaj_reddedilir() {
        let pool = db::open_in_memory().unwrap();
        assert!(prepare_outgoing(&pool, &[1; 32], ContentType::Text, "   ").is_err());
        assert!(prepare_outgoing(&pool, &[1; 32], ContentType::Text, "").is_err());
    }
}
