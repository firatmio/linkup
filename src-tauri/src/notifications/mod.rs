//! Native bildirimler (PLAN.md §2.10).
//!
//! Kural: bildirim yalnızca kullanıcı uygulamaya BAKMIYORKEN gösterilir.
//! Ekranda açık duran bir sohbete gelen mesaj için ayrıca bildirim basmak,
//! kullanıcının zaten gördüğü şeyi tekrar etmektir ve hızla rahatsız edici
//! hâle gelir.
//!
//! Bu kural olay anında bakmakla yetmiyor: toast Windows'a teslim edildikten
//! sonra geri alınamaz, dolayısıyla art arda gelen beş mesaj kullanıcı
//! uygulamaya döndükten sonra bile sırayla düşmeye devam eder. Bu yüzden
//! gösterim kısa bir süre geciktirilip o sürede gelenler tek bildirimde
//! toplanıyor ve odak kararı gecikmenin SONUNDA veriliyor (bkz. `schedule`).
//!
//! **Tıklama desteği (Faz 7'de doğrulandı):** Tauri'nin `notification`
//! eklentisi masaüstünde HİÇBİR olay yayınlamaz — `onAction` ve
//! `onNotificationReceived` yalnızca mobil içindir. Planın §2.10'da
//! işaretlediği risk buydu. Bu yüzden Windows'ta eklenti yerine doğrudan
//! `tauri-winrt-notification` kullanılıyor: `on_activated` geri çağrısı
//! sayesinde toast'a tıklayınca pencere öne getirilip ilgili ekrana
//! yönlendirilebiliyor. Diğer platformlarda eklenti kullanılmaya devam ediyor
//! (orada tıklama yönlendirmesi henüz yok).

pub mod schedule;

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use schedule::Pending;

/// Bildirime tıklandığında frontend'e gönderilen olay.
pub const EVENT_ACTIVATED: &str = "notification:activated";

/// Bildirime tıklanınca gidilecek yer.
// `rename_all` yalnızca VARYANT adlarını çevirir; alan adları için
// `rename_all_fields` gerekir. İkincisi olmadan arayüze `device_id` gidiyor,
// arayüz ise `deviceId` okuyordu: bildirime tıklayınca sohbet tanımsız bir
// kimlikle açılmaya çalışılıyor ve hata veriyordu.
#[derive(Debug, Clone, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Action {
    /// Bu cihazın sohbetini aç.
    OpenChat { device_id: String },
    /// Gelen dosyalar ekranını aç.
    OpenFiles,
}

/// Ana pencere odakta ve görünür mü?
///
/// Odak sorgusu başarısız olursa "odakta değil" kabul edilir: bildirimi
/// kaçırmak, gereksiz bildirimden daha kötüdür.
fn window_has_focus(app: &AppHandle) -> bool {
    let Some(window) = app.get_webview_window("main") else {
        return false;
    };

    let focused = window.is_focused().unwrap_or(false);
    let visible = window.is_visible().unwrap_or(false);
    let minimized = window.is_minimized().unwrap_or(false);

    focused && visible && !minimized
}

/// Pencereyi öne getirir ve frontend'i ilgili ekrana yönlendirir.
fn activate(app: &AppHandle, action: &Action) {
    if let Some(window) = app.get_webview_window("main") {
        // Simge durumundaysa önce geri al: yalnızca `set_focus` çağırmak
        // küçültülmüş pencereyi geri getirmez.
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
    let _ = app.emit(EVENT_ACTIVATED, action);
}

/// Bildirim türü. Konu anahtarının parçası: bir cihazdan gelen mesajlarla o
/// cihazdan gelen dosyalar ayrı ayrı birikir, birbirini bastırmaz.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Kind {
    Message,
    FileReceived,
    FileOffer,
}

impl Kind {
    /// Birden fazla olay biriktiğinde gösterilecek metin.
    fn summary(self, device_name: &str, count: u32) -> (String, String) {
        match self {
            Kind::Message => (device_name.to_string(), format!("{count} yeni mesaj")),
            Kind::FileReceived => (
                format!("{device_name} {count} dosya gönderdi"),
                String::new(),
            ),
            Kind::FileOffer => (
                format!("{device_name} {count} dosya göndermek istiyor"),
                String::new(),
            ),
        }
    }
}

