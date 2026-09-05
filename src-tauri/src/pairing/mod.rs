//! Eşleştirme akışı (PLAN.md §2.5, §10-K2).
//!
//! Akış iki tarafta da simetriktir: her iki cihaz aynı 6 haneli kodu hesaplar,
//! kullanıcısına gösterir ve onay bekler. Eşleşme **yalnızca iki taraf da
//! onaylarsa** tamamlanır — tek taraflı onay yetmez.

pub mod sas;

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use data_encoding::{BASE32_NOPAD, HEXLOWER};
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::sync::oneshot;

use crate::db::{devices, DbPool};
use crate::network::endpoint::{NetworkError, PeerConnection};
use crate::network::protocol::ControlMessage;

/// Kullanıcının kodu karşılaştırıp karar vermesi için tanınan süre
/// (PLAN.md §2.5).
const DECISION_TIMEOUT: Duration = Duration::from_secs(90);

pub const EVENT_REQUESTED: &str = "pairing:requested";
pub const EVENT_FINISHED: &str = "pairing:finished";
pub const EVENT_DEVICES_CHANGED: &str = "devices:changed";

#[derive(Debug, thiserror::Error)]
pub enum PairingError {
    #[error("ağ: {0}")]
    Network(#[from] NetworkError),

    #[error("doğrulama kodu üretilemedi: {0}")]
    Exporter(String),

    #[error("kullanıcı reddetti")]
    RejectedLocally,

    #[error("karşı taraf reddetti")]
    RejectedByPeer,

    #[error("süre doldu")]
    TimedOut,

    #[error("veritabanı: {0}")]
    Db(String),
}

impl PairingError {
    /// Frontend'in çevireceği kod.
    pub fn code(&self) -> &'static str {
        match self {
            PairingError::Network(_) => "pairing.error.network",
            PairingError::Exporter(_) => "pairing.error.internal",
            PairingError::RejectedLocally => "pairing.error.rejectedLocally",
            PairingError::RejectedByPeer => "pairing.error.rejectedByPeer",
            PairingError::TimedOut => "pairing.error.timeout",
            PairingError::Db(_) => "pairing.error.internal",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PairingRequested {
    pub session_id: String,
    /// Base32 kodlu device_id.
    pub device_id: String,
    pub device_name: String,
    pub fingerprint: String,
    /// Kullanıcının karşı ekranla karşılaştıracağı 6 haneli kod.
    pub code: String,
    /// Eşleştirmeyi biz mi başlattık? UI metni buna göre değişir.
    pub initiated_by_us: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PairingFinished {
    pub session_id: String,
    pub ok: bool,
    /// Başarısızsa i18n anahtarı.
    pub reason: Option<String>,
}

/// Eşleştirme olaylarını kullanıcı arayüzüne taşıyan taraf.
///
/// Soyutlanmasının sebebi test edilebilirlik: eşleştirme uygulamanın en
/// güvenlik-kritik akışı (§2.5) ve yalnızca elle tıklayarak sınanması kabul
/// edilemez. Bu arayüz sayesinde akış, Tauri olmadan uçtan uca test edilebiliyor.
pub trait PairingNotifier: Send + Sync + 'static {
    fn requested(&self, event: PairingRequested);
    fn finished(&self, event: PairingFinished);
    fn devices_changed(&self);
}

/// Üretimdeki uygulama: olayları Tauri üzerinden frontend'e yayınlar.
pub struct TauriNotifier(pub AppHandle);

impl PairingNotifier for TauriNotifier {
    fn requested(&self, event: PairingRequested) {
        let _ = self.0.emit(EVENT_REQUESTED, event);
    }

    fn finished(&self, event: PairingFinished) {
        let _ = self.0.emit(EVENT_FINISHED, event);
    }

    fn devices_changed(&self) {
        let _ = self.0.emit(EVENT_DEVICES_CHANGED, ());
    }
}

/// Bekleyen eşleştirme oturumlarını tutar; `respond` komutu kullanıcının
/// kararını buradan akışa iletir.
pub struct PairingManager {
    notifier: Box<dyn PairingNotifier>,
    db: DbPool,
    pending: Mutex<HashMap<String, oneshot::Sender<bool>>>,
}

impl PairingManager {
    pub fn new(app: AppHandle, db: DbPool) -> Self {
        Self::with_notifier(Box::new(TauriNotifier(app)), db)
    }

    pub fn with_notifier(notifier: Box<dyn PairingNotifier>, db: DbPool) -> Self {
        Self {
            notifier,
            db,
            pending: Mutex::new(HashMap::new()),
        }
    }

    /// Kullanıcının kararını bekleyen akışa iletir.
    /// Oturum yoksa (süresi dolmuş olabilir) `false` döner.
    pub fn respond(&self, session_id: &str, accept: bool) -> bool {
        let sender = {
            let mut pending = self.pending.lock().expect("eşleştirme kilidi");
            pending.remove(session_id)
        };
        match sender {
            Some(sender) => sender.send(accept).is_ok(),
            None => {
                tracing::debug!(session_id, "yanıtlanan eşleştirme oturumu bulunamadı");
                false
            }
        }
    }

    fn register(&self, session_id: String) -> oneshot::Receiver<bool> {
        let (tx, rx) = oneshot::channel();
        self.pending
            .lock()
            .expect("eşleştirme kilidi")
            .insert(session_id, tx);
        rx
    }

    fn unregister(&self, session_id: &str) {
        self.pending
            .lock()
            .expect("eşleştirme kilidi")
            .remove(session_id);
    }

    pub fn trusted_devices(&self) -> Vec<devices::TrustedDevice> {
        let Ok(conn) = self.db.get() else {
            return Vec::new();
        };
        devices::list(&conn).unwrap_or_default()
    }

    pub fn is_trusted(&self, device_id: &[u8; 32]) -> bool {
        self.db
            .get()
            .ok()
            .and_then(|conn| devices::is_trusted(&conn, device_id).ok())
            .unwrap_or(false)
    }

    pub fn forget(&self, device_id: &[u8; 32]) -> bool {
        let forgotten = self
            .db
            .get()
            .ok()
            .and_then(|conn| devices::forget(&conn, device_id).ok())
            .unwrap_or(false);
        if forgotten {
            self.emit_devices_changed();
        }
        forgotten
    }

    pub fn emit_devices_changed(&self) {
        self.notifier.devices_changed();
    }
}

/// Eşleştirmeyi baştan sona yürütür.
///
/// `initiated_by_us` doğruysa `PairingRequest` bu taraftan gönderilir; aksi
/// hâlde istek zaten alınmış demektir.
pub async fn run(
    manager: Arc<PairingManager>,
    connection: &mut PeerConnection,
    initiated_by_us: bool,
) -> Result<(), PairingError> {
    if initiated_by_us {
        connection.send(&ControlMessage::PairingRequest).await?;
    }

    let code = compute_code(connection)?;
    let session_id = new_session_id();
    let peer_device_id = connection.peer_device_id;

    tracing::info!(
        peer = %connection.peer.device_name,
        initiated_by_us,
        "eşleştirme başladı, doğrulama kodu gösteriliyor"
    );

    let decision_rx = manager.register(session_id.clone());
    manager.notifier.requested(PairingRequested {
        session_id: session_id.clone(),
        device_id: BASE32_NOPAD.encode(&peer_device_id),
        device_name: connection.peer.device_name.clone(),
        fingerprint: crate::identity::format_fingerprint(&peer_device_id),
        // Kod loglanmaz (PLAN.md §2.14).
        code,
        initiated_by_us,
    });

    let result = exchange_decisions(&manager, connection, decision_rx).await;
    manager.unregister(&session_id);

    match &result {
        Ok(()) => {
            persist(&manager, connection)?;
            tracing::info!(peer = %connection.peer.device_name, "eşleştirme tamamlandı");
        }
        Err(err) => {
            tracing::info!(
                peer = %connection.peer.device_name,
                reason = err.code(),
                "eşleştirme tamamlanmadı"
            );
        }
    }

    manager.notifier.finished(PairingFinished {
        session_id,
        ok: result.is_ok(),
        reason: result.as_ref().err().map(|e| e.code().to_string()),
    });

    if result.is_ok() {
        manager.emit_devices_changed();
    }
    result
}

/// Kullanıcı kararı ile karşı tarafın kararını eşzamanlı bekler.
///
/// İkisini aynı anda beklemek şart: yalnızca kullanıcıyı beklersek, karşı
/// taraf hemen reddettiğinde bunu ancak biz cevap verdikten sonra görürüz ve
/// kullanıcı boşuna kod karşılaştırır.
async fn exchange_decisions(
    manager: &PairingManager,
    connection: &mut PeerConnection,
    decision_rx: oneshot::Receiver<bool>,
) -> Result<(), PairingError> {
    let deadline = tokio::time::Instant::now() + DECISION_TIMEOUT;
    let mut decision_rx = Some(decision_rx);
    let mut peer_accepted: Option<bool> = None;

    loop {
        if let Some(false) = peer_accepted {
            return Err(PairingError::RejectedByPeer);
        }
        if decision_rx.is_none() && peer_accepted == Some(true) {
            return Ok(());
        }

        tokio::select! {
            biased;

            _ = tokio::time::sleep_until(deadline) => {
                let _ = connection.send(&ControlMessage::PairingReject).await;
                return Err(PairingError::TimedOut);
            }

            user = async { decision_rx.as_mut().unwrap().await }, if decision_rx.is_some() => {
                decision_rx = None;
                let accepted = user.unwrap_or(false);
                connection
                    .send(if accepted {
                        &ControlMessage::PairingConfirm
                    } else {
                        &ControlMessage::PairingReject
                    })
                    .await?;
                if !accepted {
                    return Err(PairingError::RejectedLocally);
                }
            }

            message = connection.next_control_message(), if peer_accepted.is_none() => {
                match message? {
                    ControlMessage::PairingConfirm => peer_accepted = Some(true),
                    ControlMessage::PairingReject => peer_accepted = Some(false),
                    other => {
                        tracing::debug!(?other, "eşleştirme sırasında beklenmeyen mesaj");
                    }
                }
            }
        }

        // Kullanıcı reddettiyse yukarıda döndük; buraya yalnızca onay veya
        // bekleme durumunda gelinir.
        let _ = manager;
    }
}

fn compute_code(connection: &PeerConnection) -> Result<String, PairingError> {
    let mut exporter = [0u8; sas::EXPORTER_LEN];
    connection
        .connection()
        .export_keying_material(&mut exporter, sas::EXPORTER_LABEL, b"")
        .map_err(|_| PairingError::Exporter("TLS exporter kullanılamadı".to_string()))?;

    Ok(sas::compute(
        &connection.local_device_id,
        &connection.peer_device_id,
        &exporter,
    ))
}

fn persist(manager: &PairingManager, connection: &PeerConnection) -> Result<(), PairingError> {
    let conn = manager
        .db
        .get()
        .map_err(|e| PairingError::Db(e.to_string()))?;
    devices::upsert(
        &conn,
        &connection.peer_device_id,
        &connection.peer.device_name,
        Some(&connection.remote_address().to_string()),
    )
    .map_err(|e| PairingError::Db(e.to_string()))
}

fn new_session_id() -> String {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).expect("işletim sistemi entropisi");
    HEXLOWER.encode(&bytes)
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn oturum_kimlikleri_benzersiz() {
        let ids: std::collections::HashSet<_> = (0..100).map(|_| new_session_id()).collect();
        assert_eq!(ids.len(), 100);
        assert_eq!(new_session_id().len(), 32);
    }

    #[test]
    fn hata_kodlari_i18n_anahtari() {
        assert_eq!(
            PairingError::RejectedByPeer.code(),
            "pairing.error.rejectedByPeer"
        );
        assert_eq!(PairingError::TimedOut.code(), "pairing.error.timeout");
        // Dahili ayrıntılar kullanıcıya sızmamalı: iki farklı iç hata da
        // aynı genel koda düşer.
        assert_eq!(
            PairingError::Db("tablo yok".into()).code(),
            PairingError::Exporter("tls".into()).code()
        );
    }
}
