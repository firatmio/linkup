//! Cihaz keşfi: mDNS ilanı/taraması ve elle adres ekleme (PLAN.md §2.4).
//!
//! mDNS tek başına yeterli değildir (§10-K7): Windows Firewall'un "Public
//! network" profili ve kurumsal/misafir ağlardaki client isolation onu düzenli
//! olarak keser. Bu yüzden elle ekleme temel bir çalışabilirlik gereksinimidir,
//! "gelişmiş özellik" değil.

pub mod registry;

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use data_encoding::BASE32_NOPAD;
use mdns_sd::{ResolvedService, ServiceDaemon, ServiceEvent, ServiceInfo};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::network::endpoint::NetworkEndpoint;
use registry::{DiscoveredDevice, DiscoverySource, Registry};

/// QUIC UDP üzerinde çalıştığı için `_udp` (PLAN.md §2.4).
const SERVICE_TYPE: &str = "_linkup._udp.local.";

/// TXT kayıt anahtarları. Kısa tutuluyor: TXT kaydının toplam boyutu sınırlı.
const TXT_NAME: &str = "n";
const TXT_FINGERPRINT: &str = "fp";
const TXT_VERSION: &str = "v";

/// Frontend'e yayınlanan olay adı.
pub const DISCOVERY_EVENT: &str = "discovery:changed";

const SWEEP_INTERVAL: Duration = Duration::from_secs(30);

/// Elle ekleme sırasında el sıkışmanın tamamlanması için üst sınır.
/// Yanlış girilmiş bir adres kullanıcıyı belirsiz süre bekletmemeli.
const MANUAL_PROBE_TIMEOUT: Duration = Duration::from_secs(8);

