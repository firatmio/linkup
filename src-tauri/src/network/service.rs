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
use super::manager::ConnectionManager;
use super::protocol::{ControlMessage, ProtocolError};
use crate::pairing::{self, PairingManager};

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

        Ok(Self {
            endpoint: Arc::new(endpoint),
            local_addr,
        })
    }

    /// Gelen bağlantıları kabul etmeye başlar.
    ///
    /// Kabul döngüsü, eşleştirme ve bağlantı denetleyicisi kurulduktan SONRA
    /// başlatılır: gelen bir bağlantının nereye yönleneceğini bilmeden kabul
    /// etmek, bağlantıyı sessizce düşürmek olurdu.
    pub fn start_accepting(
        &self,
        pairing: Arc<PairingManager>,
        connections: Arc<ConnectionManager>,
    ) {
        let endpoint = Arc::clone(&self.endpoint);
        tauri::async_runtime::spawn(async move {
            accept_loop(endpoint, pairing, connections).await;
        });
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

async fn accept_loop(
    endpoint: Arc<NetworkEndpoint>,
    pairing: Arc<PairingManager>,
    connections: Arc<ConnectionManager>,
) {
    while let Some(result) = endpoint.accept().await {
        match result {
            Ok(connection) => {
                let pairing = Arc::clone(&pairing);
                let connections = Arc::clone(&connections);
                tauri::async_runtime::spawn(handle_connection(pairing, connections, connection));
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

/// Gelen bağlantıyı yönlendirir.
///
/// Eşleşmemiş bir cihaz YALNIZCA eşleştirme isteği gönderebilir; başka her
/// mesaj reddedilir (PLAN.md §2.2.1 madde 4). Aksi hâlde eşleşme, güvenlik
/// kararı olmaktan çıkıp yalnızca bir kayıt işlemine dönerdi.
async fn handle_connection(
    pairing: Arc<PairingManager>,
    connections: Arc<ConnectionManager>,
    mut connection: PeerConnection,
) {
    let device_id = connection.peer_device_id;

    tracing::info!(
        peer = %connection.peer.device_name,
        addr = %connection.remote_address(),
        version = connection.negotiated_version,
        trusted = pairing.is_trusted(&device_id),
        "gelen bağlantı"
    );

    if pairing.is_trusted(&device_id) {
        connections.hold(connection).await;
        return;
    }

    match connection.next_control_message().await {
        Ok(ControlMessage::PairingRequest) => {
            let result = pairing::run(Arc::clone(&pairing), &mut connection, false).await;
            if result.is_ok() {
                connections.supervise(device_id);
            }
        }
        Ok(other) => {
            tracing::warn!(?other, "eşleşmemiş cihazdan izin verilmeyen mesaj");
            let _ = connection
                .send(&ControlMessage::Error(ProtocolError::NotPaired))
                .await;
        }
        Err(err) => tracing::debug!(error = %err, "eşleşmemiş bağlantı kapandı"),
    }

    connection.close();
}
