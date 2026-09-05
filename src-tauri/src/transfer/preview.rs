//! Görsel önizleme üretimi (PLAN.md §3.3 "Görsel inline thumbnail").
//!
//! Dosyanın kendisi arayüze verilmiyor. Alternatif, Tauri'nin `asset`
//! protokolünü indirme klasörüne açmaktı; o yol webview'a keyfi yerel dosya
//! okuma izni verirdi ve önizlemeyi boyutlandırmazdı — bir sohbette yirmi
//! fotoğraf, yirmi tam boy görsel demek olurdu. Bunun yerine küçük resim
//! burada üretilip yalnızca sonucu gönderiliyor.

use std::path::Path;

use image::imageops::FilterType;
use image::{ImageFormat, ImageReader};
use serde::Serialize;

use crate::error::{AppError, AppResult};

/// Önizlemenin en uzun kenarı için izin verilen aralık.
const MIN_EDGE: u32 = 64;
const MAX_EDGE: u32 = 2048;

/// Kaynak dosya boyutu sınırı.
///
/// Sıkıştırılmış bir görselin çözülmüş hâli çok daha büyüktür (bkz. dekompresyon
/// bombası); `image` ayrıca bellek sınırıyla da korunuyor, bu yalnızca ilk kapı.
const MAX_SOURCE_BYTES: u64 = 64 * 1024 * 1024;

/// Çözme sırasında ayrılabilecek en fazla bellek.
const MAX_DECODE_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    /// `image/png` veya `image/jpeg`.
    pub mime: String,
    /// Base64 kodlu görsel verisi.
    pub data: String,
    /// Küçültülmüş görselin ölçüleri; arayüz yer ayırmak için kullanır.
    pub width: u32,
    pub height: u32,
}

/// Uzantıya bakarak dosyanın önizlenebilir olup olmadığını söyler.
///
/// Uzantı, içeriğin kanıtı değildir — bu yüzden yalnızca gereksiz iş yapmamak
/// için kullanılıyor. Gerçek karar çözücünün kendisine ait: uzantısı `.png`
/// olan bir metin dosyası burada geçse bile çözme aşamasında reddedilir.
pub fn looks_like_image(file_name: &str) -> bool {
    let Some(ext) = Path::new(file_name).extension().and_then(|e| e.to_str()) else {
        return false;
    };
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp"
    )
}

/// Dosyadan en uzun kenarı `max_edge` olan bir küçük resim üretir.
///
/// Görsel zaten küçükse büyütülmez: küçük bir ikonu 320 piksele şişirmek
/// yalnızca bulanıklık ve boşa bant genişliği üretir.
pub fn thumbnail(path: &Path, max_edge: u32) -> AppResult<Preview> {
    let max_edge = max_edge.clamp(MIN_EDGE, MAX_EDGE);

    let size = std::fs::metadata(path)?.len();
    if size > MAX_SOURCE_BYTES {
        return Err(AppError::InvalidInput(
            "görsel önizleme için fazla büyük".to_string(),
        ));
    }

    let mut limits = image::Limits::default();
    limits.max_alloc = Some(MAX_DECODE_BYTES);

    let mut reader = ImageReader::open(path)?.with_guessed_format()?;
    reader.limits(limits);

    let image = reader
        .decode()
        .map_err(|err| AppError::InvalidInput(format!("görsel çözülemedi: {err}")))?;

    let scaled = if image.width() <= max_edge && image.height() <= max_edge {
        image
    } else {
        // `thumbnail` yerine `resize`: ilki hız için kaliteden ödün veriyor ve
        // fotoğraflarda gözle görülür şekilde tırtıklı çıkıyor.
        image.resize(max_edge, max_edge, FilterType::Triangle)
    };

    // Saydamlık varsa PNG şart; yoksa JPEG belirgin şekilde küçük çıkıyor.
    let has_alpha = scaled.color().has_alpha();
    let (format, mime) = if has_alpha {
        (ImageFormat::Png, "image/png")
    } else {
        (ImageFormat::Jpeg, "image/jpeg")
    };

    let mut buffer = std::io::Cursor::new(Vec::new());
    let encodable = if has_alpha {
        scaled.clone()
    } else {
        // JPEG alfa kanalı taşımaz; RGB8'e indirgemezsek kodlama hata verir.
        image::DynamicImage::ImageRgb8(scaled.to_rgb8())
    };
    encodable
        .write_to(&mut buffer, format)
        .map_err(|err| AppError::Internal(anyhow::anyhow!("görsel kodlanamadı: {err}")))?;

    Ok(Preview {
        mime: mime.to_string(),
        data: data_encoding::BASE64.encode(&buffer.into_inner()),
        width: scaled.width(),
        height: scaled.height(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uzantiya_gore_gorsel_ayirt_edilir() {
        assert!(looks_like_image("tatil.JPG"), "uzantı büyük harf olabilir");
        assert!(looks_like_image("ekran.png"));
        assert!(!looks_like_image("rapor.pdf"));
        assert!(!looks_like_image("uzantisiz"));
        assert!(!looks_like_image("png"), "uzantısız ad yanlış eşleşmemeli");
    }

    fn write_sample(
        dir: &Path,
        name: &str,
        width: u32,
        height: u32,
        alpha: bool,
    ) -> std::path::PathBuf {
        let path = dir.join(name);
        if alpha {
            image::RgbaImage::from_pixel(width, height, image::Rgba([10, 20, 30, 128]))
                .save(&path)
                .unwrap();
        } else {
            image::RgbImage::from_pixel(width, height, image::Rgb([10, 20, 30]))
                .save(&path)
                .unwrap();
        }
        path
    }

    #[test]
    fn buyuk_gorsel_kucultulur_ve_en_boy_orani_korunur() {
        let dir = tempdir();
        let path = write_sample(&dir, "buyuk.png", 800, 400, false);

        let preview = thumbnail(&path, 200).unwrap();
        assert_eq!((preview.width, preview.height), (200, 100));
        assert_eq!(preview.mime, "image/jpeg", "saydamlık yoksa JPEG");
        assert!(!preview.data.is_empty());
    }

    /// Küçük bir görseli büyütmek yalnızca bulanıklık üretir.
    #[test]
    fn kucuk_gorsel_buyutulmez() {
        let dir = tempdir();
        let path = write_sample(&dir, "kucuk.png", 40, 30, false);

        let preview = thumbnail(&path, 320).unwrap();
        assert_eq!((preview.width, preview.height), (40, 30));
    }

    #[test]
    fn saydam_gorsel_png_kalir() {
        let dir = tempdir();
        let path = write_sample(&dir, "saydam.png", 100, 100, true);

        assert_eq!(thumbnail(&path, 64).unwrap().mime, "image/png");
    }

    /// Uzantısı görsel olan bir metin dosyası çözme aşamasında reddedilmeli.
    #[test]
    fn gorsel_olmayan_dosya_reddedilir() {
        let dir = tempdir();
        let path = dir.join("sahte.png");
        std::fs::write(&path, b"bu bir gorsel degil").unwrap();

        assert!(thumbnail(&path, 128).is_err());
    }

    /// Testler geçici bir klasörde çalışır; `tempfile` bağımlılığı eklemek
    /// yerine süreç kimliğiyle ayrışan bir dizin yeterli.
    fn tempdir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "linkup-preview-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}
