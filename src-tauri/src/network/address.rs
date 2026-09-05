//! Adres sıralama.
//!
//! Bir makinenin birden fazla IPv4 adresi olur: gerçek LAN arayüzü, loopback,
//! link-local (169.254.x, DHCP başarısız olduğunda atanır) ve WSL/Docker/Hyper-V
//! gibi sanal adaptörler. Karşı cihazın bize ulaşabileceği adres bunların
//! yalnızca biridir; listenin "ilk" elemanını almak yanlış adaptöre bağlanma
//! denemesiyle sonuçlanır ve zaman aşımına kadar asılı kalır.

use std::net::{IpAddr, Ipv4Addr};

/// Küçük değer = daha muhtemel doğru adres.
pub fn address_rank(ip: &IpAddr) -> u8 {
    match ip {
        IpAddr::V4(v4) => rank_v4(v4),
        // IPv6 keşifte kullanılmıyor (scope id gerektirir); en sona.
        IpAddr::V6(_) => 9,
    }
}

fn rank_v4(ip: &Ipv4Addr) -> u8 {
    let [a, b, ..] = ip.octets();
    match () {
        // Ev ve ofis ağlarının ezici çoğunluğu.
        _ if a == 192 && b == 168 => 0,
        _ if a == 10 => 1,
        // Özel aralık ama WSL, Docker ve Hyper-V de burayı kullanıyor —
        // gerçek LAN'dan sonra denenmeli.
        _ if a == 172 && (16..=31).contains(&b) => 2,
        // Aynı makinedeki ikinci instance için geçerli, başka makine için değil.
        _ if ip.is_loopback() => 3,
        // DHCP başarısız olmuş demektir; karşı taraf muhtemelen ulaşamaz.
        _ if ip.is_link_local() => 5,
        _ => 4,
    }
}

/// Adresleri en muhtemelden en az muhtemele sıralar.
pub fn sort_by_reachability<T: Copy>(items: &mut [T], ip_of: impl Fn(&T) -> IpAddr) {
    items.sort_by_key(|item| address_rank(&ip_of(item)));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    #[test]
    fn lan_adresi_sanal_adaptorden_once_gelir() {
        assert!(address_rank(&ip("192.168.0.195")) < address_rank(&ip("172.17.80.1")));
        assert!(address_rank(&ip("10.0.0.5")) < address_rank(&ip("172.24.160.1")));
    }

    #[test]
    fn loopback_ve_link_local_sona_duser() {
        let lan = address_rank(&ip("192.168.0.195"));
        assert!(lan < address_rank(&ip("127.0.0.1")));
        assert!(lan < address_rank(&ip("169.254.188.223")));
        // Link-local, loopback'ten de kötü: aynı makinede loopback çalışır.
        assert!(address_rank(&ip("127.0.0.1")) < address_rank(&ip("169.254.188.223")));
    }

    #[test]
    fn ipv6_en_sonda() {
        assert!(address_rank(&ip("192.168.1.1")) < address_rank(&ip("fe80::1")));
        assert!(address_rank(&ip("169.254.1.1")) < address_rank(&ip("::1")));
    }

    /// Gerçek bir keşif çıktısıyla: bu makinenin ilan ettiği adresler.
    #[test]
    fn gercek_ilan_dogru_siralanir() {
        let mut addresses: Vec<SocketAddr> = [
            "127.0.0.1:47812",
            "172.17.80.1:47812",
            "169.254.188.223:47812",
            "192.168.0.195:47812",
            "172.24.160.1:47812",
        ]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

        sort_by_reachability(&mut addresses, |addr| addr.ip());

        assert_eq!(
            addresses[0],
            "192.168.0.195:47812".parse::<SocketAddr>().unwrap(),
            "gerçek LAN adresi ilk sırada olmalı"
        );
        assert_eq!(
            *addresses.last().unwrap(),
            "169.254.188.223:47812".parse::<SocketAddr>().unwrap(),
            "link-local en sonda olmalı"
        );
    }
}
