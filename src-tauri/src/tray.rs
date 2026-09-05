//! Sistem tepsisi ve pencere yaşam döngüsü (PLAN.md §2.11).
//!
//! LinkUp arka planda çalışmadığında dosya ve mesaj alamaz: pencereyi kapatmak
//! sessizce "erişilemez" olmak demektir. Bu yüzden kapatma düğmesi varsayılan
//! olarak pencereyi tepsiye küçültür ve uygulama yaşamaya devam eder. Kullanıcı
//! bunu ayarlardan kapatabilir — davranışın kendisi değil, kullanıcının haberi
//! olması önemli.

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, WindowEvent};

use crate::db::settings;
use crate::state::AppState;

/// Menü öğesi kimlikleri.
const ID_OPEN: &str = "open";
const ID_SETTINGS: &str = "settings";
const ID_QUIT: &str = "quit";

/// Frontend'e "ayarları aç" demek için kullanılan olay.
pub const EVENT_NAVIGATE: &str = "app:navigate";

/// Tepsi simgesini kurar.
pub fn setup(app: &AppHandle, tooltip: &str) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, ID_OPEN, "Aç", true, None::<&str>)?;
    let settings_item = MenuItem::with_id(app, ID_SETTINGS, "Ayarlar", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, ID_QUIT, "Çıkış", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &settings_item, &quit])?;

    TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().cloned().ok_or_else(|| {
            tauri::Error::AssetNotFound("uygulama simgesi bulunamadı".to_string())
        })?)
        .tooltip(tooltip)
        .menu(&menu)
        // Menü sol tıklamada AÇILMASIN: Windows'ta tepsi simgesine sol tıklamak
        // uygulamayı açar, menü sağ tıklamaya aittir.
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            ID_OPEN => show_window(app),
            ID_SETTINGS => {
                show_window(app);
                use tauri::Emitter;
                let _ = app.emit(EVENT_NAVIGATE, "/settings");
            }
            ID_QUIT => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

/// Pencereyi geri getirir ve öne alır.
pub fn show_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    // Simge durumundaysa önce geri al: `set_focus` küçültülmüş pencereyi
    // geri getirmez.
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
}

/// Pencere olaylarını yakalar: kapatma isteği ayara göre gizlemeye çevrilir.
pub fn on_window_event(window: &tauri::Window, event: &WindowEvent) {
    let WindowEvent::CloseRequested { api, .. } = event else {
        return;
    };
    if window.label() != "main" {
        return;
    }

    // Ayar okunamıyorsa pencere KAPANIR. Kapanmayan bir pencere, kullanıcının
    // uygulamadan çıkamaması demektir; okunamayan bir ayar bunu haklı çıkarmaz.
    let app = window.app_handle();
    let close_to_tray = app
        .try_state::<AppState>()
        .and_then(|state| {
            state
                .db
                .get()
                .ok()
                .and_then(|conn| settings::load(&conn).ok())
        })
        .map(|settings| settings.close_to_tray)
        .unwrap_or(false);

    if close_to_tray {
        api.prevent_close();
        let _ = window.hide();
    }
}
