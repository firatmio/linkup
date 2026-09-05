//! Cihaz kimliği: Ed25519 keypair üretimi, saklanması ve fingerprint
//! (PLAN.md §2.6, §10-K6).
//!
//! Private key OS keychain'de saklanır. Keychain kullanılamıyorsa (tipik olarak
//! Secret Service çalışmayan bir Linux masaüstü) veri dizinindeki kısıtlı izinli
//! bir dosyaya düşülür ve bu durum kullanıcıya UI'da AÇIKÇA bildirilir — sessizce
//! daha zayıf bir korumaya geçmek kabul edilemez.

use std::path::Path;

use data_encoding::BASE32_NOPAD;
use ed25519_dalek::{SigningKey, VerifyingKey, SECRET_KEY_LENGTH};
use serde::Serialize;

use crate::paths::AppPaths;

const KEYRING_USER: &str = "device-identity";
const FALLBACK_FILE: &str = "identity.key";

/// Anahtarın nerede saklandığı. UI bunu kullanıcıya gösterir.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum KeyStorage {
    /// İşletim sisteminin kasası (Credential Manager / Keychain / Secret Service).
    OsKeychain,
    /// Kısıtlı izinli düz dosya — keychain kullanılamadığı için.
    PlainFile,
}

pub struct Identity {
    signing_key: SigningKey,
    pub storage: KeyStorage,
}

impl Identity {
    pub fn verifying_key(&self) -> VerifyingKey {
        self.signing_key.verifying_key()
    }

    /// 32 byte ham public key — protokolde `device_id`, DB'de `trusted_devices.device_id`.
    pub fn device_id(&self) -> [u8; 32] {
        self.verifying_key().to_bytes()
    }

    /// Kullanıcıya gösterilen fingerprint: base32, 4'erli gruplar hâlinde.
    /// SSH tarzı manuel doğrulama için (PLAN.md §2.6, §2.13).
    pub fn fingerprint(&self) -> String {
        format_fingerprint(&self.device_id())
    }

    /// Faz 2'de TLS sertifikası bu anahtardan türetilecek (PLAN.md §2.2.1).
    #[allow(dead_code)]
    pub fn signing_key(&self) -> &SigningKey {
        &self.signing_key
    }
}

fn format_fingerprint(public_key: &[u8; 32]) -> String {
    let encoded = BASE32_NOPAD.encode(public_key);
    encoded
        .as_bytes()
        .chunks(4)
        .map(|c| String::from_utf8_lossy(c).into_owned())
        .collect::<Vec<_>>()
        .join("-")
}

/// Kimliği yükler; yoksa üretip saklar.
pub fn load_or_create(paths: &AppPaths) -> anyhow::Result<Identity> {
    let service = paths.keyring_service();
    let fallback = paths.data_dir.join(FALLBACK_FILE);

    if let Some(identity) = load_existing(&service, &fallback)? {
        tracing::info!(
            storage = ?identity.storage,
            fingerprint = %identity.fingerprint(),
            "mevcut cihaz kimliği yüklendi"
        );
        return Ok(identity);
    }

    let signing_key = generate_key()?;
    let storage = store_key(&service, &fallback, &signing_key);
    let identity = Identity {
        signing_key,
        storage,
    };

    tracing::info!(
        storage = ?identity.storage,
        fingerprint = %identity.fingerprint(),
        "yeni cihaz kimliği üretildi"
    );
    Ok(identity)
}

fn load_existing(service: &str, fallback: &Path) -> anyhow::Result<Option<Identity>> {
    // Keychain önce: dosya fallback'i sonradan keychain çalışır hâle gelse bile
    // geride kalabilir, ama keychain'deki kayıt otoritedir.
    match keyring_get(service) {
        Ok(Some(bytes)) => {
            return Ok(Some(Identity {
                signing_key: SigningKey::from_bytes(&bytes),
                storage: KeyStorage::OsKeychain,
            }));
        }
        Ok(None) => {}
        Err(err) => tracing::warn!(error = %err, "keychain okunamadı, dosyaya bakılıyor"),
    }

    if let Some(bytes) = file_get(fallback)? {
        return Ok(Some(Identity {
            signing_key: SigningKey::from_bytes(&bytes),
            storage: KeyStorage::PlainFile,
        }));
    }

    Ok(None)
}