/// Konu bazında biriken bildirim durumu.
///
/// Süreç ömrü boyunca yaşar ve tek bir pencereye aittir; bu yüzden yönetilen
/// bir Tauri durumu yerine modül düzeyinde tutuluyor.
fn pending_state() -> &'static Mutex<HashMap<(Kind, String), Pending>> {
    static STATE: OnceLock<Mutex<HashMap<(Kind, String), Pending>>> = OnceLock::new();
    STATE.get_or_init(Default::default)
}

/// Bildirimi biriktirir ve zamanı gelince gösterir.
///
/// Gösterim ANINDA yapılmaz: kısa bir gecikme boyunca aynı konudan gelen
/// olaylar tek bildirimde toplanır ve kullanıcı bu arada uygulamaya dönerse
/// bildirim hiç gösterilmez (bkz. `schedule`).
fn notify(
    app: &AppHandle,
    kind: Kind,
    // Konu anahtarı cihaz KİMLİĞİ: aynı adı taşıyan iki cihazın bildirimleri
    // birbirine karışmamalı.
    key: String,
    device_name: &str,
    title: String,
    body: String,
    action: Action,
) {
    if window_has_focus(app) {
        return;
    }

    let delay = {
        let mut state = match pending_state().lock() {
            Ok(state) => state,
            // Kilit zehirlenmişse bildirim göstermemek, panikle çökmekten iyi.
            Err(err) => {
                tracing::warn!(error = %err, "bildirim durumu okunamadı");
                return;
            }
        };
        state
            .entry((kind, key.clone()))
            .or_default()
            .record(Instant::now())
    };

    let Some(delay) = delay else {
        // Zaten bekleyen bir gösterim var; bu olay ona eklendi.
        return;
    };

    let app = app.clone();
    let device_name = device_name.to_string();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(delay).await;

        // Karar gecikme SONUNDA veriliyor: kullanıcı bu arada uygulamaya
        // dönmüş olabilir ve o durumda bildirim hiç gösterilmemeli.
        let focused = window_has_focus(&app);
        let count = {
            let Ok(mut state) = pending_state().lock() else {
                return;
            };
            let Some(pending) = state.get_mut(&(kind, key.clone())) else {
                return;
            };
            pending.take(Instant::now(), !focused)
        };

        if focused || count == 0 {
            return;
        }

        let (title, body) = if count == 1 {
            (title, body)
        } else {
            kind.summary(&device_name, count)
        };
        show_native(&app, &title, &body, action);
    });
}

#[cfg(windows)]
fn show_native(app: &AppHandle, title: &str, body: &str, action: Action) {
    use tauri_winrt_notification::Toast;

    let build = |app_id: &str| {
        let handle = app.clone();
        let action = action.clone();
        Toast::new(app_id)
            .title(title)
            .text1(body)
            .on_activated(move |_| {
                activate(&handle, &action);
                Ok(())
            })
    };

    // Uygulamanın kendi AppUserModelID'si yalnızca KURULU sürümlerde kayıtlıdır
    // (installer bir Başlat Menüsü kısayolu oluşturur). Kayıtsız bir kimlikle
    // toast göndermek HATA DÖNDÜRMEZ: Windows bildirimi sessizce yutar. Bir
    // önceki sürüm "önce kendi kimliğini dene, başarısızsa PowerShell'e düş"
    // yapıyordu; ilk deneme hep "başarılı" göründüğü için geliştirme
    // yapılarında bildirimler tamamen kayboldu.
    //
    // Bu yüzden karar denemeyle değil, yapı tipiyle veriliyor: geliştirme
    // yapısı kurulmadığı için PowerShell'in kayıtlı kimliğini ödünç alır
    // (gönderen adı "Windows PowerShell" görünür ama bildirim ÇIKAR),
    // kurulu sürüm kendi kimliğini kullanır.
    let identifier = app.config().identifier.clone();
    let app_id = if cfg!(debug_assertions) {
        Toast::POWERSHELL_APP_ID
    } else {
        &identifier
    };

    if let Err(err) = build(app_id).show() {
        tracing::debug!(error = %err, "bildirim gösterilemedi");
    }
}

