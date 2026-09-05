//! Gelen dosya adlarının güvenli hâle getirilmesi (PLAN.md §2.13.1).
//!
//! Karşı taraf `FileOffer` içinde KEYFİ bir dosya adı gönderebilir. Bu ad
//! doğrudan kullanılırsa `..\..\Windows\System32\x.dll` gibi bir değer indirme
//! klasörünün dışına yazmaya yol açar. Bu modül tek bir sorumluluk taşır:
//! dışarıdan gelen bir adı, indirme klasörünün içinde kalması garanti edilen
//! bir dosya adına indirgemek.
//!
//! Kural: buradan geçmemiş hiçbir ad dosya sistemine dokunmaz.

use std::path::{Component, Path, PathBuf};

/// Ad üretilemediğinde kullanılan yedek.
const FALLBACK_NAME: &str = "dosya";

/// Dosya adı için üst sınır. Windows'ta bileşen sınırı 255; uzantı ve
/// "(12)" gibi ekler için pay bırakılıyor.
const MAX_NAME_LEN: usize = 200;

/// Windows'ta ayrılmış aygıt adları. Uzantı eklense bile ayrılmış sayılırlar
/// (`CON.txt` de açılamaz), bu yüzden kök ada bakılır.
const RESERVED: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Dosya sisteminde geçersiz veya tehlikeli karakterler.
///
/// `:` özellikle önemli: Windows'ta alternatif veri akışı (ADS) ayracıdır ve
/// `dosya.txt:gizli` gibi bir ad, göründüğünden başka bir yere yazar.
const FORBIDDEN: &[char] = &['/', '\\', ':', '*', '?', '"', '<', '>', '|', '\0'];

/// Dışarıdan gelen bir dosya adını güvenli bir ada indirger.
///
/// Sonuç her zaman tek bir dosya adıdır: dizin bileşeni, üst dizin başvurusu
/// veya sürücü harfi içermez.
pub fn sanitize_file_name(raw: &str) -> String {
    // 1) Yalnızca son bileşeni al: hem `/` hem `\` ayracı sayılır. Karşı taraf
    //    farklı bir işletim sistemi kullanıyor olabilir.
    let last = raw.rsplit(['/', '\\']).next().unwrap_or_default();

    // 2) Yasaklı karakterleri ve kontrol karakterlerini at.
    let mut cleaned: String = last
        .chars()
        .filter(|c| !FORBIDDEN.contains(c) && !c.is_control())
        .collect();

    // 3) Windows dosya adları nokta veya boşlukla bitemez; biten adlar
    //    sessizce kırpılır ve beklenmedik bir dosyaya işaret edebilir.
    cleaned = cleaned.trim().trim_end_matches(['.', ' ']).to_string();

    // 4) `.` ve `..` tek başına ad değildir.
    if cleaned.is_empty() || cleaned == "." || cleaned == ".." {
        return FALLBACK_NAME.to_string();
    }

    // 5) Ayrılmış aygıt adlarını etkisizleştir.
    let stem = cleaned.split('.').next().unwrap_or_default().to_uppercase();
    if RESERVED.contains(&stem.as_str()) {
        cleaned = format!("_{cleaned}");
    }

    // 6) Uzunluğu sınırla — uzantıyı koruyarak kırp.
    truncate_keeping_extension(&cleaned)
}

/// Adı uzunluk sınırına indirirken uzantıyı korur: `.zip` kaybolursa dosya
/// işletim sisteminde yanlış uygulamayla açılır.
fn truncate_keeping_extension(name: &str) -> String {
    if name.len() <= MAX_NAME_LEN {
        return name.to_string();
    }

    let path = Path::new(name);
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .filter(|e| e.len() < 16)
        .unwrap_or_default();

    let keep = MAX_NAME_LEN.saturating_sub(extension.len() + 1);
    let stem: String = name.chars().take(keep).collect();

    if extension.is_empty() {
        stem
    } else {
        format!("{stem}.{extension}")
    }
}

