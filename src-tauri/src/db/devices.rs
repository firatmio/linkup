//! Güvenilir cihazlar (PLAN.md §2.12).
//!
//! Eşleştirme tamamlandığında cihazın public key'i buraya pinlenir; sonraki
//! bağlantılarda TLS doğrulaması bu kayda dayanır (PLAN.md §2.2.1).

use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;

use crate::error::AppResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustedDevice {
    pub device_id: [u8; 32],
    pub name: String,
    pub alias: Option<String>,
    pub last_address: Option<String>,
    pub last_seen: Option<i64>,
    pub paired_at: i64,
}

impl TrustedDevice {
    /// Kullanıcıya gösterilecek ad: takma ad varsa o, yoksa cihazın kendi adı.
    pub fn display_name(&self) -> &str {
        self.alias
            .as_deref()
            .filter(|alias| !alias.trim().is_empty())
            .unwrap_or(&self.name)
    }
}

/// Frontend'e giden gösterim.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustedDeviceDto {
    /// Base32 kodlu device_id.
    pub id: String,
    pub fingerprint: String,
    pub name: String,
    pub last_address: Option<String>,
    pub paired_at: i64,
    pub online: bool,
}

pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Cihazı güvenilir olarak kaydeder. Aynı cihaz yeniden eşleşirse adı ve
/// adresi tazelenir, ilk eşleşme tarihi korunur.
pub fn upsert(
    conn: &Connection,
    device_id: &[u8; 32],
    name: &str,
    address: Option<&str>,
) -> AppResult<()> {
    conn.execute(
        "INSERT INTO trusted_devices (device_id, name, last_ip, last_seen, paired_at)
         VALUES (?1, ?2, ?3, ?4, ?4)
         ON CONFLICT(device_id) DO UPDATE SET
             name = excluded.name,
             -- Adres bilinmiyorsa eskisi korunur: yeniden bağlanmanın tek
             -- ipucu o olabilir.
             last_ip = COALESCE(excluded.last_ip, trusted_devices.last_ip),
             last_seen = excluded.last_seen",
        rusqlite::params![device_id.as_slice(), name, address, now()],
    )?;
    Ok(())
}

/// Bağlantı kurulduğunda son görülen adresi tazeler.
pub fn touch(conn: &Connection, device_id: &[u8; 32], address: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE trusted_devices SET last_ip = ?2, last_seen = ?3 WHERE device_id = ?1",
        rusqlite::params![device_id.as_slice(), address, now()],
    )?;
    Ok(())
}

