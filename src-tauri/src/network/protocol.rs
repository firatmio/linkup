//! Uygulama protokolü: çerçeveleme, mesaj tipleri, sürüm anlaşması
//! (PLAN.md §2.3).

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Bu derlemenin konuştuğu protokol sürümü.
pub const PROTOCOL_VERSION: u16 = 1;
/// Konuşabildiğimiz en eski sürüm. Kırıcı bir değişiklikte ikisi de artar.
pub const MIN_SUPPORTED_VERSION: u16 = 1;

/// Kontrol çerçevesi üst sınırı (PLAN.md §2.13.2 — DoS koruması).
pub const MAX_FRAME_LEN: u32 = 1024 * 1024;

/// Yetenek bayrakları. Yeni özellikler eski istemcileri kırmadan eklenebilsin
/// diye (PLAN.md §2.3.2).
pub mod capabilities {
    pub const FOLDER_SYNC: u32 = 1 << 0;
    pub const CLIPBOARD: u32 = 1 << 1;

    /// Bu derlemenin desteklediği yetenekler. Faz 9/10'da bayraklar eklenecek.
    pub const CURRENT: u32 = 0;
}

/// Kontrol stream'inde akan mesajlar (PLAN.md §2.3.3).
///
/// **Varyant sırası wire formatının parçasıdır.** postcard, varyant indeksini
/// varint olarak yazar; mevcut varyantların sırası ASLA değişmez, yenileri
/// yalnızca SONA eklenir. Aksi hâlde farklı sürümler birbirini yanlış okur.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlMessage {
    Hello(Hello),
    HelloAck(Hello),
    Heartbeat {
        nonce: u64,
    },
    HeartbeatAck {
        nonce: u64,
    },
    Error(ProtocolError),
    /// Eşleştirme başlatma isteği (PLAN.md §2.5).
    PairingRequest,
    /// Kullanıcı kodu onayladı. Eşleşme ancak İKİ taraf da onaylarsa tamamlanır.
    PairingConfirm,
    /// Kullanıcı reddetti veya süre doldu.
    PairingReject,
    /// Sohbet mesajı (PLAN.md §2.8).
    Chat(ChatMessage),
    /// Mesaj karşı tarafa ulaştı.
    ChatAck {
        msg_id: String,
    },
    /// Karşı taraf mesajları görüntüledi.
    ReadReceipt {
        msg_ids: Vec<String>,
    },
    /// Dosya gönderme teklifi (PLAN.md §2.7.1).
    FileOffer(FileOffer),
    /// Teklif kabul edildi; `start_offset` resume noktasıdır.
    FileAccept {
        transfer_id: String,
        start_offset: u64,
    },
    FileReject {
        transfer_id: String,
        reason: RejectReason,
    },
    /// Transfer akışının ilk çerçevesi. Kontrol akışında değil, dosyaya
    /// ayrılmış tek yönlü akışın başında gönderilir; ardından ham bayt gelir.
    TransferStreamHeader {
        transfer_id: String,
        offset: u64,
    },
    /// Alıcının bütünlük doğrulaması sonucu.
    FileComplete {
        transfer_id: String,
        ok: bool,
    },
    /// İki yönlü iptal.
    TransferCancel {
        transfer_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileOffer {
    pub transfer_id: String,
    /// Karşı tarafın bildirdiği ad. GÜVENİLMEZ: kullanılmadan önce
    /// `transfer::paths::sanitize_file_name`den geçmelidir (PLAN.md §2.13.1).
    pub name: String,
    pub size: u64,
    pub mime: Option<String>,
    /// Tüm dosyanın blake3 özeti; resume sonrası birleşmenin doğruluğu
    /// bununla sınanır (PLAN.md §2.7.3).
    pub hash: [u8; 32],
    /// Kesilmiş bir transferin devamı mı?
    pub is_resume: bool,
}

/// Reddin sebebi. Metin değil kod: karşı taraf kendi dilinde gösterir.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RejectReason {
    /// Kullanıcı reddetti.
    Declined,
    /// Hedef diskte yeterli alan yok (PLAN.md §2.13.2).
    NoSpace,
    /// Ayarlardaki boyut eşiğinin üzerinde ve onay alınamadı.
    TooLarge,
    /// Dosya adı veya hedef yol kabul edilemez.
    BadName,
    Internal,
}

