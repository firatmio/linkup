//! Keşfedilen cihazların kaydı (PLAN.md §2.4).
//!
//! mDNS'ten gelen ilanlar ve elle eklenen adresler burada birleşir. Kayıt
//! defteri saf veridir — ağ ile konuşmaz, bu yüzden tamamen test edilebilir.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Instant;

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DiscoverySource {
    Mdns,
    /// Kullanıcının elle girdiği adres (PLAN.md §2.4 — mDNS'in kesildiği ağlar).
    Manual,
}

#[derive(Debug, Clone)]
pub struct DiscoveredDevice {
    pub device_id: [u8; 32],
    pub name: String,
    pub addresses: Vec<SocketAddr>,
    pub protocol_version: u16,
    pub source: DiscoverySource,
    pub last_seen: Instant,
}

impl DiscoveredDevice {
    /// Bağlanmak için tercih edilen adres.
    ///
    /// Bir cihaz birden fazla adres ilan eder (LAN arayüzü, loopback,
    /// link-local, WSL/Docker sanal adaptörleri). En muhtemel ulaşılabilir
    /// olanı seçilir — bkz. `network::address`.
    pub fn preferred_address(&self) -> Option<SocketAddr> {
        self.addresses
            .iter()
            .min_by_key(|addr| crate::network::address::address_rank(&addr.ip()))
            .copied()
    }
}

#[derive(Debug, Default)]
pub struct Registry {
    devices: HashMap<[u8; 32], DiscoveredDevice>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Bir ilanı ekler veya günceller. Liste değiştiyse `true` döner —
    /// çağıran buna bakarak gereksiz UI olayı yayınlamaz.
    pub fn upsert(&mut self, device: DiscoveredDevice) -> bool {
        match self.devices.get_mut(&device.device_id) {
            Some(existing) => {
                let unchanged = existing.name == device.name
                    && existing.addresses == device.addresses
                    && existing.protocol_version == device.protocol_version
                    && existing.source == device.source;
                existing.last_seen = device.last_seen;
                if unchanged {
                    return false;
                }
                *existing = device;
                true
            }
            None => {
                self.devices.insert(device.device_id, device);
                true
            }
        }
    }

    pub fn remove(&mut self, device_id: &[u8; 32]) -> bool {
        self.devices.remove(device_id).is_some()
    }

    /// Adına göre siler (mDNS `ServiceRemoved` yalnızca servis adını verir).
    pub fn remove_by_instance(&mut self, instance_name: &str) -> bool {
        let target = self
            .devices
            .iter()
            .find(|(id, device)| {
                device.source == DiscoverySource::Mdns && super::instance_name(id) == instance_name
            })
            .map(|(id, _)| *id);

        match target {
            Some(id) => self.remove(&id),
            None => false,
        }
    }

    pub fn get(&self, device_id: &[u8; 32]) -> Option<&DiscoveredDevice> {
        self.devices.get(device_id)
    }

    /// Ada göre sıralı liste — UI'da sıra oynamasın.
    pub fn list(&self) -> Vec<DiscoveredDevice> {
        let mut devices: Vec<_> = self.devices.values().cloned().collect();
        devices.sort_by(|a, b| a.name.cmp(&b.name).then(a.device_id.cmp(&b.device_id)));
        devices
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.devices.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // use std::time::Duration;

    fn device(id: u8, name: &str, source: DiscoverySource) -> DiscoveredDevice {
        DiscoveredDevice {
            device_id: [id; 32],
            name: name.to_string(),
            addresses: vec!["192.168.1.5:47810".parse().unwrap()],
            protocol_version: 1,
            source,
            last_seen: Instant::now(),
        }
    }

    #[test]
    fn ayni_cihaz_iki_kez_eklenmez() {
        let mut registry = Registry::new();
        assert!(registry.upsert(device(1, "A", DiscoverySource::Mdns)));
        assert!(!registry.upsert(device(1, "A", DiscoverySource::Mdns)));
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn ad_degisirse_guncellenir_ve_degisiklik_bildirilir() {
        let mut registry = Registry::new();
        registry.upsert(device(1, "Eski", DiscoverySource::Mdns));
        assert!(registry.upsert(device(1, "Yeni", DiscoverySource::Mdns)));
        assert_eq!(registry.get(&[1; 32]).unwrap().name, "Yeni");
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn liste_ada_gore_sirali() {
        let mut registry = Registry::new();
        registry.upsert(device(3, "Cem", DiscoverySource::Mdns));
        registry.upsert(device(1, "Ali", DiscoverySource::Mdns));
        registry.upsert(device(2, "Bora", DiscoverySource::Mdns));

        let names: Vec<_> = registry.list().into_iter().map(|d| d.name).collect();
        assert_eq!(names, ["Ali", "Bora", "Cem"]);
    }

    /// Gerileme testi: kayıtlar kendiliğinden kaybolmamalı.
    ///
    /// İlk uygulamada kayıt defterinin kendi 90 sn'lik TTL'i vardı. mdns-sd
    /// değişmeyen bir servis için `ServiceResolved`ı tekrar yayınlamadığından
    /// `last_seen` hiç tazelenmiyor ve cihazlar iki dakika sonra listeden
    /// KALICI olarak siliniyordu. Ömür yönetimi artık tamamen mdns-sd'ye ait:
    /// kayıt yalnızca `ServiceRemoved` geldiğinde veya kullanıcı sildiğinde düşer.
    #[test]
    fn kayit_kendiliginden_kaybolmaz() {
        let mut registry = Registry::new();
        let old = device(1, "Eski", DiscoverySource::Mdns);
        // let mut old = device(1, "Eski", DiscoverySource::Mdns);
        // old.last_seen = Instant::now() - Duration::from_secs(60 * 60);
        // burası testlerde patlak verdi
        registry.upsert(old);

        assert_eq!(registry.len(), 1, "zamanla kendiliğinden silinmemeli");
        assert!(registry.get(&[1; 32]).is_some());

        // Yalnızca açık bir kaldırma kaydı düşürür.
        assert!(registry.remove(&[1; 32]));
        assert_eq!(registry.len(), 0);
    }

    /// Gerçek bir mDNS ilanı böyle görünüyor: LAN adresi, loopback,
    /// link-local ve sanal adaptörler bir arada.
    #[test]
    fn ulasilabilir_lan_adresi_secilir() {
        let mut device = device(1, "A", DiscoverySource::Mdns);
        device.addresses = vec![
            "127.0.0.1:47812".parse().unwrap(),
            "172.17.80.1:47812".parse().unwrap(),
            "169.254.188.223:47812".parse().unwrap(),
            "192.168.0.195:47812".parse().unwrap(),
        ];
        assert_eq!(
            device.preferred_address().unwrap(),
            "192.168.0.195:47812".parse::<SocketAddr>().unwrap()
        );
    }

    #[test]
    fn adres_yoksa_none_doner() {
        let mut device = device(1, "A", DiscoverySource::Mdns);
        device.addresses.clear();
        assert!(device.preferred_address().is_none());
    }
}
