# LinkUp

LAN üzerinden cihazları eşleştirip sohbet ve dosya/klasör transferi yapabilen masaüstü uygulaması.
Tauri 2 · Rust · React 19 + TypeScript.

Ayrıntılı mimari, protokol tasarımı ve faz planı için: [PLAN.md](PLAN.md)

## Durum

**Faz 6 — Ana sayfa tamamlandı.**

- Faz 0: navigasyon, tema, i18n, loglama, çok-profilli geliştirme kurulumu
- Faz 1: SQLite şeması + migration altyapısı, kalıcı ayarlar, Ed25519 cihaz
  kimliği (OS keychain'de saklanır), fingerprint görüntüleme
- Faz 2: QUIC uç noktası, kimlikten türetilen TLS sertifikası + public key
  pinlemesi, `Hello` sürüm anlaşması, heartbeat
- Faz 3: mDNS ile otomatik cihaz keşfi, elle IP ekleme, "Bulunanlar" listesi
- Faz 4: karşılıklı doğrulama koduyla (SAS) eşleştirme, güvenilir cihaz kaydı,
  pinlenmiş anahtarla otomatik yeniden bağlanma, "Cihazı Unut"
- Faz 5: metin mesajlaşma, kalıcı geçmiş, iletildi/görüldü göstergesi,
  syntax highlight'lı kod blokları
- Faz 6: ana sayfada cihaz özet kartları — çevrimiçilik, son mesaj, göreli
  zaman, okunmamış sayısı; karta tıklayınca sohbet açılıyor

Dosya transferi henüz yok. Sohbette görsel önizleme de transferle birlikte
geliyor (bkz. PLAN.md Faz 7.5).

### Ölçüm

Loopback throughput (512 MiB, release): **263 MiB/s ≈ 2203 Mbit/s** — hedef
≥400 Mbit/s. Ayrıntı ve ölçümün sınırları: [PLAN.md §2.2.2](PLAN.md).

```bash
cd src-tauri && cargo test --release -- --ignored --nocapture throughput
```

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

Birinci terminal (derler, Vite sunucusunu ve birinci pencereyi açar):

```bash
bun run dev:a
```

Derleme bitip pencere açıldıktan sonra ikinci terminalde:

```bash
bun run dev:b
```

| | `dev:a` | `dev:b` |
|---|---|---|
| Veri dizini | `%APPDATA%/LinkUp-a` | `%APPDATA%/LinkUp-b` |
| QUIC portu | 47811 | 47812 |
| Pencere başlığı | LinkUp (A) | LinkUp (B) |

Üçüncü bir instance gerekirse: `bun run dev:c`

**Nasıl çalışıyor:** `tauri dev`i iki kez çalıştırmak Windows'ta başarısız olur —
ikinci cargo derlemesi, birincinin çalıştırdığı `linkup.exe`yi silemez. Bu yüzden
`dev:a` tek derlemeyi ve tek Vite sunucusunu yönetir; `dev:b` derlenmiş
binary'nin profile özel bir kopyasını çalıştırır. İki pencere de aynı Vite
sunucusundan beslendiği için **frontend değişiklikleri ikisinde de anında yansır**.
Rust tarafı değiştiğinde `dev:a` kendini yeniler; ikinci instance'ı elle yeniden
başlatın.

Üretim binary'sinde de aynı bayrak geçerlidir: `linkup.exe --profile a`

## Doğrulama

```bash
bun run check
```

Tip kontrolü → frontend build → `cargo fmt --check` → `cargo clippy -D warnings` → `cargo test`.

Bu, `.githooks/pre-push` hook'u ile **her push'tan önce otomatik çalışır**. Depoyu
ilk kez klonladıysanız hook'u etkinleştirin:

```bash
git config core.hooksPath .githooks
```

Atlamak gerekirse: `git push --no-verify`

Uzak bir CI servisi bilinçli olarak kullanılmıyor — gerekçe ve bu kararın hangi
koşullarda gözden geçirileceği: [PLAN.md §10-K9](PLAN.md).

## Görsel dil

Uygulama Windows 11 / Fluent (WinUI 3) görünümünü hedefler. Tüm renk, yarıçap,
tipografi ve hareket değerleri tek bir dosyada toplanmıştır:
[`src/styles/tokens.css`](src/styles/tokens.css). Bileşenlerde çıplak hex rengi
veya px yarıçapı bulunmaz — başka bir platform diline geçiş bu dosyadan yönetilir.

## Loglar

Uygulama içi: **Ayarlar → Gelişmiş → Log Klasörünü Aç**
Dosya yolu: `<veri dizini>/logs/linkup.<tarih>.log` (günlük rotasyon, son 7 gün)
