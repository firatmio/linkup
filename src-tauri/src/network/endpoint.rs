//! QUIC endpoint: bağlanma, kabul etme ve el sıkışma (PLAN.md §2.2).

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use ed25519_dalek::SigningKey;
use quinn::crypto::rustls::{QuicClientConfig, QuicServerConfig};
use quinn::{Connection, Endpoint, RecvStream, SendStream, TransportConfig, VarInt};

use super::protocol::{
    read_frame, write_frame, ControlMessage, Hello, ProtocolError, VersionMismatch, WireError,
};
use super::tls::{self, TlsIdentity};

// Akış kontrolü ve canlılık ayarları (PLAN.md §2.2.2).
//
// ÖLÇÜM NOTU: Bu değerlerin loopback throughput'una ölçülebilir etkisi YOKTUR
// (tuning'li 2203 Mbit/s, tuning'siz 2204 Mbit/s). Loopback'te darboğaz
// pencereler değil CPU'dur. Değerlerin gerekçesi aşağıda tek tek yazılı;
// hiçbiri "daha hızlı olsun diye" konmuş değildir.

/// Varsayılan 1,25 MB. Tek bir akışın yüksek gecikmeli yolda (v2'de relay
/// üzerinden internet) pencereye takılmaması için yükseltildi. LAN'da fark
/// yaratmaz; zararı da yok.
const STREAM_RECEIVE_WINDOW: u64 = 8 * 1024 * 1024;

/// Varsayılan pratikte SINIRSIZ. Burada bilinçli olarak sınırlandırılıyor:
/// bu bir hız ayarı değil, BELLEK SINIRI. Sınırsız bırakılırsa kötü niyetli
/// bir eş, aynı anda çok sayıda akış açıp bizi keyfi miktarda tamponlamaya
/// zorlayabilir. 32 MB, eşzamanlı 3 dosya transferi (§2.7.4) için fazlasıyla
/// yeterli.
const CONNECTION_WINDOW: u64 = 32 * 1024 * 1024;

/// Varsayılan 10 MB. Gönderim tarafında karşılık gelen tampon.
const SEND_WINDOW: u64 = 32 * 1024 * 1024;

/// Varsayılan 100. Düşürülmesi bir DoS sınırıdır: uygulama tasarımı gereği
/// bağlantı başına yalnızca 1 kontrol akışı kullanılır (§2.2.3), 64 fazlasıyla
/// yeterli bir tavandır.
const MAX_CONCURRENT_BIDI_STREAMS: u32 = 64;

/// Varsayılan KAPALI. NAT ve router eşleşmelerinin zaman aşımına uğramaması
/// için gerekli — v2'de internet üzerinden bağlantının ön koşulu.
const KEEP_ALIVE: Duration = Duration::from_secs(5);

/// Kopan bağlantının tespit süresi. Keep-alive'dan belirgin şekilde uzun
/// olmalı, yoksa sağlıklı bağlantılar da düşer.
const MAX_IDLE: Duration = Duration::from_secs(20);

// Aşağıdaki ilişkiler bozulursa derleme durur — bunlar hız ayarı değil,
// bellek ve canlılık sınırları (PLAN.md §2.2.2).
const _: () = assert!(
    CONNECTION_WINDOW >= STREAM_RECEIVE_WINDOW * 3,
    "eşzamanlı 3 transfer tam pencereyle sığabilmeli (§2.7.4)"
);
const _: () = assert!(
    KEEP_ALIVE.as_secs() * 2 < MAX_IDLE.as_secs(),
    "keep-alive, idle timeout'un belirgin şekilde altında olmalı"
);

