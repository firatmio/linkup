//! Eşleştirmenin uçtan uca testleri (PLAN.md §2.5, §6).
//!
//! İki gerçek `NetworkEndpoint` loopback üzerinde bağlanır, gerçek QUIC/TLS
//! el sıkışması yapılır ve eşleştirme akışı baştan sona koşar. Özellikle
//! önemli olan: doğrulama kodu, gerçek TLS oturumunun exporter'ından türetilir
//! — yani channel binding burada taklit değil, gerçekten sınanıyor.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use ed25519_dalek::SigningKey;

use super::{PairingError, PairingFinished, PairingManager, PairingNotifier, PairingRequested};
use crate::db;
use crate::network::endpoint::NetworkEndpoint;

/// Kullanıcı yerine geçen bildirim alıcısı: isteği kaydeder ve önceden
/// belirlenmiş kararı verir.
struct ScriptedUser {
    accept: bool,
    /// Kararı vermeden önce beklenecek süre — yarış durumlarını sınamak için.
    delay: Duration,
    requests: Arc<Mutex<Vec<PairingRequested>>>,
    results: Arc<Mutex<Vec<PairingFinished>>>,
    responder: Arc<Mutex<Option<Arc<PairingManager>>>>,
}

impl ScriptedUser {
    fn new(accept: bool) -> (Self, Harness) {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let results = Arc::new(Mutex::new(Vec::new()));
        let responder = Arc::new(Mutex::new(None));
        (
            Self {
                accept,
                delay: Duration::from_millis(20),
                requests: Arc::clone(&requests),
                results: Arc::clone(&results),
                responder: Arc::clone(&responder),
            },
            Harness {
                requests,
                results,
                responder,
            },
        )
    }
}

/// Testin gözlem penceresi.
struct Harness {
    requests: Arc<Mutex<Vec<PairingRequested>>>,
    results: Arc<Mutex<Vec<PairingFinished>>>,
    responder: Arc<Mutex<Option<Arc<PairingManager>>>>,
}

impl Harness {
    fn code(&self) -> Option<String> {
        self.requests
            .lock()
            .unwrap()
            .first()
            .map(|r| r.code.clone())
    }

    fn finished(&self) -> Option<PairingFinished> {
        self.results.lock().unwrap().first().cloned()
    }

    fn attach(&self, manager: Arc<PairingManager>) {
        *self.responder.lock().unwrap() = Some(manager);
    }
}

impl PairingNotifier for ScriptedUser {
    fn requested(&self, event: PairingRequested) {
        self.requests.lock().unwrap().push(event.clone());

        let manager = self.responder.lock().unwrap().clone();
        let accept = self.accept;
        let delay = self.delay;
        tokio::spawn(async move {
            tokio::time::sleep(delay).await;
            if let Some(manager) = manager {
                manager.respond(&event.session_id, accept);
            }
        });
    }

    fn finished(&self, event: PairingFinished) {
        self.results.lock().unwrap().push(event);
    }

    fn devices_changed(&self) {}
}

fn endpoint(seed: u8, name: &str) -> NetworkEndpoint {
    NetworkEndpoint::bind(
        &SigningKey::from_bytes(&[seed; 32]),
        name.to_string(),
        "127.0.0.1:0".parse().unwrap(),
    )
    .unwrap()
}

fn manager(accept: bool) -> (Arc<PairingManager>, Harness, db::DbPool) {
    let pool = db::open_in_memory().unwrap();
    let (user, harness) = ScriptedUser::new(accept);
    let manager = Arc::new(PairingManager::with_notifier(Box::new(user), pool.clone()));
    harness.attach(Arc::clone(&manager));
    (manager, harness, pool)
}

