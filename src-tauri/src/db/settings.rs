//! Kalıcı ayarlar (PLAN.md §2.12, §3.4).
//!
//! Ayarlar `settings` tablosunda key-value olarak durur. Bilinmeyen anahtar
//! yazılamaz: anahtar kümesi burada sabittir, böylece frontend'den gelen
//! keyfi bir anahtar tabloyu kirletemez ve yazım hatası sessizce kaybolmaz.

use rusqlite::Connection;
use serde::Serialize;

use crate::error::{AppError, AppResult};

/// Bilinen ayar anahtarları ve varsayılanları.
///
/// Yeni ayar eklerken: buraya bir satır + `Settings` alanı + `from_row` eşlemesi.
const DEFAULTS: &[(&str, &str)] = &[
    ("theme", "system"), // system | light | dark
    ("deviceName", ""),  // boş = makine adı kullanılır
];

/// Frontend'e giden ayar anlık görüntüsü.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub theme: String,
    pub device_name: String,
}

pub fn is_known_key(key: &str) -> bool {
    DEFAULTS.iter().any(|(k, _)| *k == key)
}

fn default_for(key: &str) -> &'static str {
    DEFAULTS
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| *v)
        .unwrap_or("")
}

/// Tek bir ayarı okur; kayıt yoksa varsayılana düşer.
pub fn get(conn: &Connection, key: &str) -> AppResult<String> {
    if !is_known_key(key) {
        return Err(AppError::InvalidInput(format!("bilinmeyen ayar: {key}")));
    }
    let stored: Option<String> = conn
        .query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| {
            r.get(0)
        })
        .ok();
    Ok(stored.unwrap_or_else(|| default_for(key).to_string()))
}

/// Tek bir ayarı yazar.
pub fn set(conn: &Connection, key: &str, value: &str) -> AppResult<()> {
    if !is_known_key(key) {
        return Err(AppError::InvalidInput(format!("bilinmeyen ayar: {key}")));
    }
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [key, value],
    )?;
    tracing::debug!(key, "ayar güncellendi");
    Ok(())
}

/// Tüm ayarları tek seferde okur (uygulama açılışında bir kez).
pub fn load(conn: &Connection) -> AppResult<Settings> {
    Ok(Settings {
        theme: get(conn, "theme")?,
        device_name: get(conn, "deviceName")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    #[test]
    fn varsayilanlar_kayit_yokken_doner() {
        let pool = db::open_in_memory().unwrap();
        let conn = pool.get().unwrap();
        assert_eq!(get(&conn, "theme").unwrap(), "system");
        assert_eq!(load(&conn).unwrap().device_name, "");
    }

    #[test]
    fn yazilan_ayar_geri_okunur_ve_ustune_yazilir() {
        let pool = db::open_in_memory().unwrap();
        let conn = pool.get().unwrap();

        set(&conn, "theme", "dark").unwrap();
        assert_eq!(get(&conn, "theme").unwrap(), "dark");

        set(&conn, "theme", "light").unwrap();
        assert_eq!(get(&conn, "theme").unwrap(), "light");

        // Tek satır kalmalı — ON CONFLICT çalışıyor mu?
        let count: i64 = conn
            .query_row(
                "SELECT count(*) FROM settings WHERE key = 'theme'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn bilinmeyen_anahtar_reddedilir() {
        let pool = db::open_in_memory().unwrap();
        let conn = pool.get().unwrap();
        assert!(set(&conn, "themee", "dark").is_err());
        assert!(get(&conn, "themee").is_err());
    }
}
