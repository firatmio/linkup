//! Dosya transferi kayıtları (PLAN.md §2.7.2, §2.12).
//!
//! Kesintide devam edebilmenin (resume) tek dayanağı burasıdır: her transfer
//! için ne kadar baytın yazıldığı kalıcı olarak tutulur. Akış sıralı olduğu
//! için durum tek bir offset'e indirgenir — chunk bitmap'i gerekmez (§10-K4).

use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;

use crate::error::AppResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferStatus {
    /// Teklif gönderildi/alındı, henüz veri akmıyor.
    Pending,
    Active,
    Paused,
    Done,
    Failed,
    Cancelled,
}

impl TransferStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            TransferStatus::Pending => "pending",
            TransferStatus::Active => "active",
            TransferStatus::Paused => "paused",
            TransferStatus::Done => "done",
            TransferStatus::Failed => "failed",
            TransferStatus::Cancelled => "cancelled",
        }
    }

    /// Sona ermiş bir transfer yeniden başlatılmadan ilerlemez.
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            TransferStatus::Done | TransferStatus::Failed | TransferStatus::Cancelled
        )
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Transfer {
    pub transfer_id: String,
    /// Base32 kodlu karşı cihaz kimliği.
    pub device_id: String,
    /// "in" | "out"
    pub direction: String,
    pub file_name: String,
    pub file_size: i64,
    pub mime: Option<String>,
    /// Nihai yol (tamamlanmışsa) veya hedef yol.
    pub save_path: Option<String>,
    pub bytes_done: i64,
    pub status: String,
    pub error: Option<String>,
    pub started_at: i64,
    pub completed_at: Option<i64>,
}

pub struct NewTransfer<'a> {
    pub transfer_id: &'a str,
    pub device_id: &'a [u8; 32],
    pub incoming: bool,
    pub file_name: &'a str,
    pub file_size: u64,
    pub mime: Option<&'a str>,
    pub expected_hash: &'a [u8; 32],
    /// Gelen transferlerde `.part` dosyası, giden transferlerde kaynak dosya.
    pub part_path: Option<&'a str>,
    pub save_path: Option<&'a str>,
}

pub fn insert(conn: &Connection, transfer: NewTransfer<'_>) -> AppResult<()> {
    conn.execute(
        "INSERT INTO transfers (transfer_id, device_id, direction, file_name, file_size,
                                mime, expected_hash, save_path, part_path, bytes_done,
                                status, started_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 0, ?11, ?10)
         ON CONFLICT(transfer_id) DO UPDATE SET
             file_name = excluded.file_name,
             file_size = excluded.file_size,
             status = excluded.status",
        rusqlite::params![
            transfer.transfer_id,
            transfer.device_id.as_slice(),
            if transfer.incoming { "in" } else { "out" },
            transfer.file_name,
            transfer.file_size as i64,
            transfer.mime,
            transfer.expected_hash.as_slice(),
            transfer.save_path,
            transfer.part_path,
            crate::db::devices::now(),
            TransferStatus::Pending.as_str(),
        ],
    )?;
    Ok(())
}

/// İlerlemeyi kaydeder. Sık çağrılır (≈500 ms), bu yüzden tek satırlık ve
/// indeksli bir güncelleme.
pub fn update_progress(conn: &Connection, transfer_id: &str, bytes_done: u64) -> AppResult<()> {
    conn.execute(
        "UPDATE transfers SET bytes_done = ?2, status = ?3 WHERE transfer_id = ?1",
        rusqlite::params![
            transfer_id,
            bytes_done as i64,
            TransferStatus::Active.as_str()
        ],
    )?;
    Ok(())
}

pub fn set_status(
    conn: &Connection,
    transfer_id: &str,
    status: TransferStatus,
    error: Option<&str>,
) -> AppResult<()> {
    conn.execute(
        "UPDATE transfers SET status = ?2, error = ?3,
             completed_at = CASE WHEN ?4 THEN ?5 ELSE completed_at END
         WHERE transfer_id = ?1",
        rusqlite::params![
            transfer_id,
            status.as_str(),
            error,
            status.is_terminal(),
            crate::db::devices::now(),
        ],
    )?;
    Ok(())
}

pub fn set_paths(
    conn: &Connection,
    transfer_id: &str,
    part_path: Option<&str>,
    save_path: Option<&str>,
) -> AppResult<()> {
    conn.execute(
        "UPDATE transfers SET
             part_path = COALESCE(?2, part_path),
             save_path = COALESCE(?3, save_path)
         WHERE transfer_id = ?1",
        rusqlite::params![transfer_id, part_path, save_path],
    )?;
    Ok(())
}

