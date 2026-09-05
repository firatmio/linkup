//! Uygulama dizinleri ve profile bağlı isimlendirme.
//!
//! Her profil kendi veri dizinini, log dizinini, veritabanını, keyring girdisini ve
//! QUIC portunu kullanır; böylece aynı makinede iki instance çakışmadan çalışır.

use std::path::PathBuf;

/// Varsayılan QUIC portu. Profiller bunun üzerine sabit bir kayma ekler.
pub const DEFAULT_QUIC_PORT: u16 = 47810;

#[derive(Debug, Clone)]
pub struct AppPaths {
    /// `None` = üretim (varsayılan) profili.
    pub profile: Option<String>,
    pub data_dir: PathBuf,
    pub log_dir: PathBuf,
    pub db_path: PathBuf,
    pub downloads_dir: PathBuf,
    pub quic_port: u16,
}

impl AppPaths {
    pub fn resolve(profile: Option<String>) -> anyhow::Result<Self> {
        let base = dirs::data_dir()
            .ok_or_else(|| anyhow::anyhow!("işletim sisteminin veri dizini bulunamadı"))?;

        let dir_name = match &profile {
            Some(p) => format!("LinkUp-{p}"),
            None => "LinkUp".to_string(),
        };
        let data_dir = base.join(dir_name);
        let log_dir = data_dir.join("logs");

        let downloads_dir = dirs::download_dir()
            .unwrap_or_else(|| data_dir.clone())
            .join("LinkUp");

        std::fs::create_dir_all(&log_dir)?;

        Ok(Self {
            quic_port: derive_port(profile.as_deref()),
            db_path: data_dir.join("linkup.db"),
            data_dir,
            log_dir,
            downloads_dir,
            profile,
        })
    }

    /// OS keychain'de kullanılacak servis adı (PLAN.md §2.6).
    /// Faz 1'de kimlik anahtarı saklanırken kullanılacak.
    #[allow(dead_code)]
    pub fn keyring_service(&self) -> String {
        match &self.profile {
            Some(p) => format!("com.quacomes.linkup.{p}"),
            None => "com.quacomes.linkup".to_string(),
        }
    }

    pub fn profile_label(&self) -> &str {
        self.profile.as_deref().unwrap_or("default")
    }
}

/// Profil adından deterministik port türetir. Bilinen dev profilleri sabit
/// portlara oturur; diğerleri ad üzerinden dağıtılır.
fn derive_port(profile: Option<&str>) -> u16 {
    match profile {
        None => DEFAULT_QUIC_PORT,
        Some("a") => DEFAULT_QUIC_PORT + 1,
        Some("b") => DEFAULT_QUIC_PORT + 2,
        Some("c") => DEFAULT_QUIC_PORT + 3,
        Some(other) => {
            let sum: u32 = other.bytes().map(u32::from).sum();
            DEFAULT_QUIC_PORT + 10 + (sum % 100) as u16
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profiller_farkli_port_alir() {
        assert_eq!(derive_port(None), 47810);
        assert_eq!(derive_port(Some("a")), 47811);
        assert_eq!(derive_port(Some("b")), 47812);
        assert_ne!(derive_port(Some("a")), derive_port(Some("b")));
        // Bilinmeyen profil de base ile çakışmamalı.
        assert!(derive_port(Some("deneme")) > DEFAULT_QUIC_PORT + 9);
    }

    #[test]
    fn keyring_servisi_profile_gore_ayrisir() {
        let mut p = AppPaths {
            profile: None,
            data_dir: PathBuf::new(),
            log_dir: PathBuf::new(),
            db_path: PathBuf::new(),
            downloads_dir: PathBuf::new(),
            quic_port: 0,
        };
        assert_eq!(p.keyring_service(), "com.quacomes.linkup");
        p.profile = Some("a".into());
        assert_eq!(p.keyring_service(), "com.quacomes.linkup.a");
    }
}
