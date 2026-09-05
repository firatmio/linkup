//! TLS kimlik bağlama (PLAN.md §2.2.1).
//!
//! P2P'de sertifika otoritesi yoktur. Bunun yerine:
//!
//! - Her cihaz, kendi Ed25519 kimlik anahtarından self-signed bir sertifika
//!   üretir. Sertifikanın public key'i = cihazın `device_id`'si. Böylece
//!   "TLS peer" ile "LinkUp cihazı" iki ayrı şey olmaktan çıkar.
//! - Doğrulayıcılar CA zinciri aramaz; sertifikadaki public key'i pinlenmiş
//!   anahtarla karşılaştırır.
//! - Karşılıklı TLS zorunludur: iki taraf da sertifika sunar.
//!
//! Eşleşmemiş bir cihazla bağlantıda TLS katmanı yapısal doğrulama yapar
//! (geçerli Ed25519 sertifikası mı), kimlik kararı el sıkışmadan SONRA
//! `peer_device_id()` ile verilir.

use std::sync::Arc;

use ed25519_dalek::pkcs8::EncodePrivateKey;
use ed25519_dalek::SigningKey;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName, UnixTime};
use rustls::server::danger::{ClientCertVerified, ClientCertVerifier};
use rustls::{DigitallySignedStruct, DistinguishedName, SignatureScheme};

/// SNI olarak kullanılan sabit ad. P2P'de alan adı yok; doğrulama public key
/// üzerinden yapıldığı için bu ad yalnızca TLS'in biçimsel gereksinimini
/// karşılar ve doğrulamada KULLANILMAZ.
pub const SERVER_NAME: &str = "linkup.local";

/// QUIC ALPN protokol tanımlayıcısı. Sürüm burada da görünür: ileride kırıcı
/// bir değişiklik olursa farklı ALPN ile eski istemciler el sıkışamaz.
pub const ALPN: &[u8] = b"linkup/1";

#[derive(Debug, thiserror::Error)]
pub enum TlsError {
    #[error("sertifika üretilemedi: {0}")]
    Generate(String),

    #[error("sertifika çözümlenemedi: {0}")]
    Parse(String),

    #[error("sertifikadaki anahtar Ed25519 değil")]
    NotEd25519,

    #[error("rustls yapılandırması: {0}")]
    Rustls(#[from] rustls::Error),
}

/// Cihazın kimlik anahtarından türetilmiş TLS malzemesi.
pub struct TlsIdentity {
    pub certificate: CertificateDer<'static>,
    pub private_key: PrivateKeyDer<'static>,
    pub device_id: [u8; 32],
}

/// Kimlik anahtarından self-signed Ed25519 sertifikası üretir.
///
/// Aynı anahtardan üretilen sertifikaların public key'i her zaman aynıdır —
/// pinleme buna dayanır. (Seri numarası ve geçerlilik tarihleri sertifikadan
/// sertifikaya değişebilir; pinlenen şey sertifikanın kendisi değil, içindeki
/// public key'dir.)
pub fn derive_identity(signing_key: &SigningKey) -> Result<TlsIdentity, TlsError> {
    let device_id = signing_key.verifying_key().to_bytes();

    let pkcs8 = signing_key
        .to_pkcs8_der()
        .map_err(|e| TlsError::Generate(format!("pkcs8: {e}")))?;
    let pkcs8_der = PrivatePkcs8KeyDer::from(pkcs8.as_bytes().to_vec());

    let key_pair = rcgen::KeyPair::from_pkcs8_der_and_sign_algo(&pkcs8_der, &rcgen::PKCS_ED25519)
        .map_err(|e| TlsError::Generate(format!("anahtar çifti: {e}")))?;

    let mut params = rcgen::CertificateParams::new(vec![SERVER_NAME.to_string()])
        .map_err(|e| TlsError::Generate(format!("parametreler: {e}")))?;
    params.distinguished_name = rcgen::DistinguishedName::new();
    params
        .distinguished_name
        .push(rcgen::DnType::CommonName, SERVER_NAME);

    let certificate = params
        .self_signed(&key_pair)
        .map_err(|e| TlsError::Generate(format!("imzalama: {e}")))?;

    Ok(TlsIdentity {
        certificate: certificate.der().clone(),
        private_key: PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(pkcs8.as_bytes().to_vec())),
        device_id,
    })
}