/// Resume için gereken kayıt: beklenen özet, boyut ve `.part` yolu.
#[derive(Debug, Clone)]
pub struct ResumeInfo {
    pub expected_hash: [u8; 32],
    pub file_size: i64,
    pub part_path: Option<String>,
}

pub fn resume_info(conn: &Connection, transfer_id: &str) -> AppResult<Option<ResumeInfo>> {
    let mut stmt = conn.prepare(
        "SELECT expected_hash, file_size, part_path FROM transfers WHERE transfer_id = ?1",
    )?;
    Ok(stmt
        .query_row([transfer_id], |row| {
            let hash: Vec<u8> = row.get(0)?;
            Ok(ResumeInfo {
                expected_hash: hash.try_into().unwrap_or([0; 32]),
                file_size: row.get(1)?,
                part_path: row.get(2)?,
            })
        })
        .optional()?)
}

pub fn get(conn: &Connection, transfer_id: &str) -> AppResult<Option<Transfer>> {
    let mut stmt = conn.prepare(&format!("{SELECT_COLUMNS} WHERE transfer_id = ?1"))?;
    Ok(stmt.query_row([transfer_id], row_to_transfer).optional()?)
}

/// Alınan dosyaların geçmişi (PLAN.md §3.2 "Gelen Dosyalar").
pub fn list_incoming(conn: &Connection, limit: u32) -> AppResult<Vec<Transfer>> {
    let mut stmt = conn.prepare(&format!(
        "{SELECT_COLUMNS} WHERE direction = 'in' ORDER BY started_at DESC LIMIT ?1"
    ))?;
    let rows = stmt.query_map([limit], row_to_transfer)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// Sürmekte olan transferler — arayüzdeki ilerleme paneli için.
pub fn list_active(conn: &Connection) -> AppResult<Vec<Transfer>> {
    let mut stmt = conn.prepare(&format!(
        "{SELECT_COLUMNS} WHERE status IN ('pending', 'active', 'paused') ORDER BY started_at"
    ))?;
    let rows = stmt.query_map([], row_to_transfer)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// Bir cihazla olan yarım kalan transferleri sonlandırır ve kimliklerini
/// döndürür.
///
/// Bağlantı koptuğunda çağrılır. Bir zamanlar bunlar "duraklatıldı" olarak
/// işaretleniyordu; yanlıştı. Duraklatılmış bir transfer devam ettirilebilir
/// olmalıdır, oysa yeniden bağlanınca kimse teklifi tekrar göndermiyor
/// (resume Faz 11'e ertelendi). Sonuç: listede sonsuza kadar duran, çoğu
/// zaman %100 dolu görünen hayalet satırlar. `.part` dosyası ve `bytes_done`
/// korunuyor — resume geldiğinde bu kayıtlar hâlâ kaldığı yeri biliyor.
pub fn fail_for_device(conn: &Connection, device_id: &[u8; 32]) -> AppResult<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT transfer_id FROM transfers
         WHERE device_id = ?1 AND status IN (?2, ?3)",
    )?;
    let ids: Vec<String> = stmt
        .query_map(
            rusqlite::params![
                device_id.as_slice(),
                TransferStatus::Active.as_str(),
                TransferStatus::Pending.as_str(),
            ],
            |row| row.get(0),
        )?
        .collect::<Result<_, _>>()?;

    if !ids.is_empty() {
        conn.execute(
            "UPDATE transfers SET status = ?1, error = ?5, completed_at = ?6
             WHERE device_id = ?2 AND status IN (?3, ?4)",
            rusqlite::params![
                TransferStatus::Failed.as_str(),
                device_id.as_slice(),
                TransferStatus::Active.as_str(),
                TransferStatus::Pending.as_str(),
                "bağlantı kesildi",
                crate::db::devices::now(),
            ],
        )?;
    }
    Ok(ids)
}

/// Uygulama kapanınca yarım kalan transferler `active` durumunda donar.
///
/// Açılışta bunlar BAŞARISIZ sayılır, duraklatılmış değil. Sebebi dürüstlük:
/// duraklatılmış bir transfer devam ettirilebilir olmalı, ama yeniden başlatma
/// sonrası kimse teklifi tekrar göndermiyor — kullanıcıya "duraklatıldı"
/// demek, hiç gelmeyecek bir devamı beklettirmek olurdu.
pub fn fail_stale(conn: &Connection) -> AppResult<usize> {
    Ok(conn.execute(
        "UPDATE transfers SET status = ?1, error = ?5, completed_at = ?6
         WHERE status IN (?2, ?3, ?4)",
        rusqlite::params![
            TransferStatus::Failed.as_str(),
            TransferStatus::Active.as_str(),
            TransferStatus::Pending.as_str(),
            // Eski sürümlerin bıraktığı "duraklatıldı" kayıtları da burada
            // temizlenir; onları da devam ettirecek bir mekanizma yok.
            TransferStatus::Paused.as_str(),
            "uygulama kapandığı için yarıda kaldı",
            crate::db::devices::now(),
        ],
    )?)
}

