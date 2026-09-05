//! Ağ servisi: uygulama açılışında QUIC uç noktasını açar ve gelen
//! bağlantıları kabul eder (PLAN.md §2.1, §2.2).
//!
//! Faz 2 kapsamı: uç nokta ayakta, el sıkışma çalışıyor. Bağlanacak adresleri
//! keşif (Faz 3) sağlayacak; eşleşme kararını pairing (Faz 4) verecek. Bu
//! yüzden şimdilik kabul edilen bağlantı el sıkışıp kapatılır — henüz
//! yetkilendirilmiş bir eş kavramı yok.

use std::net::SocketAddr;
use std::sync::Arc;

use ed25519_dalek::SigningKey;

use super::endpoint::{NetworkEndpoint, PeerConnection};

pub struct NetworkService {
    endpoint: Arc<NetworkEndpoint>,
    local_addr: SocketAddr,
}

impl NetworkService {
    /// Uç noktayı açar ve kabul döngüsünü arka planda başlatır.
    ///
    /// İstenen port meşgulse (aynı makinede profilsiz ikinci bir instance gibi)
    /// işletim sisteminin verdiği boş bir porta düşülür: uygulamanın hiç
    /// açılmaması yerine, ağ özelliklerinin farklı bir portta çalışması yeğdir.
    pub fn start(
        signing_key: &SigningKey,
        device_name: String,
        preferred_port: u16,
    ) -> anyhow::Result<Self> {
        let endpoint = match bind(signing_key, &device_name, preferred_port) {
            Ok(endpoint) => endpoint,
            Err(err) => {
                tracing::warn!(
                    port = preferred_port,
                    error = %err,
                    "tercih edilen port kullanılamadı, boş bir porta düşülüyor"
                );
                bind(signing_key, &device_name, 0)?
            }
        };

        let local_addr = endpoint.local_addr()?;
        let endpoint = Arc::new(endpoint);

        let accept_endpoint = Arc::clone(&endpoint);
        tauri::async_runtime::spawn(async move {
            accept_loop(accept_endpoint).await;
        });

        Ok(Self {
            endpoint,
            local_addr,
        })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    pub fn device_id(&self) -> [u8; 32] {
        self.endpoint.device_id()
    }

    /// Bu makinenin LAN'da erişilebilir adresleri.
    ///
    /// Uç nokta 0.0.0.0'a bağlandığı için `local_addr` tek başına kullanıcıya
    /// bir şey söylemez; karşı cihazda elle ekleme yapabilmesi için gerçek
    /// arayüz adresleri gerekir (PLAN.md §2.4).
    pub fn reachable_addresses(&self) -> Vec<SocketAddr> {
        let port = self.local_addr.port();
        let mut addresses: Vec<SocketAddr> = if_addrs::get_if_addrs()
            .unwrap_or_default()
            .into_iter()
            .filter(|iface| !iface.is_loopback())
            .filter_map(|iface| match iface.ip() {
                // IPv6 link-local adresleri scope id olmadan işe yaramaz,
                // kullanıcıya göstermek kafa karıştırır.
                std::net::IpAddr::V4(ip) => Some(SocketAddr::new(ip.into(), port)),
                std::net::IpAddr::V6(_) => None,
            })
            .collect();
        addresses.sort();
        addresses.dedup();
        // Kullanıcıya en muhtemel doğru adresi ilk sırada göster.
        super::address::sort_by_reachability(&mut addresses, |addr| addr.ip());
        addresses
    }

    /// Keşif servisi elle ekleme sırasında bu uç nokta üzerinden bağlanır.
    pub fn endpoint(&self) -> Arc<NetworkEndpoint> {
        Arc::clone(&self.endpoint)
    }
}

impl Drop for NetworkService {
    fn drop(&mut self) {
        self.endpoint.close();
    }
}

fn bind(
    signing_key: &SigningKey,
    device_name: &str,
    port: u16,
) -> Result<NetworkEndpoint, super::endpoint::NetworkError> {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    NetworkEndpoint::bind(signing_key, device_name.to_string(), addr)
}

async fn accept_loop(endpoint: Arc<NetworkEndpoint>) {
    while let Some(result) = endpoint.accept().await {
        match result {
            Ok(connection) => {
                tauri::async_runtime::spawn(handle_connection(connection));
            }
            Err(err) => {
                // Tek bir başarısız el sıkışma dinlemeyi durdurmamalı: sürüm
                // uyumsuz bir cihaz veya bozuk bir paket, uygulamayı ağa
                // kapatmak için yeterli sebep değil.
                tracing::debug!(error = %err, "gelen bağlantı el sıkışamadı");
            }
        }
    }
    tracing::info!("kabul döngüsü sona erdi");
}

async fn handle_connection(connection: PeerConnection) {
    tracing::info!(
        peer = %connection.peer.device_name,
        addr = %connection.remote_address(),
        version = connection.negotiated_version,
        "bağlantı kuruldu"
    );

    // Faz 4'te burada eşleşme kontrolü yapılacak; şu an yetkilendirilmiş eş
    // kavramı olmadığı için bağlantı kapatılıyor.
    connection.close();
}