/// Sertifikadan 32 byte Ed25519 public key'i (yani `device_id`'yi) çıkarır.
pub fn device_id_from_certificate(cert: &CertificateDer<'_>) -> Result<[u8; 32], TlsError> {
    use x509_parser::prelude::FromDer;

    let (_, parsed) = x509_parser::certificate::X509Certificate::from_der(cert.as_ref())
        .map_err(|e| TlsError::Parse(e.to_string()))?;

    let spki = parsed.public_key();
    if spki.algorithm.algorithm != x509_parser::oid_registry::OID_SIG_ED25519 {
        return Err(TlsError::NotEd25519);
    }

    spki.subject_public_key
        .data
        .as_ref()
        .try_into()
        .map_err(|_| TlsError::NotEd25519)
}

/// Hem sunucu hem istemci sertifikalarını doğrulayan pinleme doğrulayıcısı.
///
/// `expected` verilmişse sertifikadaki public key ona eşit olmalıdır
/// (eşleşmiş cihaza bağlanma). `None` ise yapısal doğrulama yapılır ve kimlik
/// kararı el sıkışma sonrasına bırakılır (eşleştirme akışı).
#[derive(Debug)]
struct PinnedKeyVerifier {
    expected: Option<[u8; 32]>,
    provider: Arc<rustls::crypto::CryptoProvider>,
}

impl PinnedKeyVerifier {
    fn new(expected: Option<[u8; 32]>) -> Self {
        Self {
            expected,
            provider: Arc::new(rustls::crypto::ring::default_provider()),
        }
    }

    fn check(&self, cert: &CertificateDer<'_>) -> Result<(), rustls::Error> {
        let presented = device_id_from_certificate(cert)
            .map_err(|e| rustls::Error::General(format!("sertifika reddedildi: {e}")))?;

        match self.expected {
            Some(expected) if presented != expected => Err(rustls::Error::General(
                "sertifikadaki cihaz kimliği beklenenle eşleşmiyor".to_string(),
            )),
            _ => Ok(()),
        }
    }

    fn schemes(&self) -> Vec<SignatureScheme> {
        vec![SignatureScheme::ED25519]
    }

    fn verify_tls13(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }
}

impl ServerCertVerifier for PinnedKeyVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        // Alan adı ve geçerlilik tarihi doğrulaması KASTEN yapılmaz: P2P'de
        // isim yoktur ve sertifikayı üreten cihazın saati güvenilmezdir.
        // Güvenin dayanağı public key pinlemesidir.
        self.check(end_entity)?;
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        // QUIC yalnızca TLS 1.3 kullanır; buraya düşülmesi bir hatadır.
        Err(rustls::Error::PeerIncompatible(
            rustls::PeerIncompatible::Tls12NotOffered,
        ))
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        self.verify_tls13(message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.schemes()
    }
}

impl ClientCertVerifier for PinnedKeyVerifier {
    fn root_hint_subjects(&self) -> &[DistinguishedName] {
        &[]
    }

