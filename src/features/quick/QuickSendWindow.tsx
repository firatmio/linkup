import { useEffect, useState } from "react";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import { readText } from "@tauri-apps/plugin-clipboard-manager";
import { Monitor, Paperclip, Send, X, ClipboardType } from "lucide-react";
import { t, translateError } from "../../i18n";
import { cn } from "../../lib/cn";
import { Button } from "../../components/Button";
import { Callout } from "../../components/Callout";
import { api, type TrustedDevice } from "../../lib/tauri";

/** Panodaki metnin önizlemede gösterilen en fazla uzunluğu. */
const PREVIEW_LIMIT = 400;

/**
 * Global kısayolun açtığı küçük pencere (PLAN.md §2.11).
 *
 * Ana pencereden bağımsız çalışır: kendi verisini çeker, kendi hatasını
 * gösterir ve iş bitince kendini kapatır. Uygulamanın store'larına
 * bağlanmıyor — bu pencere ana pencere hiç açılmadan da açılabilir ve o
 * store'ların abonelikleri burada gereksiz iş olurdu.
 *
 * Açılışta PANO OKUNUR: kullanıcı çoğu zaman bir şeyi kopyaladıktan hemen
 * sonra kısayola basıyor. Metin varsa doğrudan "gönder" olarak sunulur;
 * kopyaladığı şeyi bir kez daha yapıştırmak zorunda kalması gereksiz bir adım
 * olurdu. Pano dosya YOLU okunmuyor — Tauri eklentisi yalnızca metin ve görsel
 * veriyor, dosya için platforma özel kod gerekiyor (PLAN.md §2.9, Faz 9).
 *
 * Yalnızca ÇEVRİMİÇİ cihazlar listeleniyor: çevrimdışı bir cihaza dosya
 * gönderilemez, listede göstermek başarısız olacak bir seçim sunmak olurdu.
 */