/// Yolun, verilen kökün ALTINDA kaldığını doğrular.
///
/// Sanitizasyondan sonra bile son bir kontrol: sembolik bağlantılar ve
/// beklenmedik bileşenler yüzünden yol köke dışarı çıkabilir. Kök henüz
/// oluşmamış olabileceği için karşılaştırma normalleştirilmiş bileşenler
/// üzerinden yapılır.
pub fn is_within(root: &Path, candidate: &Path) -> bool {
    let root = normalize(root);
    let candidate = normalize(candidate);
    candidate.starts_with(&root) && candidate != root
}

/// `.` bileşenlerini atar ve `..` bileşenlerini çözer.
fn normalize(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                result.pop();
            }
            other => result.push(other.as_os_str()),
        }
    }
    result
}

/// Hedef klasörde çakışmayan bir yol üretir: `rapor.pdf` doluysa
/// `rapor (1).pdf`, o da doluysa `rapor (2).pdf`.
///
/// Üzerine yazmak kabul edilemez: karşı taraf, adını bildiği bir dosyayı
/// sessizce değiştirebilirdi.
pub fn unique_path(directory: &Path, file_name: &str) -> PathBuf {
    let candidate = directory.join(file_name);
    if !candidate.exists() {
        return candidate;
    }

    let path = Path::new(file_name);
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(FALLBACK_NAME);
    let extension = path.extension().and_then(|e| e.to_str());

    for index in 1..10_000 {
        let name = match extension {
            Some(ext) => format!("{stem} ({index}).{ext}"),
            None => format!("{stem} ({index})"),
        };
        let candidate = directory.join(name);
        if !candidate.exists() {
            return candidate;
        }
    }

    // Pratikte ulaşılamaz; yine de bir ad döndürmek gerekiyor.
    directory.join(format!("{stem}-{}", crate::db::devices::now()))
}

