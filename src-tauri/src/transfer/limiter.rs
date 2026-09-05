//! Gönderim hızı sınırı (PLAN.md §2.7.4).
//!
//! Token-bucket: saniyede `rate` bayt biriktirilir, gönderilen her bayt bir
//! token harcar. Amaç aynı ağdaki diğer işleri boğmamak — bir dosya transferi
//! yüzünden görüntülü görüşmenin donması kullanıcının kabul edeceği bir şey
//! değil.
//!
//! Sınır kapalıyken (0) hiçbir hesap yapılmaz; sıcak yol bedavaya gelir.

use std::time::Duration;

use tokio::time::Instant;

/// Kova kapasitesi: bir saniyelik hakkın üstünde birikim olmaz. Aksi hâlde
/// uzun bir duraklamadan sonra tek seferde patlama yaşanır ve sınır anlamını
/// yitirir.
const BURST_SECONDS: f64 = 1.0;

#[derive(Debug)]
pub struct RateLimiter {
    /// Bayt/saniye. 0 = sınırsız.
    rate: u64,
    tokens: f64,
    last_refill: Instant,
}

impl RateLimiter {
    pub fn new(bytes_per_second: u64) -> Self {
        Self {
            rate: bytes_per_second,
            tokens: bytes_per_second as f64,
            last_refill: Instant::now(),
        }
    }

    /// `amount` bayt göndermeden önce beklenmesi gereken süre.
    /// Saf hesap — test edilebilir olması için uyku burada yapılmaz.
    fn delay_for(&mut self, amount: u64, now: Instant) -> Duration {
        if self.rate == 0 {
            return Duration::ZERO;
        }

        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.last_refill = now;
        self.tokens =
            (self.tokens + elapsed * self.rate as f64).min(self.rate as f64 * BURST_SECONDS);

        let amount = amount as f64;
        if self.tokens >= amount {
            self.tokens -= amount;
            return Duration::ZERO;
        }

        let missing = amount - self.tokens;
        self.tokens = 0.0;
        Duration::from_secs_f64(missing / self.rate as f64)
    }

    /// Gerekiyorsa bekler, sonra token'ları düşer.
    pub async fn acquire(&mut self, amount: u64) {
        if self.rate == 0 {
            return;
        }
        let delay = self.delay_for(amount, Instant::now());
        if !delay.is_zero() {
            tokio::time::sleep(delay).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn sinirsizken_hic_beklenmez() {
        let mut limiter = RateLimiter::new(0);
        assert_eq!(
            limiter.delay_for(u64::MAX, Instant::now()),
            Duration::ZERO,
            "sınır kapalıyken hesap yapılmamalı"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn kova_dolu_baslar_ilk_gonderim_beklemez() {
        let mut limiter = RateLimiter::new(1000);
        assert_eq!(limiter.delay_for(1000, Instant::now()), Duration::ZERO);
    }

    #[tokio::test(start_paused = true)]
    async fn kova_bosalinca_beklenir() {
        let now = Instant::now();
        let mut limiter = RateLimiter::new(1000);

        // İlk 1000 bayt kovadan çıkar.
        assert_eq!(limiter.delay_for(1000, now), Duration::ZERO);

        // Hemen ardından 500 bayt daha: yarım saniye beklemeli.
        let delay = limiter.delay_for(500, now);
        assert!(
            (delay.as_secs_f64() - 0.5).abs() < 0.01,
            "beklenen ~0.5 sn, gelen {delay:?}"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn zaman_gectikce_token_birikir() {
        let start = Instant::now();
        let mut limiter = RateLimiter::new(1000);
        limiter.delay_for(1000, start);

        // Yarım saniye sonra 500 token birikmiş olmalı.
        let later = start + Duration::from_millis(500);
        assert_eq!(limiter.delay_for(500, later), Duration::ZERO);
    }

    /// Uzun duraklamadan sonra sınırsız birikim olmamalı; aksi hâlde tek
    /// seferde patlama yaşanır ve sınır anlamını yitirir.
    #[tokio::test(start_paused = true)]
    async fn birikim_bir_saniyelik_hakla_sinirli() {
        let start = Instant::now();
        let mut limiter = RateLimiter::new(1000);
        limiter.delay_for(1000, start);

        // On saniye bekledik ama kova en fazla 1000 token tutar.
        let later = start + Duration::from_secs(10);
        assert_eq!(limiter.delay_for(1000, later), Duration::ZERO);

        // Kova boşaldı: bir sonraki istek beklemeli.
        let delay = limiter.delay_for(1000, later);
        assert!(delay > Duration::ZERO, "kova sınırsız birikmemeli");
    }
}
