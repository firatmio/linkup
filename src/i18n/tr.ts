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
  "nav.discovered.manual": "elle eklendi",

  // Elle cihaz ekleme
  "addDevice.title": "Cihaz Ekle",
  "addDevice.body":
    "Cihaz otomatik bulunamıyorsa IP adresini elle girebilirsiniz. Adres, hedef cihazın Ayarlar → Gelişmiş bölümünde yazılı.",
  "addDevice.label": "IP adresi",
  "addDevice.placeholder": "192.168.1.42",
  "addDevice.hint": "Port belirtmezseniz varsayılan port kullanılır.",
  "addDevice.submit": "Bağlan",
  "addDevice.cancel": "Vazgeç",
  "addDevice.connecting": "Bağlanılıyor…",
  "device.forget": "Listeden çıkar",
  "device.pair": "Eşleştir",

  // Eşleştirme
  "pairing.title": "Cihazı Eşleştir",
  "pairing.outgoing":
    "Bu kod {device} cihazında da görünüyor olmalı. İki kod aynı mı?",
  "pairing.incoming":
    "{device} bu cihazla eşleşmek istiyor. Aşağıdaki kod o cihazda da aynı mı?",
  "pairing.warning":
    "Kodlar farklıysa ONAYLAMAYIN — araya biri girmiş olabilir.",
  "pairing.accept": "Kodlar aynı, onayla",
  "pairing.reject": "Reddet",
  "pairing.waiting": "Karşı tarafın onayı bekleniyor…",
  "pairing.starting": "Bağlanılıyor…",
  "pairing.success": "Cihaz eşleştirildi.",
  "pairing.error.network": "Cihaza bağlanılamadı.",
  "pairing.error.internal": "Eşleştirme tamamlanamadı.",
  "pairing.error.rejectedLocally": "Eşleştirme reddedildi.",
  "pairing.error.rejectedByPeer": "Karşı taraf eşleştirmeyi reddetti.",
  "pairing.error.timeout": "Eşleştirme zaman aşımına uğradı.",

  // Güvenilir cihazlar
  "devices.title": "Cihazlar",
  "devices.forget": "Cihazı Unut",
  "devices.forget.confirm":
    "{device} unutulacak. Bu cihazla olan mesaj geçmişi de silinir ve yeniden eşleşmek için kod doğrulaması gerekir.",
  "devices.forget.cancel": "Vazgeç",

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
  "dashboard.empty.discovered":
    "Ağda {count} cihaz bulundu. Sol alttaki listeden eşleştirebilirsiniz.",
  "dashboard.summary.online": "{online} / {total} cihaz çevrimiçi",
  "dashboard.summary.unread": "{count} okunmamış mesaj",
  "dashboard.openChat": "Sohbeti aç",

  // Sohbetler
  "chats.title": "Sohbetler",
  "chats.empty.title": "Sohbet yok",
  "chats.empty.body": "Bir cihazla eşleştiğinizde sohbetler burada listelenir.",
  "chats.select": "Soldan bir sohbet seçin",
  "chats.back": "Geri",
  "chats.today": "Bugün",
  "chats.yesterday": "Dün",
  "chats.noMessages": "Henüz mesaj yok. İlk mesajı siz yazın.",
  "chats.placeholder": "Mesaj yaz…",
  "chats.placeholder.offline": "Cihaz çevrimdışı",
  "chats.send": "Gönder",
  "chats.sendHint": "Enter ile gönder, Shift+Enter ile alt satır",
  "chats.codeToggle": "Kod bloğu olarak gönder",
  "chats.copyCode": "Kopyala",
  "chats.copied": "Kopyalandı",
  "chats.attach": "Dosya gönder",
  "chats.dropHere": "Göndermek için bırakın",
  "transfers.title": "Aktarımlar",
  "transfers.sending": "Gönderiliyor",
  "transfers.receiving": "Alınıyor",
  "transfers.remaining": "{time} kaldı",

  // Mesaj durumları
  "message.status.sending": "Gönderiliyor",
  "message.status.sent": "Gönderildi",
  "message.status.delivered": "İletildi",
  "message.status.read": "Görüldü",
  "message.status.failed": "Gönderilemedi",

  // Gelen dosyalar
  "files.title": "Gelen Dosyalar",
  "files.count": "{count} dosya",
  "files.open": "Aç",
  "files.reveal": "Klasörde Göster",
  "files.status.done": "Tamamlandı",
  "files.status.failed": "Başarısız",
  "files.status.cancelled": "İptal edildi",
  "files.status.paused": "Duraklatıldı",
  "files.status.active": "Aktarılıyor",
  "files.status.pending": "Bekliyor",
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
  "settings.address": "Bu cihazın adresi",
  "settings.address.desc": "Karşı cihazda \"Cihaz Ekle\" ile bu adresi girebilirsiniz",
  "settings.address.none": "Ağ arayüzü bulunamadı",
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
  "error.invalidAddress": "Geçerli bir IP adresi girin.",
  "error.unreachable": "Cihaza ulaşılamadı. Adresi ve hedef cihazın açık olduğunu kontrol edin.",
  "error.internal": "Beklenmeyen bir hata oluştu.",
  "error.unknown": "Bilinmeyen bir hata oluştu.",
} as const;

export type TranslationKey = keyof typeof tr;