/// mDNS servis örneği adı. Yalnızca `device_id`'den türetilir: cihaz adı
/// değişince kayıt kimliğini kaybetmesin.
pub fn instance_name(device_id: &[u8; 32]) -> String {
    format!("linkup-{}", &BASE32_NOPAD.encode(device_id)[..16])
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredDeviceDto {
    /// Base32 kodlu `device_id` — frontend'de kimlik anahtarı olarak kullanılır.
    pub id: String,
    /// Kullanıcıya gösterilen, gruplanmış fingerprint.
    pub fingerprint: String,
    pub name: String,
    pub address: Option<String>,
    pub protocol_version: u16,
    pub source: DiscoverySource,
}

impl From<&DiscoveredDevice> for DiscoveredDeviceDto {
    fn from(device: &DiscoveredDevice) -> Self {
        Self {
            id: BASE32_NOPAD.encode(&device.device_id),
            fingerprint: crate::identity::format_fingerprint(&device.device_id),
            name: device.name.clone(),
            address: device.preferred_address().map(|a| a.to_string()),
            protocol_version: device.protocol_version,
            source: device.source,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    #[error("mDNS başlatılamadı: {0}")]
    Daemon(String),

    #[error("adres çözümlenemedi: {0}")]
    BadAddress(String),

    #[error("cihaza ulaşılamadı: {0}")]
    Unreachable(String),
}

pub struct DiscoveryService {
    registry: Arc<Mutex<Registry>>,
    endpoint: Arc<NetworkEndpoint>,
    /// Kapanışta ilanı geri çekmek için tutuluyor.
    daemon: Option<ServiceDaemon>,
    instance: String,
}

impl DiscoveryService {
    /// mDNS ilanını yayınlar, taramayı ve süre dolum temizliğini arka planda
    /// başlatır.
    ///
    /// mDNS başlatılamazsa servis yine de kurulur: elle eklenen cihazlar
    /// çalışmaya devam etmeli. Ağ keşfinin kapalı olması uygulamayı
    /// kullanılamaz hâle getirmemeli.
    pub fn start(
        app: AppHandle,
        endpoint: Arc<NetworkEndpoint>,
        device_name: String,
        port: u16,
    ) -> Self {
        let device_id = endpoint.device_id();
        let instance = instance_name(&device_id);
        let registry = Arc::new(Mutex::new(Registry::new()));

        let daemon = match Self::spawn_mdns(
            app.clone(),
            Arc::clone(&registry),
            &instance,
            &device_name,
            device_id,
            port,
        ) {
            Ok(daemon) => Some(daemon),
            Err(err) => {
                tracing::warn!(
                    error = %err,
                    "mDNS başlatılamadı — otomatik keşif devre dışı, elle ekleme çalışmaya devam ediyor"
                );
                None
            }
        };

        Self::spawn_sweeper(app, Arc::clone(&registry));

        Self {
            registry,
            endpoint,
            daemon,
            instance,
        }
    }

    fn spawn_mdns(
        app: AppHandle,
        registry: Arc<Mutex<Registry>>,
        instance: &str,
        device_name: &str,
        device_id: [u8; 32],
        port: u16,
    ) -> Result<ServiceDaemon, DiscoveryError> {
        let daemon = ServiceDaemon::new().map_err(|e| DiscoveryError::Daemon(e.to_string()))?;

        let properties = [
            (TXT_NAME, device_name.to_string()),
            (TXT_FINGERPRINT, BASE32_NOPAD.encode(&device_id)),
            (
                TXT_VERSION,
                crate::network::protocol::PROTOCOL_VERSION.to_string(),
            ),
        ];

        let service = ServiceInfo::new(
            SERVICE_TYPE,
            instance,
            &format!("{instance}.local."),
            (),
            port,
            &properties[..],
        )
        .map_err(|e| DiscoveryError::Daemon(e.to_string()))?
        // Ağ arayüzü adreslerini kendisi bulsun; IP'yi elle vermek, birden
        // fazla arayüzü olan makinelerde yanlış adresi ilan etmeye yol açar.
        .enable_addr_auto();

        daemon
            .register(service)
            .map_err(|e| DiscoveryError::Daemon(e.to_string()))?;

        let receiver = daemon
            .browse(SERVICE_TYPE)
            .map_err(|e| DiscoveryError::Daemon(e.to_string()))?;

        tracing::info!(instance, port, "mDNS ilanı yayınlandı, tarama başladı");

        tauri::async_runtime::spawn(async move {
            while let Ok(event) = receiver.recv_async().await {
                let changed = match event {
                    ServiceEvent::ServiceResolved(info) => {
                        handle_resolved(&registry, &info, device_id)
                    }
                    ServiceEvent::ServiceRemoved(_, full_name) => {
                        let instance = full_name.split('.').next().unwrap_or_default().to_string();
                        let mut registry = registry.lock().expect("kayıt defteri kilidi");
                        registry.remove_by_instance(&instance)
                    }
                    _ => false,
                };

                if changed {
                    emit(&app, &registry);
                }
            }
            tracing::info!("mDNS tarama döngüsü sona erdi");
        });

        Ok(daemon)
    }

    fn spawn_sweeper(app: AppHandle, registry: Arc<Mutex<Registry>>) {
        tauri::async_runtime::spawn(async move {
            loop {
                tokio::time::sleep(SWEEP_INTERVAL).await;
                let changed = {
                    let mut registry = registry.lock().expect("kayıt defteri kilidi");
                    registry.sweep_expired(Instant::now())
                };
                if changed {
                    emit(&app, &registry);
                }
            }
        });
    }

    pub fn list(&self) -> Vec<DiscoveredDeviceDto> {
        let registry = self.registry.lock().expect("kayıt defteri kilidi");
        registry.list().iter().map(Into::into).collect()
    }

    /// Elle girilen bir adrese bağlanıp cihazı tanır (PLAN.md §2.4).
    ///
    /// Yalnızca listeye yazmak yeterli değil: kullanıcı yanlış bir adres
    /// girdiyse bunu anında öğrenmeli. Bu yüzden gerçekten bağlanılır, el
    /// sıkışılır, cihazın kimliği ve adı öğrenilir, sonra bağlantı kapatılır.
    pub async fn add_manual(&self, address: &str) -> Result<DiscoveredDeviceDto, DiscoveryError> {
        let addr = parse_address(address)?;

        let connection = tokio::time::timeout(
            MANUAL_PROBE_TIMEOUT,
            // Kimlik önceden bilinmiyor: pinleme yok, kimlik el sıkışmadan
            // sonra sertifikadan okunuyor (PLAN.md §2.2.1).
            self.endpoint.connect(addr, None),
        )
        .await
        .map_err(|_| DiscoveryError::Unreachable("zaman aşımı".to_string()))?
        .map_err(|e| DiscoveryError::Unreachable(e.to_string()))?;

        let device = DiscoveredDevice {
            device_id: connection.peer_device_id,
            name: connection.peer.device_name.clone(),
            addresses: vec![addr],
            protocol_version: connection.negotiated_version,
            source: DiscoverySource::Manual,
            last_seen: Instant::now(),
        };
        connection.close();

        let dto = DiscoveredDeviceDto::from(&device);
        tracing::info!(name = %device.name, %addr, "cihaz elle eklendi");

        let mut registry = self.registry.lock().expect("kayıt defteri kilidi");
        registry.upsert(device);
        Ok(dto)
    }

    pub fn remove(&self, id: &str) -> bool {
        let Some(device_id) = decode_device_id(id) else {
            return false;
        };
        let mut registry = self.registry.lock().expect("kayıt defteri kilidi");
        registry.remove(&device_id)
    }
}

impl Drop for DiscoveryService {
    fn drop(&mut self) {
        if let Some(daemon) = self.daemon.take() {
            // İlanı geri çek: diğer cihazlar bizi TTL dolana kadar "çevrimiçi"
            // sanmasın.
            let full_name = format!("{}.{SERVICE_TYPE}", self.instance);
            let _ = daemon.unregister(&full_name);
            let _ = daemon.shutdown();
        }
    }
}

/// Çözümlenen bir ilanı kayıt defterine işler.
///
/// Yalnızca IPv4 adresleri alınır: mDNS'in ilan ettiği IPv6 link-local
/// adresleri bağlanmak için scope id (arayüz) gerektirir ve yanlış arayüzle
/// denenince zaman aşımına kadar asılı kalır. IPv6 keşfi, gerçek bir ihtiyaç
/// doğduğunda ayrıca ele alınacak.
fn handle_resolved(
    registry: &Arc<Mutex<Registry>>,
    info: &ResolvedService,
    our_device_id: [u8; 32],
) -> bool {
    let Some(device_id) = info
        .get_property_val_str(TXT_FINGERPRINT)
        .and_then(decode_device_id)
    else {
        tracing::debug!(
            instance = info.get_fullname(),
            "TXT kaydında geçerli fingerprint yok, yok sayıldı"
        );
        return false;
    };

    // Kendi ilanımızı görürüz; listeye kendimizi koymamalıyız.
    if device_id == our_device_id {
        return false;
    }

    let port = info.get_port();
    let addresses: Vec<SocketAddr> = info
        .get_addresses_v4()
        .into_iter()
        .map(|ip| SocketAddr::new(ip.into(), port))
        .collect();

    if addresses.is_empty() {
        return false;
    }

    let name = info
        .get_property_val_str(TXT_NAME)
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| info.get_fullname())
        .to_string();

    let protocol_version = info
        .get_property_val_str(TXT_VERSION)
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    let mut registry = registry.lock().expect("kayıt defteri kilidi");
    let changed = registry.upsert(DiscoveredDevice {
        device_id,
        name: name.clone(),
        addresses: addresses.clone(),
        protocol_version,
        source: DiscoverySource::Mdns,
        last_seen: Instant::now(),
    });

    if changed {
        tracing::info!(
            %name,
            addresses = ?addresses,
            protocol_version,
            "cihaz keşfedildi"
        );
    }
    changed
}

fn emit(app: &AppHandle, registry: &Arc<Mutex<Registry>>) {
    let devices: Vec<DiscoveredDeviceDto> = {
        let registry = registry.lock().expect("kayıt defteri kilidi");
        registry.list().iter().map(Into::into).collect()
    };
    if let Err(err) = app.emit(DISCOVERY_EVENT, &devices) {
        tracing::warn!(error = %err, "keşif olayı yayınlanamadı");
    }
}

fn decode_device_id(encoded: &str) -> Option<[u8; 32]> {
    BASE32_NOPAD
        .decode(encoded.trim().to_uppercase().as_bytes())
        .ok()?
        .try_into()
        .ok()
}

/// `192.168.1.5:47810` veya `192.168.1.5` (varsayılan porta düşer) kabul eder.
fn parse_address(input: &str) -> Result<SocketAddr, DiscoveryError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(DiscoveryError::BadAddress("adres boş".to_string()));
    }

    if let Ok(addr) = trimmed.parse::<SocketAddr>() {
        return Ok(addr);
    }
    if let Ok(ip) = trimmed.parse::<std::net::IpAddr>() {
        return Ok(SocketAddr::new(ip, crate::paths::DEFAULT_QUIC_PORT));
    }
    Err(DiscoveryError::BadAddress(trimmed.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ornek_adi_yalnizca_kimlikten_turer() {
        let id = [7u8; 32];
        assert_eq!(instance_name(&id), instance_name(&id));
        assert_ne!(instance_name(&id), instance_name(&[8u8; 32]));
        assert!(instance_name(&id).starts_with("linkup-"));
    }

    #[test]
    fn adres_portlu_ve_portsuz_kabul_edilir() {
        assert_eq!(
            parse_address("192.168.1.5:1234").unwrap(),
            "192.168.1.5:1234".parse::<SocketAddr>().unwrap()
        );
        assert_eq!(
            parse_address(" 192.168.1.5 ").unwrap().port(),
            crate::paths::DEFAULT_QUIC_PORT
        );
        assert_eq!(
            parse_address("[::1]:9000").unwrap(),
            "[::1]:9000".parse::<SocketAddr>().unwrap()
        );
    }

    #[test]
    fn gecersiz_adres_reddedilir() {
        assert!(parse_address("").is_err());
        assert!(parse_address("   ").is_err());
        assert!(parse_address("cihazim").is_err());
        assert!(parse_address("999.1.1.1").is_err());
    }

    #[test]
    fn kimlik_kodlanip_cozulur() {
        let id = [42u8; 32];
        assert_eq!(decode_device_id(&BASE32_NOPAD.encode(&id)).unwrap(), id);
        assert!(decode_device_id("KISA").is_none());
        assert!(decode_device_id("bu base32 değil!").is_none());
    }
}