/// El sıkışmanın tamamlanması için üst sınır. Sessiz bir karşı taraf
/// bağlantıyı süresiz açık tutamamalı.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
    #[error("tls: {0}")]
    Tls(#[from] tls::TlsError),

    #[error("soket bağlanamadı: {0}")]
    Bind(#[source] std::io::Error),

    #[error("bağlantı kurulamadı: {0}")]
    Connect(String),

    #[error("protokol: {0}")]
    Wire(#[from] WireError),

    #[error("karşı taraf sertifika sunmadı")]
    NoPeerCertificate,

    #[error("karşı tarafın bildirdiği cihaz kimliği sertifikasıyla uyuşmuyor")]
    IdentityMismatch,

    #[error("sürüm uyumsuz: {0:?}")]
    IncompatibleVersion(VersionMismatch),

    #[error("el sıkışma zaman aşımına uğradı")]
    HandshakeTimeout,

    #[error("beklenmeyen mesaj alındı")]
    UnexpectedMessage,
}

fn transport_config() -> TransportConfig {
    let mut transport = TransportConfig::default();
    transport.stream_receive_window(VarInt::from_u64(STREAM_RECEIVE_WINDOW).unwrap());
    transport.receive_window(VarInt::from_u64(CONNECTION_WINDOW).unwrap());
    transport.send_window(SEND_WINDOW);
    transport.max_concurrent_bidi_streams(VarInt::from_u32(MAX_CONCURRENT_BIDI_STREAMS));
    transport.keep_alive_interval(Some(KEEP_ALIVE));
    transport.max_idle_timeout(Some(MAX_IDLE.try_into().expect("geçerli idle timeout")));
    transport
}

/// Bu cihazın QUIC uç noktası. Aynı soket hem gelen hem giden bağlantılar
/// için kullanılır (NAT traversal'ın v2'de çalışabilmesi buna bağlı).
pub struct NetworkEndpoint {
    endpoint: Endpoint,
    identity: Arc<TlsIdentity>,
    device_name: String,
}

impl NetworkEndpoint {
    pub fn bind(
        signing_key: &SigningKey,
        device_name: String,
        addr: SocketAddr,
    ) -> Result<Self, NetworkError> {
        let identity = tls::derive_identity(signing_key)?;

        let quic_server = QuicServerConfig::try_from(tls::server_config(&identity)?)
            .map_err(|e| NetworkError::Connect(format!("sunucu kripto yapılandırması: {e}")))?;
        let mut server_config = quinn::ServerConfig::with_crypto(Arc::new(quic_server));
        server_config.transport_config(Arc::new(transport_config()));

        let endpoint = Endpoint::server(server_config, addr).map_err(NetworkError::Bind)?;

        tracing::info!(addr = %endpoint.local_addr().map_err(NetworkError::Bind)?, "QUIC uç noktası açıldı");

        Ok(Self {
            endpoint,
            identity: Arc::new(identity),
            device_name,
        })
    }

    pub fn local_addr(&self) -> std::io::Result<SocketAddr> {
        self.endpoint.local_addr()
    }

    /// Faz 3'te mDNS ilanında ve Faz 4'te eşleştirmede kullanılacak.
    pub fn device_id(&self) -> [u8; 32] {
        self.identity.device_id
    }

    /// Bir cihaza bağlanır ve el sıkışmayı tamamlar.
    ///
    /// `expected_peer`: eşleşmiş cihaza bağlanıyorsak `device_id`'si (TLS
    /// katmanında pinlenir); eşleştirme akışındaysak `None`.
    /// Faz 3 keşif adresleri sağladığında çağrılacak; şu an yalnızca testler
    /// kullanıyor.
    pub async fn connect(
        &self,
        addr: SocketAddr,
        expected_peer: Option<[u8; 32]>,
    ) -> Result<PeerConnection, NetworkError> {
        let client_crypto = tls::client_config(&self.identity, expected_peer)?;
        let quic_client = QuicClientConfig::try_from(client_crypto)
            .map_err(|e| NetworkError::Connect(format!("istemci kripto yapılandırması: {e}")))?;
        let mut client_config = quinn::ClientConfig::new(Arc::new(quic_client));
        client_config.transport_config(Arc::new(transport_config()));

        let connection = self
            .endpoint
            .connect_with(client_config, addr, tls::SERVER_NAME)
            .map_err(|e| NetworkError::Connect(e.to_string()))?
            .await
            .map_err(|e| NetworkError::Connect(e.to_string()))?;

        with_timeout(PeerConnection::initiate(
            connection,
            self.device_name.clone(),
            self.identity.device_id,
        ))
        .await
    }

    /// Gelen bir bağlantıyı kabul eder ve el sıkışmayı tamamlar.
    /// Endpoint kapandığında `None` döner.
    pub async fn accept(&self) -> Option<Result<PeerConnection, NetworkError>> {
        let incoming = self.endpoint.accept().await?;
        let device_name = self.device_name.clone();
        let device_id = self.identity.device_id;

        Some(match incoming.await {
            Ok(connection) => {
                with_timeout(PeerConnection::respond(connection, device_name, device_id)).await
            }
            Err(err) => Err(NetworkError::Connect(err.to_string())),
        })
    }

    pub fn close(&self) {
        self.endpoint.close(VarInt::from_u32(0), b"kapaniyor");
    }
}

async fn with_timeout<F>(future: F) -> Result<PeerConnection, NetworkError>
where
    F: std::future::Future<Output = Result<PeerConnection, NetworkError>>,
{
    tokio::time::timeout(HANDSHAKE_TIMEOUT, future)
        .await
        .unwrap_or(Err(NetworkError::HandshakeTimeout))
}

/// El sıkışması tamamlanmış bir bağlantı.
/// Alanların bir kısmı Faz 4'te (eşleştirme) ve Faz 5'te (chat) okunacak;
/// kontrol akışı şu an yalnızca el sıkışma ve heartbeat için kullanılıyor.
pub struct PeerConnection {
    connection: Connection,
    control_send: SendStream,
    control_recv: RecvStream,
    /// Karşı tarafın TLS sertifikasından okunan kimlik. Kaynağı sertifikadır,
    /// karşı tarafın beyanı değil — güvenilecek olan budur.
    pub peer_device_id: [u8; 32],
    pub peer: Hello,
    pub negotiated_version: u16,
}

impl PeerConnection {
    /// Bağlantıyı başlatan taraf: kontrol stream'ini açar, `Hello` gönderir,
    /// `HelloAck` bekler.
    async fn initiate(
        connection: Connection,
        device_name: String,
        device_id: [u8; 32],
    ) -> Result<Self, NetworkError> {
        let peer_device_id = peer_device_id(&connection)?;
        let (mut send, mut recv) = connection
            .open_bi()
            .await
            .map_err(|e| NetworkError::Connect(e.to_string()))?;

        let hello = Hello::new(device_name, device_id);
        write_frame(&mut send, &ControlMessage::Hello(hello)).await?;

        let peer = match read_frame(&mut recv).await? {
            ControlMessage::HelloAck(hello) => hello,
            ControlMessage::Error(_) => return Err(NetworkError::UnexpectedMessage),
            _ => return Err(NetworkError::UnexpectedMessage),
        };

        Self::finish(connection, send, recv, peer, peer_device_id)
    }

    /// Bağlantıyı kabul eden taraf: kontrol stream'ini bekler, `Hello` okur,
    /// sürümü doğrular, `HelloAck` yollar.
    async fn respond(
        connection: Connection,
        device_name: String,
        device_id: [u8; 32],
    ) -> Result<Self, NetworkError> {
        let peer_device_id = peer_device_id(&connection)?;
        let (mut send, mut recv) = connection
            .accept_bi()
            .await
            .map_err(|e| NetworkError::Connect(e.to_string()))?;

        let peer = match read_frame(&mut recv).await? {
            ControlMessage::Hello(hello) => hello,
            _ => {
                write_frame(
                    &mut send,
                    &ControlMessage::Error(ProtocolError::UnexpectedMessage),
                )
                .await?;
                return Err(NetworkError::UnexpectedMessage);
            }
        };

        // Uyumsuzluğu karşı tarafa bildir: sessizce kapanmak, kullanıcıya
        // "neden bağlanamıyorum" sorusunu cevapsız bırakır.
        if let Err(mismatch) = peer.negotiate() {
            write_frame(
                &mut send,
                &ControlMessage::Error(ProtocolError::IncompatibleVersion),
            )
            .await?;
            return Err(NetworkError::IncompatibleVersion(mismatch));
        }

        let ours = Hello::new(device_name, device_id);
        write_frame(&mut send, &ControlMessage::HelloAck(ours)).await?;

        Self::finish(connection, send, recv, peer, peer_device_id)
    }

    fn finish(
        connection: Connection,
        control_send: SendStream,
        control_recv: RecvStream,
        peer: Hello,
        peer_device_id: [u8; 32],
    ) -> Result<Self, NetworkError> {
        // Karşı taraf `Hello` içinde istediği kimliği beyan edebilir; sertifika
        // ise yalanlayamaz. İkisi ayrışıyorsa bağlantı sahtedir.
        if peer.device_id != peer_device_id {
            tracing::warn!("Hello'daki cihaz kimliği sertifikayla uyuşmuyor, bağlantı reddedildi");
            return Err(NetworkError::IdentityMismatch);
        }

        let negotiated_version = peer
            .negotiate()
            .map_err(NetworkError::IncompatibleVersion)?;

        tracing::info!(
            peer = %peer.device_name,
            version = negotiated_version,
            "el sıkışma tamamlandı"
        );

        Ok(Self {
            connection,
            control_send,
            control_recv,
            peer_device_id,
            peer,
            negotiated_version,
        })
    }

    pub fn remote_address(&self) -> SocketAddr {
        self.connection.remote_address()
    }

    pub fn connection(&self) -> &Connection {
        &self.connection
    }

    /// Canlılık yoklaması. Gidiş-dönüş süresini döndürür.
    pub async fn heartbeat(&mut self) -> Result<Duration, NetworkError> {
        let nonce = rand_nonce();
        let started = std::time::Instant::now();

        write_frame(&mut self.control_send, &ControlMessage::Heartbeat { nonce }).await?;

        match read_frame(&mut self.control_recv).await? {
            // Nonce eşleşmesi şart: eski bir yanıtı yeni bir yoklamanın cevabı
            // sanmak, ölü bir bağlantıyı canlı göstermeye yeter.
            ControlMessage::HeartbeatAck { nonce: echoed } if echoed == nonce => {
                Ok(started.elapsed())
            }
            _ => Err(NetworkError::UnexpectedMessage),
        }
    }

    /// Kontrol stream'inden gelen bir mesajı işler. `Heartbeat` yerinde
    /// yanıtlanır; diğerleri çağırana döner.
    pub async fn next_control_message(&mut self) -> Result<ControlMessage, NetworkError> {
        loop {
            let message = read_frame(&mut self.control_recv).await?;
            if let ControlMessage::Heartbeat { nonce } = message {
                write_frame(
                    &mut self.control_send,
                    &ControlMessage::HeartbeatAck { nonce },
                )
                .await?;
                continue;
            }
            return Ok(message);
        }
    }

    pub fn close(&self) {
        self.connection.close(VarInt::from_u32(0), b"kapaniyor");
    }
}

fn peer_device_id(connection: &Connection) -> Result<[u8; 32], NetworkError> {
    let identity = connection
        .peer_identity()
        .ok_or(NetworkError::NoPeerCertificate)?;
    let certs = identity
        .downcast::<Vec<rustls::pki_types::CertificateDer<'static>>>()
        .map_err(|_| NetworkError::NoPeerCertificate)?;
    let first = certs.first().ok_or(NetworkError::NoPeerCertificate)?;
    Ok(tls::device_id_from_certificate(first)?)
}

fn rand_nonce() -> u64 {
    let mut bytes = [0u8; 8];
    // Nonce'un gizli olması gerekmiyor, tekrarlamaması yeterli.
    getrandom::fill(&mut bytes).expect("işletim sistemi entropisi");
    u64::from_le_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transport_config_kurulabilir() {
        let _ = transport_config();
    }
}