fn store_key(service: &str, fallback: &Path, key: &SigningKey) -> KeyStorage {
    match keyring_set(service, key.as_bytes()) {
        Ok(()) => KeyStorage::OsKeychain,
        Err(err) => {
            tracing::warn!(
                error = %err,
                "anahtar sistem kasasına yazılamadı, kısıtlı izinli dosyaya düşülüyor"
            );
            if let Err(err) = file_set(fallback, key.as_bytes()) {
                // Anahtar hiç saklanamadıysa uygulama çalışmaya devam eder ama
                // her açılışta kimlik değişir; bu, eşleşmeleri geçersiz kılar.
                tracing::error!(error = %err, "anahtar dosyaya da yazılamadı");
            }
            KeyStorage::PlainFile
        }
    }
}

fn generate_key() -> anyhow::Result<SigningKey> {
    // `rand` sürüm uyumsuzluklarına girmemek için doğrudan OS entropisi.
    let mut secret = [0u8; SECRET_KEY_LENGTH];
    getrandom::fill(&mut secret)?;
    Ok(SigningKey::from_bytes(&secret))
}

fn keyring_get(service: &str) -> anyhow::Result<Option<[u8; SECRET_KEY_LENGTH]>> {
    let entry = keyring::Entry::new(service, KEYRING_USER)?;
    match entry.get_password() {
        Ok(encoded) => Ok(Some(decode_secret(&encoded)?)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(err) => Err(err.into()),
    }
}

fn keyring_set(service: &str, secret: &[u8; SECRET_KEY_LENGTH]) -> anyhow::Result<()> {
    let entry = keyring::Entry::new(service, KEYRING_USER)?;
    entry.set_password(&BASE32_NOPAD.encode(secret))?;
    Ok(())
}

fn file_get(path: &Path) -> anyhow::Result<Option<[u8; SECRET_KEY_LENGTH]>> {
    match std::fs::read_to_string(path) {
        Ok(contents) => Ok(Some(decode_secret(contents.trim())?)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err.into()),
    }
}

fn file_set(path: &Path, secret: &[u8; SECRET_KEY_LENGTH]) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, BASE32_NOPAD.encode(secret))?;
    restrict_permissions(path)?;
    Ok(())
}

#[cfg(unix)]
fn restrict_permissions(path: &Path) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    Ok(())
}

#[cfg(not(unix))]
fn restrict_permissions(_path: &Path) -> anyhow::Result<()> {
    // Windows'ta dosya, kullanıcının profil dizininde (%APPDATA%) oluşur ve
    // varsayılan ACL'i zaten kullanıcıyla sınırlıdır.
    Ok(())
}

fn decode_secret(encoded: &str) -> anyhow::Result<[u8; SECRET_KEY_LENGTH]> {
    let bytes = BASE32_NOPAD.decode(encoded.as_bytes())?;
    let sized: [u8; SECRET_KEY_LENGTH] = bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("kimlik anahtarı bozuk: beklenen uzunluk 32 byte"))?;
    Ok(sized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_gruplu_ve_kararli() {
        let key = [7u8; 32];
        let fp = format_fingerprint(&key);
        assert_eq!(
            fp,
            format_fingerprint(&key),
            "aynı anahtar aynı fingerprint"
        );
        assert!(fp.contains('-'), "gruplandırılmış olmalı");
        // 32 byte → 52 base32 karakteri → 13 grup, 12 tire.
        assert_eq!(fp.matches('-').count(), 12);
        assert_eq!(fp.replace('-', "").len(), 52);
    }

    #[test]
    fn farkli_anahtar_farkli_fingerprint() {
        assert_ne!(
            format_fingerprint(&[1u8; 32]),
            format_fingerprint(&[2u8; 32])
        );
    }

    #[test]
    fn gizli_anahtar_kodlanip_cozulur() {
        let secret = [42u8; SECRET_KEY_LENGTH];
        let encoded = BASE32_NOPAD.encode(&secret);
        assert_eq!(decode_secret(&encoded).unwrap(), secret);
    }

    #[test]
    fn bozuk_anahtar_reddedilir() {
        assert!(decode_secret("KISA").is_err());
        assert!(decode_secret("bu base32 değil!").is_err());
    }

    #[test]
    fn uretilen_anahtarlar_farkli() {
        let a = generate_key().unwrap();
        let b = generate_key().unwrap();
        assert_ne!(a.to_bytes(), b.to_bytes());
    }

    #[test]
    fn dosyaya_yazilan_anahtar_geri_okunur() {
        let dir = std::env::temp_dir().join(format!("linkup-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("identity.key");

        assert!(
            file_get(&path).unwrap().is_none(),
            "dosya yokken None dönmeli"
        );

        let secret = generate_key().unwrap().to_bytes();
        file_set(&path, &secret).unwrap();
        assert_eq!(file_get(&path).unwrap().unwrap(), secret);

        std::fs::remove_dir_all(&dir).ok();
    }
}
