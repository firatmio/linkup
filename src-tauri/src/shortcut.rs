//! Global kısayol ve hızlı gönder penceresi (PLAN.md §2.11).
//!
//! Kısayol uygulamanın DIŞINDA da çalışır; bu yüzden başarısız olması sessiz
//! geçilemez: başka bir uygulama aynı kombinasyonu almışsa kullanıcı tuşa
//! basıp hiçbir şey olmamasını "uygulama bozuk" diye okur. Kayıt sonucu
//! saklanıyor ve ayarlar ekranında gösteriliyor.

use std::sync::Mutex;

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::error::{AppError, AppResult};

/// Hızlı gönder penceresinin etiketi.
pub const QUICK_WINDOW: &str = "quick";

/// Şu an kayıtlı olan kısayol. Yenisini kaydetmeden önce eskisi kaldırılır.
fn current() -> &'static Mutex<Option<Shortcut>> {
    static CURRENT: std::sync::OnceLock<Mutex<Option<Shortcut>>> = std::sync::OnceLock::new();
    CURRENT.get_or_init(|| Mutex::new(None))
}

/// Kısayolu kaydeder; öncekini kaldırır.
///
/// Boş bir hızlandırıcı "kısayol istemiyorum" demektir ve hata değildir.
pub fn register(app: &AppHandle, accelerator: &str) -> AppResult<()> {
    let mut slot = current()
        .lock()
        .map_err(|_| AppError::Internal(anyhow::anyhow!("kısayol durumu kilitlendi")))?;

    if let Some(previous) = slot.take() {
        let _ = app.global_shortcut().unregister(previous);
    }

    let accelerator = accelerator.trim();
    if accelerator.is_empty() {
        return Ok(());
    }

    let shortcut: Shortcut = accelerator
        .parse()
        .map_err(|_| AppError::InvalidInput("error.shortcut.invalid".to_string()))?;

    app.global_shortcut()
        .on_shortcut(shortcut, |app, _shortcut, event| {
            // Yalnızca BASILDIĞINDA: bırakma olayı da gelir ve pencereyi iki
            // kez açmaya çalışırdı.
            if event.state() == ShortcutState::Pressed {
                open_quick_send(app);
            }
        })
        .map_err(|_| AppError::InvalidInput("error.shortcut.taken".to_string()))?;

    *slot = Some(shortcut);
    tracing::info!(accelerator, "global kısayol kaydedildi");
    Ok(())
}

/// Hızlı gönder penceresini açar; zaten açıksa öne getirir.
///
/// Ana pencereden ayrı bir pencere: kısayolun amacı, uygulamanın tamamını
/// açmadan tek bir dosyayı yollamak.
pub fn open_quick_send(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(QUICK_WINDOW) {
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }

    let result = WebviewWindowBuilder::new(
        app,
        QUICK_WINDOW,
        WebviewUrl::App("index.html#/quick-send".into()),
    )
    .title("LinkUp — Hızlı Gönder")
    .inner_size(420.0, 420.0)
    .resizable(false)
    .always_on_top(true)
    .center()
    // Kendi başlık çubuğu var: bu pencere sistem çerçevesi taşıyacak kadar
    // büyük değil, kullanıcı onu tuşla açıp saniyeler içinde kapatıyor.
    .decorations(false)
    .shadow(true)
    // Görev çubuğunda ikinci bir giriş oluşturmaz: bu bir araç penceresi,
    // uygulamanın kendisi değil.
    .skip_taskbar(true)
    .build();

    if let Err(err) = result {
        tracing::warn!(error = %err, "hızlı gönder penceresi açılamadı");
    }
}
