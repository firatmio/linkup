//! Sohbet mesajları (PLAN.md §2.8, §2.12).

use rusqlite::Connection;
use serde::Serialize;

use crate::error::AppResult;

/// Mesajın yaşam döngüsü (PLAN.md §2.8).
///
/// Sıra önemli: bir mesaj yalnızca ileri gidebilir. Geç gelen bir `delivered`
/// bildirimi, çoktan `read` olmuş bir mesajı geriye çekmemeli.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MessageStatus {
    Sending,
    Sent,
    Delivered,
    Read,
    Failed,
}

impl MessageStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            MessageStatus::Sending => "sending",
            MessageStatus::Sent => "sent",
            MessageStatus::Delivered => "delivered",
            MessageStatus::Read => "read",
            MessageStatus::Failed => "failed",
        }
    }

    /// İlerleme sırasındaki yeri. `Failed` bu sıraya dâhil DEĞİLDİR:
    /// ona geçiş `advance_status` içinde ayrıca ele alınır.
    fn rank(self) -> u8 {
        match self {
            MessageStatus::Sending => 0,
            MessageStatus::Sent => 1,
            MessageStatus::Delivered => 2,
            MessageStatus::Read => 3,
            MessageStatus::Failed => u8::MAX,
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub msg_id: String,
    /// "in" | "out"
    pub direction: String,
    /// "text" | "code" | "image" | "file_ref"
    pub content_type: String,
    pub content: String,
    pub sent_at: i64,
    pub status: String,
}

pub struct NewMessage<'a> {
    pub msg_id: &'a str,
    pub device_id: &'a [u8; 32],
    pub outgoing: bool,
    pub content_type: &'a str,
    pub content: &'a str,
    pub sent_at: i64,
    pub status: MessageStatus,
}

/// Mesajı kaydeder. Aynı `msg_id` ikinci kez gelirse yok sayılır —
/// yeniden bağlanma sonrası tekrar gönderim mesajı ikilememelidir.
pub fn insert(conn: &Connection, message: NewMessage<'_>) -> AppResult<bool> {
    let affected = conn.execute(
        "INSERT INTO messages (msg_id, conversation_id, device_id, direction,
                               content_type, content, sent_at, status)
         VALUES (?1, ?2, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(msg_id) DO NOTHING",
        rusqlite::params![
            message.msg_id,
            message.device_id.as_slice(),
            if message.outgoing { "out" } else { "in" },
            message.content_type,
            message.content,
            message.sent_at,
            message.status.as_str(),
        ],
    )?;
    Ok(affected > 0)
}

/// Bir cihazla olan sohbeti eskiden yeniye döndürür.
///
/// `before_id` verilirse ondan öncekiler getirilir (geçmişe kaydırma).
pub fn list(
    conn: &Connection,
    device_id: &[u8; 32],
    limit: u32,
    before_id: Option<i64>,
) -> AppResult<Vec<Message>> {
    // Sondan `limit` kadar alınıp ters çevrilir: kullanıcı sohbetin sonunu
    // görmek ister, başını değil.
    let mut stmt = conn.prepare(
        "SELECT msg_id, direction, content_type, content, sent_at, status
         FROM messages
         WHERE conversation_id = ?1 AND (?2 IS NULL OR id < ?2)
         ORDER BY id DESC
         LIMIT ?3",
    )?;

    let rows = stmt.query_map(
        rusqlite::params![device_id.as_slice(), before_id, limit],
        |row| {
            Ok(Message {
                msg_id: row.get(0)?,
                direction: row.get(1)?,
                content_type: row.get(2)?,
                content: row.get(3)?,
                sent_at: row.get(4)?,
                status: row.get(5)?,
            })
        },
    )?;

    let mut messages = rows.collect::<Result<Vec<_>, _>>()?;
    messages.reverse();
    Ok(messages)
}

