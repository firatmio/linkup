//! Güvenilir cihazlara bağlantıyı ayakta tutan denetleyici (PLAN.md §2.14).
//!
//! Her güvenilir cihaz için bir gözetmen görev çalışır: adresi bulur, pinlenmiş
//! anahtarla bağlanır, bağlantı yaşadığı sürece gelen mesajları işler,
//! koptuğunda üstel gecikmeyle yeniden dener. Eşleştirme sonrası kullanıcıya bir daha kod
//! sorulmaz — kimlik doğrulaması TLS katmanında pinlenmiş anahtarla yapılır
//! (PLAN.md §2.2.1).

use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::{AppHandle, Emitter};

use super::backoff::Backoff;
use super::endpoint::{NetworkEndpoint, PeerConnection};
use crate::db::{devices, DbPool};
use crate::discovery::registry::Registry;
use crate::network::protocol::ControlMessage;
use crate::pairing::{self, PairingManager};

/// Cihaz zaten çevrimiçiyken gözetmenin ne sıklıkla durumu yoklayacağı.
/// Bağlantı canlılığı QUIC'in kendi keep-alive'ına bırakılmıştır.
const ONLINE_POLL_INTERVAL: Duration = Duration::from_secs(15);

pub const EVENT_PRESENCE: &str = "devices:presence";

/// Hangi cihazların o an bağlı olduğunu tutar.
#[derive(Default)]
pub struct Presence {
    online: Mutex<HashSet<[u8; 32]>>,
}

impl Presence {
    /// Cihazı çevrimiçi olarak işaretler. Zaten çevrimiçiyse `false` döner —
    /// çağıran bunu ikinci bağlantıyı kapatmak için kullanır.
    fn claim(&self, device_id: [u8; 32]) -> bool {
        self.online
            .lock()
            .expect("presence kilidi")
            .insert(device_id)
    }

    fn release(&self, device_id: &[u8; 32]) {
        self.online
            .lock()
            .expect("presence kilidi")
            .remove(device_id);
    }

    pub fn is_online(&self, device_id: &[u8; 32]) -> bool {
        self.online
            .lock()
            .expect("presence kilidi")
            .contains(device_id)
    }

    pub fn online_count(&self) -> usize {
        self.online.lock().expect("presence kilidi").len()
    }
}

pub struct ConnectionManager {
    app: AppHandle,
    db: DbPool,
    endpoint: Arc<NetworkEndpoint>,
    discovery: Arc<Mutex<Registry>>,
    pairing: Arc<PairingManager>,
    pub presence: Arc<Presence>,
}

impl ConnectionManager {
    pub fn new(
        app: AppHandle,
        db: DbPool,
        endpoint: Arc<NetworkEndpoint>,
        discovery: Arc<Mutex<Registry>>,
        pairing: Arc<PairingManager>,
    ) -> Arc<Self> {
        Arc::new(Self {
            app,
            db,
            endpoint,
            discovery,
            pairing,
            presence: Arc::new(Presence::default()),
        })
    }

    /// Kayıtlı tüm güvenilir cihazlar için gözetmen görevleri başlatır.
    pub fn start(self: &Arc<Self>) {
        let trusted = match self.db.get() {
            Ok(conn) => devices::list(&conn).unwrap_or_default(),
            Err(err) => {
                tracing::warn!(error = %err, "güvenilir cihaz listesi okunamadı");
                return;
            }
        };

        tracing::info!(count = trusted.len(), "güvenilir cihazlara bağlanılıyor");
        for device in trusted {
            self.supervise(device.device_id);
        }
    }

    /// Yeni eşleşen bir cihaz için gözetmen başlatır.
    pub fn supervise(self: &Arc<Self>, device_id: [u8; 32]) {
        let manager = Arc::clone(self);
        tauri::async_runtime::spawn(async move {
            manager.supervise_loop(device_id).await;
        });
    }