#[cfg(not(windows))]
fn show_native(app: &AppHandle, title: &str, body: &str, _action: Action) {
    use tauri_plugin_notification::NotificationExt;

    // Bu platformlarda eklenti tıklama olayı vermiyor; bildirim gösterilir
    // ama yönlendirme yapılamaz.
    if let Err(err) = app.notification().builder().title(title).body(body).show() {
        tracing::debug!(error = %err, "bildirim gösterilemedi");
    }
}

/// Yeni sohbet mesajı geldi.
pub fn message_received(app: &AppHandle, device_id: &str, device_name: &str, preview: &str) {
    notify(
        app,
        Kind::Message,
        device_id.to_string(),
        device_name,
        device_name.to_string(),
        truncate(preview, 140),
        Action::OpenChat {
            device_id: device_id.to_string(),
        },
    );
}

/// Dosya alındı.
pub fn file_received(app: &AppHandle, device_id: &str, device_name: &str, file_name: &str) {
    notify(
        app,
        Kind::FileReceived,
        device_id.to_string(),
        device_name,
        format!("{device_name} bir dosya gönderdi"),
        file_name.to_string(),
        Action::OpenFiles,
    );
}

/// Dosya gönderme isteği onay bekliyor.
pub fn file_offer(app: &AppHandle, device_id: &str, device_name: &str, file_name: &str) {
    notify(
        app,
        Kind::FileOffer,
        device_id.to_string(),
        device_name,
        format!("{device_name} dosya göndermek istiyor"),
        file_name.to_string(),
        Action::OpenChat {
            device_id: device_id.to_string(),
        },
    );
}

/// Uzun metni bildirime sığacak şekilde kısaltır.
fn truncate(text: &str, max: usize) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() <= max {
        return trimmed.to_string();
    }
    let cut: String = trimmed.chars().take(max).collect();
    format!("{cut}…")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kisa_metin_bozulmaz() {
        assert_eq!(truncate("merhaba", 140), "merhaba");
        assert_eq!(truncate("  boşluklu  ", 140), "boşluklu");
    }

    #[test]
    fn uzun_metin_kisaltilir() {
        let long = "a".repeat(200);
        let result = truncate(&long, 140);

        assert_eq!(result.chars().count(), 141, "140 karakter + üç nokta");
        assert!(result.ends_with('…'));
    }

    /// Türkçe karakterler çok baytlıdır; bayt sınırından kesmek metni bozar.
    #[test]
    fn cok_baytli_karakterler_bozulmaz() {
        let text = "ğüşiöçĞÜŞİÖÇ".repeat(20);
        let result = truncate(&text, 10);

        assert_eq!(result.chars().count(), 11);
        assert!(result.starts_with("ğüşiöçĞÜŞİ"));
    }

    /// Biriken bildirimlerin özeti, tek olayın metnini tekrar etmemeli:
    /// kullanıcı beş mesajın hangisi olduğunu değil, kaç tane olduğunu
    /// öğrenmek ister.
    #[test]
    fn birden_fazla_olay_ozetlenir() {
        assert_eq!(
            Kind::Message.summary("FIRAT (B)", 3),
            ("FIRAT (B)".to_string(), "3 yeni mesaj".to_string())
        );
        assert_eq!(
            Kind::FileReceived.summary("FIRAT (B)", 2).0,
            "FIRAT (B) 2 dosya gönderdi"
        );
        assert_eq!(
            Kind::FileOffer.summary("FIRAT (B)", 4).0,
            "FIRAT (B) 4 dosya göndermek istiyor"
        );
    }

    /// Yönlendirme bilgisi frontend'in ayırt edebileceği biçimde gitmeli.
    ///
    /// Alan adı ARAYÜZÜN OKUDUĞU adla birebir sınanır. Bu testin ilk hâli
    /// yalnızca değerin (`ABC`) çıktıda geçtiğine bakıyordu; alan adı yanlış
    /// olduğu hâlde geçiyordu ve hatayı kaçırdı.
    #[test]
    fn eylem_serilestirmesi_ayirt_edilebilir() {
        let chat = serde_json::to_string(&Action::OpenChat {
            device_id: "ABC".into(),
        })
        .unwrap();
        assert_eq!(chat, r#"{"kind":"openChat","deviceId":"ABC"}"#);

        let files = serde_json::to_string(&Action::OpenFiles).unwrap();
        assert_eq!(files, r#"{"kind":"openFiles"}"#);
    }
}
