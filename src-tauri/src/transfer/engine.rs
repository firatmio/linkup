//! Dosya transferi motoru (PLAN.md §2.7).
//!
//! Akış tasarımı: dosya başına TEK yönlü, TEK bir QUIC akışı; veri sıralı
//! gider. Paralel chunk kullanılmıyor — tek bağlantıdaki tüm akışlar aynı
//! congestion controller'ı paylaştığı için throughput artmaz, yalnızca
//! karmaşıklık eklenir (§10-K4). Sıralı akışın yan faydası: resume durumu tek
//! bir byte offset'ine indirgenir, chunk bitmap'i gerekmez.
//!
//! Bütünlük yalnızca dosya sonunda blake3 ile sınanır; QUIC/TLS zaten her
//! baytı authenticated encryption ile koruduğu için chunk başına hash yalnızca
//! CPU yakardı (§10-K5).

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use quinn::{Connection, RecvStream};
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

use super::limiter::RateLimiter;
use super::paths;
use crate::db::transfers::{self, NewTransfer, TransferStatus};
use crate::db::{settings, DbPool};
use crate::error::{AppError, AppResult};
use crate::network::protocol::{read_frame, write_frame, ControlMessage, FileOffer, RejectReason};

/// Okuma/yazma parçası. Ağ çerçevesi değil, yalnızca bellek tamponu boyutu.
const CHUNK: usize = 256 * 1024;

/// İlerleme bu sıklıktan daha sık kaydedilmez: her parçada veritabanına
/// yazmak ve olay yayınlamak, transferin kendisinden pahalıya gelir.
const PROGRESS_INTERVAL: Duration = Duration::from_millis(500);

/// Alıcının, teklif edilen dosya için diskte istediği ek pay.
const FREE_SPACE_MARGIN: u64 = 32 * 1024 * 1024;

pub const EVENT_PROGRESS: &str = "transfer:progress";
pub const EVENT_CHANGED: &str = "transfer:changed";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressEvent {
    pub transfer_id: String,
    pub bytes_done: u64,
    pub total: u64,
    /// Anlık hız (bayt/sn); kalan süreyi arayüz hesaplar.
    pub bytes_per_second: u64,
}

/// Motorun ihtiyaç duyduğu paylaşılan kaynaklar.
#[derive(Clone)]
pub struct TransferContext {
    pub db: DbPool,
    pub app: AppHandle,
    /// Ayarlarda indirme klasörü boşsa kullanılan varsayılan.
    pub default_download_dir: PathBuf,
}

impl TransferContext {
    fn settings(&self) -> AppResult<settings::Settings> {
        let conn = self.db.get().map_err(pool_error)?;
        settings::load(&conn)
    }

    /// Kullanıcının seçtiği indirme klasörü; boşsa varsayılan.
    fn download_dir(&self) -> PathBuf {
        let configured = self
            .settings()
            .ok()
            .map(|s| s.download_dir)
            .filter(|dir| !dir.trim().is_empty());

        configured
            .map(PathBuf::from)
            .unwrap_or_else(|| self.default_download_dir.clone())
    }

    fn emit_changed(&self) {
        let _ = self.app.emit(EVENT_CHANGED, ());
    }
}

// ---------------------------------------------------------------- gönderen

/// Bir dosyayı göndermeye hazırlar: özetini alır, kaydını oluşturur ve
/// karşı tarafa yollanacak teklifi döndürür.
///
/// Özet hesaplama büyük dosyalarda saniyeler sürebilir; bu yüzden bloklayan
/// iş ayrı bir thread'e alınır.
pub async fn prepare_offer(
    ctx: &TransferContext,
    device_id: &[u8; 32],
    source: PathBuf,
) -> AppResult<(String, ControlMessage)> {
    let metadata = tokio::fs::metadata(&source).await?;
    if !metadata.is_file() {
        return Err(AppError::InvalidInput(
            "yalnızca dosya gönderilebilir".to_string(),
        ));
    }

    let file_name = source
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("dosya")
        .to_string();
    let size = metadata.len();

    let hash_path = source.clone();
    let hash = tokio::task::spawn_blocking(move || hash_file(&hash_path))
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("özet görevi: {e}")))??;

    let transfer_id = new_transfer_id();
    {
        let conn = ctx.db.get().map_err(pool_error)?;
        transfers::insert(
            &conn,
            NewTransfer {
                transfer_id: &transfer_id,
                device_id,
                incoming: false,
                file_name: &file_name,
                file_size: size,
                mime: None,
                expected_hash: &hash,
                part_path: source.to_str(),
                save_path: source.to_str(),
            },
        )?;
    }
    ctx.emit_changed();

    Ok((
        transfer_id.clone(),
        ControlMessage::FileOffer(FileOffer {
            transfer_id,
            name: file_name,
            size,
            mime: None,
            hash,
            is_resume: false,
        }),
    ))
}