    async fn supervise_loop(self: Arc<Self>, device_id: [u8; 32]) {
        let mut backoff = Backoff::new();

        loop {
            // Cihaz unutulduysa gözetmen de sona ermeli.
            if !self.still_trusted(&device_id) {
                tracing::debug!("cihaz artık güvenilir değil, gözetmen sona eriyor");
                return;
            }

            if self.presence.is_online(&device_id) {
                // Karşı taraf bize bağlanmış olabilir; ikinci bir bağlantı açma.
                tokio::time::sleep(ONLINE_POLL_INTERVAL).await;
                continue;
            }

            match self.try_connect(&device_id).await {
                Some(connection) => {
                    backoff.reset();
                    self.hold(connection).await;
                }
                None => {
                    let delay = backoff.next_delay();
                    tracing::debug!(
                        attempt = backoff.attempts(),
                        delay_secs = delay.as_secs(),
                        "bağlanılamadı, yeniden denenecek"
                    );
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }

    async fn try_connect(&self, device_id: &[u8; 32]) -> Option<PeerConnection> {
        let address = self.resolve_address(device_id)?;

        // Anahtar pinlenerek bağlanılır: karşı taraf başka bir sertifika
        // sunarsa TLS el sıkışması başarısız olur (PLAN.md §2.2.1).
        match self.endpoint.connect(address, Some(*device_id)).await {
            Ok(connection) => Some(connection),
            Err(err) => {
                tracing::debug!(%address, error = %err, "güvenilir cihaza bağlanılamadı");
                None
            }
        }
    }

    /// Adres önce keşiften, yoksa son bilinen adresten alınır.
    ///
    /// Keşif önceliklidir: cihazın IP'si değişmiş olabilir ve mDNS güncel
    /// bilgiyi taşır. Son bilinen adres, mDNS'in çalışmadığı ağlar için
    /// (§10-K7) yedektir.
    fn resolve_address(&self, device_id: &[u8; 32]) -> Option<SocketAddr> {
        let discovered = {
            let registry = self.discovery.lock().expect("kayıt defteri kilidi");
            registry.get(device_id).and_then(|d| d.preferred_address())
        };
        if discovered.is_some() {
            return discovered;
        }

        let conn = self.db.get().ok()?;
        devices::get(&conn, device_id)
            .ok()
            .flatten()
            .and_then(|d| d.last_address)
            .and_then(|addr| addr.parse().ok())
    }

    fn still_trusted(&self, device_id: &[u8; 32]) -> bool {
        self.db
            .get()
            .ok()
            .and_then(|conn| devices::is_trusted(&conn, device_id).ok())
            .unwrap_or(false)
    }

    /// Kurulmuş bir bağlantıyı, kopana kadar ayakta tutar.
    ///
    /// Gelen bağlantılar için de kullanılır: kaynağı ne olursa olsun bağlantı
    /// yaşam döngüsü aynıdır.
    pub async fn hold(&self, mut connection: PeerConnection) {
        let device_id = connection.peer_device_id;

        if !self.presence.claim(device_id) {
            // Aynı cihaza iki bağlantı: iki taraf da aynı anda bağlanmış.
            // İkincisi kapatılır, veri akışı tek bağlantıda kalır.
            tracing::debug!("bu cihaza zaten bağlantı var, ikincisi kapatılıyor");
            connection.close();
            return;
        }

        let address = connection.remote_address();
        if let Ok(conn) = self.db.get() {
            let _ = devices::touch(&conn, &device_id, &address.to_string());
        }

        tracing::info!(
            peer = %connection.peer.device_name,
            %address,
            "güvenilir cihaza bağlandı"
        );
        self.emit_presence();

        // Bağlantı kopana kadar gelen mesajlar işlenir.
        //
        // Uygulama seviyesinde ayrıca heartbeat DÖNGÜSÜ kurulmuyor: QUIC'in
        // kendi keep-alive'ı (5 sn) bağlantıyı canlı tutuyor ve max_idle_timeout
        // (20 sn) ölü bağlantıyı zaten hataya çeviriyor (§2.2.2). İkinci bir
        // canlılık mekanizması yalnızca trafik ve karmaşıklık eklerdi.
        // `Heartbeat` mesajı RTT ölçmek için duruyor, istendiğinde kullanılır.
        loop {
            match connection.next_control_message().await {
                // Güvendiğimiz bir cihaz yeniden eşleşmek isteyebilir: karşı
                // taraf bizi unutmuşsa (veya eşleşme tek tarafta kalmışsa) tek
                // çıkış yolu budur. Reddedersek iki cihaz birbirine bir daha
                // asla bağlanamaz.
                Ok(ControlMessage::PairingRequest) => {
                    tracing::info!(
                        peer = %connection.peer.device_name,
                        "güvenilir cihaz yeniden eşleşme istedi"
                    );
                    if pairing::run(Arc::clone(&self.pairing), &mut connection, false)
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                // Faz 5'ten itibaren chat ve transfer mesajları burada işlenecek.
                Ok(message) => tracing::debug!(?message, "kontrol mesajı alındı"),
                Err(err) => {
                    tracing::info!(
                        peer = %connection.peer.device_name,
                        error = %err,
                        "bağlantı koptu"
                    );
                    break;
                }
            }
        }

        self.presence.release(&device_id);
        self.emit_presence();
    }

    fn emit_presence(&self) {
        let _ = self.app.emit(EVENT_PRESENCE, ());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ilk_talep_kazanir_ikincisi_reddedilir() {
        let presence = Presence::default();
        assert!(presence.claim([1; 32]), "ilk bağlantı kabul edilmeli");
        assert!(
            !presence.claim([1; 32]),
            "aynı cihaza ikinci bağlantı reddedilmeli"
        );
        assert!(presence.is_online(&[1; 32]));
        assert_eq!(presence.online_count(), 1);
    }

    #[test]
    fn birakilan_cihaz_yeniden_baglanabilir() {
        let presence = Presence::default();
        presence.claim([1; 32]);
        presence.release(&[1; 32]);

        assert!(!presence.is_online(&[1; 32]));
        assert_eq!(presence.online_count(), 0);
        assert!(
            presence.claim([1; 32]),
            "kopan bağlantı yeniden kurulabilmeli"
        );
    }

    #[test]
    fn farkli_cihazlar_birbirini_engellemez() {
        let presence = Presence::default();
        assert!(presence.claim([1; 32]));
        assert!(presence.claim([2; 32]));
        assert_eq!(presence.online_count(), 2);

        presence.release(&[1; 32]);
        assert!(!presence.is_online(&[1; 32]));
        assert!(presence.is_online(&[2; 32]));
    }
}