/// Gelen bir dosya için nihai yolu belirler: adı temizler, çakışmayı çözer ve
/// sonucun indirme klasörünün altında kaldığını doğrular.
pub fn resolve_download_path(
    download_dir: &Path,
    raw_name: &str,
) -> Result<PathBuf, TransferPathError> {
    let safe_name = sanitize_file_name(raw_name);
    let target = unique_path(download_dir, &safe_name);

    if !is_within(download_dir, &target) {
        return Err(TransferPathError::Escapes);
    }
    Ok(target)
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TransferPathError {
    #[error("hedef yol indirme klasörünün dışına çıkıyor")]
    Escapes,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Saldırgan girdileri: hiçbiri dizin bileşeni içeren bir ad üretmemeli.
    #[test]
    fn yol_kacisi_denemeleri_etkisizlenir() {
        let hostile = [
            "../../Windows/System32/evil.dll",
            r"..\..\Windows\System32\evil.dll",
            "/etc/passwd",
            r"C:\Windows\evil.exe",
            r"\\sunucu\pay\evil.exe",
            "....//....//evil",
            "dir/sub/dosya.txt",
        ];

        for raw in hostile {
            let safe = sanitize_file_name(raw);
            assert!(!safe.contains('/'), "{raw} → {safe}");
            assert!(!safe.contains('\\'), "{raw} → {safe}");
            assert!(!safe.contains(':'), "{raw} → {safe}");
            assert_ne!(safe, "..", "{raw} → {safe}");
            assert!(!safe.is_empty(), "{raw} → boş");
        }
    }

    #[test]
    fn son_bilesen_korunur() {
        assert_eq!(sanitize_file_name("dir/sub/rapor.pdf"), "rapor.pdf");
        assert_eq!(sanitize_file_name(r"C:\indir\rapor.pdf"), "rapor.pdf");
    }

    /// Windows'ta ADS ayracı: `dosya.txt:gizli` göründüğünden başka yere yazar.
    #[test]
    fn alternatif_veri_akisi_ayraci_temizlenir() {
        let safe = sanitize_file_name("dosya.txt:gizli:$DATA");
        assert!(!safe.contains(':'), "{safe}");
    }

    #[test]
    fn ayrilmis_aygit_adlari_etkisizlenir() {
        for raw in ["CON", "con", "NUL.txt", "com1.log", "LPT9"] {
            let safe = sanitize_file_name(raw);
            let stem = safe.split('.').next().unwrap().to_uppercase();
            assert!(
                !RESERVED.contains(&stem.as_str()),
                "{raw} → {safe} hâlâ ayrılmış"
            );
        }
        // Ayrılmış OLMAYAN adlara dokunulmamalı.
        assert_eq!(sanitize_file_name("console.log"), "console.log");
    }

    #[test]
    fn sondaki_nokta_ve_bosluk_kirpilir() {
        assert_eq!(sanitize_file_name("rapor.pdf."), "rapor.pdf");
        assert_eq!(sanitize_file_name("rapor.pdf   "), "rapor.pdf");
        assert_eq!(sanitize_file_name("  rapor.pdf  "), "rapor.pdf");
    }

    #[test]
    fn bos_ve_dejenere_adlar_yedege_duser() {
        for raw in ["", "   ", ".", "..", "///", "\u{0}", "..."] {
            assert_eq!(sanitize_file_name(raw), FALLBACK_NAME, "girdi: {raw:?}");
        }
    }

    #[test]
    fn uzun_ad_kirpilir_ama_uzanti_korunur() {
        let raw = format!("{}.zip", "a".repeat(500));
        let safe = sanitize_file_name(&raw);

        assert!(safe.len() <= MAX_NAME_LEN, "uzunluk: {}", safe.len());
        assert!(safe.ends_with(".zip"), "uzantı korunmalı: {safe}");
    }

    #[test]
    fn normal_adlar_bozulmaz() {
        for raw in [
            "rapor.pdf",
            "Ekran Görüntüsü 2026-09-05.png",
            "arşiv.tar.gz",
        ] {
            assert_eq!(sanitize_file_name(raw), raw);
        }
    }

    #[test]
    fn kok_disina_cikan_yol_reddedilir() {
        let root = Path::new("/indirilenler/LinkUp");
        assert!(is_within(root, Path::new("/indirilenler/LinkUp/a.txt")));
        assert!(is_within(root, Path::new("/indirilenler/LinkUp/alt/a.txt")));

        assert!(!is_within(root, Path::new("/indirilenler/baska/a.txt")));
        assert!(!is_within(root, Path::new("/indirilenler/LinkUp/../a.txt")));
        assert!(!is_within(root, root), "kökün kendisi hedef olamaz");
    }

    #[test]
    fn cakisan_ad_numaralandirilir() {
        let dir = std::env::temp_dir().join(format!("linkup-paths-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let first = unique_path(&dir, "rapor.pdf");
        assert_eq!(first.file_name().unwrap(), "rapor.pdf");
        std::fs::write(&first, b"x").unwrap();

        let second = unique_path(&dir, "rapor.pdf");
        assert_eq!(second.file_name().unwrap(), "rapor (1).pdf");
        std::fs::write(&second, b"x").unwrap();

        let third = unique_path(&dir, "rapor.pdf");
        assert_eq!(third.file_name().unwrap(), "rapor (2).pdf");

        // Uzantısız dosyalar da numaralandırılmalı.
        let plain = unique_path(&dir, "notlar");
        std::fs::write(&plain, b"x").unwrap();
        assert_eq!(
            unique_path(&dir, "notlar").file_name().unwrap(),
            "notlar (1)"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    /// Uçtan uca: saldırgan bir ad, indirme klasörünün içinde kalan bir yola
    /// dönüşmeli.
    #[test]
    fn cozumlenen_yol_indirme_klasorunun_altinda_kalir() {
        let dir = std::env::temp_dir().join(format!("linkup-resolve-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        for raw in ["../../evil.dll", r"..\..\evil.dll", "/etc/passwd", "CON"] {
            let path = resolve_download_path(&dir, raw).unwrap();
            assert!(
                is_within(&dir, &path),
                "{raw} → {} klasör dışına çıktı",
                path.display()
            );
            assert_eq!(path.parent().unwrap(), dir);
        }

        std::fs::remove_dir_all(&dir).ok();
    }
}