/// Sohbet mesajı gövdesi.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatMessage {
    /// UUID; iki uçta aynı kimlik kullanılır, böylece ack eşleştirilebilir.
    pub msg_id: String,
    pub content_type: ContentType,
    pub body: String,
    /// Gönderenin saatine göre Unix saniyesi. Yalnızca bilgi amaçlı:
    /// sıralama alıcının kendi kaydına göre yapılır, karşı tarafın saatine
    /// güvenilmez.
    pub sent_at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContentType {
    Text,
    /// Fenced code block olarak yazılmış içerik.
    Code,
}

impl ContentType {
    pub fn as_str(self) -> &'static str {
        match self {
            ContentType::Text => "text",
            ContentType::Code => "code",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hello {
    pub protocol_version: u16,
    pub min_supported_version: u16,
    /// Yalnızca bilgi/teşhis amaçlı; karar verirken kullanılmaz.
    pub app_version: String,
    pub device_name: String,
    /// 32 byte Ed25519 public key.
    pub device_id: [u8; 32],
    pub capabilities: u32,
}

impl Hello {
    pub fn new(device_name: String, device_id: [u8; 32]) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            min_supported_version: MIN_SUPPORTED_VERSION,
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            device_name,
            device_id,
            capabilities: capabilities::CURRENT,
        }
    }

    pub fn supports(&self, capability: u32) -> bool {
        self.capabilities & capability != 0
    }