export function QuickSendWindow() {
  const [devices, setDevices] = useState<TrustedDevice[] | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [clipboard, setClipboard] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let cancelled = false;

    void api
      .quickSendDevices()
      .then((list) => {
        if (cancelled) return;
        setDevices(list);
        // Tek cihaz varsa seçim diye bir karar yok; hazır seçili gelsin.
        if (list.length === 1) setSelected(list[0].id);
      })
      .catch((err) => {
        if (!cancelled) setError(translateError(err));
      });

    // Pano boş veya erişilemez olabilir; ikisi de hata değil, sadece
    // "gönderilecek metin yok" demek.
    void readText()
      .then((text) => {
        if (!cancelled && text?.trim()) setClipboard(text);
      })
      .catch(() => {});

    return () => {
      cancelled = true;
    };
  }, []);

  const close = () => void api.closeQuickSend();

  // Esc ile kapanmalı: bu bir araç penceresi, kullanıcı onu tuşla açtı.
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") close();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const run = async (action: () => Promise<void>) => {
    setBusy(true);
    setError(null);
    try {
      await action();
      close();
    } catch (err) {
      // İptal bir hata değil: kullanıcı dosya seçicisinden vazgeçtiyse
      // pencere açık kalır ve hiçbir uyarı gösterilmez.
      if (!(err instanceof CancelledError)) setError(translateError(err));
      setBusy(false);
    }
  };

  const sendClipboard = () =>
    run(async () => {
      if (!selected || !clipboard) return;
      await api.sendMessage(selected, clipboard, false);
    });

  const pickAndSend = () =>
    run(async () => {
      if (!selected) return;
      const chosen = await openFileDialog({ multiple: true });
      // Kullanıcı seçiciyi iptal etti: pencere kapanmamalı.
      if (!chosen) throw new CancelledError();

      for (const path of Array.isArray(chosen) ? chosen : [chosen]) {
        await api.sendFile(selected, path);
      }
    });

  return (
    // Tüm yüzey sürüklenebilir. `data-tauri-drag-region` yalnızca özniteliğin
    // BULUNDUĞU elemanda sürükleme başlatır, dolayısıyla üstteki düğmeler ve
    // liste normal şekilde tıklanmaya devam eder.
    <div
      data-tauri-drag-region
      className="flex h-screen flex-col bg-base text-fg select-none"
    >
      <header
        data-tauri-drag-region
        className="flex h-11 shrink-0 items-center justify-between border-b border-divider px-4"
      >
        <span data-tauri-drag-region className="pointer-events-none min-w-0 truncate">
          <span className="font-display text-[length:var(--lu-text-body)] font-semibold">
            {t("app.name")}
          </span>
          <span className="ml-2 text-[length:var(--lu-text-caption)] text-fg-secondary">
            {t("quick.title")}
          </span>
        </span>
        <button
          type="button"
          onClick={close}
          aria-label={t("common.close")}
          className="-mr-2 flex size-8 shrink-0 items-center justify-center rounded-lu-sm text-fg-secondary hover:bg-[#c42b1c] hover:text-white"
        >
          <X size={16} />
        </button>
      </header>

      <div data-tauri-drag-region className="flex min-h-0 flex-1 flex-col gap-3 p-4">
        {error ? <Callout tone="warning">{error}</Callout> : null}

        {devices === null ? (
          <p className="text-[length:var(--lu-text-body)] text-fg-secondary">
            {t("common.loading")}
          </p>
        ) : devices.length === 0 ? (
          <p className="text-[length:var(--lu-text-body)] text-fg-secondary">
            {t("quick.noDevices")}
          </p>
        ) : (
          <ul data-tauri-drag-region className="min-h-0 flex-1 space-y-1 overflow-y-auto">
            {devices.map((device) => (
              <li key={device.id}>
                <button
                  type="button"
                  onClick={() => setSelected(device.id)}
                  className={cn(
                    "flex w-full items-center gap-2.5 rounded-lu-sm px-2.5 py-2 text-left",
                    "transition-colors duration-[var(--lu-dur-fast)]",
                    device.id === selected ? "bg-selected" : "hover:bg-hover active:bg-press",
                  )}
                >
                  <span className="flex size-8 shrink-0 items-center justify-center rounded-lu-full bg-layer-alt">
                    <Monitor size={16} className="text-fg-secondary" />
                  </span>
                  <span className="min-w-0 flex-1 truncate text-[length:var(--lu-text-body)]">
                    {device.name}
                  </span>
                  <span aria-hidden className="size-2 shrink-0 rounded-full bg-success" />
                </button>
              </li>
            ))}
          </ul>
        )}

        {clipboard ? (
          <div className="shrink-0 space-y-1.5">
            <p className="flex items-center gap-1.5 text-[length:var(--lu-text-caption)] text-fg-secondary">
              <ClipboardType size={14} />
              {t("quick.clipboard")}
            </p>
            <p className="lu-selectable max-h-20 overflow-y-auto rounded-lu-sm border border-stroke bg-layer-alt px-2.5 py-2 text-[length:var(--lu-text-caption)] break-words whitespace-pre-wrap">
              {clipboard.slice(0, PREVIEW_LIMIT)}
              {clipboard.length > PREVIEW_LIMIT ? "…" : ""}
            </p>
            <Button
              variant="accent"
              className="w-full"
              icon={<Send size={16} />}
              disabled={!selected || busy}
              onClick={() => void sendClipboard()}
            >
              {t("quick.sendClipboard")}
            </Button>
          </div>
        ) : null}

        <Button
          className="shrink-0"
          icon={<Paperclip size={16} />}
          disabled={!selected || busy}
          onClick={() => void pickAndSend()}
        >
          {busy ? t("quick.sending") : t("quick.pick")}
        </Button>
      </div>
    </div>
  );
}

/**
 * Dosya seçici iptal edildiğinde pencerenin kapanmasını engellemek için
 * kullanılan işaret. Hata olarak GÖSTERİLMEZ: iptal bir hata değil.
 */
class CancelledError extends Error {}
