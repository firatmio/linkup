//! Şema sürümleme (PLAN.md §2.12).
//!
//! Migration'lar derleme zamanında binary'e gömülür ve sıralı olarak,
//! SQLite'ın `user_version` pragma'sı takip edilerek uygulanır. Her migration
//! kendi işlemi (transaction) içinde çalışır: yarım uygulanmış şema kalmaz.
//!
//! Elle yazılmış bir runner kullanılıyor (bir migration kütüphanesi yerine):
//! kütüphaneler `libsqlite3-sys`'i kendi sürümlerine kilitleyip rusqlite ile
//! çakışıyor, kazanç ise bu ~40 satırdan ibaret.

use rusqlite::Connection;

struct Migration {
    version: i32,
    name: &'static str,
    sql: &'static str,
}

/// Sıra ÖNEMLİ: yeni migration'lar sona eklenir, mevcutlar asla değiştirilmez.
const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    name: "initial",
    sql: include_str!("../../migrations/001_initial.sql"),
}];

pub fn run(conn: &mut Connection) -> rusqlite::Result<()> {
    let current: i32 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;

    let pending: Vec<&Migration> = MIGRATIONS.iter().filter(|m| m.version > current).collect();
    if pending.is_empty() {
        tracing::debug!(schema_version = current, "şema güncel");
        return Ok(());
    }

    for migration in pending {
        tracing::info!(
            version = migration.version,
            name = migration.name,
            "migration uygulanıyor"
        );
        let tx = conn.transaction()?;
        tx.execute_batch(migration.sql)?;
        // PRAGMA parametre bağlamayı desteklemiyor; sürüm sabit bir i32.
        tx.execute_batch(&format!("PRAGMA user_version = {}", migration.version))?;
        tx.commit()?;
    }

    let new_version: i32 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    tracing::info!(schema_version = new_version, "şema hazır");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrationlar_sirali_ve_tekil() {
        for (expected, m) in (1..).zip(MIGRATIONS.iter()) {
            assert_eq!(
                m.version, expected,
                "migration sürümleri 1'den başlayıp artmalı"
            );
        }
    }

    #[test]
    fn semayi_kurar_ve_tekrar_calistirmak_guvenlidir() {
        let mut conn = Connection::open_in_memory().unwrap();
        run(&mut conn).unwrap();

        let version: i32 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, MIGRATIONS.len() as i32);

        // İkinci çalıştırma hiçbir şey yapmamalı.
        run(&mut conn).unwrap();

        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();

        for expected in [
            "messages",
            "messages_fts",
            "settings",
            "synced_folders",
            "transfers",
            "trusted_devices",
        ] {
            assert!(
                tables.contains(&expected.to_string()),
                "eksik tablo: {expected}"
            );
        }
    }

    /// PLAN.md §2.12 ve §10-K3: FTS5'in `bundled` derlemede etkin olduğu
    /// Faz 1'de doğrulanacaktı — doğrulaması budur.
    #[test]
    fn fts5_calisir_ve_turkce_arar() {
        let mut conn = Connection::open_in_memory().unwrap();
        run(&mut conn).unwrap();

        conn.execute(
            "INSERT INTO trusted_devices (device_id, name, paired_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![vec![1u8; 32], "Cihaz", 0i64],
        )
        .unwrap();

        for (idx, text) in ["yarın görüşürüz", "dosyayı gönderdim", "İstanbul'dayım"]
            .iter()
            .enumerate()
        {
            conn.execute(
                "INSERT INTO messages (msg_id, conversation_id, device_id, direction,
                                       content_type, content, sent_at, status)
                 VALUES (?1, ?2, ?2, 'in', 'text', ?3, 0, 'read')",
                rusqlite::params![format!("m{idx}"), vec![1u8; 32], text],
            )
            .unwrap();
        }

        let hit: String = conn
            .query_row(
                "SELECT m.content FROM messages_fts f
                 JOIN messages m ON m.id = f.rowid
                 WHERE messages_fts MATCH ?1",
                ["gönderdim"],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(hit, "dosyayı gönderdim");

        // remove_diacritics=2 sayesinde şapkasız/aksansız arama da bulmalı.
        let count: i64 = conn
            .query_row(
                "SELECT count(*) FROM messages_fts WHERE messages_fts MATCH ?1",
                ["gonderdim"],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "diakritiksiz arama eşleşmeli");
    }

    /// Silme trigger'ı olmadan FTS indeksi mesajlardan sapar.
    #[test]
    fn fts_indeksi_silmede_senkron_kalir() {
        let mut conn = Connection::open_in_memory().unwrap();
        run(&mut conn).unwrap();

        conn.execute(
            "INSERT INTO trusted_devices (device_id, name, paired_at) VALUES (?1, 'C', 0)",
            rusqlite::params![vec![2u8; 32]],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO messages (msg_id, conversation_id, device_id, direction,
                                   content_type, content, sent_at, status)
             VALUES ('x', ?1, ?1, 'out', 'text', 'silinecek mesaj', 0, 'sent')",
            rusqlite::params![vec![2u8; 32]],
        )
        .unwrap();

        conn.execute("DELETE FROM messages WHERE msg_id = 'x'", [])
            .unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT count(*) FROM messages_fts WHERE messages_fts MATCH 'silinecek'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 0, "silinen mesaj FTS indeksinde kalmamalı");
    }
}
