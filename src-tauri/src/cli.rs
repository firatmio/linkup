//! Komut satırı argümanları.
//!
//! `--profile` bayrağı, aynı makinede birden fazla LinkUp instance'ı çalıştırmayı
//! sağlar: her profil kendi veri dizinini, veritabanını, anahtarını ve QUIC portunu
//! kullanır. P2P geliştirmesi için zorunludur (bkz. PLAN.md §6).

use clap::Parser;

#[derive(Parser, Debug, Clone, Default)]
#[command(
    name = "linkup",
    version,
    about = "LinkUp — cihazlar arası sohbet ve dosya transferi"
)]
pub struct Cli {
    /// Geliştirme profili. Ayrı veri dizini, DB, anahtar ve port kullanır (örn. --profile a).
    #[arg(long, value_name = "AD")]
    pub profile: Option<String>,

    /// Log seviyesi: trace | debug | info | warn | error
    #[arg(long, value_name = "SEVIYE")]
    pub log_level: Option<String>,
}

impl Cli {
    /// Tanınmayan argümanlar yüzünden uygulamanın açılmaması kabul edilemez:
    /// işletim sistemi ve Tauri dev runner'ı beklenmedik argümanlar geçirebilir.
    /// Bu yüzden ayrıştırma hatasında varsayılana düşülür; yalnızca --help/--version
    /// normal clap davranışını korur.
    pub fn parse_lenient() -> Self {
        match Self::try_parse() {
            Ok(cli) => cli,
            Err(err) => {
                use clap::error::ErrorKind;
                if matches!(
                    err.kind(),
                    ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
                ) {
                    let _ = err.print();
                    std::process::exit(0);
                }
                Self::default()
            }
        }
    }

    /// Profil adını dosya sistemi ve keyring için güvenli hale getirir.
    pub fn normalized_profile(&self) -> Option<String> {
        let raw = self.profile.as_ref()?.trim();
        if raw.is_empty() {
            return None;
        }
        let cleaned: String = raw
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .take(32)
            .collect();
        if cleaned.is_empty() {
            None
        } else {
            Some(cleaned.to_ascii_lowercase())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_profile(p: &str) -> Cli {
        Cli {
            profile: Some(p.to_string()),
            log_level: None,
        }
    }

    #[test]
    fn profil_adi_temizlenir() {
        assert_eq!(with_profile("A").normalized_profile().as_deref(), Some("a"));
        assert_eq!(
            with_profile("dev-1").normalized_profile().as_deref(),
            Some("dev-1")
        );
        // Yol ayraçları ve nokta atılır — profil adı dizin adına giriyor.
        assert_eq!(
            with_profile("../etc").normalized_profile().as_deref(),
            Some("etc")
        );
        assert_eq!(
            with_profile("a/b").normalized_profile().as_deref(),
            Some("ab")
        );
        assert_eq!(with_profile("   ").normalized_profile(), None);
        assert_eq!(with_profile("///").normalized_profile(), None);
    }
}
