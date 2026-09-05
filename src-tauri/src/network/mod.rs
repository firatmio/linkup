//! Ağ katmanı: QUIC transport, uygulama protokolü, bağlantı yaşam döngüsü
//! (PLAN.md §2.2, §2.3).
//!
//! Bu katman tüketicilerinden önce yazıldı: `connect`, `heartbeat` ve kontrol
//! akışı okuma Faz 3 (keşif), Faz 4 (eşleştirme) ve Faz 5'te (chat) devreye
//! girecek. Hepsi uçtan uca testlerle kaplı; yalnızca üretim kodundan çağıran
//! henüz yok. Faz 5 bittiğinde bu istisna kaldırılmalı.
#![allow(dead_code)]

pub mod address;
pub mod backoff;
pub mod endpoint;
pub mod manager;
pub mod protocol;
pub mod service;
pub mod tls;

#[cfg(test)]
mod tests {
    //! Uçtan uca ağ testleri (PLAN.md §6).
    //!
    //! İki `NetworkEndpoint`'i aynı süreçte loopback üzerinde kurup gerçek
    //! QUIC/TLS el sıkışması yaparlar — sahte (mock) bir transport değil.

    use std::net::SocketAddr;

    use ed25519_dalek::SigningKey;

    use super::endpoint::{NetworkEndpoint, NetworkError, PeerConnection};

    fn key(seed: u8) -> SigningKey {
        SigningKey::from_bytes(&[seed; 32])
    }

    fn loopback() -> SocketAddr {
        // Port 0: işletim sistemi boş bir port versin — testler paralel koşabilsin.
        "127.0.0.1:0".parse().unwrap()
    }

    fn bind(seed: u8, name: &str) -> NetworkEndpoint {
        NetworkEndpoint::bind(&key(seed), name.to_string(), loopback()).unwrap()
    }

    /// İki ucu bağlar; (bağlanan taraf, kabul eden taraf) döner.
    async fn connect_pair(
        client: &NetworkEndpoint,
        server: &NetworkEndpoint,
        expected_peer: Option<[u8; 32]>,
    ) -> (
        Result<PeerConnection, NetworkError>,
        Option<Result<PeerConnection, NetworkError>>,
    ) {
        let addr = server.local_addr().unwrap();
        tokio::join!(client.connect(addr, expected_peer), server.accept())
    }

    #[tokio::test]
    async fn el_sikisma_iki_uc_arasinda_tamamlanir() {
        let a = bind(1, "Cihaz A");
        let b = bind(2, "Cihaz B");

        let (client, server) = connect_pair(&a, &b, Some(b.device_id())).await;
        let client = client.expect("bağlanan taraf el sıkışmalı");
        let server = server.unwrap().expect("kabul eden taraf el sıkışmalı");

        // Her iki taraf da karşısındakini SERTİFİKADAN doğru tanımalı.
        assert_eq!(client.peer_device_id, b.device_id());
        assert_eq!(server.peer_device_id, a.device_id());

        assert_eq!(client.peer.device_name, "Cihaz B");
        assert_eq!(server.peer.device_name, "Cihaz A");

        assert_eq!(client.negotiated_version, super::protocol::PROTOCOL_VERSION);
        assert_eq!(server.negotiated_version, super::protocol::PROTOCOL_VERSION);
    }

    /// Eşleşmemiş akış: karşı tarafın kimliği önceden bilinmiyor.
    /// Bağlantı kurulmalı, kimlik el sıkışmadan sonra öğrenilmeli.
    #[tokio::test]
    async fn pinsiz_baglanti_kimligi_sonradan_ogrenir() {
        let a = bind(3, "A");
        let b = bind(4, "B");

        let (client, server) = connect_pair(&a, &b, None).await;
        let client = client.unwrap();
        assert!(server.unwrap().is_ok());
        assert_eq!(client.peer_device_id, b.device_id());
    }

