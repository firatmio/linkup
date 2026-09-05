//! Gelen dosya onayı (PLAN.md §2.13.3).
//!
//! Karşı taraf güvenilir olsa bile gönderdiği her dosya sessizce diske
//! yazılmamalı: kullanıcı ne aldığını bilmeli. Bu modül, teklifi kullanıcıya
//! sorup kararını bekleyen kısmı yönetir.
//!
//! Bekleme bağlantı döngüsünde YAPILMAZ — döngü o sırada başka hiçbir mesajı
//! işleyemezdi. Karar ayrı bir görevde beklenir, yanıt giden kuyruğa yazılır.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use tokio::sync::oneshot;

/// Kullanıcı bu süre içinde karar vermezse teklif reddedilir. Karşı tarafı
/// süresiz bekletmek, bağlantıyı ve göndericinin dosyasını rehin tutmak olur.
pub const DECISION_TIMEOUT: Duration = Duration::from_secs(60);

pub const EVENT_REQUESTED: &str = "transfer:requested";
pub const EVENT_RESOLVED: &str = "transfer:resolved";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalRequest {
    pub transfer_id: String,
    /// Base32 kodlu gönderen cihaz kimliği.
    pub device_id: String,
    pub device_name: String,
    pub file_name: String,
    pub file_size: u64,
}

#[derive(Default)]
pub struct ApprovalManager {
    pending: Mutex<HashMap<String, oneshot::Sender<bool>>>,
}

impl ApprovalManager {
    /// Kullanıcının kararını bekleyen akışa iletir.
    /// Oturum yoksa (süresi dolmuş olabilir) `false` döner.
    pub fn respond(&self, transfer_id: &str, accept: bool) -> bool {
        let sender = self
            .pending
            .lock()
            .expect("onay kilidi")
            .remove(transfer_id);

        match sender {
            Some(sender) => sender.send(accept).is_ok(),
            None => {
                tracing::debug!(transfer_id, "yanıtlanan onay isteği bulunamadı");
                false
            }
        }
    }

    pub fn register(&self, transfer_id: &str) -> oneshot::Receiver<bool> {
        let (tx, rx) = oneshot::channel();
        self.pending
            .lock()
            .expect("onay kilidi")
            .insert(transfer_id.to_string(), tx);
        rx
    }

    pub fn cancel(&self, transfer_id: &str) {
        self.pending
            .lock()
            .expect("onay kilidi")
            .remove(transfer_id);
    }

    /// Yalnızca testlerde kullanılır: bekleyen kayıtların temizlendiğini
    /// doğrulamak için.
    #[cfg(test)]
    pub fn pending_count(&self) -> usize {
        self.pending.lock().expect("onay kilidi").len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn kullanici_karari_bekleyen_akisa_ulasir() {
        let manager = ApprovalManager::default();
        let rx = manager.register("t1");
        assert_eq!(manager.pending_count(), 1);

        assert!(manager.respond("t1", true));
        assert!(rx.await.unwrap(), "onay kararı iletilmeli");
        assert_eq!(manager.pending_count(), 0, "yanıtlanan kayıt temizlenmeli");
    }

    #[tokio::test]
    async fn red_karari_da_iletilir() {
        let manager = ApprovalManager::default();
        let rx = manager.register("t1");

        assert!(manager.respond("t1", false));
        assert!(!rx.await.unwrap(), "red kararı iletilmeli");
    }

    #[test]
    fn bilinmeyen_isteğe_yanit_false_doner() {
        let manager = ApprovalManager::default();
        assert!(!manager.respond("yok", true));
    }

    /// Aynı isteğe iki kez yanıt gelirse ikincisi yok sayılmalı: kullanıcı
    /// çift tıklamış olabilir, karşı tarafa iki yanıt gitmemeli.
    #[tokio::test]
    async fn ikinci_yanit_yok_sayilir() {
        let manager = ApprovalManager::default();
        let rx = manager.register("t1");

        assert!(manager.respond("t1", true));
        assert!(!manager.respond("t1", false));
        assert!(rx.await.unwrap(), "onay kararı iletilmeli");
    }

    #[tokio::test]
    async fn iptal_edilen_istek_yanitlanamaz() {
        let manager = ApprovalManager::default();
        let _rx = manager.register("t1");

        manager.cancel("t1");
        assert!(!manager.respond("t1", true));
        assert_eq!(manager.pending_count(), 0);
    }
}
