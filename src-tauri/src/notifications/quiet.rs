//! Sessiz saatler (PLAN.md §3.4).
//!
//! Ayar `"HH:MM-HH:MM"` biçiminde tek bir dize; boş olması "kapalı" demek.
//! Gece yarısını geçen aralıklar (`23:00-07:00`) desteklenmek zorunda —
//! sessiz saatlerin tipik kullanımı zaten bu.

/// Aralığı gün içindeki dakikalara çevirir.
fn parse(range: &str) -> Option<(u32, u32)> {
    let (start, end) = range.trim().split_once('-')?;
    Some((minutes(start)?, minutes(end)?))
}

fn minutes(time: &str) -> Option<u32> {
    let (hours, mins) = time.trim().split_once(':')?;
    let hours: u32 = hours.trim().parse().ok()?;
    let mins: u32 = mins.trim().parse().ok()?;
    (hours < 24 && mins < 60).then_some(hours * 60 + mins)
}

/// Verilen dakika, aralığın içinde mi?
///
/// Bozuk bir ayar "sessiz DEĞİL" sayılır: kullanıcının yanlış yazdığı bir
/// aralık yüzünden bildirimleri sessizce kaybetmek, en kötü sonuç.
pub fn is_quiet_at(range: &str, minute_of_day: u32) -> bool {
    let Some((start, end)) = parse(range) else {
        return false;
    };

    if start == end {
        // Sıfır uzunluklu aralık: kullanıcı muhtemelen bir şeyi yanlış
        // girdi. "Hep sessiz" varsaymak yerine kapalı sayılır.
        return false;
    }

    if start < end {
        (start..end).contains(&minute_of_day)
    } else {
        // Gece yarısını geçiyor: 23:00-07:00 → gün sonu VEYA gün başı.
        minute_of_day >= start || minute_of_day < end
    }
}

/// Şu anki yerel saate göre sessiz mi?
pub fn is_quiet_now(range: &str) -> bool {
    if range.trim().is_empty() {
        return false;
    }
    is_quiet_at(range, local_minute_of_day())
}

/// Yerel saatin gün içindeki dakikası.
///
/// Zaman dilimi kütüphanesi eklemek yerine platformun kendi yerel saatinden
/// okunuyor: burada gereken tek şey "kullanıcının duvar saatinde kaç?".
fn local_minute_of_day() -> u32 {
    #[cfg(windows)]
    {
        use windows::Win32::System::SystemInformation::GetLocalTime;
        let now = unsafe { GetLocalTime() };
        u32::from(now.wHour) * 60 + u32::from(now.wMinute)
    }
    #[cfg(not(windows))]
    {
        // Diğer platformlarda UTC'ye düşülüyor; sessiz saatler orada yanlış
        // dilimde çalışır. Windows-öncelikli bir uygulamada kabul edilen sınır.
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        ((secs % 86_400) / 60) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bos_ayar_sessiz_degildir() {
        assert!(!is_quiet_now(""));
        assert!(!is_quiet_at("", 12 * 60));
    }

    #[test]
    fn gun_ici_aralik() {
        let range = "09:00-17:30";
        assert!(!is_quiet_at(range, 8 * 60 + 59));
        assert!(is_quiet_at(range, 9 * 60));
        assert!(is_quiet_at(range, 17 * 60 + 29));
        assert!(!is_quiet_at(range, 17 * 60 + 30), "bitiş dâhil değil");
    }

    /// Sessiz saatlerin asıl kullanımı bu: gece yarısını geçen aralık.
    #[test]
    fn gece_yarisini_gecen_aralik() {
        let range = "23:00-07:00";
        assert!(is_quiet_at(range, 23 * 60));
        assert!(is_quiet_at(range, 0), "gece yarısı");
        assert!(is_quiet_at(range, 6 * 60 + 59));
        assert!(!is_quiet_at(range, 7 * 60));
        assert!(!is_quiet_at(range, 12 * 60));
    }

    /// Bozuk ayar yüzünden bildirim kaybetmek, en kötü sonuç.
    #[test]
    fn bozuk_ayar_sessiz_saymaz() {
        for range in [
            "saçma",
            "25:00-07:00",
            "23:60-07:00",
            "23:00",
            "-",
            "a:b-c:d",
        ] {
            assert!(!is_quiet_at(range, 0), "{range} sessiz sayılmamalı");
            assert!(!is_quiet_at(range, 23 * 60 + 30), "{range}");
        }
    }

    #[test]
    fn sifir_uzunluklu_aralik_hep_sessiz_degildir() {
        assert!(!is_quiet_at("22:00-22:00", 22 * 60));
        assert!(!is_quiet_at("22:00-22:00", 3 * 60));
    }
}