    /// Pinleme testinin özü: saldırgan geçerli ama BAŞKA bir sertifika sunuyor.
    /// TLS katmanı bunu el sıkışma tamamlanmadan reddetmeli.
    #[tokio::test]
    async fn beklenenden_farkli_cihaza_baglanti_reddedilir() {
        let a = bind(5, "A");
        let impostor = bind(6, "Sahte");
        let expected = key(7).verifying_key().to_bytes();

        let (client, _server) = connect_pair(&a, &impostor, Some(expected)).await;
        assert!(
            client.is_err(),
            "pinlenmiş anahtarla eşleşmeyen sertifika kabul edilmemeli"
        );
    }

    #[tokio::test]
    async fn heartbeat_rtt_doner() {
        let a = bind(8, "A");
        let b = bind(9, "B");

        let addr = b.local_addr().unwrap();
        let server_task = tokio::spawn(async move {
            let mut server = b.accept().await.unwrap().unwrap();
            // Kontrol döngüsü: heartbeat'i yerinde yanıtlar.
            let _ = server.next_control_message().await;
            server
        });

        let mut client = a.connect(addr, None).await.unwrap();
        let rtt = client.heartbeat().await.expect("heartbeat yanıtlanmalı");
        assert!(rtt.as_micros() > 0, "ölçülebilir bir RTT dönmeli");

        client.close();
        let _ = server_task.await;
    }

    /// Karşı taraf ulaşılamazsa bağlanma girişimi hata döndürmeli, asılmamalı.
    #[tokio::test]
    async fn ulasilamayan_adres_hata_doner() {
        let a = bind(10, "A");
        // Kapalı bir port: kimse dinlemiyor.
        let dead: SocketAddr = "127.0.0.1:1".parse().unwrap();
        assert!(a.connect(dead, None).await.is_err());
    }

    /// LAN throughput ölçümü (PLAN.md §2.2.2 — Faz 2'nin zorunlu ölçümü).
    ///
    /// Loopback üzerinde çalışır: gerçek bir NIC'in yerini tutmaz, ama
    /// yığınımızın tavanını ölçer — buradaki bir darboğaz gerçek ağda da
    /// darboğazdır. Sonuç PLAN.md'ye kaydedilir.
    ///
    /// Çalıştırmak için:
    ///   cargo test --release -- --ignored --nocapture throughput
    #[tokio::test]
    #[ignore = "ölçüm testi — elle çalıştırılır"]
    async fn throughput_olcumu() {
        const TOTAL: usize = 512 * 1024 * 1024;
        const CHUNK: usize = 256 * 1024;

        let a = bind(20, "Gönderen");
        let b = bind(21, "Alan");
        let addr = b.local_addr().unwrap();

        let receiver = tokio::spawn(async move {
            let server = b.accept().await.unwrap().unwrap();
            let mut stream = server.connection().accept_uni().await.unwrap();
            let mut received = 0usize;
            let mut buffer = vec![0u8; CHUNK];
            while let Ok(Some(n)) = stream.read(&mut buffer).await {
                received += n;
            }
            received
        });

        let client = a.connect(addr, None).await.unwrap();
        let mut stream = client.connection().open_uni().await.unwrap();
        let payload = vec![0xABu8; CHUNK];

        let started = std::time::Instant::now();
        let mut sent = 0usize;
        while sent < TOTAL {
            stream.write_all(&payload).await.unwrap();
            sent += payload.len();
        }
        stream.finish().unwrap();

        let received = receiver.await.unwrap();
        let elapsed = started.elapsed();

        assert_eq!(
            received, sent,
            "gönderilen ve alınan byte sayısı eşit olmalı"
        );

        let mbytes = sent as f64 / (1024.0 * 1024.0);
        let seconds = elapsed.as_secs_f64();
        let mbit_s = (sent as f64 * 8.0) / seconds / 1_000_000.0;

        println!(
            "\n=== LinkUp throughput (loopback) ===\n\
             Aktarılan : {mbytes:.0} MiB\n\
             Süre      : {seconds:.2} sn\n\
             Hız       : {:.0} MiB/s  ({mbit_s:.0} Mbit/s)\n",
            mbytes / seconds
        );
    }
}
