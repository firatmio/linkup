# LinkUp — Proje Planı (rev. 2)

LAN üzerinden (ileride internet üzerinden de) cihazları eşleştirip chat ve dosya/klasör transferi yapabilen masaüstü uygulaması.

> Bu doküman **uygulanabilir** olacak şekilde yazılmıştır: her bölüm, o bölümü kodlarken verilecek kararları içerir.
> Mimari kararların gerekçeleri ve reddedilen alternatifler §10 Karar Günlüğü'ndedir.

---

## 1. Genel Bakış

| | |
|---|---|
| **Ad** | LinkUp |
| **Platform** | Masaüstü — Windows (birincil), macOS, Linux |
| **Framework** | Tauri 2.x |
| **Backend** | Rust |
| **Frontend** | React 19 + TypeScript |
| **Kapsam** | v1: aynı LAN. v2: internet üzerinden (relay / NAT traversal) — mimari buna göre baştan hazırlanır |
| **UI dili** | Türkçe (i18n altyapısı baştan kurulur, v1'de tek dil yüklenir) |

### 1.1 Temel Özellikler (v1)

- Otomatik cihaz keşfi (mDNS) + **manuel IP ile ekleme** (mDNS'in engellendiği ağlar için)
- Karşılıklı doğrulama kodu (SAS) ile güvenli eşleştirme
- **Ana sayfa (Dashboard):** cihaz başına özet kart (son mesaj + son medyalar), açılış ekranı
- 1-1 metin sohbeti: düz metin, görsel önizleme, kod bloğu (syntax highlight), mesaj arama (FTS5), okundu bilgisi
- Dosya transferi: yüksek hızlı akış, kesintide kaldığı yerden devam (resume)
- Seçili klasör senkronizasyonu
- Gelen dosyalar için özelleştirilebilir indirme konumu (varsayılan: `İndirilenler/LinkUp/`)
- Native bildirimler + tıklayınca ilgili ekrana yönlendirme
- Panodan hızlı transfer (`Ctrl+V` ile dosya yapıştırma)
- Sistem tepsisi (tray), açılışta başlatma, tek instance
- Global kısayol ile hızlı dosya gönderme
- Sistem temasını takip eden UI (açık/koyu)
- Otomatik güncelleme

### 1.2 Açıkça Kapsam Dışı (v1)

Grup sohbeti · mobil istemci · hesap sistemi · uçtan uca versiyonlama/CRDT senkronizasyon · sesli/görüntülü görüşme · ekran paylaşımı

---

## 2. Mimari

### 2.1 Katmanlar

```
┌───────────────────────────────────────────────┐
│  React Frontend (TS)                          │
│  Zustand store · i18n · Tauri invoke/listen   │
└──────────────────────┬────────────────────────┘
                       │ Tauri IPC (command + event)
┌──────────────────────┴────────────────────────┐
│  Rust Backend                                 │
│                                               │
│  commands.rs  ── frontend'e açılan tek API    │
│       │                                       │
│  ┌────┴─────┐ ┌──────────┐ ┌───────────────┐  │
│  │Discovery │ │ Network  │ │   Transfer    │  │
│  │  (mDNS)  │ │  (QUIC)  │ │(stream/resume)│  │
│  └──────────┘ └────┬─────┘ └───────┬───────┘  │
│                    │ protocol.rs   │          │
│  ┌──────────┐ ┌────┴─────┐ ┌───────┴───────┐  │
│  │ Identity │ │ Pairing  │ │     Sync      │  │
│  │(keyring) │ │  (SAS)   │ │   (notify)    │  │
│  └──────────┘ └──────────┘ └───────────────┘  │
│                                               │
│  ┌──────────┐ ┌──────────┐ ┌───────────────┐  │
│  │    DB    │ │  Tray +  │ │ Notifications │  │
│  │ (SQLite) │ │ Shortcut │ │               │  │
│  └──────────┘ └──────────┘ └───────────────┘  │
└───────────────────────────────────────────────┘
```

**Kural:** Frontend hiçbir zaman ağ veya dosya sistemiyle doğrudan konuşmaz. Tüm yan etkiler `commands.rs` üzerinden geçer; backend durum değişikliklerini Tauri event'i olarak yayınlar, frontend store'ları bu event'lerle güncellenir.

### 2.2 Transport: QUIC (`quinn`)

**Neden QUIC:** NAT traversal ve connection migration (IP değişince bağlantı kopmaz) v2 için yerleşik; TLS 1.3 dahili; multiplexed stream sayesinde chat mesajı dosya transferinin arkasında beklemez.

#### 2.2.1 Sertifika ve kimlik bağlama (kritik)

P2P'de CA yoktur. Doğrulama zinciri şöyle kurulur:

1. Cihaz ilk açılışta bir **Ed25519 identity keypair** üretir (§2.6).
2. Bu identity key'den **deterministik olarak** self-signed bir TLS sertifikası üretilir (`rcgen`, Ed25519 imza algoritmasıyla). Sertifikanın public key'i = cihazın identity public key'i. Böylece "TLS peer" ile "LinkUp cihaz kimliği" iki ayrı şey olmaktan çıkar, tek bir şey olur.
3. `rustls`'e **özel `ServerCertVerifier` ve `ClientCertVerifier`** takılır. Bu verifier CA zinciri aramaz; sertifikanın public key'ini alıp `trusted_devices` tablosundaki pinlenmiş public key ile karşılaştırır.
4. Eşleşmemiş bir cihazla bağlantıda verifier "henüz bilinmiyor" modunda çalışır: bağlantıya izin verir ama oturumu **yalnızca pairing mesajlarına** izin veren kısıtlı bir moda sokar. Pairing tamamlanmadan `ChatMessage` / `FileOffer` kabul edilmez.
5. **Karşılıklı TLS (mTLS) zorunlu** — hem client hem server tarafı sertifika sunar ve doğrular.

> Bu tasarımın sonucu: pairing tamamlandıktan sonra kimlik doğrulaması tamamen TLS katmanında halledilir. Protokol seviyesinde ayrıca imza/challenge taşımaya gerek kalmaz.

#### 2.2.2 Akış kontrolü ve canlılık ayarları

> **Bu bölüm Faz 2'de ölçümle düzeltildi.** İlk hâli "varsayılan pencereler gigabit LAN'da darboğazdır" diyordu. Ölçüm bunu doğrulamadı: aynı test tuning'li **2203 Mbit/s**, tuning'siz **2204 Mbit/s** verdi. Loopback'te darboğaz pencereler değil **CPU**'dur. Ayrıca ilk tablodaki `receive_window = 32 MB`, quinn'in varsayılanına (pratikte sınırsız) göre bir *düşürme*ydi ve `max_concurrent_bidi_streams = 64` de varsayılan 100'ün altındaydı. Değerler korundu ama gerekçeleri dürüstçe yeniden yazıldı — hiçbiri "daha hızlı olsun diye" konmuş değil.

`quinn::TransportConfig` üzerinde ayarlananlar:

| Ayar | quinn varsayılanı | LinkUp | Gerekçe |
|---|---|---|---|
| `stream_receive_window` | 1,25 MB | 8 MB | Tek akışın yüksek gecikmeli yolda (v2'de relay üzerinden internet) pencereye takılmaması. LAN'da fark yaratmaz, zararı da yok |
| `receive_window` | pratikte sınırsız | 32 MB | **Hız değil bellek sınırı.** Sınırsız bırakılırsa kötü niyetli bir eş çok sayıda akış açıp keyfi miktarda tamponlatabilir |
| `send_window` | 10 MB | 32 MB | Gönderim tarafında karşılık gelen tampon |
| `max_concurrent_bidi_streams` | 100 | 64 | **DoS sınırı.** Tasarım gereği bağlantı başına 1 kontrol akışı kullanılıyor (§2.2.3) |
| `keep_alive_interval` | kapalı | 5 sn | NAT/router eşleşmeleri zaman aşımına uğramasın — v2'nin ön koşulu |
| `max_idle_timeout` | — | 20 sn | Kopma tespiti; keep-alive'ın belirgin şekilde üstünde olmalı |

Son iki satırdaki ilişki (`keep_alive * 2 < max_idle`) ve `receive_window ≥ 3 × stream_receive_window` (eşzamanlı 3 transfer, §2.7.4) **derleme zamanı `const assert`'leri** ile korunuyor: sessizce gevşetilemezler.

**Ölçüm sonucu (Faz 2, loopback, release):**

| | |
|---|---|
| Aktarılan | 512 MiB, tek unidirectional stream |
| Hız | **263 MiB/s ≈ 2203 Mbit/s** |
| Hedef | ≥ 400 Mbit/s ✔ |

**Ölçümün sınırı:** Loopback gerçek bir NIC değil; ölçtüğü şey yığınımızın CPU tavanıdır. Bu sayı "gigabit LAN'da 2 Gbit/s alırız" demek değildir — gigabit hattın kendisi 1 Gbit/s ile sınırlı. Anlamı şudur: **QUIC/TLS/çerçeveleme katmanımız gigabit hattı doyurabilecek kadar hızlı, darboğaz orada değil.** Gerçek LAN ölçümü Faz 7'de (dosya transferi) gerçek donanımla yapılacak.

#### 2.2.3 Stream stratejisi

- **Kontrol stream'i:** bağlantı başına 1 adet uzun ömürlü bidirectional stream. Chat, heartbeat, pairing, read receipt, sync manifest burada akar. Küçük ve gecikmeye duyarlı.
- **Transfer stream'i:** **dosya başına 1 adet** unidirectional stream, veriyi sıralı akıtır.
- **Paralellik dosya seviyesindedir**, chunk seviyesinde değil: aynı anda en fazla 3 dosya (ayarlanabilir).

> Tek bir QUIC bağlantısındaki tüm stream'ler aynı congestion controller'ı paylaşır; chunk'ları paralel stream'lere bölmek throughput'u **artırmaz**, yalnızca karmaşıklık ekler. Multiplexing'in gerçek faydası chat'in transfer arkasında beklememesidir — yukarıdaki ayrım tam olarak bunu sağlar.

### 2.3 Uygulama Protokolü

#### 2.3.1 Framing

Kontrol stream'inde length-prefixed binary çerçeveler:

```
[4 byte u32 LE: payload uzunluğu][1 byte: mesaj tipi][payload: postcard]
```

- Maksimum kontrol çerçevesi: **1 MB** (aşan bağlantı kapatılır — DoS koruması)
- Serialization: `postcard` (Rust-native, kompakt, `serde` uyumlu)
- Transfer stream'inde çerçeveleme yoktur: stream'in başında `TransferStreamHeader`, kalanı ham dosya verisidir. Stream'in kapanması dosyanın bittiğini gösterir.

#### 2.3.2 Sürüm anlaşması (zorunlu)

Bağlantı kurulur kurulmaz ilk çerçeve `Hello`'dur:

```rust
struct Hello {
    protocol_version: u16,      // v1 = 1
    min_supported_version: u16,
    app_version: String,        // "0.1.0" — yalnızca bilgi amaçlı
    device_name: String,
    device_id: [u8; 32],        // identity public key
    capabilities: u32,          // bit alanı: FOLDER_SYNC, CLIPBOARD, ...
}
```

Uyumsuz sürümde bağlantı `IncompatibleVersion` ile kapatılır ve kullanıcıya "karşı cihazdaki LinkUp güncellenmeli" uyarısı gösterilir. `capabilities` bit alanı sayesinde yeni özellikler eski istemcileri kırmadan eklenebilir.

#### 2.3.3 Mesaj tipleri

| Tip | Yön | Açıklama |
|---|---|---|
| `Hello` / `HelloAck` | ↔ | Sürüm + kimlik anlaşması |
| `PairingRequest` | → | Eşleştirme başlatma |
| `PairingConfirm` / `PairingReject` | ↔ | SAS onayı sonucu |
| `Heartbeat` | ↔ | Canlılık + RTT ölçümü |
| `ChatMessage` | → | `{ msg_id, content_type, body, sent_at }` |
| `ChatAck` | ← | `delivered` durumu |
| `ReadReceipt` | ← | `read` durumu |
| `FileOffer` | → | `{ transfer_id, name, size, mime, blake3, is_resume }` |
| `FileAccept` | ← | `{ transfer_id, start_offset }` — resume noktası burada bildirilir |
| `FileReject` | ← | `{ transfer_id, reason }` |
| `TransferStreamHeader` | → | Transfer stream'inin ilk çerçevesi: `{ transfer_id, offset }` |
| `FileComplete` | ← | Bütün-dosya hash doğrulaması sonucu |
| `TransferCancel` | ↔ | İki yönlü iptal |
| `SyncManifest` | ↔ | Klasör senkronizasyon manifesti |
| `ClipboardOffer` | → | Pano içeriği teklifi (dosya ise `FileOffer`'a devreder) |
| `Error` | ↔ | `{ code, message }` |

**İleriye dönük alan:** Tüm chat/transfer mesajlarında `conversation_id` alanı bulunur. v1'de bu her zaman peer'ın `device_id`'sidir; grup sohbeti eklendiğinde protokol değişmeden grup id'si taşıyabilir.

### 2.4 Cihaz Keşfi

**Birincil — mDNS (`mdns-sd`):**
- Servis tipi: `_linkup._udp.local` (QUIC UDP üzerinde çalıştığı için `_udp`)
- TXT kayıtları: `name` (cihaz adı), `fp` (identity public key fingerprint, base32), `v` (protokol sürümü)
- Port: SRV kaydında yayınlanır

**Fallback — manuel ekleme (v1'de zorunlu, sonraya bırakılmaz):**
- Windows Firewall'un "Public network" profili mDNS'i keser; kurumsal ve misafir Wi-Fi ağlarında client isolation yaygındır. mDNS'e tek başına güvenilemez.
- Ayarlar → "Cihaz Ekle" → IP + port girişi → doğrudan QUIC bağlantısı → pairing akışı normal şekilde işler.
- Eşleşmiş cihazların son bilinen IP'si DB'de tutulur; mDNS başarısız olsa bile son IP denenir.

**Kayıt ömrü (Faz 4'te düzeltildi):** Keşfedilen cihazların ömrü tamamen `mdns-sd`'ye aittir; kayıt yalnızca `ServiceRemoved` geldiğinde veya kullanıcı sildiğinde düşer. İlk uygulamada kendi 90 saniyelik TTL'imiz vardı ve bu, kütüphanenin önbellek yönetimiyle çakışıyordu: `mdns-sd` değişmeyen bir servis için `ServiceResolved`ı tekrar yayınlamadığından "son görülme" hiç tazelenmiyor ve cihazlar açılıştan tam iki dakika sonra listeden **kalıcı olarak** siliniyordu. Elle eklenen adresler her hâlükârda süresizdir.

**Adres seçimi (Faz 3'te ölçümle eklendi):** Bir cihaz mDNS'te birden fazla IPv4 adresi ilan eder. Gerçek bir ilan şöyle görünüyor:
`[192.168.0.195, 127.0.0.1, 172.17.80.1, 172.24.160.1, 169.254.188.223]` — LAN arayüzü, loopback, WSL ve Hyper-V sanal adaptörleri, bir de DHCP başarısız olduğunda atanan link-local adres. Listenin ilk elemanını almak yanlış adaptöre bağlanma denemesiyle ve zaman aşımıyla sonuçlanır. Bu yüzden adresler ulaşılabilirlik ihtimaline göre sıralanır: `192.168.x` → `10.x` → `172.16-31.x` (sanal adaptörler burada) → loopback → diğer → link-local. Aynı sıralama, kullanıcıya kendi adresini gösterirken de kullanılır.

**Firewall:** Windows'ta ilk çalıştırmada UDP portu için firewall izni gerekir. Installer bu kuralı ekler; eklenemezse uygulama içinde açıklayıcı bir uyarı gösterilir.

### 2.5 Pairing — Karşılıklı Doğrulama Kodu (SAS)

PIN girme yerine **her iki cihazda da aynı kodu gösterip kullanıcıya onaylatma** yöntemi kullanılır (Signal safety number / Bluetooth numeric comparison mantığı).

**Akış:**
1. Kullanıcı A, keşfedilen Cihaz B'ye "Eşleştir" der.
2. QUIC/TLS bağlantısı kurulur. Her iki taraf da karşı sertifikanın public key'ini görür (henüz güvenmez).
3. Her iki taraf bağımsız olarak aynı kodu hesaplar:
   ```
   sas = BLAKE3("LinkUp-SAS-v1" || min(pkA,pkB) || max(pkA,pkB) || tls_exporter)
   kod = sas'ın ilk 20 bitinden türetilen 6 haneli sayı
   ```
   `tls_exporter`, RFC 5705 TLS exporter değeridir — kodu somut TLS oturumuna bağlar (**channel binding**). Ortadaki saldırgan iki ayrı TLS oturumu kurarsa iki tarafta farklı kod çıkar.
4. Her iki ekranda kod ve karşı cihazın adı gösterilir: *"Bu kod diğer cihazda da aynı mı?"* → Onayla / Reddet.
5. **İki taraf da onaylarsa** eşleşme tamamlanır; public key `trusted_devices` tablosuna pinlenir.
6. Sonraki bağlantılarda kod sorulmaz — pinlenmiş key ile TLS seviyesinde otomatik doğrulama.
7. Zaman aşımı: 90 saniye. Kod oturum başına tek kullanımlıktır.

> **Neden PIN girme değil:** Tek yönlü PIN girişi, PIN'i gerçek bir PAKE (SPAKE2) olarak kullanmadıkça MITM'i engellemez — düz HMAC-challenge, saldırgan mesajları relay ettiğinde geçer. SAS, SPAKE2 kadar güvenli ama uygulaması kat kat basit; UX'i de daha az sürtünmeli. Detay: §10-K2.

**Uygulama notu (Faz 4) — eşleşme sonrası bağlantı kapatılmaz:** İlk uygulamada eşleşme biter bitmez bağlantı kapatılıyor ve yeniden bağlanma denetleyicisine bırakılıyordu. Gerçek iki cihazla denendiğinde asimetrik bir sonuç çıktı: bir taraf "tamamlandı", diğeri 0,6 ms sonra "ağ hatası" dedi. Sebep, QUIC'te `close()` çağrısının akıştaki teslim edilmemiş veriyi atması — karşı taraf son `PairingConfirm`i alamadan bağlantı düşüyordu. Eşleşme başarılıysa bağlantı artık kapatılmıyor, doğrudan bağlantı denetleyicisine devrediliyor.

**Kendini onaran yeniden eşleşme:** Güvendiğimiz bir cihaz `PairingRequest` gönderirse istek kabul edilip eşleştirme yeniden koşulur. Karşı taraf bizi unutmuşsa (veya eşleşme geçmişte tek tarafta kalmışsa) iki cihazın birbirine yeniden bağlanabilmesinin tek yolu budur.

**Uygulama notu (Faz 4):** Uygulama seviyesinde ayrıca heartbeat DÖNGÜSÜ kurulmadı. QUIC'in kendi keep-alive'ı (5 sn) bağlantıyı canlı tutuyor ve `max_idle_timeout` (20 sn) ölü bağlantıyı zaten hataya çeviriyor (§2.2.2); ikinci bir canlılık mekanizması yalnızca trafik ve karmaşıklık eklerdi. `Heartbeat` mesajı RTT ölçmek için duruyor.

**Eşleşme kaldırma:** Ayarlarda cihazı "Unut" → `trusted_devices`'tan silinir, aktif bağlantı kapatılır. Karşı tarafa bilgi gitmez (o taraf da manuel unutmalıdır); UI bunu açıkça belirtir.

### 2.6 Kimlik ve Anahtar Yönetimi

- İlk açılışta **Ed25519 keypair** üretilir (`ed25519-dalek`).
- Private key **OS keychain**'de saklanır (`keyring` crate): Windows Credential Manager · macOS Keychain · Linux Secret Service.
- **Linux fallback:** Secret Service (gnome-keyring / KWallet) yoksa `keyring` hata verir. Bu durumda anahtar, veri dizininde `0600` izinli düz dosyaya yazılır ve **bu durum kullanıcıya UI'da açıkça bildirilir** ("Anahtarınız sistem kasasında saklanamadı, dosyada tutuluyor").
- Public key fingerprint kullanıcıya gösterilebilir (base32, gruplu format) — SSH tarzı manuel doğrulama için; cihaz detay ekranında bulunur.
- Anahtar kaybolursa/silinirse cihaz yeni kimlik üretir ve **tüm eşleşmeler geçersiz olur**; UI bunu net bir uyarıyla anlatır.

### 2.7 Dosya Transferi

#### 2.7.1 Temel akış

1. Gönderen `FileOffer` yollar: ad, boyut, MIME, tüm dosyanın blake3 hash'i.
2. Alıcı kabul politikasına göre karar verir (§2.13.3) → `FileAccept { start_offset }`.
3. Gönderen yeni bir unidirectional stream açar, `TransferStreamHeader { transfer_id, offset }` yazar, ardından dosyayı `offset`'ten itibaren **sıralı** akıtır.
4. Alıcı `.part` uzantılı geçici dosyaya yazar; ilerlemeyi periyodik olarak (≈500 ms) DB'ye ve UI'a bildirir.
5. Stream kapanınca alıcı blake3 hash'i doğrular → `FileComplete { ok }` → `.part` dosyası nihai adına taşınır.

#### 2.7.2 Resume

> **Uygulama durumu (Faz 7):** Resume makinesi kuruldu — `.part` dosyası, offset takibi, `FileAccept { start_offset }` ve dosya sonu blake3 doğrulaması çalışıyor; alıcı, aynı `transfer_id` ile gelen bir teklifi kaldığı yerden kabul ediyor. Ama **kesintiden sonra teklifi yeniden gönderen bir mekanizma henüz yok**: bağlantı koptuğunda transfer başarısız işaretleniyor ve kullanıcı dosyayı elle yeniden göndermek zorunda; bu da yeni bir `transfer_id` ürettiği için baştan başlıyor. Otomatik yeniden teklif Faz 11'de (cilalama) tamamlanacak.

- Transfer başında `transfers` tablosuna kayıt açılır: `transfer_id`, dosya adı, boyut, beklenen hash, `bytes_done`, `part_path`, durum.
- Sıralı akış sayesinde resume durumu **tek bir byte offset**'idir — chunk bitmap'i gerekmez.
- Kopma sonrası yeniden bağlanınca: gönderen aynı `transfer_id` ile `FileOffer { is_resume: true }` yollar; alıcı `.part` dosyasının boyutunu okuyup `FileAccept { start_offset }` ile bildirir.
- Dosya değişmişse (boyut veya hash farklı) resume reddedilir, transfer baştan başlar.
- Yarım kalan `.part` dosyaları 7 gün sonra temizlenir (açılışta housekeeping).

#### 2.7.3 Bütünlük

Chunk başına hash **kullanılmaz**. QUIC/TLS 1.3 zaten her byte'ı authenticated encryption ile korur; ağ kaynaklı bozulma stream'e ulaşamaz. Doğrulanması gereken tek şey, resume sonrası dosyanın doğru birleşmesidir → **dosya sonunda tek blake3 doğrulaması** yeterlidir.

#### 2.7.4 Kuyruk ve hız kontrolü

- Global bir transfer kuyruğu; eşzamanlı aktif transfer sayısı ayarlanabilir (varsayılan 3).
- **Bant genişliği limiti** (ayarlardan, KB/s): gönderim tarafında token-bucket ile uygulanır. Aynı ağdaki diğer işleri yavaşlatmamak için.
- **Çoklu dosya seçimi:** birden fazla dosya tek seferde kuyruğa alınır. Çok sayıda küçük dosya tespit edilirse (>50 dosya veya ortalama <256 KB) kullanıcıya *"tek arşiv olarak gönderilsin mi?"* seçeneği sunulur — kabul edilirse akış halinde zip'lenip karşıda açılır.

#### 2.7.5 Klasör senkronizasyonu

- Kullanıcı arayüzden klasör seçer → "Bu klasörü [Cihaz] ile senkronla".
- Tetikleme: `notify` crate ile dosya sistemi event'i + **debounce (2 sn)**; güvenlik ağı olarak periyodik tam tarama (varsayılan 5 dk).
- Manifest karşılaştırması: göreli yol + boyut + değişiklik zamanı (mtime). Şüpheli durumda (aynı boyut, farklı mtime) blake3 ile doğrulanır.
- Politika: **son değişen kazanır + çakışma uyarısı**. İki taraf da değiştirmişse otomatik overwrite yapılmaz; çakışan dosya `ad.conflict-<cihaz>-<tarih>.uzantı` olarak yan yana kaydedilir ve kullanıcıya bildirilir.
- Silme senkronizasyonu **v1'de yoktur** (kaza riski yüksek). Bir tarafta silinen dosya diğerinde kalır. Bu, UI'da açıkça yazılır.
- Görmezden gelinecekler: `.git/`, `node_modules/`, `~$*`, `*.tmp`, `.DS_Store` + kullanıcı tanımlı desenler.
- Bu bir Syncthing değildir; CRDT/versiyonlama yoktur. Sınırlar UI'da dürüstçe belirtilir.

### 2.8 Okundu Bilgisi ve Mesaj Arama

- Mesaj durumu: `sending` → `sent` → `delivered` (`ChatAck`) → `read` (`ReadReceipt`). Gönderilemeyenler `failed`.
- `ReadReceipt`, sohbet ekranı **görünür ve pencere odakta** iken tetiklenir (tray'de arka plandayken değil).
- UI'da mesaj balonu altında ince "İletildi / Görüldü" göstergesi.
- Arama: SQLite **FTS5** sanal tablosu, `messages` üzerine trigger'larla senkron tutulur. Hem sohbet içi arama hem global arama aynı indeksi kullanır.
- Türkçe için `unicode61` tokenizer + diakritik kaldırma; "İ/ı" davranışı test edilecek (küçük harf dönüşümü Türkçe'de tuzaklı).

### 2.9 Panodan Hızlı Transfer

- **Sürekli pano izleme yapılmaz** (gizlilik açısından hassas). Yalnızca kullanıcı chat kutusunda `Ctrl+V` yaptığında pano okunur.
- Pano içeriği metin ise → mesaj kutusuna yapıştırılır. Görsel ise → görsel mesajı olarak gönderilmeye hazırlanır. **Dosya yolu ise** → `FileOffer` akışı tetiklenir.
- ⚠ **Fizibilite riski:** Tauri `clipboard-manager` plugin'i yalnızca metin ve görsel okur; **dosya yolu (Windows `CF_HDROP`) okumaz.** Bu özellik için platforma özel kod gerekir (Windows: `clipboard-win`; macOS: NSPasteboard; Linux: `text/uri-list`). **Faz 9'a girmeden önce spike ile doğrulanacak.** Doğrulanamazsa özellik metin + görselle sınırlanır ve dosya için sürükle-bırak yeterli sayılır.

### 2.10 Bildirimler

- Tauri `notification` plugin'i (Windows toast · macOS Notification Center · Linux libnotify).
- Tetikleyiciler: yeni chat mesajı · transfer tamamlandı · transfer isteği onay bekliyor · eşleştirme isteği.
- Pencere odaktayken ve ilgili sohbet açıkken bildirim gösterilmez.
- Tıklama davranışı ayarlardan seçilir:
  - **Varsayılan:** uygulama içi ilgili ekranı aç (mesaj → o sohbet; dosya → dosya bilgi ekranı)
  - **Alternatif:** doğrudan dosya sistemi konumunu aç
- ✔ **Fizibilite riski çözüldü (Faz 7).** Risk gerçekti: Tauri'nin `notification` eklentisi masaüstünde HİÇBİR olay yayınlamıyor — kaynak incelendi, `desktop.rs` içinde tek bir `emit` yok; `onAction`/`onNotificationReceived` yalnızca mobil için. Çözüm: Windows'ta eklenti yerine doğrudan `tauri-winrt-notification` kullanılıyor, `on_activated` geri çağrısıyla pencere öne getirilip ilgili ekrana yönlendiriliyor (mesaj → o sohbet, dosya → Gelen Dosyalar).
  - **Bilinen sınır:** Uygulamanın kendi AppUserModelID'si yalnızca KURULU sürümlerde kayıtlıdır (installer Başlat Menüsü kısayolu oluşturur). Geliştirme sırasında kayıtlı olmadığı için PowerShell kimliğine düşülür; bildirim çıkar ama gönderen adı "Windows PowerShell" görünür. Kurulu sürümde bu sorun yoktur.
  - **Diğer platformlar:** macOS ve Linux'ta eklenti kullanılmaya devam ediyor; oralarda tıklama yönlendirmesi henüz yok.

### 2.11 Sistem Tepsisi, Kısayol, Yaşam Döngüsü

- **Tek instance** (`tauri-plugin-single-instance`): ikinci çalıştırma mevcut pencereyi öne getirir. Tray uygulamalarında zorunlu.
- **Tray:** pencere kapatma (X) varsayılan olarak tray'e küçültür (ayarlardan "tamamen kapat"a çevrilebilir). Tray menüsü: Aç · Ayarlar · Çıkış + online cihaz sayısı.
- **Açılışta başlatma** (`tauri-plugin-autostart`): ayarlardan açılıp kapatılır, varsayılan kapalı. İlk kurulumda kullanıcıya sorulur.
- **Global kısayol** (`tauri-plugin-global-shortcut`, varsayılan `Ctrl+Shift+L`): küçük bir "hızlı gönder" popup'ı açar (dosya seçici + hedef cihaz). Kısayol çakışırsa ayarlarda uyarı gösterilir ve kullanıcı yeniden atayabilir.

### 2.12 Veritabanı (SQLite)

**Erişim:** `rusqlite` (`bundled` + `fts5` feature) + `r2d2` bağlantı havuzu; DB çağrıları `tokio::task::spawn_blocking` içinde. `sqlx`'in compile-time makroları `DATABASE_URL` bağımlılığı ve CI sürtünmesi getirdiği için tercih edilmedi (§10-K3).

**Migration:** `refinery` ile sıralı `migrations/NNN_*.sql`. Şema **Faz 1'de** kurulur (pairing'in ön koşulu).

**PRAGMA:** `journal_mode=WAL`, `synchronous=NORMAL`, `foreign_keys=ON`, `busy_timeout=5000`.

```sql
trusted_devices (
  id INTEGER PK, device_id BLOB UNIQUE,   -- 32 byte ed25519 public key
  name TEXT, alias TEXT, color TEXT,
  last_ip TEXT, last_port INTEGER,
  last_seen INTEGER, paired_at INTEGER
)

messages (
  id INTEGER PK, msg_id TEXT UNIQUE,       -- UUID, iki uçta aynı
  conversation_id BLOB,                    -- v1: device_id; v2: grup id
  device_id BLOB REFERENCES trusted_devices(device_id),
  direction TEXT,                          -- 'in' | 'out'
  content_type TEXT,                       -- 'text' | 'image' | 'code' | 'file_ref'
  content TEXT,
  transfer_id TEXT NULL,                   -- content_type='file_ref' ise
  sent_at INTEGER, status TEXT             -- sending|sent|delivered|read|failed
)

messages_fts (FTS5 virtual, content='messages', content_rowid='id')
  -- INSERT/UPDATE/DELETE trigger'ları ile senkron

transfers (
  id INTEGER PK, transfer_id TEXT UNIQUE,
  device_id BLOB, direction TEXT,
  file_name TEXT, file_size INTEGER, mime TEXT,
  expected_hash BLOB, save_path TEXT, part_path TEXT,
  bytes_done INTEGER, status TEXT,         -- pending|active|paused|done|failed|cancelled
  error TEXT NULL,
  started_at INTEGER, completed_at INTEGER
)

synced_folders (
  id INTEGER PK, device_id BLOB,
  local_path TEXT, remote_path TEXT,
  ignore_patterns TEXT, enabled INTEGER,
  last_synced_at INTEGER
)

settings (key TEXT PK, value TEXT)
```

**İndeksler:** `messages(conversation_id, sent_at DESC)` · `transfers(device_id, started_at DESC)` · `transfers(status)`

### 2.13 Güvenlik Modeli

**Tehdit modeli (v1):** Aynı LAN'daki pasif dinleyici ve aktif MITM saldırganı. Kapsam dışı: cihazı ele geçirmiş yerel saldırgan, kötü niyetli **eşleşmiş** cihaz (eşleştirme bir güven kararıdır).

- Tüm trafik QUIC/TLS 1.3 ile şifreli; kimlik doğrulama pinlenmiş public key ile (§2.2.1).
- Pairing'de channel-binding'li SAS ile MITM engellenir (§2.5).
- Identity key OS keychain'de (§2.6).

#### 2.13.1 Gelen dosya güvenliği (zorunlu kontroller)

Karşı taraf keyfi bir `file_name` gönderebilir. `FileOffer` işlenirken:

1. Dosya adından **tüm dizin bileşenleri atılır** (yalnızca son bileşen alınır).
2. `..`, mutlak yol, sürücü harfi (`C:`), ADS ayracı (`:`) ve yol ayraçları reddedilir.
3. Windows'ta rezerve adlar (`CON`, `PRN`, `AUX`, `NUL`, `COM1..9`, `LPT1..9`) ve sondaki nokta/boşluk düzeltilir.
4. Yazılacak nihai yol `canonicalize` edilir ve **indirme klasörünün altında olduğu doğrulanır**; değilse transfer reddedilir.
5. Dosya adı ve toplam yol uzunluğu sınırlanır (Windows MAX_PATH).
6. **Ad çakışması:** üzerine yazılmaz → `dosya (1).zip`, `dosya (2).zip`.
7. Klasör senkronizasyonunda gelen göreli yollar için de aynı kontroller uygulanır (her bileşen ayrı ayrı).

#### 2.13.2 Kaynak sınırları

- `FileOffer` öncesi **disk alanı kontrolü**; yetersizse `FileReject { reason: NoSpace }`.
- Kontrol çerçevesi boyut limiti 1 MB; aşımda bağlantı kapatılır.
- Eşleşmemiş peer başına bağlantı ve pairing denemesi rate limit'i.

#### 2.13.3 Dosya kabul politikası (ayarlanabilir)

| Mod | Davranış |
|---|---|
| **Her zaman sor** (varsayılan) | Her dosya için kullanıcı onayı istenir |
| Boyut eşiğiyle | Eşiğin (varsayılan 100 MB) altı otomatik, üstü onay ister |
| Güvenilir cihazlardan otomatik kabul | Eşleşmiş cihazlardan gelen dosyalar sorulmadan indirilir |

**Cihaz başına güven (kullanıcı isteği):** Sohbet başlığındaki üç nokta menüsünden açılan cihaz kartında bir "Güvenli cihaz" anahtarı bulunur. Açıkken o cihazdan gelen dosyalar onay sorulmadan kabul edilir. Karar cihaz bazındadır ve varsayılan kapalıdır — bir cihaza güvenmek hepsine güvenmek değildir. Bu işaret, genel kabul politikasının ÖNÜNDE gelir.

> **Varsayılan neden "sor":** eşleşme bir güven kararıdır ama sınırsız yazma yetkisi değil. Kullanıcı ne aldığını bilmeli. Onay 60 saniye içinde gelmezse teklif reddedilir — karşı tarafı süresiz bekletmek hem bağlantıyı hem göndericinin dosyasını rehin tutmak olurdu.
>
> Bekleme, bağlantı döngüsünde YAPILMAZ: döngü o sırada başka hiçbir mesajı işleyemezdi. Karar ayrı bir görevde beklenir, yanıt giden kuyruğa yazılır.

Eşleşmemiş cihazlardan dosya **hiçbir modda** kabul edilmez.

### 2.14 Gözlemlenebilirlik ve Hata Yönetimi

- `tracing` + `tracing-subscriber` + `tracing-appender`; rotasyonlu dosya logu uygulama veri dizininde (son 7 gün), konsola da yazar.
- Log seviyesi ayarlardan değiştirilebilir; ayarlarda **"Log klasörünü aç"** butonu (destek için).
- Hata tipleri: `thiserror` ile modül bazlı. Frontend'e giden hatalar `{ code, message, detail }` şeklinde yapısal taşınır — UI kullanıcıya i18n'li mesaj gösterir, `detail` yalnızca logda.
- **Loglara asla girmeyecekler:** mesaj içeriği, dosya içeriği, private key, SAS kodu. Dosya adları yalnızca `debug` seviyesinde.
- Bağlantı kopmalarında otomatik yeniden bağlanma: exponential backoff (1s → 2s → 4s … max 30s), UI'da "Yeniden bağlanılıyor…" durumu.

### 2.15 Dağıtım

- **Otomatik güncelleme:** `tauri-plugin-updater`. Protokolü olan bir P2P uygulamada sürüm dağılımı kritiktir — eski istemciler ağda kalırsa `capabilities` / `protocol_version` yükü büyür.
- **Windows:** NSIS installer + firewall kuralı ekleme. Kod imzalama sertifikası yoksa SmartScreen uyarısı çıkacaktır — bilinen durum, README'de belirtilir.
- **macOS:** notarization gerekir (aksi halde Gatekeeper engeller).
- **Linux:** AppImage + .deb.
- Sürümleme: SemVer. `protocol_version`, uygulama sürümünden **bağımsız** ilerler.

---

## 3. Arayüz (UI/UX)

### 3.1 Tema ve i18n

- Tema sistemi takip eder (açık/koyu), ayarlardan manuel override edilebilir. Tailwind `dark:` + Tauri tema API'si.
- **i18n:** tüm kullanıcı metinleri `src/i18n/tr.ts` sözlüğünde. Tip güvenli `t()` sarmalayıcı (anahtarlar TypeScript'te literal union olarak çıkarılır → eksik/yanlış anahtar derleme hatası verir). v1'de yalnızca `tr` yüklenir; ikinci dil eklemek tek dosya işidir.
- Rust tarafındaki hata kodları da i18n anahtarlarına eşlenir — backend asla kullanıcıya gösterilecek metin üretmez.

### 3.2 Navigasyon

Ana giriş noktası **Dashboard**'dur, chat değil. Sol sidebar sabit navigasyon.

```
┌──────────┬────────────────────────────────────────┐
│  LinkUp  │  Genel Bakış                           │
│──────────│────────────────────────────────────────│
│ 🏠 Ana   │  ┌─────────────┐  ┌─────────────┐      │
│    Sayfa │  │ 💻 Cihaz1   │  │ 💻 Cihaz2   │      │
│          │  │ ●Online     │  │ ○Offline    │      │
│ 💬 Sohbet│  │ "son mesaj."│  │ "son mesaj."│      │
│          │  │ 2dk önce    │  │ dün         │      │
│ 📁 Gelen │  └─────────────┘  └─────────────┘      │
│  Dosyalar│                                        │
│          │  Aktif Transferler            (varsa)  │
│ ⚙️ Ayarlar│  ▓▓▓▓▓░░░ rapor.pdf · 12 MB/s · 8sn   │
│          │                                        │
│──────────│  Son Gelen Medyalar                    │
│Bulunanlar│  [🖼️][🖼️][📄][🖼️][📦]        (→ tümü)  │
│ + Cihaz3 │                                        │
└──────────┴────────────────────────────────────────┘
```

**Sidebar bölümleri:**
- **Ana Sayfa:** cihaz özet kartları (son mesaj, online durumu, göreli zaman, okunmamış sayısı; kart tıklanınca o sohbet açılır) · aktif transfer özeti (Faz 7) · son gelen medyalar şeridi (Faz 8)

  > Not: "Aktif Transferler" ve "Son Gelen Medyalar" başlıkları, gösterecekleri veri var olduğunda ekleniyor. Boş bir başlık göstermek ekranı doldurmaktan başka işe yaramaz.
- **Sohbetler:** eşleşmiş cihaz listesi → chat ekranı
- **Gelen Dosyalar:** tüm alınan dosyaların geçmişi, filtre (cihaz / tip: medya·döküman·arşiv) + arama
- **Ayarlar:** §3.4
- Sidebar altında **"Bulunanlar"** — ağda görünen eşleşmemiş cihazlar + "Manuel Ekle" butonu

### 3.3 Chat ekranı

```
┌────────────────────────────────────────┐
│  [Cihaz Adı]  [●Online]  [🔍] [⋮]      │
│────────────────────────────────────────│
│      Mesajlar (metin/görsel/kod)       │
│      ✓✓ Görüldü — 14:22                │
│────────────────────────────────────────│
│  [📎] [Mesaj yaz...]        [Gönder]   │
└────────────────────────────────────────┘
```

- Sürükle-bırak ile transfer başlatma; `Ctrl+V` ile pano yapıştırma
- Kod bloğu otomatik algılama (``` fence) + syntax highlight
- Görsel inline thumbnail, tıklayınca lightbox
- `⋮` menüsü: cihaz detayı · senkronize klasörler · fingerprint görüntüle · cihazı unut
- **Sanallaştırılmış mesaj listesi** (uzun geçmişte performans için)

**Transferler paneli:** progress bar, anlık hız, kalan süre, duraklat/devam/iptal. Dashboard'da özet, kendi sekmesinde detaylı.

**Dosya Bilgi Ekranı (modal):** önizleme · gönderen · boyut · tarih · transfer süresi · Aç / Klasörde Göster / Sil / Tekrar İndir

### 3.4 Ayarlar sayfası (bölümler)

| Bölüm | İçerik |
|---|---|
| Genel | Cihaz adı, tema, dil, açılışta başlat |
| Dosyalar | İndirme konumu, kabul politikası + boyut eşiği, eşzamanlı transfer sayısı, hız limiti |
| Bildirimler | Aç/kapa, tıklama davranışı, sessiz saatler |
| Pencere | Kapatma davranışı (tray'e küçült / çık), global kısayol |
| Cihazlar | Eşleşmiş cihaz listesi, takma ad/renk, fingerprint, "Unut" |
| Ağ | QUIC portu, manuel cihaz ekleme, mDNS aç/kapa |
| Gizlilik ve Güvenlik | Kendi fingerprint'in, anahtar saklama durumu, pano davranışı |
| Gelişmiş | Log seviyesi, log klasörünü aç, DB konumu, sürüm + güncelleme kontrolü |

### 3.5 Bileşen kütüphanesi

Tailwind CSS · shadcn/ui (buton, dialog, tooltip, dropdown) · `shiki` (fine-grained bundle — tam paket WASM'ı bundle'ı şişirir; yalnızca kullanılan dil/tema yüklenir) · `@tanstack/react-virtual` (mesaj listesi)

---

## 4. Klasör Yapısı

```
linkup/
├── src-tauri/
│   ├── migrations/          # NNN_*.sql
│   └── src/
│       ├── identity/        # keypair üretimi, keyring, fingerprint, rcgen sertifika
│       ├── discovery/       # mDNS yayın + keşif, manuel ekleme
│       ├── network/
│       │   ├── protocol.rs  # mesaj tipleri, framing, sürüm anlaşması
│       │   ├── tls.rs       # custom cert verifier, key pinning
│       │   ├── endpoint.rs  # quinn endpoint, TransportConfig tuning
│       │   ├── connection.rs# bağlantı yaşam döngüsü, reconnect, heartbeat
│       │   └── mod.rs
│       ├── pairing/         # SAS hesaplama, onay akışı
│       ├── transfer/
│       │   ├── send.rs
│       │   ├── recv.rs
│       │   ├── queue.rs     # kuyruk, eşzamanlılık, hız limiti
│       │   ├── paths.rs     # dosya adı sanitizasyonu (§2.13.1)
│       │   ├── sync.rs      # klasör senkronizasyonu, manifest
│       │   └── mod.rs
│       ├── chat/            # mesaj gönderme, ack/receipt
│       ├── notifications/
│       ├── tray/            # tray, global kısayol, pencere davranışı
│       ├── db/
│       │   ├── schema.rs
│       │   ├── queries.rs
│       │   └── mod.rs
│       ├── settings.rs
│       ├── error.rs         # thiserror, frontend'e giden hata kodları
│       ├── commands.rs      # Tauri command API
│       ├── events.rs        # frontend'e yayınlanan event tipleri
│       ├── state.rs
│       ├── cli.rs           # --profile bayrağı (§6)
│       ├── lib.rs
│       └── main.rs
├── src/
│   ├── features/
│   │   ├── dashboard/       Dashboard · DeviceSummaryCard · RecentMediaStrip · ActiveTransfers
│   │   ├── chat/            ChatWindow · MessageList · MessageBubble · MessageSearch · CodeBlock · ImagePreview
│   │   ├── transfer/        TransferList · TransferProgress · IncomingFilesHistory · QuickSendPopup
│   │   ├── devices/         DeviceSidebar · DiscoveredDevices · PairingDialog · ManualAddDialog · DeviceDetail
│   │   ├── sync/            SyncedFolders · ConflictDialog
│   │   ├── file-detail/     FileDetailModal
│   │   └── settings/        SettingsPage + bölüm bileşenleri
│   ├── stores/              deviceStore · chatStore · transferStore · settingsStore · uiStore
│   ├── lib/
│   │   ├── tauri.ts         # tip güvenli invoke/listen sarmalayıcıları
│   │   ├── events.ts        # backend event tipleri (Rust ile eşlenir)
│   │   └── format.ts        # boyut, hız, süre biçimlendirme
│   ├── i18n/                index.ts · tr.ts
│   ├── layout/              AppShell · AppSidebar · TitleBar
│   └── App.tsx
└── package.json
```

---

## 5. Teknoloji Listesi

| Alan | Seçim |
|---|---|
| Framework | Tauri 2.x |
| Frontend | React 19 + TypeScript + Vite |
| State | Zustand |
| Routing | React Router |
| Stil | Tailwind CSS + shadcn/ui |
| Sanallaştırma | `@tanstack/react-virtual` |
| Kod highlight | `shiki` (fine-grained) |
| i18n | Tip güvenli kendi `t()` sarmalayıcısı |
| Transport | `quinn` (QUIC) + `rustls` |
| Sertifika üretimi | `rcgen` |
| Kriptografi | `ed25519-dalek` |
| Discovery | `mdns-sd` |
| Serialization | `postcard` + `serde` |
| Hash | `blake3` |
| DB | `rusqlite` (bundled, fts5) + `r2d2` |
| Migration | `refinery` |
| Anahtar saklama | `keyring` |
| Dosya izleme | `notify` |
| Async runtime | `tokio` |
| Loglama | `tracing` + `tracing-subscriber` + `tracing-appender` |
| Hata | `thiserror` (kütüphane), `anyhow` (bin) |
| Tauri plugin'leri | `notification` · `global-shortcut` · `single-instance` · `autostart` · `updater` · `dialog` · `opener` · `clipboard-manager` |

---

## 6. Geliştirme Ergonomisi (erken kurulmalı)

P2P bir uygulamayı iki fiziksel makineyle geliştirmek imkânsıza yakındır. **Faz 0'da** şunlar kurulur:

- **`--profile <ad>` CLI bayrağı:** ayrı veri dizini (`AppData/LinkUp-<ad>`), ayrı DB, ayrı keychain girdisi, ayrı QUIC portu. Böylece aynı makinede iki (veya üç) instance yan yana çalışıp birbirini keşfedebilir.
- `npm run dev:a` / `dev:b` script'leri — iki instance'ı tek komutla ayağa kaldırır.
- **Loopback keşfi:** mDNS aynı makinede iki instance'ı görebilmeli; görmezse manuel ekleme ile `127.0.0.1:<port>` kullanılır.
- **Test stratejisi:**
  - Rust birim testleri: protokol serialization round-trip · dosya adı sanitizasyonu (§2.13.1, saldırgan girdileriyle) · SAS hesaplama · resume offset mantığı
  - Entegrasyon testi: iki `Endpoint`'i aynı süreçte kurup uçtan uca pairing + mesaj + dosya transferi (gerçek ağ gerekmez, loopback)
  - Frontend: kritik store mantığı için Vitest
- **Doğrulama:** `bun run check` — tip kontrolü · frontend build · `cargo fmt --check` · `cargo clippy -D warnings` · `cargo test`. `.githooks/pre-push` bunu her push'tan önce otomatik çalıştırır; projenin CI'ı budur (§10-K9).

---

## 7. Geliştirme Fazları

Her fazın sonunda **çalışan ve elle doğrulanabilir** bir çıktı vardır.

| Faz | İçerik | Bitiş kriteri |
|---|---|---|
| **0 — İskelet** | git init · Tailwind + shadcn + Zustand + router + i18n kurulumu · AppShell + sabit sidebar · boş Dashboard/Sohbetler/Gelen Dosyalar/Ayarlar sayfaları · tema desteği · `tracing` kurulumu · **`--profile` bayrağı ve dev:a/dev:b** · CI | İki instance yan yana açılıyor, navigasyon çalışıyor, tema sistemi takip ediyor |
| **1 — Kimlik + DB** | SQLite şeması + migration altyapısı · settings okuma/yazma · Ed25519 keypair + `keyring` (+ Linux fallback) · fingerprint UI'da görünür | Uygulama açılışta kimlik üretiyor, yeniden açılışta aynı kimliği okuyor, ayarlar kalıcı |
| **2 — QUIC transport** | quinn endpoint · rcgen sertifika · custom cert verifier · TransportConfig tuning · `Hello` sürüm anlaşması · heartbeat · reconnect/backoff · **throughput benchmark** | İki instance loopback'te el sıkışıyor; benchmark sonucu dokümante edildi (hedef ≥400 Mbit/s) |
| **3 — Keşif** | mDNS yayın + keşif · "Bulunanlar" listesi · manuel IP ile ekleme · adres sıralama | İki instance birbirini otomatik buluyor; mDNS kapalıyken manuel ekleme çalışıyor |
| **4 — Pairing** | SAS hesaplama (channel binding) · iki taraflı onay dialogu · `trusted_devices` kaydı · son bilinen IP ile yeniden bağlanma (backoff denetleyicisi) · pinlenmiş key ile otomatik doğrulama · "Unut" | Eşleştirme tamamlanıyor; uygulama yeniden başlatılınca kod sormadan bağlanıyor |
| **5 — Chat** | `ChatMessage`/`ChatAck`/`ReadReceipt` · mesaj persist · sohbet UI · kod bloğu · durum göstergesi | İki instance arasında mesajlaşma ve "Görüldü" çalışıyor, geçmiş kalıcı |
| **6 — Dashboard** | Cihaz özet kartları gerçek verilerle · online durumu · son mesaj + göreli zaman · okunmamış sayısı · karttan sohbete geçiş · boş durumlar | Açılış ekranı anlamlı veri gösteriyor |
| **7 — Dosya transferi + resume** | `FileOffer`/`FileAccept` · stream akışı · **dosya adı sanitizasyonu** · disk alanı kontrolü · kabul politikası · progress UI · resume · kuyruk · hız limiti · çoklu dosya | Büyük dosya transfer ediliyor; ağ kesilip geri gelince kaldığı yerden devam ediyor; kötü niyetli dosya adları reddediliyor |
| **7.5 — Sohbette medya** | Görsel önizleme · mesaj listesinin sanallaştırılması + geçmişe kaydırma | Gönderilen görsel sohbette küçük resim olarak görünüyor; uzun geçmiş akıcı kayıyor |
| **8 — Gelen Dosyalar + Tray** | Dosya geçmişi + filtre/arama · dosya bilgi modalı · son medyalar şeridi · native bildirim (**önce tıklama event'i doğrulanır**) · tray · single-instance · autostart · global kısayol + hızlı gönder popup | Uygulama tray'de yaşıyor; bildirim geliyor ve tıklayınca doğru ekran açılıyor |
| **9 — Arama + Pano** | FTS5 arama (sohbet içi + global) · Türkçe tokenizer doğrulaması · **pano dosya spike'ı** → `Ctrl+V` akışı | Arama çalışıyor; pano özelliği ya çalışıyor ya da kapsamı dürüstçe daraltıldı |
| **10 — Klasör senkronizasyonu** | Klasör seçimi · manifest karşılaştırma · `notify` + debounce + periyodik tarama · çakışma yönetimi · ignore desenleri | İki cihaz arasında klasör senkron kalıyor; çakışma kaybolmadan yan yana kaydediliyor |
| **11 — Cilalama + dağıtım** | Hata mesajlarının tamamlanması · boş/hata durumları · ayarlar sayfasının tamamı · performans testleri · installer + firewall kuralı · updater · imzalama/notarization | Kurulabilir sürüm çıkıyor, temiz bir makinede çalışıyor |
| **12 — (ileride) İnternet üzerinden** | Relay sunucu tasarımı · NAT traversal (hole punching) · hesap sistemi · uzaktan pairing onayı | — |

**Bağımlılık notları:**
- Faz 1, Faz 4'ün ön koşuludur (DB olmadan pairing kaydedilemez).
- Faz 2'nin benchmark'ı Faz 7'yi kilitler — hedefe ulaşılamazsa Faz 7'ye geçmeden tuning yapılır.
- Native bildirimler Faz 7'de öne alındı (kullanıcı isteği): mesaj ve dosya geldiğinde, pencere odakta DEĞİLSE gösteriliyor. Bildirime tıklama yönlendirmesi hâlâ Faz 8'de.
- Faz 8 ve 9'daki spike'lar (bildirim tıklama, pano dosyası) o fazın **ilk işidir**; başarısız olurlarsa kapsam daraltılır, faz ertelenmez.

---

## 8. Sonraya Bırakılanlar (v1 sonrası)

- Ekran görüntüsü paylaşımı (kısayolla alıp aktif sohbete gönderme)
- Cihaz takma adı / avatar renkleri — *DB'de `alias` ve `color` alanları baştan var, UI sonra*
- "Aktif cihaz" senkronizasyonu (birden fazla makinede açıkken bildirim tekrarını önleme)
- Bir dosyayı aynı anda birden fazla cihaza gönderme
- Sohbet geçmişi dışa aktarma
- Klasör senkronizasyonunda silme senkronu (çöp kutusu mantığıyla)

---

## 9. Açık Sorular

- **Relay sunucusu** kendi mi barındırılacak, üçüncü parti mi (v2'de netleşecek)
- **Grup sohbeti** kapsam dışı; ancak `conversation_id` alanı ve protokol mesaj tipleri buna kapı açık bırakacak şekilde tasarlandı (§2.3.3)
- **Mobil** kapsam dışı
- **Hesap sistemi** v2'de internet bağlantısıyla birlikte gelecek

---

## 10. Karar Günlüğü (ADR)

Aşağıdaki kararlar bilinçli olarak alınmıştır; değiştirilmeden önce gerekçeleri okunmalıdır.

**K1 — Transport: ham `quinn`, `iroh` değil.**
`iroh` (quinn üzerine kurulu) Ed25519 node kimliği, mDNS keşfi, hole-punching ve relay'i hazır getirirdi; Faz 2–4 ile 12'nin büyük kısmını ortadan kaldırırdı. Buna rağmen ham quinn seçildi: protokol ve bağlantı yaşam döngüsü üzerinde tam kontrol, büyük bir dış bağımlılığa bağlanmama. **Bedeli kabul edildi:** v2'de relay ve NAT traversal sıfırdan yazılacak.

**K2 — Pairing: PIN girme değil, karşılıklı doğrulama kodu (SAS).**
Tek yönlü PIN girişi, PIN gerçek bir PAKE (SPAKE2) olarak kullanılmadıkça MITM'i engellemez: zaten kurulmuş ama peer'ı doğrulanmamış bir TLS kanalında düz HMAC-challenge, saldırgan iki ayrı oturum kurup mesajları relay ettiğinde başarılı olur. SPAKE2 doğru çözümdür ama implementasyon yükü yüksektir. TLS exporter'a bağlanmış SAS aynı güvenliği çok daha az kodla sağlar ve UX'i daha akıcıdır.

**K3 — DB erişimi: `rusqlite` + `r2d2`, `sqlx` değil.**
`sqlx`'in async'i cazip ama compile-time query makroları `DATABASE_URL` ortam değişkeni bağımlılığı ve CI sürtünmesi getiriyor. Gömülü SQLite'ta gerçek async I/O zaten yok; `spawn_blocking` ile `rusqlite` pratikte eşdeğer performans, daha az sürtünme. FTS5'in `bundled` derlemede etkin olduğu Faz 1'de doğrulanacak.

**K4 — Transferde paralel chunk yok; dosya başına tek sıralı stream var.**
Tek bir QUIC bağlantısındaki tüm stream'ler aynı congestion controller'ı paylaşır; chunk'ları paralel stream'lere bölmek throughput artırmaz. Gerçek hız, pencere/buffer tuning'inden gelir — sanılmıştı; **Faz 2 ölçümü bunu da düzeltti**: loopback'te darboğaz CPU, tuning'in ölçülebilir etkisi yok (§2.2.2). Yan fayda: resume, chunk bitmap'i yerine tek byte offset'ine indirgenir — belirgin bir sadeleşme.

**K5 — Chunk başına hash yok; dosya sonunda tek blake3 var.**
QUIC/TLS 1.3 her byte'ı authenticated encryption ile korur; ağ kaynaklı bozulma stream'e ulaşamaz. Chunk hash'i yalnızca CPU tüketir. Doğrulanması gereken tek şey, resume sonrası dosyanın doğru birleşmesidir.

**K6 — Anahtar saklama: OS keychain (`keyring`).**
"Parolasız şifreli dosya" şifreleme değil, obfuscation'dır (anahtar da aynı diskte). Kullanıcı parolası istemek ise tray'de arka planda yaşayan bir uygulama için ağır bir UX yüküdür. OS keychain doğru dengedir. Linux'ta Secret Service yoksa `0600` dosyaya düşülür ve **bu durum kullanıcıya açıkça bildirilir**.

**K7 — mDNS tek başına yeterli değil; manuel ekleme v1'de.**
Windows Firewall "Public network" profili ve kurumsal/misafir ağlardaki client isolation mDNS'i düzenli olarak kırar. Manuel IP ile ekleme bir "gelişmiş özellik" değil, temel çalışabilirlik gereksinimidir.

**K8 — UI dili Türkçe, ama i18n altyapısı baştan.**
Metinler sözlük dosyasında toplanır; v1'de tek dil yüklenir. Sonradan i18n eklemek tüm UI dosyalarına dokunmayı gerektirirdi.

**K10 — Eşleştirme bildirimleri arayüzden soyutlandı.**
`PairingManager` doğrudan Tauri'ye yayın yapmıyor; `PairingNotifier` arayüzü üzerinden yapıyor. Gerekçe test edilebilirlik: eşleştirme uygulamanın en güvenlik-kritik akışı ve yalnızca elle tıklayarak sınanması kabul edilemezdi. Bu soyutlama sayesinde akış, iki gerçek QUIC/TLS uç noktası arasında — dolayısıyla gerçek channel binding ile — otomatik olarak test ediliyor: iki taraf da onaylayınca aynı kodun göründüğü, tek taraflı onayın kayıt oluşturmadığı ve eşleşme sonrası pinlenmiş bağlantının kod sormadığı testlerle kapalı.

**K9 — Uzak CI servisi yok; doğrulama pre-push hook'u ile yerelde.**
Uzak koşucunun (GitHub Actions vb.) üç değeri var: commit öncesi unutulanı yakalamak, temiz oda/tekrarlanabilirlik, çapraz platform doğrulama. Tek geliştirici, tek makine ve Windows-öncelikli bir uygulamada ikincisi ve üçüncüsü henüz spekülatif; birincisi ise `.githooks/pre-push` ile bedava çözülüyor. Docker'lı bir yerel CI da değerlendirildi ve elendi: Linux'u doğrular ama asıl riskin bulunduğu Windows'a özgü kodu (keyring backend'i, rezerve dosya adları, MAX_PATH — §2.6, §2.13.1) test edemez.
**Bu karar şu koşullarda gözden geçirilmeli:** projeye ikinci bir geliştirici katıldığında, ya da macOS/Linux gerçekten hedef sürüm hâline geldiğinde (§2.15).
