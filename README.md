# LinkUp

LAN üzerinden cihazları eşleştirip sohbet ve dosya/klasör transferi yapabilen masaüstü uygulaması.
Tauri 2 · Rust · React 19 + TypeScript.

Ayrıntılı mimari, protokol tasarımı ve faz planı için: [PLAN.md](PLAN.md)

## Durum

**Faz 0 — İskelet.** Navigasyon, tema, i18n, loglama ve çok-profilli geliştirme kurulumu hazır.
Ağ katmanı (QUIC), keşif (mDNS), eşleştirme ve transfer henüz yok.

## Gereksinimler

- [Bun](https://bun.sh) 1.3+
- Rust 1.95+ (stable)
- [Tauri ön koşulları](https://tauri.app/start/prerequisites/) (Windows'ta WebView2 + MSVC build tools)

## Kurulum

```bash
bun install
```

## Çalıştırma

```bash
bun run app
```

### Aynı makinede iki instance (P2P geliştirmesi için)

LinkUp bir P2P uygulaması; test etmek için iki cihaz gerekir. `--profile` bayrağı
bunu tek makinede mümkün kılar — her profil kendi veri dizinini, veritabanını,
kimlik anahtarını ve QUIC portunu kullanır.

İki ayrı terminalde:

```bash
bun run dev:a
```

```bash
bun run dev:b
```

| | `dev:a` | `dev:b` |
|---|---|---|
| Veri dizini | `%APPDATA%/LinkUp-a` | `%APPDATA%/LinkUp-b` |
| Vite portu | 1420 | 1422 |
| QUIC portu | 47811 | 47812 |
| Pencere başlığı | LinkUp (A) | LinkUp (B) |

> İkisi aynı `target/` dizinini paylaşır; ilk derleme biterken ikincisi cargo
> kilidinde bekler. Sonraki başlatmalar hızlıdır.

Üretim binary'sinde de aynı bayrak geçerlidir: `linkup.exe --profile a`

## Doğrulama

```bash
bun run typecheck
```

```bash
cd src-tauri && cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo test
```

## Görsel dil

Uygulama Windows 11 / Fluent (WinUI 3) görünümünü hedefler. Tüm renk, yarıçap,
tipografi ve hareket değerleri tek bir dosyada toplanmıştır:
[`src/styles/tokens.css`](src/styles/tokens.css). Bileşenlerde çıplak hex rengi
veya px yarıçapı bulunmaz — başka bir platform diline geçiş bu dosyadan yönetilir.

## Loglar

Uygulama içi: **Ayarlar → Gelişmiş → Log Klasörünü Aç**
Dosya yolu: `<veri dizini>/logs/linkup.<tarih>.log` (günlük rotasyon, son 7 gün)