/// Kabul edilen bir teklifin verisini gönderir.
pub async fn send_file(
    ctx: TransferContext,
    connection: Connection,
    transfer_id: String,
    start_offset: u64,
) {
    if let Err(err) = send_file_inner(&ctx, &connection, &transfer_id, start_offset).await {
        tracing::warn!(transfer_id, error = %err, "dosya gönderilemedi");
        if let Ok(conn) = ctx.db.get() {
            let _ = transfers::set_status(
                &conn,
                &transfer_id,
                TransferStatus::Failed,
                Some(&err.to_string()),
            );
        }
        ctx.emit_changed();
    }
}

async fn send_file_inner(
    ctx: &TransferContext,
    connection: &Connection,
    transfer_id: &str,
    start_offset: u64,
) -> AppResult<()> {
    let record = {
        let conn = ctx.db.get().map_err(pool_error)?;
        transfers::get(&conn, transfer_id)?
    }
    .ok_or_else(|| AppError::InvalidInput("transfer kaydı yok".to_string()))?;

    let source = record
        .save_path
        .clone()
        .ok_or_else(|| AppError::InvalidInput("kaynak dosya bilinmiyor".to_string()))?;

    let mut file = tokio::fs::File::open(&source).await?;
    file.seek(std::io::SeekFrom::Start(start_offset)).await?;

    let mut stream = connection
        .open_uni()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("akış açılamadı: {e}")))?;

    write_frame(
        &mut stream,
        &ControlMessage::TransferStreamHeader {
            transfer_id: transfer_id.to_string(),
            offset: start_offset,
        },
    )
    .await
    .map_err(wire_error)?;

    let speed_limit = ctx.settings().map(|s| s.speed_limit_bytes).unwrap_or(0);
    let mut limiter = RateLimiter::new(speed_limit);

    let mut buffer = vec![0u8; CHUNK];
    let mut sent = start_offset;
    let mut reporter = ProgressReporter::new(ctx, transfer_id, record.file_size as u64, sent);

    loop {
        let read = file.read(&mut buffer).await?;
        if read == 0 {
            break;
        }

        limiter.acquire(read as u64).await;
        stream.write_all(&buffer[..read]).await?;

        sent += read as u64;
        reporter.report(sent).await;
    }

    stream
        .finish()
        .map_err(|e| AppError::Internal(anyhow::anyhow!("akış kapatılamadı: {e}")))?;

    // Tamamlanma kararı ALICIYA ait: bütünlüğü o doğrular ve `FileComplete`
    // gönderir. Gönderen tarafta yalnızca ilerleme kaydedilir.
    reporter.flush(sent).await;
    tracing::info!(transfer_id, bytes = sent, "dosya gönderildi");
    Ok(())
}

// ------------------------------------------------------------------ alıcı

