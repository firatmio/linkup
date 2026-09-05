//! SQLite erişimi (PLAN.md §2.12, §10-K3).
//!
//! `rusqlite` + `r2d2` bağlantı havuzu. Gömülü SQLite'ta gerçek async I/O
//! olmadığı için çağrılar bloklar; Tauri komutlarından çağrıldığında
//! `spawn_blocking` içine alınırlar.

pub mod migrations;
pub mod settings;

use std::path::Path;

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;

pub type DbPool = Pool<SqliteConnectionManager>;

/// Her bağlantıda uygulanan PRAGMA'lar (PLAN.md §2.12).
///
/// `foreign_keys` bağlantı başına ayarlanır — veritabanına kalıcı yazılmaz,
/// bu yüzden havuzun açtığı HER bağlantıda tekrar edilmeli.
fn configure(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA foreign_keys = ON;
         PRAGMA busy_timeout = 5000;",
    )
}

/// Veritabanını açar, şemayı günceller ve havuzu döndürür.
pub fn open(db_path: &Path) -> anyhow::Result<DbPool> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Migration'lar havuzdan önce, tek bir bağlantıda çalışır: birden fazla
    // bağlantının aynı anda şema değiştirmesi diye bir durum oluşmasın.
    let mut conn = Connection::open(db_path)?;
    configure(&conn)?;
    migrations::run(&mut conn)?;
    drop(conn);

    let manager = SqliteConnectionManager::file(db_path).with_init(|c| configure(c));
    let pool = Pool::builder().max_size(8).build(manager)?;

    tracing::info!(path = %db_path.display(), "veritabanı hazır");
    Ok(pool)
}

#[cfg(test)]
pub fn open_in_memory() -> anyhow::Result<DbPool> {
    let manager = SqliteConnectionManager::memory().with_init(|c| configure(c));
    // Bellek içi veritabanında her bağlantı ayrı bir veritabanıdır; testlerin
    // aynı veriyi görmesi için havuz tek bağlantıya sabitlenir.
    let pool = Pool::builder().max_size(1).build(manager)?;
    let mut conn = pool.get()?;
    migrations::run(&mut conn)?;
    drop(conn);
    Ok(pool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foreign_keys_her_baglantida_acik() {
        let pool = open_in_memory().unwrap();
        let conn = pool.get().unwrap();
        let on: i64 = conn
            .query_row("PRAGMA foreign_keys", [], |r| r.get(0))
            .unwrap();
        assert_eq!(on, 1);
    }

    #[test]
    fn bilinmeyen_cihaza_mesaj_reddedilir() {
        let pool = open_in_memory().unwrap();
        let conn = pool.get().unwrap();
        let result = conn.execute(
            "INSERT INTO messages (msg_id, conversation_id, device_id, direction,
                                   content_type, content, sent_at, status)
             VALUES ('a', ?1, ?1, 'in', 'text', 'merhaba', 0, 'read')",
            rusqlite::params![vec![9u8; 32]],
        );
        assert!(result.is_err(), "foreign key kısıtı devrede olmalı");
    }
}