pub fn list(conn: &Connection) -> AppResult<Vec<TrustedDevice>> {
    let mut stmt = conn.prepare(
        "SELECT device_id, name, alias, last_ip, last_seen, paired_at
         FROM trusted_devices ORDER BY name COLLATE NOCASE",
    )?;
    let rows = stmt.query_map([], row_to_device)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn get(conn: &Connection, device_id: &[u8; 32]) -> AppResult<Option<TrustedDevice>> {
    let mut stmt = conn.prepare(
        "SELECT device_id, name, alias, last_ip, last_seen, paired_at
         FROM trusted_devices WHERE device_id = ?1",
    )?;
    Ok(stmt
        .query_row([device_id.as_slice()], row_to_device)
        .optional()?)
}

pub fn is_trusted(conn: &Connection, device_id: &[u8; 32]) -> AppResult<bool> {
    Ok(get(conn, device_id)?.is_some())
}

/// Cihazı unutur. Mesajları ve senkronize klasörleri de foreign key zinciriyle
/// birlikte silinir — kullanıcı "unut" derken bunu bekler.
pub fn forget(conn: &Connection, device_id: &[u8; 32]) -> AppResult<bool> {
    let affected = conn.execute(
        "DELETE FROM trusted_devices WHERE device_id = ?1",
        [device_id.as_slice()],
    )?;
    Ok(affected > 0)
}

fn row_to_device(row: &rusqlite::Row<'_>) -> rusqlite::Result<TrustedDevice> {
    let raw: Vec<u8> = row.get(0)?;
    let device_id: [u8; 32] = raw.try_into().map_err(|_| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Blob,
            "device_id 32 bayt değil".into(),
        )
    })?;
    Ok(TrustedDevice {
        device_id,
        name: row.get(1)?,
        alias: row.get(2)?,
        last_address: row.get(3)?,
        last_seen: row.get(4)?,
        paired_at: row.get(5)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn sample(name: &str) -> TrustedDevice {
        TrustedDevice {
            device_id: [1; 32],
            name: name.to_string(),
            alias: None,
            last_address: None,
            last_seen: None,
            paired_at: 0,
        }
    }

    #[test]
    fn eslesen_cihaz_kaydedilir_ve_okunur() {
        let pool = db::open_in_memory().unwrap();
        let conn = pool.get().unwrap();

        upsert(&conn, &[1; 32], "Dizüstü", Some("192.168.1.5:47810")).unwrap();

        let device = get(&conn, &[1; 32]).unwrap().unwrap();
        assert_eq!(device.name, "Dizüstü");
        assert_eq!(device.last_address.as_deref(), Some("192.168.1.5:47810"));
        assert!(is_trusted(&conn, &[1; 32]).unwrap());
        assert!(!is_trusted(&conn, &[2; 32]).unwrap());
    }

    #[test]
    fn yeniden_eslesme_eslesme_tarihini_korur() {
        let pool = db::open_in_memory().unwrap();
        let conn = pool.get().unwrap();

        upsert(&conn, &[1; 32], "Eski Ad", Some("192.168.1.5:47810")).unwrap();
        let first = get(&conn, &[1; 32]).unwrap().unwrap();

        upsert(&conn, &[1; 32], "Yeni Ad", Some("192.168.1.9:47810")).unwrap();
        let second = get(&conn, &[1; 32]).unwrap().unwrap();

        assert_eq!(second.name, "Yeni Ad");
        assert_eq!(second.last_address.as_deref(), Some("192.168.1.9:47810"));
        assert_eq!(
            second.paired_at, first.paired_at,
            "eşleşme tarihi korunmalı"
        );
        assert_eq!(list(&conn).unwrap().len(), 1, "kayıt ikilenmemeli");
    }

    /// Adres bilinmiyorsa eski adres korunmalı — yeniden bağlanmanın tek
    /// ipucu o olabilir.
    #[test]
    fn adressiz_guncelleme_eski_adresi_silmez() {
        let pool = db::open_in_memory().unwrap();
        let conn = pool.get().unwrap();

        upsert(&conn, &[1; 32], "A", Some("192.168.1.5:47810")).unwrap();
        upsert(&conn, &[1; 32], "A", None).unwrap();

        assert_eq!(
            get(&conn, &[1; 32])
                .unwrap()
                .unwrap()
                .last_address
                .as_deref(),
            Some("192.168.1.5:47810")
        );
    }

    #[test]
    fn unutulan_cihazin_mesajlari_da_silinir() {
        let pool = db::open_in_memory().unwrap();
        let conn = pool.get().unwrap();

        upsert(&conn, &[1; 32], "A", None).unwrap();
        conn.execute(
            "INSERT INTO messages (msg_id, conversation_id, device_id, direction,
                                   content_type, content, sent_at, status)
             VALUES ('m1', ?1, ?1, 'in', 'text', 'merhaba', 0, 'read')",
            rusqlite::params![vec![1u8; 32]],
        )
        .unwrap();

        assert!(forget(&conn, &[1; 32]).unwrap());
        assert!(get(&conn, &[1; 32]).unwrap().is_none());

        let messages: i64 = conn
            .query_row("SELECT count(*) FROM messages", [], |r| r.get(0))
            .unwrap();
        assert_eq!(messages, 0, "cihazın mesajları da silinmeli");
    }

    #[test]
    fn olmayan_cihazi_unutmak_false_doner() {
        let pool = db::open_in_memory().unwrap();
        let conn = pool.get().unwrap();
        assert!(!forget(&conn, &[9; 32]).unwrap());
    }

    #[test]
    fn takma_ad_varsa_gosterilen_ad_odur() {
        let mut device = sample("FIRAT-PC");
        assert_eq!(device.display_name(), "FIRAT-PC");

        device.alias = Some("Ofis".to_string());
        assert_eq!(device.display_name(), "Ofis");

        // Boş veya yalnızca boşluktan oluşan takma ad yok sayılmalı.
        device.alias = Some("   ".to_string());
        assert_eq!(device.display_name(), "FIRAT-PC");
    }

    #[test]
    fn liste_ada_gore_buyuk_kucuk_harf_duyarsiz_sirali() {
        let pool = db::open_in_memory().unwrap();
        let conn = pool.get().unwrap();
        upsert(&conn, &[3; 32], "Cem", None).unwrap();
        upsert(&conn, &[1; 32], "ali", None).unwrap();
        upsert(&conn, &[2; 32], "Bora", None).unwrap();

        let names: Vec<_> = list(&conn).unwrap().into_iter().map(|d| d.name).collect();
        assert_eq!(names, ["ali", "Bora", "Cem"]);
    }

    #[test]
    fn touch_adresi_ve_son_gorulmeyi_gunceller() {
        let pool = db::open_in_memory().unwrap();
        let conn = pool.get().unwrap();

        upsert(&conn, &[1; 32], "A", None).unwrap();
        touch(&conn, &[1; 32], "10.0.0.7:47810").unwrap();

        let device = get(&conn, &[1; 32]).unwrap().unwrap();
        assert_eq!(device.last_address.as_deref(), Some("10.0.0.7:47810"));
        assert!(device.last_seen.is_some());
    }
}