/// Gelen bir teklifi değerlendirir ve verilecek yanıtı döndürür.
///
/// Dosya adı burada güvenli hâle getirilir (PLAN.md §2.13.1) ve disk alanı
/// burada kontrol edilir (§2.13.2); bunlar reddedilebilir sebeplerdir,
/// transferin ortasında patlamamalıdır.
pub fn handle_offer(
    ctx: &TransferContext,
    device_id: &[u8; 32],
    offer: &FileOffer,
) -> ControlMessage {
    let reject = |reason: RejectReason| ControlMessage::FileReject {
        transfer_id: offer.transfer_id.clone(),
        reason,
    };

    let settings = match ctx.settings() {
        Ok(settings) => settings,
        Err(_) => return reject(RejectReason::Internal),
    };

    // Kabul politikası (PLAN.md §2.13.3). Eşleşmemiş cihazdan buraya zaten
    // gelinemez: eşleşmemiş bağlantılar yalnızca eşleştirme mesajı gönderebilir.
    let accepted = match settings.accept_policy.as_str() {
        "always" => false,
        "threshold" => offer.size <= settings.accept_size_threshold,
        // "trusted": güvenilir cihazdan gelen her dosya kabul edilir.
        _ => true,
    };
    if !accepted {
        return reject(RejectReason::TooLarge);
    }

    let download_dir = ctx.download_dir();
    if let Err(err) = std::fs::create_dir_all(&download_dir) {
        tracing::warn!(error = %err, "indirme klasörü oluşturulamadı");
        return reject(RejectReason::Internal);
    }

    if !has_free_space(&download_dir, offer.size) {
        tracing::info!(
            size = offer.size,
            "diskte yeterli alan yok, teklif reddedildi"
        );
        return reject(RejectReason::NoSpace);
    }

    // Var olan bir transferin devamı mı?
    let existing = ctx
        .db
        .get()
        .ok()
        .and_then(|conn| transfers::resume_info(&conn, &offer.transfer_id).ok())
        .flatten();

    if let Some(info) = existing {
        // Dosya değişmişse resume geçersizdir; baştan başlanır.
        if info.expected_hash == offer.hash && info.file_size == offer.size as i64 {
            let done = info
                .part_path
                .as_deref()
                .and_then(|path| std::fs::metadata(path).ok())
                .map(|meta| meta.len())
                .unwrap_or(0);

            tracing::info!(transfer_id = %offer.transfer_id, offset = done, "transfer kaldığı yerden");
            return ControlMessage::FileAccept {
                transfer_id: offer.transfer_id.clone(),
                start_offset: done,
            };
        }
        tracing::info!("dosya değişmiş, transfer baştan başlıyor");
    }

    let target = match paths::resolve_download_path(&download_dir, &offer.name) {
        Ok(path) => path,
        Err(err) => {
            tracing::warn!(name = %offer.name, error = %err, "hedef yol reddedildi");
            return reject(RejectReason::BadName);
        }
    };
    let part = with_part_extension(&target);

    let conn = match ctx.db.get() {
        Ok(conn) => conn,
        Err(_) => return reject(RejectReason::Internal),
    };

    if transfers::insert(
        &conn,
        NewTransfer {
            transfer_id: &offer.transfer_id,
            device_id,
            incoming: true,
            file_name: &offer.name,
            file_size: offer.size,
            mime: offer.mime.as_deref(),
            expected_hash: &offer.hash,
            part_path: part.to_str(),
            save_path: target.to_str(),
        },
    )
    .is_err()
    {
        return reject(RejectReason::Internal);
    }

    ctx.emit_changed();
    ControlMessage::FileAccept {
        transfer_id: offer.transfer_id.clone(),
        start_offset: 0,
    }
}

/// Gelen bir transfer akışını okur, `.part` dosyasına yazar, bütünlüğü
/// doğrular ve nihai adına taşır.
pub async fn receive_stream(ctx: TransferContext, connection: Connection, mut stream: RecvStream) {
    let header = match read_frame(&mut stream).await {
        Ok(ControlMessage::TransferStreamHeader {
            transfer_id,
            offset,
        }) => (transfer_id, offset),
        Ok(other) => {
            tracing::warn!(?other, "transfer akışının başında beklenmeyen çerçeve");
            return;
        }
        Err(err) => {
            tracing::debug!(error = %err, "transfer akışı okunamadı");
            return;
        }
    };

    let (transfer_id, offset) = header;
    match receive_stream_inner(&ctx, &mut stream, &transfer_id, offset).await {
        Ok(saved) => {
            tracing::info!(transfer_id, path = %saved.display(), "dosya alındı");
            send_completion(&connection, &transfer_id, true).await;
        }
        Err(err) => {
            tracing::warn!(transfer_id, error = %err, "dosya alınamadı");
            if let Ok(conn) = ctx.db.get() {
                let _ = transfers::set_status(
                    &conn,
                    &transfer_id,
                    TransferStatus::Failed,
                    Some(&err.to_string()),
                );
            }
            send_completion(&connection, &transfer_id, false).await;
        }
    }
    ctx.emit_changed();
}

