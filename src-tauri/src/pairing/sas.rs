//! Karşılıklı doğrulama kodu — SAS (PLAN.md §2.5, §10-K2).
//!
//! İki cihazda da aynı 6 haneli kod gösterilir ve kullanıcıdan "bu kod diğer
//! ekranda da aynı mı?" onayı istenir. Kodun MITM'e karşı koruması iki şeye
//! dayanır:
//!
//! 1. **Her iki tarafın public key'i** hesaba girer — saldırgan kendi
//!    anahtarını araya sokarsa kod değişir.
//! 2. **TLS exporter** (RFC 5705) hesaba girer — bu, kodu somut TLS oturumuna
//!    bağlar (channel binding). Saldırgan iki ayrı TLS oturumu kurup mesajları
//!    aktarırsa iki tarafta farklı exporter, dolayısıyla farklı kod çıkar.
//!
//! İkincisi olmadan saldırgan mesajları relay ederek aynı kodu iki tarafa da
//! gösterebilirdi; asıl koruma oradadır.

use blake3::Hasher;

/// Alan ayracı: aynı hash fonksiyonunun başka bir amaçla üretilmiş çıktısı
/// buraya karışmasın.
const DOMAIN: &[u8] = b"LinkUp-SAS-v1";

/// TLS exporter etiketi (RFC 5705).
pub const EXPORTER_LABEL: &[u8] = b"EXPORTER-LinkUp-pairing";

/// Exporter'dan çekilecek bayt sayısı.
pub const EXPORTER_LEN: usize = 32;

/// Kod uzunluğu. 6 hane ≈ 20 bit: kaba kuvvetle tutturma şansı milyonda bir
/// ve kod tek kullanımlık, 90 saniye geçerli.
const DIGITS: u32 = 6;

/// İki cihazın ortak doğrulama kodunu hesaplar.
///
/// Anahtarlar sıralanarak karıştırılır: hangi tarafın "başlatan" olduğundan
/// bağımsız olarak iki cihaz aynı kodu bulmalıdır.
pub fn compute(local_key: &[u8; 32], remote_key: &[u8; 32], exporter: &[u8]) -> String {
    let (first, second) = if local_key <= remote_key {
        (local_key, remote_key)
    } else {
        (remote_key, local_key)
    };

    let mut hasher = Hasher::new();
    hasher.update(DOMAIN);
    hasher.update(first);
    hasher.update(second);
    hasher.update(exporter);

    let digest = hasher.finalize();
    let bytes = digest.as_bytes();

    // İlk 4 baytı sayıya çevirip 10^6 ile modunu al.
    let raw = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    let modulus = 10u32.pow(DIGITS);
    format!("{:0width$}", raw % modulus, width = DIGITS as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXPORTER: &[u8] = &[9u8; EXPORTER_LEN];

    #[test]
    fn iki_taraf_ayni_kodu_bulur() {
        let a = [1u8; 32];
        let b = [2u8; 32];
        // Başlatan taraf kim olursa olsun kod aynı olmalı.
        assert_eq!(compute(&a, &b, EXPORTER), compute(&b, &a, EXPORTER));
    }

    #[test]
    fn kod_alti_haneli() {
        let code = compute(&[1u8; 32], &[2u8; 32], EXPORTER);
        assert_eq!(code.len(), 6);
        assert!(code.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn kucuk_sayilar_sifirla_doldurulur() {
        // Biçimlendirme, baştaki sıfırları kırpmamalı — "042137" ile "42137"
        // farklı kodlardır ve kullanıcı karşılaştırması buna dayanır.
        let padded = format!("{:0width$}", 42u32, width = DIGITS as usize);
        assert_eq!(padded, "000042");
    }

    /// MITM koruması: saldırgan araya kendi anahtarını soksa kod değişir.
    #[test]
    fn farkli_anahtar_farkli_kod() {
        let honest = compute(&[1u8; 32], &[2u8; 32], EXPORTER);
        let attacker = compute(&[1u8; 32], &[99u8; 32], EXPORTER);
        assert_ne!(honest, attacker);
    }

    /// Channel binding'in özü: aynı anahtar çifti, farklı TLS oturumu →
    /// farklı kod. Saldırgan iki ayrı oturum kurup mesajları aktaramaz.
    #[test]
    fn farkli_oturum_farkli_kod() {
        let a = [1u8; 32];
        let b = [2u8; 32];
        let session_one = compute(&a, &b, &[1u8; EXPORTER_LEN]);
        let session_two = compute(&a, &b, &[2u8; EXPORTER_LEN]);
        assert_ne!(
            session_one, session_two,
            "exporter değişince kod da değişmeli — MITM koruması buna dayanıyor"
        );
    }

    #[test]
    fn ayni_girdi_ayni_kodu_uretir() {
        let a = [7u8; 32];
        let b = [8u8; 32];
        assert_eq!(compute(&a, &b, EXPORTER), compute(&a, &b, EXPORTER));
    }

    /// Kodlar sayı uzayına makul biçimde dağılmalı; hepsi aynı çıkarsa
    /// karşılaştırma anlamsızlaşır.
    #[test]
    fn kodlar_dagilir() {
        let mut seen = std::collections::HashSet::new();
        for i in 0..200u8 {
            seen.insert(compute(&[1u8; 32], &[i; 32], EXPORTER));
        }
        assert!(
            seen.len() > 190,
            "kodlar çakışmamalı: {} farklı",
            seen.len()
        );
    }
}
