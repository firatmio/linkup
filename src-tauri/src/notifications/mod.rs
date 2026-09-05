//! Native bildirimler (PLAN.md §2.10).
//!
//! Kural: bildirim yalnızca kullanıcı uygulamaya BAKMIYORKEN gösterilir.
//! Ekranda açık duran bir sohbete gelen mesaj için ayrıca bildirim basmak,
//! kullanıcının zaten gördüğü şeyi tekrar etmektir ve hızla rahatsız edici
//! hâle gelir.

use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

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

fn notify(app: &AppHandle, title: &str, body: &str) {
    if window_has_focus(app) {
        return;
    }

    if let Err(err) = app.notification().builder().title(title).body(body).show() {
        // Bildirim gösterilememesi akışı durdurmamalı; kullanıcı izin
        // vermemiş olabilir.
        tracing::debug!(error = %err, "bildirim gösterilemedi");
    }
}

/// Yeni sohbet mesajı geldi.
pub fn message_received(app: &AppHandle, device_name: &str, preview: &str) {
    // Mesaj içeriği loglanmaz ama bildirimde gösterilir: bildirim zaten
    // kullanıcının kendi ekranı (PLAN.md §2.14 log kuralıyla çelişmez).
    notify(app, device_name, &truncate(preview, 140));
}

/// Dosya alındı.
pub fn file_received(app: &AppHandle, device_name: &str, file_name: &str) {
    notify(app, &format!("{device_name} bir dosya gönderdi"), file_name);
}

/// Dosya gönderme isteği onay bekliyor.
pub fn file_offer(app: &AppHandle, device_name: &str, file_name: &str) {
    notify(
        app,
        &format!("{device_name} dosya göndermek istiyor"),
        file_name,
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
}