/// Giden bir mesajın durumunu ilerletir.
///
/// Yalnızca ileri yönde günceller: ağdaki gecikme yüzünden `delivered`
/// bildirimi `read`ten sonra gelebilir ve göstergeyi geri almamalıdır.
pub fn advance_status(conn: &Connection, msg_id: &str, status: MessageStatus) -> AppResult<bool> {
    // `Failed` sıralamanın parçası değil: yalnızca henüz yola çıkmamış
    // (`sending`) bir mesaj başarısız olabilir. Karşı tarafa ulaşmış bir
    // mesajı sonradan "gönderilemedi" göstermek yanlış olurdu.
    if status == MessageStatus::Failed {
        return Ok(conn.execute(
            "UPDATE messages SET status = 'failed'
             WHERE msg_id = ?1 AND direction = 'out' AND status = 'sending'",
            [msg_id],
        )? > 0);
    }

    let ranks: Vec<&str> = [
        MessageStatus::Sending,
        MessageStatus::Sent,
        MessageStatus::Delivered,
        MessageStatus::Read,
        MessageStatus::Failed,
    ]
    .iter()
    .filter(|candidate| candidate.rank() < status.rank())
    .map(|candidate| candidate.as_str())
    .collect();

    if ranks.is_empty() {
        return Ok(false);
    }

    let placeholders = ranks.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!(
        "UPDATE messages SET status = ?1
         WHERE msg_id = ?2 AND direction = 'out' AND status IN ({placeholders})"
    );

    let new_status = status.as_str();
    let mut params: Vec<&dyn rusqlite::ToSql> = vec![&new_status, &msg_id];
    for rank in &ranks {
        params.push(rank);
    }

    Ok(conn.execute(&sql, params.as_slice())? > 0)
}

/// Gelen mesajı okundu işaretler; karşı tarafa bildirilecek olanları döndürür.
///
/// Yalnızca daha önce bildirilmemiş olanlar döner — her ekran açılışında
/// aynı makbuzu tekrar göndermek gereksiz trafiktir.
pub fn mark_incoming_read(conn: &Connection, device_id: &[u8; 32]) -> AppResult<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT msg_id FROM messages
         WHERE conversation_id = ?1 AND direction = 'in' AND status != 'read'",
    )?;
    let ids: Vec<String> = stmt
        .query_map([device_id.as_slice()], |row| row.get(0))?
        .collect::<Result<_, _>>()?;

    if ids.is_empty() {
        return Ok(ids);
    }

    conn.execute(
        "UPDATE messages SET status = 'read'
         WHERE conversation_id = ?1 AND direction = 'in' AND status != 'read'",
        [device_id.as_slice()],
    )?;
    Ok(ids)
}

/// Bir cihaza giden, henüz yola çıkmamış mesajları başarısız işaretler ve
/// kimliklerini döndürür.
///
/// Bağlantı koptuğunda çağrılır: kuyrukta bekleyen mesaj artık gitmeyecektir,
/// kullanıcı bunu saat ikonuna bakarak tahmin etmek zorunda kalmamalı.
pub fn fail_pending_for_device(conn: &Connection, device_id: &[u8; 32]) -> AppResult<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT msg_id FROM messages
         WHERE conversation_id = ?1 AND direction = 'out' AND status = 'sending'",
    )?;
    let ids: Vec<String> = stmt
        .query_map([device_id.as_slice()], |row| row.get(0))?
        .collect::<Result<_, _>>()?;

    if !ids.is_empty() {
        conn.execute(
            "UPDATE messages SET status = 'failed'
             WHERE conversation_id = ?1 AND direction = 'out' AND status = 'sending'",
            [device_id.as_slice()],
        )?;
    }
    Ok(ids)
}

/// Uygulama kapanırken yolda kalan mesajlar `sending` durumunda donar;
/// açılışta bunlar başarısız sayılır ki gösterge sonsuza kadar dönmesin.
pub fn fail_stuck_outgoing(conn: &Connection) -> AppResult<usize> {
    Ok(conn.execute(
        "UPDATE messages SET status = 'failed' WHERE direction = 'out' AND status = 'sending'",
        [],
    )?)
}

/// Sohbet listesi için: cihaz başına son mesaj.
pub fn last_message(conn: &Connection, device_id: &[u8; 32]) -> AppResult<Option<Message>> {
    Ok(list(conn, device_id, 1, None)?.pop())
}