async fn receive_stream_inner(
    ctx: &TransferContext,
    stream: &mut RecvStream,
    transfer_id: &str,
    offset: u64,
) -> AppResult<PathBuf> {
    let record = {
        let conn = ctx.db.get().map_err(pool_error)?;
        transfers::get(&conn, transfer_id)?
    }
    .ok_or_else(|| AppError::InvalidInput("bilinmeyen transfer".to_string()))?;

    let final_path = PathBuf::from(
        record
            .save_path
            .clone()
            .ok_or_else(|| AppError::InvalidInput("hedef yol yok".to_string()))?,
    );

    // Son savunma hattı: kayıt bozulmuş olsa bile indirme klasörünün dışına
    // yazılmamalı (PLAN.md §2.13.1 madde 4).
    let download_dir = ctx.download_dir();
    if !paths::is_within(&download_dir, &final_path) {
        return Err(AppError::InvalidInput(
            "hedef yol indirme klasörünün dışında".to_string(),
        ));
    }

    let part_path = with_part_extension(&final_path);
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(offset == 0)
        .open(&part_path)
        .await?;
    file.seek(std::io::SeekFrom::Start(offset)).await?;

    let total = record.file_size as u64;
    let mut written = offset;
    let mut reporter = ProgressReporter::new(ctx, transfer_id, total, written);
    let mut buffer = vec![0u8; CHUNK];

    while let Some(read) = stream
        .read(&mut buffer)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("akış okunamadı: {e}")))?
    {
        if read == 0 {
            continue;
        }
        file.write_all(&buffer[..read]).await?;
        written += read as u64;
        reporter.report(written).await;
    }

    file.flush().await?;
    drop(file);
    reporter.flush(written).await;

    if written != total {
        return Err(AppError::InvalidInput(format!(
            "eksik veri: {written}/{total} bayt"
        )));
    }

    // Bütünlük: resume sonrası dosyanın doğru birleştiğini doğrulayan tek şey.
    let verify_path = part_path.clone();
    let expected = {
        let conn = ctx.db.get().map_err(pool_error)?;
        transfers::resume_info(&conn, transfer_id)?
            .map(|info| info.expected_hash)
            .unwrap_or([0; 32])
    };
    let actual = tokio::task::spawn_blocking(move || hash_file(&verify_path))
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("özet görevi: {e}")))??;

    if actual != expected {
        let _ = tokio::fs::remove_file(&part_path).await;
        return Err(AppError::InvalidInput(
            "dosya bütünlüğü doğrulanamadı".to_string(),
        ));
    }

    // Ad çakışması yeniden çözülür: transfer sürerken aynı adla başka bir
    // dosya oluşmuş olabilir.
    let target = paths::unique_path(
        final_path.parent().unwrap_or(&download_dir),
        &paths::sanitize_file_name(&record.file_name),
    );
    tokio::fs::rename(&part_path, &target).await?;

    let conn = ctx.db.get().map_err(pool_error)?;
    transfers::set_paths(&conn, transfer_id, None, target.to_str())?;
    transfers::set_status(&conn, transfer_id, TransferStatus::Done, None)?;
    Ok(target)
}

async fn send_completion(connection: &Connection, transfer_id: &str, ok: bool) {
    // Sonuç, kontrol akışı yerine kısa ömürlü bir akıştan bildirilir:
    // kontrol akışının sahibi bağlantı döngüsüdür ve buradan yazılamaz.
    let Ok(mut stream) = connection.open_uni().await else {
        return;
    };
    let _ = write_frame(
        &mut stream,
        &ControlMessage::FileComplete {
            transfer_id: transfer_id.to_string(),
            ok,
        },
    )
    .await;
    let _ = stream.finish();
}

// ------------------------------------------------------------- yardımcılar

/// İlerlemeyi kısıtlı sıklıkta veritabanına yazar ve arayüze bildirir.
struct ProgressReporter<'a> {
    ctx: &'a TransferContext,
    transfer_id: &'a str,
    total: u64,
    last_report: Instant,
    last_bytes: u64,
}

impl<'a> ProgressReporter<'a> {
    fn new(ctx: &'a TransferContext, transfer_id: &'a str, total: u64, start: u64) -> Self {
        Self {
            ctx,
            transfer_id,
            total,
            last_report: Instant::now(),
            last_bytes: start,
        }
    }

    async fn report(&mut self, bytes: u64) {
        if self.last_report.elapsed() < PROGRESS_INTERVAL {
            return;
        }
        self.flush(bytes).await;
    }

    async fn flush(&mut self, bytes: u64) {
        let elapsed = self.last_report.elapsed().as_secs_f64().max(0.001);
        let speed = ((bytes.saturating_sub(self.last_bytes)) as f64 / elapsed) as u64;

        self.last_report = Instant::now();
        self.last_bytes = bytes;

        if let Ok(conn) = self.ctx.db.get() {
            let _ = transfers::update_progress(&conn, self.transfer_id, bytes);
        }
        let _ = self.ctx.app.emit(
            EVENT_PROGRESS,
            ProgressEvent {
                transfer_id: self.transfer_id.to_string(),
                bytes_done: bytes,
                total: self.total,
                bytes_per_second: speed,
            },
        );
    }
}

/// `rapor.pdf` → `rapor.pdf.part`
fn with_part_extension(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(".part");
    PathBuf::from(name)
}

