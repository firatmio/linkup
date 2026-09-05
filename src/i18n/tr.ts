/**
 * Türkçe sözlük — kullanıcıya görünen TÜM metinler burada.
 *
 * Bileşenlerde çıplak metin bulunmaz. Yeni dil eklemek, bu dosyanın
 * bir kopyasını çevirmek ve `i18n/index.ts`'e kaydetmekten ibarettir
 * (PLAN.md §3.1, §10-K8).
 */
export const tr = {
  "app.name": "LinkUp",

  // Navigasyon
  "nav.dashboard": "Ana Sayfa",
  "nav.chats": "Sohbetler",
  "nav.files": "Gelen Dosyalar",
  "nav.settings": "Ayarlar",
  "nav.discovered": "Bulunanlar",
  "nav.discovered.empty": "Ağda cihaz aranıyor…",
  "nav.addManually": "Cihaz Ekle",

  // Ana sayfa
  "dashboard.title": "Genel Bakış",
  "dashboard.devices": "Cihazlar",
  "dashboard.activeTransfers": "Aktif Transferler",
  "dashboard.recentMedia": "Son Gelen Medyalar",
  "dashboard.seeAll": "Tümü",
  "dashboard.empty.title": "Henüz eşleşmiş cihaz yok",
  "dashboard.empty.body":
    "Aynı ağdaki bir cihazla eşleşerek mesajlaşmaya ve dosya göndermeye başlayın.",
  "dashboard.empty.action": "Cihaz Ekle",

  // Sohbetler
  "chats.title": "Sohbetler",
  "chats.empty.title": "Sohbet yok",
  "chats.empty.body": "Bir cihazla eşleştiğinizde sohbetler burada listelenir.",

  // Gelen dosyalar
  "files.title": "Gelen Dosyalar",
  "files.empty.title": "Henüz dosya alınmadı",
  "files.empty.body": "Aldığınız dosyalar burada geçmişiyle birlikte listelenir.",

  // Ayarlar
  "settings.title": "Ayarlar",
  "settings.section.general": "Genel",
  "settings.section.security": "Gizlilik ve Güvenlik",
  "settings.section.advanced": "Gelişmiş",
  "settings.theme": "Tema",
  "settings.theme.desc": "Uygulamanın görünümü",
  "settings.theme.system": "Sistemi takip et",
  "settings.theme.light": "Açık",
  "settings.theme.dark": "Koyu",
  "settings.fingerprint": "Bu cihazın kimliği",
  "settings.fingerprint.desc":
    "Karşı tarafla eşleşirken bu kodu karşılaştırarak doğrulama yapabilirsiniz",
  "settings.keyStorage": "Kimlik anahtarı",
  "settings.keyStorage.osKeychain": "İşletim sisteminin parola kasasında",
  "settings.keyStorage.plainFile": "Veri klasöründeki dosyada",
  "settings.keyStorage.plainFile.warning":
    "Anahtarınız sistem kasasına yazılamadı, veri klasöründeki bir dosyada tutuluyor. Diske erişebilen bir yazılım bu anahtarı okuyabilir.",
  "settings.version": "Sürüm",
  "settings.profile": "Geliştirme profili",
  "settings.profile.none": "Yok (varsayılan)",
  "settings.quicPort": "QUIC portu",
  "settings.dataDir": "Veri klasörü",
  "settings.downloadsDir": "İndirme klasörü",
  "settings.logs": "Günlük kayıtları",
  "settings.logs.desc": "Sorun bildirirken bu klasördeki dosyalar işe yarar",
  "settings.logs.open": "Log Klasörünü Aç",

  // Durum
  "status.online": "Çevrimiçi",
  "status.offline": "Çevrimdışı",

  // Genel
  "common.loading": "Yükleniyor…",
  "common.comingSoon": "Bu bölüm sonraki fazlarda geliyor.",

  // Hata kodları — Rust tarafındaki AppError::code() ile birebir eşleşir.
  "error.io": "Dosya işlemi başarısız oldu.",
  "error.db": "Veritabanı işlemi başarısız oldu.",
  "error.settingNotFound": "Ayar bulunamadı.",
  "error.invalidInput": "Geçersiz istek.",
  "error.internal": "Beklenmeyen bir hata oluştu.",
  "error.unknown": "Bilinmeyen bir hata oluştu.",
} as const;

export type TranslationKey = keyof typeof tr;
