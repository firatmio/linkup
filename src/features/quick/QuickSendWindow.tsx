import { useEffect, useState } from "react";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import { Monitor, Paperclip, Send, X, TextCursorInput } from "lucide-react";
import { t, translateError } from "../../i18n";
import { cn } from "../../lib/cn";
import { Button } from "../../components/Button";
import { Callout } from "../../components/Callout";
import { api, type TrustedDevice } from "../../lib/tauri";

/** Önizlemede gösterilen en fazla metin uzunluğu. */
const PREVIEW_LIMIT = 400;

/**
 * Global kısayolun açtığı küçük pencere (PLAN.md §2.11).
 *
 * Ana pencereden bağımsız çalışır: kendi verisini çeker, kendi hatasını
 * gösterir ve iş bitince kendini kapatır. Uygulamanın store'larına
 * bağlanmıyor — bu pencere ana pencere hiç açılmadan da açılabilir ve o
 * store'ların abonelikleri burada gereksiz iş olurdu.
 *
 * Açılışta metin HAZIR GELİR: önce öndeki uygulamada seçili olan metin
 * yakalanır, o yoksa panodaki metne düşülür (bkz. `selection.rs`). Kullanıcı
 * çoğu zaman bir şeyi seçtikten veya kopyaladıktan hemen sonra kısayola
 * basıyor; ona bir kez daha yapıştırtmak gereksiz bir adım olurdu.
 *
 * Pano dosya YOLU okunmuyor — Tauri eklentisi yalnızca metin ve görsel
 * veriyor, dosya için platforma özel kod gerekiyor (PLAN.md §2.9, Faz 9).
 *
 * Yalnızca ÇEVRİMİÇİ cihazlar listeleniyor: çevrimdışı bir cihaza dosya
 * gönderilemez, listede göstermek başarısız olacak bir seçim sunmak olurdu.
 */
export function QuickSendWindow() {
  const [devices, setDevices] = useState<TrustedDevice[] | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [text, setText] = useState<string | null>(null);
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

    // Seçim ve pano boş olabilir; ikisi de hata değil, sadece
    // "gönderilecek metin yok" demek.
    void api
      .quickSendText()
      .then((value) => {
        if (!cancelled && value?.trim()) setText(value);
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

  const sendText = () =>
    run(async () => {
      if (!selected || !text) return;
      await api.sendMessage(selected, text, false);
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

        {text ? (
          <div className="shrink-0 space-y-1.5">
            <p className="flex items-center gap-1.5 text-[length:var(--lu-text-caption)] text-fg-secondary">
              <TextCursorInput size={14} />
              {t("quick.text")}
            </p>
            <p className="lu-selectable max-h-20 overflow-y-auto rounded-lu-sm border border-stroke bg-layer-alt px-2.5 py-2 text-[length:var(--lu-text-caption)] break-words whitespace-pre-wrap">
              {text.slice(0, PREVIEW_LIMIT)}
              {text.length > PREVIEW_LIMIT ? "…" : ""}
            </p>
            <Button
              variant="accent"
              className="w-full"
              icon={<Send size={16} />}
              disabled={!selected || busy}
              onClick={() => void sendText()}
            >
              {t("quick.sendText")}
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