/// İki taraf da onaylarsa: aynı kod görünür, iki tarafta da cihaz güvenilir olur.
#[tokio::test]
async fn iki_taraf_onaylarsa_eslesme_tamamlanir() {
    let a = endpoint(11, "Cihaz A");
    let b = endpoint(12, "Cihaz B");
    let addr = b.local_addr().unwrap();

    let (manager_a, harness_a, db_a) = manager(true);
    let (manager_b, harness_b, db_b) = manager(true);

    let (client, server) = tokio::join!(a.connect(addr, None), b.accept());
    let mut client = client.unwrap().into_parts();
    let mut server = server.unwrap().unwrap().into_parts();

    let (result_a, result_b) = tokio::join!(
        super::run(Arc::clone(&manager_a), &mut client, true),
        super::run(Arc::clone(&manager_b), &mut server, false),
    );

    assert!(result_a.is_ok(), "başlatan taraf: {result_a:?}");
    assert!(result_b.is_ok(), "kabul eden taraf: {result_b:?}");

    // Asıl mesele: iki cihaz AYNI kodu görmeli. Kod gerçek TLS oturumunun
    // exporter'ından türediği için bu, channel binding'in çalıştığının kanıtı.
    let code_a = harness_a.code().expect("başlatan tarafa kod gösterilmeli");
    let code_b = harness_b
        .code()
        .expect("kabul eden tarafa kod gösterilmeli");
    assert_eq!(code_a, code_b, "iki tarafta aynı kod görünmeli");
    assert_eq!(code_a.len(), 6);

    // Her iki taraf da karşısını güvenilir olarak kaydetmeli.
    assert!(db::devices::is_trusted(&db_a.get().unwrap(), &b.device_id()).unwrap());
    assert!(db::devices::is_trusted(&db_b.get().unwrap(), &a.device_id()).unwrap());

    assert!(harness_a.finished().unwrap().ok);
    assert!(harness_b.finished().unwrap().ok);
}

/// Tek taraflı onay yetmez: karşı taraf reddederse hiçbir yerde kayıt oluşmaz.
#[tokio::test]
async fn karsi_taraf_reddederse_hicbir_tarafta_kayit_olusmaz() {
    let a = endpoint(13, "Cihaz A");
    let b = endpoint(14, "Cihaz B");
    let addr = b.local_addr().unwrap();

    let (manager_a, harness_a, db_a) = manager(true);
    let (manager_b, _harness_b, db_b) = manager(false);

    let (client, server) = tokio::join!(a.connect(addr, None), b.accept());
    let mut client = client.unwrap().into_parts();
    let mut server = server.unwrap().unwrap().into_parts();

    let (result_a, result_b) = tokio::join!(
        super::run(Arc::clone(&manager_a), &mut client, true),
        super::run(Arc::clone(&manager_b), &mut server, false),
    );

    assert!(matches!(result_a, Err(PairingError::RejectedByPeer)));
    assert!(matches!(result_b, Err(PairingError::RejectedLocally)));

    assert!(
        !db::devices::is_trusted(&db_a.get().unwrap(), &b.device_id()).unwrap(),
        "onaylayan taraf bile kaydetmemeli — eşleşme iki taraflıdır"
    );
    assert!(!db::devices::is_trusted(&db_b.get().unwrap(), &a.device_id()).unwrap());

    let finished = harness_a.finished().unwrap();
    assert!(!finished.ok);
    assert_eq!(
        finished.reason.as_deref(),
        Some("pairing.error.rejectedByPeer")
    );
}

/// Başlatan taraf vazgeçerse de kayıt oluşmaz.
#[tokio::test]
async fn baslatan_taraf_reddederse_kayit_olusmaz() {
    let a = endpoint(15, "Cihaz A");
    let b = endpoint(16, "Cihaz B");
    let addr = b.local_addr().unwrap();

    let (manager_a, _harness_a, db_a) = manager(false);
    let (manager_b, _harness_b, db_b) = manager(true);

    let (client, server) = tokio::join!(a.connect(addr, None), b.accept());
    let mut client = client.unwrap().into_parts();
    let mut server = server.unwrap().unwrap().into_parts();

    let (result_a, result_b) = tokio::join!(
        super::run(Arc::clone(&manager_a), &mut client, true),
        super::run(Arc::clone(&manager_b), &mut server, false),
    );

    assert!(matches!(result_a, Err(PairingError::RejectedLocally)));
    assert!(matches!(result_b, Err(PairingError::RejectedByPeer)));
    assert!(!db::devices::is_trusted(&db_a.get().unwrap(), &b.device_id()).unwrap());
    assert!(!db::devices::is_trusted(&db_b.get().unwrap(), &a.device_id()).unwrap());
}

