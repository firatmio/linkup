//! Bildirim biriktirme kuralları (PLAN.md §2.10).
//!
//! Sorun şuydu: her mesaj için ayrı bir toast üretiliyordu. Karşı taraf arka
//! arkaya beş mesaj yazdığında Windows'un kuyruğuna beş toast giriyor ve
//! bunlar teker teker gösteriliyordu — kullanıcı uygulamaya geri döndükten
//! sonra bile sırayla düşmeye devam ediyorlardı, çünkü toast'lar çoktan
//! Windows'a teslim edilmişti.
//!
//! Çözüm iki parçalı:
//!
//! 1. **Geciktirme.** Bildirim olay anında gösterilmez; kısa bir süre
//!    beklenir. O sürede gelen diğer olaylar aynı bildirimde toplanır ve
//!    kullanıcı bu arada uygulamaya dönerse bildirim hiç gösterilmez.
//! 2. **Bekleme süresi.** Bir bildirim gösterildikten sonra aynı konu için
//!    bir süre yenisi çıkmaz; o sürede birikenler tek bir özet olur.
//!
//! Bu modül yalnızca KARARI verir; göstermeyi çağıran katman yapar. Zaman
//! dışarıdan geçirilir, böylece kurallar saat beklemeden test edilebilir.

use std::time::{Duration, Instant};

/// Olay geldikten sonra bildirimin gösterilmesi için beklenen süre.
///
/// Yeterince kısa: kullanıcı bildirimin geciktiğini fark etmez. Yeterince
/// uzun: peş peşe yazılan mesajlar tek bildirimde toplanır ve uygulamaya
/// dönen kullanıcı gereksiz bir toast görmez.
pub const DEBOUNCE: Duration = Duration::from_millis(1500);

/// Aynı konu için iki bildirim arasındaki en az süre.
pub const COOLDOWN: Duration = Duration::from_secs(20);

/// Bir konunun (cihaz + bildirim türü) biriken durumu.
#[derive(Debug, Default)]
pub struct Pending {
    /// Henüz gösterilmemiş olay sayısı.
    pub count: u32,
    /// Bekleyen bir gösterim görevi var mı?
    pub scheduled: bool,
    /// Bu ana kadar yeni bildirim gösterilmez.
    pub next_allowed: Option<Instant>,
}

impl Pending {
    /// Yeni bir olay kaydeder ve gerekiyorsa ne kadar sonra gösterileceğini
    /// döndürür.
    ///
    /// `None`, "gösterim zaten planlanmış" demektir — ikinci bir görev
    /// başlatmak, biriktirmenin tüm amacını bozar.
    pub fn record(&mut self, now: Instant) -> Option<Duration> {
        self.count += 1;
        if self.scheduled {
            return None;
        }
        self.scheduled = true;

        // Bekleme süresi hâlâ sürüyorsa gösterim onun bitimine ertelenir.
        let cooldown_left = self
            .next_allowed
            .and_then(|at| at.checked_duration_since(now))
            .unwrap_or_default();

        Some(DEBOUNCE.max(cooldown_left))
    }

    /// Bekleyen görev çalıştığında çağrılır: gösterilecek olay sayısını verir
    /// ve durumu sıfırlar.
    ///
    /// `shown` yanlışsa (kullanıcı bu arada uygulamaya döndü) bekleme süresi
    /// başlatılmaz: gösterilmemiş bir bildirim sonrakini geciktirmemeli.
    pub fn take(&mut self, now: Instant, shown: bool) -> u32 {
        let count = std::mem::take(&mut self.count);
        self.scheduled = false;
        if shown && count > 0 {
            self.next_allowed = Some(now + COOLDOWN);
        }
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ilk_olay_kisa_gecikmeyle_planlanir() {
        let mut pending = Pending::default();
        assert_eq!(pending.record(Instant::now()), Some(DEBOUNCE));
    }

    /// Biriktirmenin özü: peş peşe gelen olaylar tek gösterime düşer.
    #[test]
    fn ayni_pencerede_gelen_olaylar_tek_gosterimde_toplanir() {
        let now = Instant::now();
        let mut pending = Pending::default();

        assert!(pending.record(now).is_some());
        assert_eq!(pending.record(now), None, "ikinci görev başlatılmamalı");
        assert_eq!(pending.record(now), None);

        assert_eq!(
            pending.take(now, true),
            3,
            "üçü tek bildirimde gösterilmeli"
        );
    }

    #[test]
    fn gosterimden_sonra_bekleme_suresi_uygulanir() {
        let start = Instant::now();
        let mut pending = Pending::default();

        pending.record(start);
        pending.take(start, true);

        // Bekleme süresinin ortasında gelen olay, süre bitince gösterilir.
        let mid = start + COOLDOWN / 2;
        assert_eq!(pending.record(mid), Some(COOLDOWN / 2));
    }

    /// Kullanıcı uygulamaya döndüğü için gösterilmeyen bildirim, sonraki
    /// bildirimi geciktirmemeli.
    #[test]
    fn gosterilmeyen_bildirim_bekleme_baslatmaz() {
        let start = Instant::now();
        let mut pending = Pending::default();

        pending.record(start);
        assert_eq!(pending.take(start, false), 1);

        assert_eq!(pending.record(start), Some(DEBOUNCE));
    }

    #[test]
    fn bekleme_bittikten_sonra_gecikme_normale_doner() {
        let start = Instant::now();
        let mut pending = Pending::default();

        pending.record(start);
        pending.take(start, true);

        let later = start + COOLDOWN + Duration::from_secs(1);
        assert_eq!(pending.record(later), Some(DEBOUNCE));
    }
}