pub fn unread_count(conn: &Connection, device_id: &[u8; 32]) -> AppResult<i64> {
    Ok(conn.query_row(
        "SELECT count(*) FROM messages
         WHERE conversation_id = ?1 AND direction = 'in' AND status != 'read'",
        [device_id.as_slice()],
        |row| row.get(0),
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{self, devices};

    const DEVICE: [u8; 32] = [1; 32];

    fn setup() -> db::DbPool {
        let pool = db::open_in_memory().unwrap();
        {
            let conn = pool.get().unwrap();
            devices::upsert(&conn, &DEVICE, "Cihaz", None).unwrap();
        }
        pool
    }

    fn send(conn: &Connection, id: &str, outgoing: bool, status: MessageStatus) {
        insert(
            conn,
            NewMessage {
                msg_id: id,
                device_id: &DEVICE,
                outgoing,
                content_type: "text",
                content: id,
                sent_at: 0,
                status,
            },
        )
        .unwrap();
    }

    #[test]
    fn mesaj_kaydedilir_ve_sirasiyla_okunur() {
        let pool = setup();
        let conn = pool.get().unwrap();

        send(&conn, "m1", true, MessageStatus::Sent);
        send(&conn, "m2", false, MessageStatus::Delivered);
        send(&conn, "m3", true, MessageStatus::Sent);

        let messages = list(&conn, &DEVICE, 50, None).unwrap();
        let ids: Vec<_> = messages.iter().map(|m| m.msg_id.as_str()).collect();
        assert_eq!(ids, ["m1", "m2", "m3"], "eskiden yeniye sıralanmalı");
        assert_eq!(messages[1].direction, "in");
    }

    /// Yeniden bağlanma sonrası tekrar gönderim mesajı ikilememeli.
    #[test]
    fn ayni_mesaj_iki_kez_kaydedilmez() {
        let pool = setup();
        let conn = pool.get().unwrap();

        assert!(insert(
            &conn,
            NewMessage {
                msg_id: "m1",
                device_id: &DEVICE,
                outgoing: false,
                content_type: "text",
                content: "merhaba",
                sent_at: 0,
                status: MessageStatus::Delivered,
            }
        )
        .unwrap());

        assert!(
            !insert(
                &conn,
                NewMessage {
                    msg_id: "m1",
                    device_id: &DEVICE,
                    outgoing: false,
                    content_type: "text",
                    content: "merhaba",
                    sent_at: 0,
                    status: MessageStatus::Delivered,
                }
            )
            .unwrap(),
            "ikinci kayıt yok sayılmalı"
        );
        assert_eq!(list(&conn, &DEVICE, 50, None).unwrap().len(), 1);
    }

    #[test]
    fn durum_ileri_dogru_ilerler() {
        let pool = setup();
        let conn = pool.get().unwrap();
        send(&conn, "m1", true, MessageStatus::Sending);

        assert!(advance_status(&conn, "m1", MessageStatus::Sent).unwrap());
        assert!(advance_status(&conn, "m1", MessageStatus::Delivered).unwrap());
        assert!(advance_status(&conn, "m1", MessageStatus::Read).unwrap());

        assert_eq!(list(&conn, &DEVICE, 1, None).unwrap()[0].status, "read");
    }

    /// Geç gelen `delivered`, çoktan `read` olmuş mesajı geriye çekmemeli.
    /// Gerileme testi: `failed` gerçekten yazılabilmeli.
    ///
    /// İlk uygulamada `Failed`in sırası 0'dı ve "sırası daha küçük olanlardan
    /// geç" filtresi hiçbir durumu eşleştirmiyordu; gönderilemeyen mesaj
    /// sonsuza kadar `sending` (dönen saat) olarak kalıyordu.
    #[test]
    fn gonderilemeyen_mesaj_basarisiz_isaretlenir() {
        let pool = setup();
        let conn = pool.get().unwrap();
        send(&conn, "m1", true, MessageStatus::Sending);

        assert!(advance_status(&conn, "m1", MessageStatus::Failed).unwrap());
        assert_eq!(list(&conn, &DEVICE, 1, None).unwrap()[0].status, "failed");
    }

    /// Karşıya ulaşmış bir mesaj sonradan "gönderilemedi" olmamalı.
    #[test]
    fn ulasmis_mesaj_basarisiz_olamaz() {
        let pool = setup();
        let conn = pool.get().unwrap();
        send(&conn, "m1", true, MessageStatus::Delivered);

        assert!(!advance_status(&conn, "m1", MessageStatus::Failed).unwrap());
        assert_eq!(
            list(&conn, &DEVICE, 1, None).unwrap()[0].status,
            "delivered"
        );
    }

    #[test]
    fn baglanti_kopunca_bekleyenler_basarisiz_olur() {
        let pool = setup();
        let conn = pool.get().unwrap();
        send(&conn, "m1", true, MessageStatus::Sending);
        send(&conn, "m2", true, MessageStatus::Sent);
        send(&conn, "m3", false, MessageStatus::Delivered);

        let failed = fail_pending_for_device(&conn, &DEVICE).unwrap();
        assert_eq!(failed, ["m1"], "yalnızca yola çıkmamış giden mesaj");

        let messages = list(&conn, &DEVICE, 10, None).unwrap();
        assert_eq!(messages[0].status, "failed");
        assert_eq!(messages[1].status, "sent");
        assert_eq!(messages[2].status, "delivered");
    }

    #[test]
    fn durum_geriye_gitmez() {
        let pool = setup();
        let conn = pool.get().unwrap();
        send(&conn, "m1", true, MessageStatus::Read);

        assert!(!advance_status(&conn, "m1", MessageStatus::Delivered).unwrap());
        assert!(!advance_status(&conn, "m1", MessageStatus::Sent).unwrap());
        assert_eq!(list(&conn, &DEVICE, 1, None).unwrap()[0].status, "read");
    }

    /// Gelen mesajın durumu karşı tarafın makbuzuyla değişmemeli.
    #[test]
    fn gelen_mesaj_durumu_disaridan_degistirilemez() {
        let pool = setup();
        let conn = pool.get().unwrap();
        send(&conn, "m1", false, MessageStatus::Delivered);

        assert!(!advance_status(&conn, "m1", MessageStatus::Read).unwrap());
    }

    #[test]
    fn okundu_isaretleme_yalnizca_bildirilmemisleri_doner() {
        let pool = setup();
        let conn = pool.get().unwrap();
        send(&conn, "m1", false, MessageStatus::Delivered);
        send(&conn, "m2", false, MessageStatus::Delivered);
        send(&conn, "m3", true, MessageStatus::Sent);

        let mut ids = mark_incoming_read(&conn, &DEVICE).unwrap();
        ids.sort();
        assert_eq!(ids, ["m1", "m2"], "yalnızca gelen mesajlar işaretlenmeli");
        assert_eq!(unread_count(&conn, &DEVICE).unwrap(), 0);

        assert!(
            mark_incoming_read(&conn, &DEVICE).unwrap().is_empty(),
            "aynı makbuz ikinci kez gönderilmemeli"
        );
    }

    #[test]
    fn yolda_kalan_giden_mesajlar_basarisiz_isaretlenir() {
        let pool = setup();
        let conn = pool.get().unwrap();
        send(&conn, "m1", true, MessageStatus::Sending);
        send(&conn, "m2", true, MessageStatus::Sent);
        send(&conn, "m3", false, MessageStatus::Delivered);

        assert_eq!(fail_stuck_outgoing(&conn).unwrap(), 1);

        let messages = list(&conn, &DEVICE, 50, None).unwrap();
        assert_eq!(messages[0].status, "failed");
        assert_eq!(
            messages[1].status, "sent",
            "gönderilmiş mesaja dokunulmamalı"
        );
        assert_eq!(
            messages[2].status, "delivered",
            "gelen mesaja dokunulmamalı"
        );
    }

    #[test]
    fn son_mesaj_ve_okunmamis_sayisi() {
        let pool = setup();
        let conn = pool.get().unwrap();
        assert!(last_message(&conn, &DEVICE).unwrap().is_none());

        send(&conn, "m1", false, MessageStatus::Delivered);
        send(&conn, "m2", false, MessageStatus::Delivered);

        assert_eq!(last_message(&conn, &DEVICE).unwrap().unwrap().msg_id, "m2");
        assert_eq!(unread_count(&conn, &DEVICE).unwrap(), 2);
    }

    #[test]
    fn sayfalama_gecmise_dogru_calisir() {
        let pool = setup();
        let conn = pool.get().unwrap();
        for i in 0..10 {
            send(&conn, &format!("m{i}"), true, MessageStatus::Sent);
        }

        let recent = list(&conn, &DEVICE, 3, None).unwrap();
        assert_eq!(
            recent.iter().map(|m| m.msg_id.as_str()).collect::<Vec<_>>(),
            ["m7", "m8", "m9"],
            "son mesajlar getirilmeli"
        );

        let first_id: i64 = conn
            .query_row("SELECT id FROM messages WHERE msg_id = 'm7'", [], |r| {
                r.get(0)
            })
            .unwrap();
        let older = list(&conn, &DEVICE, 3, Some(first_id)).unwrap();
        assert_eq!(
            older.iter().map(|m| m.msg_id.as_str()).collect::<Vec<_>>(),
            ["m4", "m5", "m6"]
        );
    }

    /// FTS indeksi mesajlarla senkron kalmalı (PLAN.md §2.8, Faz 9'un temeli).
    #[test]
    fn kaydedilen_mesaj_aramada_bulunur() {
        let pool = setup();
        let conn = pool.get().unwrap();
        insert(
            &conn,
            NewMessage {
                msg_id: "m1",
                device_id: &DEVICE,
                outgoing: false,
                content_type: "text",
                content: "yarın toplantı var",
                sent_at: 0,
                status: MessageStatus::Delivered,
            },
        )
        .unwrap();

        let hits: i64 = conn
            .query_row(
                "SELECT count(*) FROM messages_fts WHERE messages_fts MATCH 'toplantı'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(hits, 1);
    }
}
