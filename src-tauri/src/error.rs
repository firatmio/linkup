//! Hata tipleri ve frontend'e taşınan yapısal hata gösterimi (PLAN.md §2.14).
//!
//! Backend kullanıcıya gösterilecek metin ÜRETMEZ. Yalnızca bir `code` üretir;
//! frontend bu kodu i18n sözlüğünden çevirir. `detail` yalnızca log ve
//! "Gelişmiş" ekranı içindir, kullanıcı arayüzünde birincil mesaj olarak kullanılmaz.

use serde::Serialize;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, thiserror::Error)]
// Bazı varyantlar ilk kullanıcılarını sonraki fazlarda bulacak (ayarlar, pairing, transfer).
#[allow(dead_code)]
pub enum AppError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("ayar bulunamadı: {0}")]
    SettingNotFound(String),

    #[error("geçersiz istek: {0}")]
    InvalidInput(String),

    #[error("beklenmeyen hata: {0}")]
    Internal(#[from] anyhow::Error),
}

impl AppError {
    /// i18n anahtarı olarak kullanılan sabit kod.
    pub fn code(&self) -> &'static str {
        match self {
            AppError::Io(_) => "error.io",
            AppError::SettingNotFound(_) => "error.settingNotFound",
            AppError::InvalidInput(_) => "error.invalidInput",
            AppError::Internal(_) => "error.internal",
        }
    }
}

/// Frontend'e giden hata gövdesi.
#[derive(Debug, Serialize)]
pub struct ErrorPayload {
    pub code: &'static str,
    pub detail: String,
}

impl Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // Hata her zaman loglanır; frontend'e giden kısım kısadır.
        tracing::warn!(code = self.code(), detail = %self, "komut hatası");
        ErrorPayload {
            code: self.code(),
            detail: self.to_string(),
        }
        .serialize(serializer)
    }
}