/// Eşleştikten sonra pinlenmiş anahtarla yeniden bağlanmak kod sormamalı —
/// Faz 4'ün bitiş kriteri (PLAN.md §7).
#[tokio::test]
async fn eslesme_sonrasi_pinlenmis_baglanti_kod_sormaz() {
    let a = endpoint(17, "Cihaz A");
    let b = endpoint(18, "Cihaz B");
    let addr = b.local_addr().unwrap();

    let (manager_a, _ha, _db_a) = manager(true);
    let (manager_b, harness_b, _db_b) = manager(true);

    let (client, server) = tokio::join!(a.connect(addr, None), b.accept());
    let (mut client, mut server) = (
        client.unwrap().into_parts(),
        server.unwrap().unwrap().into_parts(),
    );
    let (ra, rb) = tokio::join!(
        super::run(Arc::clone(&manager_a), &mut client, true),
        super::run(Arc::clone(&manager_b), &mut server, false),
    );
    assert!(ra.is_ok() && rb.is_ok());
    client.close();
    server.close();

    let requests_before = harness_b.requests.lock().unwrap().len();

    // Yeniden bağlanma: bu kez karşı tarafın anahtarı pinlenmiş durumda.
    let (client, server) = tokio::join!(a.connect(addr, Some(b.device_id())), b.accept());
    let client = client.expect("pinlenmiş anahtarla bağlantı kurulmalı");
    assert!(server.unwrap().is_ok());
    assert_eq!(client.peer_device_id, b.device_id());

    assert_eq!(
        harness_b.requests.lock().unwrap().len(),
        requests_before,
        "yeniden bağlanmada kullanıcıya kod sorulmamalı"
    );
}

/// Gerileme testi: eşleşme bittiğinde bağlantı HÂLÂ kullanılabilir olmalı.
///
/// İlk uygulamada eşleşme biter bitmez bağlantı kapatılıyordu; QUIC'te
/// `close()` akıştaki teslim edilmemiş veriyi attığı için karşı taraf onay
/// mesajını alamıyor ve bir tarafın eşleşmiş, diğerinin eşleşmemiş sayıldığı
/// asimetrik bir durum oluşuyordu. Bağlantının eşleşmeden sonra da sağlam
/// olması, çağıranın onu kapatmak yerine devredebilmesinin ön koşuludur.
#[tokio::test]
async fn eslesme_sonrasi_baglanti_kullanilabilir_kalir() {
    let a = endpoint(19, "Cihaz A");
    let b = endpoint(20, "Cihaz B");
    let addr = b.local_addr().unwrap();

    let (manager_a, _ha, db_a) = manager(true);
    let (manager_b, _hb, db_b) = manager(true);

    let (client, server) = tokio::join!(a.connect(addr, None), b.accept());
    let mut client = client.unwrap().into_parts();
    let mut server = server.unwrap().unwrap().into_parts();

    let (ra, rb) = tokio::join!(
        super::run(Arc::clone(&manager_a), &mut client, true),
        super::run(Arc::clone(&manager_b), &mut server, false),
    );
    assert!(ra.is_ok() && rb.is_ok());

    // İKİ taraf da kaydetmiş olmalı — asimetrik güven kabul edilemez.
    assert!(db::devices::is_trusted(&db_a.get().unwrap(), &b.device_id()).unwrap());
    assert!(db::devices::is_trusted(&db_b.get().unwrap(), &a.device_id()).unwrap());

    // Bağlantı üzerinden hâlâ konuşulabilmeli.
    //
    // Karşı taraf ayrı bir görevde dinler: `next_control_message` heartbeat'i
    // yerinde yanıtlayıp dinlemeye devam eder, yani hiç dönmez — onu doğrudan
    // await etmek testi asardı.
    let server_task = tokio::spawn(async move {
        let _ = server.next_control_message().await;
    });

    let rtt = tokio::time::timeout(Duration::from_secs(5), client.heartbeat()).await;
    server_task.abort();

    assert!(
        matches!(rtt, Ok(Ok(_))),
        "eşleşmeden sonra bağlantı kullanılabilir kalmalı: {rtt:?}"
    );
}