fn has_free_space(directory: &Path, needed: u64) -> bool {
    match fs4::available_space(directory) {
        Ok(available) => available >= needed.saturating_add(FREE_SPACE_MARGIN),
        // Alan öğrenilemiyorsa transferi engellemek yerine denemek yeğdir;
        // yazma hatası zaten yakalanır.
        Err(err) => {
            tracing::debug!(error = %err, "boş disk alanı öğrenilemedi");
            true
        }
    }
}

fn hash_file(path: &Path) -> AppResult<[u8; 32]> {
    let mut hasher = blake3::Hasher::new();
    let mut file = std::fs::File::open(path)?;
    std::io::copy(&mut file, &mut hasher)?;
    Ok(*hasher.finalize().as_bytes())
}

fn new_transfer_id() -> String {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).expect("işletim sistemi entropisi");
    data_encoding::HEXLOWER.encode(&bytes)
}

fn pool_error(err: r2d2::Error) -> AppError {
    AppError::Internal(anyhow::anyhow!("veritabanı bağlantısı alınamadı: {err}"))
}

fn wire_error(err: crate::network::protocol::WireError) -> AppError {
    AppError::Internal(anyhow::anyhow!("çerçeve yazılamadı: {err}"))
}

/// Karşı taraf teklifi reddetti: transfer sonlandırılır.
pub fn mark_rejected(ctx: &TransferContext, transfer_id: &str, reason: RejectReason) {
    let detail = match reason {
        RejectReason::Declined => "karşı taraf reddetti",
        RejectReason::NoSpace => "karşı tarafta yeterli disk alanı yok",
        RejectReason::TooLarge => "dosya karşı tarafın boyut sınırının üstünde",
        RejectReason::BadName => "dosya adı kabul edilmedi",
        RejectReason::Internal => "karşı tarafta hata oluştu",
    };
    finish(ctx, transfer_id, TransferStatus::Failed, Some(detail));
}

/// Alıcı bütünlük sonucunu bildirdi. Gönderen taraftaki transferin
/// tamamlanma kararı buradan gelir: dosyanın doğru birleştiğini yalnızca
/// alıcı bilebilir.
pub fn mark_sender_complete(ctx: &TransferContext, transfer_id: &str, ok: bool) {
    if ok {
        finish(ctx, transfer_id, TransferStatus::Done, None);
    } else {
        finish(
            ctx,
            transfer_id,
            TransferStatus::Failed,
            Some("karşı taraf bütünlüğü doğrulayamadı"),
        );
    }
}

pub fn mark_cancelled(ctx: &TransferContext, transfer_id: &str) {
    finish(ctx, transfer_id, TransferStatus::Cancelled, None);
}

fn finish(ctx: &TransferContext, transfer_id: &str, status: TransferStatus, error: Option<&str>) {
    if let Ok(conn) = ctx.db.get() {
        let _ = transfers::set_status(&conn, transfer_id, status, error);
    }
    ctx.emit_changed();
}

/// Motor, bağlantı döngüsüyle paylaşılan bağlamı `Arc` ile taşır.
pub type SharedContext = Arc<TransferContext>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn part_uzantisi_eklenir() {
        assert_eq!(
            with_part_extension(Path::new("/indir/rapor.pdf")),
            PathBuf::from("/indir/rapor.pdf.part")
        );
        // Uzantısız dosyalarda da ad korunur.
        assert_eq!(
            with_part_extension(Path::new("/indir/notlar")),
            PathBuf::from("/indir/notlar.part")
        );
    }

    #[test]
    fn transfer_kimlikleri_benzersiz() {
        let ids: std::collections::HashSet<_> = (0..200).map(|_| new_transfer_id()).collect();
        assert_eq!(ids.len(), 200);
    }

    #[test]
    fn ozet_dosya_icerigine_bagli() {
        let dir = std::env::temp_dir().join(format!("linkup-hash-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let a = dir.join("a.bin");
        let b = dir.join("b.bin");
        std::fs::write(&a, b"merhaba").unwrap();
        std::fs::write(&b, b"merhaba").unwrap();

        assert_eq!(hash_file(&a).unwrap(), hash_file(&b).unwrap());

        std::fs::write(&b, b"merhabaa").unwrap();
        assert_ne!(hash_file(&a).unwrap(), hash_file(&b).unwrap());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn alan_ogrenilemezse_transfer_engellenmez() {
        // Var olmayan bir yol için alan öğrenilemez; yine de denenmelidir.
        assert!(has_free_space(Path::new("/olmayan/dizin/xyz"), 1));
    }
}