/// Sonlanmış kayıtları listeden temizler. Dosyalar silinmez.
pub fn clear_finished(conn: &Connection) -> AppResult<usize> {
    Ok(conn.execute(
        "DELETE FROM transfers WHERE status IN (?1, ?2, ?3)",
        rusqlite::params![
            TransferStatus::Done.as_str(),
            TransferStatus::Failed.as_str(),
            TransferStatus::Cancelled.as_str(),
        ],
    )?)
}

const SELECT_COLUMNS: &str = "SELECT transfer_id, device_id, direction, file_name, file_size,
            mime, save_path, bytes_done, status, error, started_at, completed_at
     FROM transfers";

fn row_to_transfer(row: &rusqlite::Row<'_>) -> rusqlite::Result<Transfer> {
    let device: Vec<u8> = row.get(1)?;
    Ok(Transfer {
        transfer_id: row.get(0)?,
        device_id: data_encoding::BASE32_NOPAD.encode(&device),
        direction: row.get(2)?,
        file_name: row.get(3)?,
        file_size: row.get(4)?,
        mime: row.get(5)?,
        save_path: row.get(6)?,
        bytes_done: row.get(7)?,
        status: row.get(8)?,
        error: row.get(9)?,
        started_at: row.get(10)?,
        completed_at: row.get(11)?,
    })
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

    fn new(id: &str, incoming: bool) -> NewTransfer<'static> {
        NewTransfer {
            transfer_id: Box::leak(id.to_string().into_boxed_str()),
            device_id: &DEVICE,
            incoming,
            file_name: "rapor.pdf",
            file_size: 1000,
            mime: Some("application/pdf"),
            expected_hash: &[7; 32],
            part_path: Some("C:/tmp/rapor.pdf.part"),
            save_path: Some("C:/indir/rapor.pdf"),
        }
    }

    #[test]
    fn transfer_kaydedilir_ve_okunur() {
        let pool = setup();
        let conn = pool.get().unwrap();

        insert(&conn, new("t1", true)).unwrap();
        let transfer = get(&conn, "t1").unwrap().unwrap();

        assert_eq!(transfer.file_name, "rapor.pdf");
        assert_eq!(transfer.direction, "in");
        assert_eq!(transfer.status, "pending");
        assert_eq!(transfer.bytes_done, 0);
    }

    #[test]
    fn ilerleme_kaydedilir() {
        let pool = setup();
        let conn = pool.get().unwrap();
        insert(&conn, new("t1", true)).unwrap();

        update_progress(&conn, "t1", 512).unwrap();
        let transfer = get(&conn, "t1").unwrap().unwrap();

        assert_eq!(transfer.bytes_done, 512);
        assert_eq!(
            transfer.status, "active",
            "ilerleme kaydı transferi aktif yapar"
        );
    }

    /// Resume'un dayanağı: kesintiden sonra yazılan bayt sayısı ve beklenen
    /// özet kalıcı olmalı.
    #[test]
    fn resume_bilgisi_kesintiden_sonra_okunabilir() {
        let pool = setup();
        {
            let conn = pool.get().unwrap();
            insert(&conn, new("t1", true)).unwrap();
            update_progress(&conn, "t1", 400).unwrap();
        }

        // Yeni bağlantı, yeni sorgu — uygulama yeniden başlamış gibi.
        let conn = pool.get().unwrap();
        let info = resume_info(&conn, "t1").unwrap().unwrap();

        assert_eq!(info.expected_hash, [7; 32]);
        assert_eq!(info.file_size, 1000);
        assert_eq!(info.part_path.as_deref(), Some("C:/tmp/rapor.pdf.part"));
        assert_eq!(get(&conn, "t1").unwrap().unwrap().bytes_done, 400);
    }

    #[test]
    fn sonlanan_transfer_tamamlanma_zamani_alir() {
        let pool = setup();
        let conn = pool.get().unwrap();
        insert(&conn, new("t1", true)).unwrap();

        set_status(&conn, "t1", TransferStatus::Done, None).unwrap();
        let done = get(&conn, "t1").unwrap().unwrap();
        assert_eq!(done.status, "done");
        assert!(done.completed_at.is_some());

        // Sonlanmamış durumlar tamamlanma zamanı yazmamalı.
        insert(&conn, new("t2", true)).unwrap();
        set_status(&conn, "t2", TransferStatus::Active, None).unwrap();
        assert!(get(&conn, "t2").unwrap().unwrap().completed_at.is_none());
    }

    #[test]
    fn hata_mesaji_saklanir() {
        let pool = setup();
        let conn = pool.get().unwrap();
        insert(&conn, new("t1", true)).unwrap();

        set_status(&conn, "t1", TransferStatus::Failed, Some("disk dolu")).unwrap();
        assert_eq!(
            get(&conn, "t1").unwrap().unwrap().error.as_deref(),
            Some("disk dolu")
        );
    }

    #[test]
    fn yarim_kalan_transferler_acilista_basarisiz_olur() {
        let pool = setup();
        let conn = pool.get().unwrap();

        insert(&conn, new("t1", true)).unwrap();
        update_progress(&conn, "t1", 100).unwrap();
        insert(&conn, new("t2", true)).unwrap();
        set_status(&conn, "t2", TransferStatus::Done, None).unwrap();
        // Eski sürümlerden kalma "duraklatıldı" kaydı da temizlenmeli.
        insert(&conn, new("t3", true)).unwrap();
        set_status(&conn, "t3", TransferStatus::Paused, None).unwrap();

        assert_eq!(fail_stale(&conn).unwrap(), 2);
        assert_eq!(get(&conn, "t3").unwrap().unwrap().status, "failed");

        let stale = get(&conn, "t1").unwrap().unwrap();
        assert_eq!(stale.status, "failed");
        assert!(stale.error.is_some(), "sebebi kullanıcıya söylenmeli");
        assert_eq!(
            get(&conn, "t2").unwrap().unwrap().status,
            "done",
            "tamamlanmış transfere dokunulmamalı"
        );
    }

    #[test]
    fn sonlanmis_kayitlar_temizlenir() {
        let pool = setup();
        let conn = pool.get().unwrap();

        insert(&conn, new("t1", true)).unwrap();
        set_status(&conn, "t1", TransferStatus::Done, None).unwrap();
        insert(&conn, new("t2", true)).unwrap();
        set_status(&conn, "t2", TransferStatus::Failed, Some("hata")).unwrap();
        insert(&conn, new("t3", true)).unwrap();
        update_progress(&conn, "t3", 10).unwrap();

        assert_eq!(clear_finished(&conn).unwrap(), 2);
        assert!(get(&conn, "t1").unwrap().is_none());
        assert!(
            get(&conn, "t3").unwrap().is_some(),
            "süren aktarım silinmemeli"
        );
    }

    /// Bağlantı koptuğunda o cihazın yarım transferleri sonlanmalı;
    /// başka cihazınkilere dokunulmamalı.
    #[test]
    fn kopan_baglantinin_transferleri_sonlandirilir() {
        let pool = setup();
        let conn = pool.get().unwrap();
        devices::upsert(&conn, &[2; 32], "Diğer", None).unwrap();

        insert(&conn, new("t1", true)).unwrap();
        update_progress(&conn, "t1", 50).unwrap();
        insert(&conn, new("t2", true)).unwrap();
        set_status(&conn, "t2", TransferStatus::Done, None).unwrap();

        let mut other = new("t3", true);
        other.device_id = &[2; 32];
        insert(&conn, other).unwrap();
        update_progress(&conn, "t3", 10).unwrap();

        let ended = fail_for_device(&conn, &DEVICE).unwrap();
        assert_eq!(ended, ["t1"]);

        let broken = get(&conn, "t1").unwrap().unwrap();
        assert_eq!(broken.status, "failed");
        assert!(broken.error.is_some(), "sebebi kullanıcıya söylenmeli");
        assert_eq!(get(&conn, "t2").unwrap().unwrap().status, "done");
        assert_eq!(
            get(&conn, "t3").unwrap().unwrap().status,
            "active",
            "başka cihazın transferine dokunulmamalı"
        );
    }

    #[test]
    fn gelen_ve_aktif_listeleri_ayrisir() {
        let pool = setup();
        let conn = pool.get().unwrap();

        insert(&conn, new("in1", true)).unwrap();
        insert(&conn, new("out1", false)).unwrap();
        set_status(&conn, "in1", TransferStatus::Done, None).unwrap();

        let incoming = list_incoming(&conn, 50).unwrap();
        assert_eq!(incoming.len(), 1);
        assert_eq!(incoming[0].transfer_id, "in1");

        let active = list_active(&conn).unwrap();
        assert_eq!(active.len(), 1, "tamamlanan aktif listesinde olmamalı");
        assert_eq!(active[0].transfer_id, "out1");
    }

    #[test]
    fn sonlanma_kontrolu() {
        assert!(TransferStatus::Done.is_terminal());
        assert!(TransferStatus::Failed.is_terminal());
        assert!(TransferStatus::Cancelled.is_terminal());
        assert!(!TransferStatus::Active.is_terminal());
        assert!(!TransferStatus::Paused.is_terminal());
        assert!(!TransferStatus::Pending.is_terminal());
    }
}