    fn verify_client_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _now: UnixTime,
    ) -> Result<ClientCertVerified, rustls::Error> {
        self.check(end_entity)?;
        Ok(ClientCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Err(rustls::Error::PeerIncompatible(
            rustls::PeerIncompatible::Tls12NotOffered,
        ))
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        self.verify_tls13(message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.schemes()
    }
}

/// Gelen bağlantılar için rustls yapılandırması (mTLS zorunlu).
pub fn server_config(identity: &TlsIdentity) -> Result<rustls::ServerConfig, TlsError> {
    // Sunucu tarafında pinleme yapılamaz: kimin bağlandığını sertifikayı
    // görmeden bilemeyiz. Yapısal doğrulama burada, kimlik kararı el
    // sıkışmadan sonra (PLAN.md §2.2.1 madde 4).
    let verifier = Arc::new(PinnedKeyVerifier::new(None));

    let mut config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_protocol_versions(&[&rustls::version::TLS13])?
    .with_client_cert_verifier(verifier)
    .with_single_cert(
        vec![identity.certificate.clone()],
        identity.private_key.clone_key(),
    )?;

    config.alpn_protocols = vec![ALPN.to_vec()];
    Ok(config)
}

/// Giden bağlantılar için rustls yapılandırması.
///
/// `expected_peer`: eşleşmiş bir cihaza bağlanıyorsak onun `device_id`'si;
/// eşleştirme akışındaysak `None`.
pub fn client_config(
    identity: &TlsIdentity,
    expected_peer: Option<[u8; 32]>,
) -> Result<rustls::ClientConfig, TlsError> {
    let verifier = Arc::new(PinnedKeyVerifier::new(expected_peer));

    let mut config = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_protocol_versions(&[&rustls::version::TLS13])?
    .dangerous()
    .with_custom_certificate_verifier(verifier)
    .with_client_auth_cert(
        vec![identity.certificate.clone()],
        identity.private_key.clone_key(),
    )?;

    config.alpn_protocols = vec![ALPN.to_vec()];
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(seed: u8) -> SigningKey {
        SigningKey::from_bytes(&[seed; 32])
    }

    #[test]
    fn sertifikanin_anahtari_cihaz_kimligidir() {
        let signing = key(1);
        let identity = derive_identity(&signing).unwrap();

        assert_eq!(identity.device_id, signing.verifying_key().to_bytes());
        assert_eq!(
            device_id_from_certificate(&identity.certificate).unwrap(),
            identity.device_id,
            "sertifikadan okunan anahtar kimlik anahtarıyla aynı olmalı"
        );
    }

    #[test]
    fn farkli_anahtar_farkli_cihaz_kimligi() {
        let a = derive_identity(&key(1)).unwrap();
        let b = derive_identity(&key(2)).unwrap();
        assert_ne!(a.device_id, b.device_id);
    }

    #[test]
    fn pinlenmis_dogrulayici_eslesen_anahtari_kabul_eder() {
        let identity = derive_identity(&key(5)).unwrap();
        let verifier = PinnedKeyVerifier::new(Some(identity.device_id));
        assert!(verifier.check(&identity.certificate).is_ok());
    }

    #[test]
    fn pinlenmis_dogrulayici_baska_anahtari_reddeder() {
        let ours = derive_identity(&key(5)).unwrap();
        let attacker = derive_identity(&key(6)).unwrap();

        // Saldırgan kendi geçerli sertifikasını sunuyor — pinleme onu tutmalı.
        let verifier = PinnedKeyVerifier::new(Some(ours.device_id));
        assert!(verifier.check(&attacker.certificate).is_err());
    }

    #[test]
    fn pinsiz_dogrulayici_gecerli_sertifikayi_kabul_eder() {
        let identity = derive_identity(&key(7)).unwrap();
        let verifier = PinnedKeyVerifier::new(None);
        assert!(verifier.check(&identity.certificate).is_ok());
    }

    #[test]
    fn bozuk_sertifika_reddedilir() {
        let verifier = PinnedKeyVerifier::new(None);
        let garbage = CertificateDer::from(vec![0u8; 32]);
        assert!(verifier.check(&garbage).is_err());
        assert!(device_id_from_certificate(&garbage).is_err());
    }

    #[test]
    fn yapilandirmalar_alpn_tasir() {
        let identity = derive_identity(&key(9)).unwrap();
        assert_eq!(
            server_config(&identity).unwrap().alpn_protocols,
            vec![ALPN.to_vec()]
        );
        assert_eq!(
            client_config(&identity, None).unwrap().alpn_protocols,
            vec![ALPN.to_vec()]
        );
    }
}