    /// İki tarafın ortak bir sürümde buluşup buluşmadığını kontrol eder.
    ///
    /// Aralıklar kesişmiyorsa hangi tarafın eski kaldığını da söyler — kullanıcıya
    /// "sen güncelle" mi "karşı taraf güncellesin" mi diyeceğimizi bu belirler.
    pub fn negotiate(&self) -> Result<u16, VersionMismatch> {
        if self.protocol_version < MIN_SUPPORTED_VERSION {
            return Err(VersionMismatch::PeerTooOld {
                peer_version: self.protocol_version,
                our_min: MIN_SUPPORTED_VERSION,
            });
        }
        if self.min_supported_version > PROTOCOL_VERSION {
            return Err(VersionMismatch::PeerTooNew {
                peer_min: self.min_supported_version,
                our_version: PROTOCOL_VERSION,
            });
        }
        // Ortak en yüksek sürüm.
        Ok(PROTOCOL_VERSION.min(self.protocol_version))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionMismatch {
    /// Karşı taraf çok eski — onun güncellenmesi gerekiyor.
    PeerTooOld { peer_version: u16, our_min: u16 },
    /// Karşı taraf çok yeni — bizim güncellenmemiz gerekiyor.
    PeerTooNew { peer_min: u16, our_version: u16 },
}

/// Karşı tarafa gönderilebilen protokol hatası.
///
/// Frontend'e giden `AppError` gibi, bu da metin değil KOD taşır: karşı
/// taraftaki uygulama kendi dilinde gösterir.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProtocolError {
    IncompatibleVersion,
    /// Eşleşme tamamlanmadan izin verilmeyen bir mesaj geldi (PLAN.md §2.2.1).
    NotPaired,
    UnexpectedMessage,
    FrameTooLarge,
    Internal,
}

#[derive(Debug, thiserror::Error)]
pub enum WireError {
    #[error("bağlantı io hatası: {0}")]
    Io(#[from] std::io::Error),

    #[error("çerçeve çözümlenemedi: {0}")]
    Decode(#[from] postcard::Error),

    #[error("çerçeve çok büyük: {len} byte (üst sınır {MAX_FRAME_LEN})")]
    FrameTooLarge { len: u32 },

    #[error("bağlantı beklenmedik şekilde kapandı")]
    Closed,
}

/// Bir kontrol çerçevesi yazar: `[4 byte u32 LE uzunluk][postcard gövde]`.
///
/// PLAN.md §2.3.1'deki `[uzunluk][1 byte tip][payload]` şeması ile aynı baytları
/// üretir: postcard, enum varyant indeksini gövdenin ilk baytına varint olarak
/// yazar — yani "tip baytı" zaten oradadır, ayrıca yazmak onu ikilerdi.
pub async fn write_frame<W: AsyncWrite + Unpin>(
    writer: &mut W,
    message: &ControlMessage,
) -> Result<(), WireError> {
    let body = postcard::to_stdvec(message)?;
    let len = u32::try_from(body.len()).map_err(|_| WireError::FrameTooLarge { len: u32::MAX })?;
    if len > MAX_FRAME_LEN {
        return Err(WireError::FrameTooLarge { len });
    }

    writer.write_all(&len.to_le_bytes()).await?;
    writer.write_all(&body).await?;
    writer.flush().await?;
    Ok(())
}

/// Bir kontrol çerçevesi okur. Üst sınırı aşan uzunluk, gövdeyi okumadan
/// reddedilir — saldırgan tarafından bildirilen boyuta göre bellek ayırmayız.
pub async fn read_frame<R: AsyncRead + Unpin>(reader: &mut R) -> Result<ControlMessage, WireError> {
    let mut len_bytes = [0u8; 4];
    match reader.read_exact(&mut len_bytes).await {
        Ok(_) => {}
        Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => {
            return Err(WireError::Closed)
        }
        Err(err) => return Err(err.into()),
    }

    let len = u32::from_le_bytes(len_bytes);
    if len > MAX_FRAME_LEN {
        return Err(WireError::FrameTooLarge { len });
    }

    let mut body = vec![0u8; len as usize];
    reader.read_exact(&mut body).await.map_err(|err| {
        if err.kind() == std::io::ErrorKind::UnexpectedEof {
            WireError::Closed
        } else {
            err.into()
        }
    })?;

    Ok(postcard::from_bytes(&body)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_hello() -> Hello {
        Hello::new("Test Cihazı".to_string(), [3u8; 32])
    }

    #[tokio::test]
    async fn cerceve_yazilip_okunur() {
        let messages = [
            ControlMessage::Hello(sample_hello()),
            ControlMessage::HelloAck(sample_hello()),
            ControlMessage::Heartbeat { nonce: u64::MAX },
            ControlMessage::HeartbeatAck { nonce: 0 },
            ControlMessage::Error(ProtocolError::NotPaired),
        ];

        let mut buffer = Vec::new();
        for message in &messages {
            write_frame(&mut buffer, message).await.unwrap();
        }

        let mut cursor = std::io::Cursor::new(buffer);
        for expected in &messages {
            assert_eq!(&read_frame(&mut cursor).await.unwrap(), expected);
        }
    }

    #[tokio::test]
    async fn kapanan_akis_closed_doner() {
        let mut empty = std::io::Cursor::new(Vec::new());
        assert!(matches!(
            read_frame(&mut empty).await,
            Err(WireError::Closed)
        ));

        // Uzunluk okunup gövde gelmezse de "kapandı" sayılır.
        let mut truncated = std::io::Cursor::new(vec![10, 0, 0, 0, 1, 2]);
        assert!(matches!(
            read_frame(&mut truncated).await,
            Err(WireError::Closed)
        ));
    }

    #[tokio::test]
    async fn asiri_buyuk_uzunluk_govde_okunmadan_reddedilir() {
        // Uzunluk alanı 4 GB diyor ama arkasında tek bayt bile yok:
        // reddin gövde okunmadan gerçekleştiğinin kanıtı.
        let mut hostile = std::io::Cursor::new(u32::MAX.to_le_bytes().to_vec());
        assert!(matches!(
            read_frame(&mut hostile).await,
            Err(WireError::FrameTooLarge { .. })
        ));
    }

    #[test]
    fn ayni_surum_uyumlu() {
        assert_eq!(sample_hello().negotiate().unwrap(), PROTOCOL_VERSION);
    }

    #[test]
    fn eski_karsi_taraf_ayirt_edilir() {
        let mut hello = sample_hello();
        hello.protocol_version = 0;
        hello.min_supported_version = 0;
        assert!(matches!(
            hello.negotiate(),
            Err(VersionMismatch::PeerTooOld { .. })
        ));
    }

    #[test]
    fn yeni_karsi_taraf_ayirt_edilir() {
        let mut hello = sample_hello();
        hello.protocol_version = 99;
        hello.min_supported_version = 99;
        assert!(matches!(
            hello.negotiate(),
            Err(VersionMismatch::PeerTooNew { .. })
        ));
    }

    #[test]
    fn ileri_surumlu_ama_geriye_uyumlu_karsi_taraf_kabul_edilir() {
        // Karşı taraf v9 konuşuyor ama v1'i de destekliyorsa anlaşabiliriz.
        let mut hello = sample_hello();
        hello.protocol_version = 9;
        hello.min_supported_version = 1;
        assert_eq!(hello.negotiate().unwrap(), PROTOCOL_VERSION);
    }

    /// Wire formatı kilitlenir: varyant sırası değişirse bu test düşer ve
    /// sürüm uyumluluğunu sessizce kırmış olmayız.
    #[test]
    fn varyant_indeksleri_sabit() {
        let encode = |m: &ControlMessage| postcard::to_stdvec(m).unwrap()[0];
        assert_eq!(encode(&ControlMessage::Hello(sample_hello())), 0);
        assert_eq!(encode(&ControlMessage::HelloAck(sample_hello())), 1);
        assert_eq!(encode(&ControlMessage::Heartbeat { nonce: 1 }), 2);
        assert_eq!(encode(&ControlMessage::HeartbeatAck { nonce: 1 }), 3);
        assert_eq!(encode(&ControlMessage::Error(ProtocolError::Internal)), 4);
        assert_eq!(encode(&ControlMessage::PairingRequest), 5);
        assert_eq!(encode(&ControlMessage::PairingConfirm), 6);
        assert_eq!(encode(&ControlMessage::PairingReject), 7);
        assert_eq!(
            encode(&ControlMessage::Chat(ChatMessage {
                msg_id: "x".into(),
                content_type: ContentType::Text,
                body: "merhaba".into(),
                sent_at: 0,
            })),
            8
        );
        assert_eq!(encode(&ControlMessage::ChatAck { msg_id: "x".into() }), 9);
        assert_eq!(encode(&ControlMessage::ReadReceipt { msg_ids: vec![] }), 10);

        let offer = FileOffer {
            transfer_id: "t".into(),
            name: "a.txt".into(),
            size: 1,
            mime: None,
            hash: [0; 32],
            is_resume: false,
        };
        assert_eq!(encode(&ControlMessage::FileOffer(offer)), 11);
        assert_eq!(
            encode(&ControlMessage::FileAccept {
                transfer_id: "t".into(),
                start_offset: 0
            }),
            12
        );
        assert_eq!(
            encode(&ControlMessage::FileReject {
                transfer_id: "t".into(),
                reason: RejectReason::Declined
            }),
            13
        );
        assert_eq!(
            encode(&ControlMessage::TransferStreamHeader {
                transfer_id: "t".into(),
                offset: 0
            }),
            14
        );
        assert_eq!(
            encode(&ControlMessage::FileComplete {
                transfer_id: "t".into(),
                ok: true
            }),
            15
        );
        assert_eq!(
            encode(&ControlMessage::TransferCancel {
                transfer_id: "t".into()
            }),
            16
        );
    }

    #[test]
    fn yetenek_bayraklari_okunur() {
        let mut hello = sample_hello();
        assert!(!hello.supports(capabilities::FOLDER_SYNC));
        hello.capabilities = capabilities::FOLDER_SYNC;
        assert!(hello.supports(capabilities::FOLDER_SYNC));
        assert!(!hello.supports(capabilities::CLIPBOARD));
    }
}
