//! Native bildirimler (PLAN.md §2.10).
//!
//! Kural: bildirim yalnızca kullanıcı uygulamaya BAKMIYORKEN gösterilir.
//! Ekranda açık duran bir sohbete gelen mesaj için ayrıca bildirim basmak,
//! kullanıcının zaten gördüğü şeyi tekrar etmektir ve hızla rahatsız edici
//! hâle gelir.
//!
//! **Tıklama desteği (Faz 7'de doğrulandı):** Tauri'nin `notification`
//! eklentisi masaüstünde HİÇBİR olay yayınlamaz — `onAction` ve
//! `onNotificationReceived` yalnızca mobil içindir. Planın §2.10'da
//! işaretlediği risk buydu. Bu yüzden Windows'ta eklenti yerine doğrudan
//! `tauri-winrt-notification` kullanılıyor: `on_activated` geri çağrısı
//! sayesinde toast'a tıklayınca pencere öne getirilip ilgili ekrana
//! yönlendirilebiliyor. Diğer platformlarda eklenti kullanılmaya devam ediyor
//! (orada tıklama yönlendirmesi henüz yok).

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

/// Bildirime tıklandığında frontend'e gönderilen olay.
pub const EVENT_ACTIVATED: &str = "notification:activated";

/// Bildirime tıklanınca gidilecek yer.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
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

fn notify(app: &AppHandle, title: &str, body: &str, action: Action) {
    if window_has_focus(app) {
        return;
    }
    show_native(app, title, body, action);
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
    // (installer bir Başlat Menüsü kısayolu oluşturur). Geliştirme sırasında
    // kayıtlı olmadığı için toast gösterilemez; o durumda PowerShell'in
    // kimliğine düşülür. Bildirimin hiç çıkmaması, farklı bir isimle
    // çıkmasından kötü.
    let identifier = app.config().identifier.clone();
    if build(&identifier).show().is_ok() {
        return;
    }

    if let Err(err) = build(Toast::POWERSHELL_APP_ID).show() {
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
        device_name,
        &truncate(preview, 140),
        Action::OpenChat {
            device_id: device_id.to_string(),
        },
    );
}

/// Dosya alındı.
pub fn file_received(app: &AppHandle, device_name: &str, file_name: &str) {
    notify(
        app,
        &format!("{device_name} bir dosya gönderdi"),
        file_name,
        Action::OpenFiles,
    );
}

/// Dosya gönderme isteği onay bekliyor.
pub fn file_offer(app: &AppHandle, device_id: &str, device_name: &str, file_name: &str) {
    notify(
        app,
        &format!("{device_name} dosya göndermek istiyor"),
        file_name,
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

    /// Yönlendirme bilgisi frontend'in ayırt edebileceği biçimde gitmeli.
    #[test]
    fn eylem_serilestirmesi_ayirt_edilebilir() {
        let chat = serde_json::to_string(&Action::OpenChat {
            device_id: "ABC".into(),
        })
        .unwrap();
        assert!(chat.contains("openChat") && chat.contains("ABC"), "{chat}");

        let files = serde_json::to_string(&Action::OpenFiles).unwrap();
        assert!(files.contains("openFiles"), "{files}");
    }
}
