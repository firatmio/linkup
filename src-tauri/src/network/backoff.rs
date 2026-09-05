//! Yeniden bağlanma gecikmesi (PLAN.md §2.14).
//!
//! Kopan bağlantı üstel olarak artan aralıklarla denenir: 1s → 2s → 4s … 30s.
//! Üst sınır olmadan bekleme süresi saatlere çıkar; alt sınır olmadan da kopuk
//! bir ağda saniyede onlarca deneme yapılır.
//!
//! Faz 3'te devreye girecek: yeniden bağlanma denetleyicisi, bağlanacak
//! adresleri keşiften (mDNS) aldığında bu gecikmeleri kullanacak. Mantık ve
//! testleri burada hazır duruyor.

use std::time::Duration;

const INITIAL: Duration = Duration::from_secs(1);
const MAX: Duration = Duration::from_secs(30);
const FACTOR: u32 = 2;

/// Bir hedefe yeniden bağlanma denemelerinin gecikmesini üretir.
#[derive(Debug, Clone)]
pub struct Backoff {
    current: Duration,
    attempts: u32,
}

impl Default for Backoff {
    fn default() -> Self {
        Self::new()
    }
}

impl Backoff {
    pub fn new() -> Self {
        Self {
            current: INITIAL,
            attempts: 0,
        }
    }

    /// Sıradaki bekleme süresini döndürür ve ilerler.
    pub fn next_delay(&mut self) -> Duration {
        let delay = self.current;
        self.current = (self.current * FACTOR).min(MAX);
        self.attempts += 1;
        delay
    }

    /// Bağlantı kurulduğunda çağrılır; sayaç başa döner.
    pub fn reset(&mut self) {
        self.current = INITIAL;
        self.attempts = 0;
    }

    pub fn attempts(&self) -> u32 {
        self.attempts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ustel_artar_ve_tavanda_kalir() {
        let mut backoff = Backoff::new();
        assert_eq!(backoff.next_delay(), Duration::from_secs(1));
        assert_eq!(backoff.next_delay(), Duration::from_secs(2));
        assert_eq!(backoff.next_delay(), Duration::from_secs(4));
        assert_eq!(backoff.next_delay(), Duration::from_secs(8));
        assert_eq!(backoff.next_delay(), Duration::from_secs(16));

        // Tavana oturur ve orada kalır — süresiz büyümez.
        for _ in 0..20 {
            assert_eq!(backoff.next_delay(), MAX);
        }
    }

    #[test]
    fn basarili_baglanti_sifirlar() {
        let mut backoff = Backoff::new();
        for _ in 0..5 {
            backoff.next_delay();
        }
        assert!(backoff.attempts() > 0);

        backoff.reset();
        assert_eq!(backoff.attempts(), 0);
        assert_eq!(backoff.next_delay(), INITIAL);
    }
}
