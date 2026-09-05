//! Öndeki uygulamada SEÇİLİ olan metni yakalar (PLAN.md §2.9).
//!
//! Windows'ta başka bir uygulamanın seçimini okuyan bir API yok. Tek yol,
//! bunu yapan diğer araçların (sözlük/çeviri açılır pencereleri) yaptığı şey:
//! öndeki pencereye Ctrl+C göndermek ve panoyu okumak.
//!
//! **Bunun bir bedeli var ve gizlenmiyor:** Ctrl+C her uygulamada "kopyala"
//! demek değildir. Konsol pencerelerinde seçim yokken çalışan komutu
//! durdurur. Bu yüzden davranış ayarlardan kapatılabiliyor ve açıklaması
//! bu uyarıyla birlikte duruyor.
//!
//! Kullanıcının panosu KORUNUR: yakalamadan önceki metin geri yazılır.
//! Kullanıcı yalnızca bir şey seçmişti, kopyalamamıştı — panosunu sessizce
//! değiştirmek ondan istenmeyen bir şey yapmak olurdu. Panoda metin dışında
//! bir şey varsa (görsel, dosya) geri yazılamaz; bu sınır kabul edildi.

use std::time::Duration;

/// Kopyalamanın panoya yansıması için beklenen en uzun süre.
const CAPTURE_TIMEOUT: Duration = Duration::from_millis(400);

/// Pano iki denemesi arasındaki bekleme.
const POLL_INTERVAL: Duration = Duration::from_millis(25);

/// Öndeki uygulamadaki seçimi döndürür.
///
/// Seçim yoksa, uygulama Ctrl+C'yi desteklemiyorsa veya süre dolarsa `None`
/// döner — hiçbiri hata değil, hepsi "seçili metin yok" demek.
#[cfg(windows)]
pub fn capture(app: &tauri::AppHandle) -> Option<String> {
    use tauri_plugin_clipboard_manager::ClipboardExt;

    let clipboard = app.clipboard();
    // Boş pano da geçerli bir "önceki hâl": karşılaştırma için normalize edilir.
    let before = clipboard.read_text().ok();

    send_copy()?;

    let deadline = std::time::Instant::now() + CAPTURE_TIMEOUT;
    while std::time::Instant::now() < deadline {
        std::thread::sleep(POLL_INTERVAL);

        let Ok(current) = clipboard.read_text() else {
            continue;
        };
        if current.trim().is_empty() || Some(&current) == before.as_ref() {
            continue;
        }

        // Yakalandı. Kullanıcının panosu geri yazılır.
        if let Some(previous) = before {
            let _ = clipboard.write_text(previous);
        }
        return Some(current);
    }

    None
}

/// Öndeki pencereye Ctrl+C gönderir.
///
/// `SendInput` odaktaki pencereye gider; bu yüzden yakalama, kendi
/// penceremizi AÇMADAN önce yapılmalıdır — açtıktan sonra odak bizde olurdu
/// ve boş bir seçimi kopyalamaya çalışırdık.
#[cfg(windows)]
fn send_copy() -> Option<()> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
        VK_C, VK_CONTROL,
    };

    fn key(code: u16, up: bool) -> INPUT {
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(code),
                    wScan: 0,
                    dwFlags: if up {
                        KEYEVENTF_KEYUP
                    } else {
                        KEYBD_EVENT_FLAGS(0)
                    },
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }
    }

    let inputs = [
        key(VK_CONTROL.0, false),
        key(VK_C.0, false),
        key(VK_C.0, true),
        key(VK_CONTROL.0, true),
    ];

    let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    (sent == inputs.len() as u32).then_some(())
}

/// Diğer platformlarda seçim yakalama yok; pano yeterli.
#[cfg(not(windows))]
pub fn capture(_app: &tauri::AppHandle) -> Option<String> {
    None
}
