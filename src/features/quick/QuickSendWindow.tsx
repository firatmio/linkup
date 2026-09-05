import { useEffect, useState } from "react";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import { Monitor, Paperclip, Send, X } from "lucide-react";
import { t } from "../../i18n";
import { cn } from "../../lib/cn";
import { Button } from "../../components/Button";
import { Callout } from "../../components/Callout";
import { api, type TrustedDevice } from "../../lib/tauri";
import { translateError } from "../../i18n";

/**
 * Global kısayolun açtığı küçük pencere (PLAN.md §2.11).
 *
 * Ana pencereden bağımsız çalışır: kendi verisini çeker, kendi hatasını
 * gösterir ve iş bitince kendini kapatır. Uygulamanın store'larına
 * bağlanmıyor — bu pencere ana pencere hiç açılmadan da açılabilir ve o
 * store'ların abonelikleri burada gereksiz iş olurdu.
 *
 * Yalnızca ÇEVRİMİÇİ cihazlar listeleniyor: çevrimdışı bir cihaza dosya
 * gönderilemez, listede göstermek başarısız olacak bir seçim sunmak olurdu.
 */
export function QuickSendWindow() {
  const [devices, setDevices] = useState<TrustedDevice[] | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [sending, setSending] = useState(false);

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

  const pickAndSend = async () => {
    if (!selected) return;

    const chosen = await openFileDialog({ multiple: true });
    if (!chosen) return;

    setSending(true);
    setError(null);
    try {
      for (const path of Array.isArray(chosen) ? chosen : [chosen]) {
        await api.sendFile(selected, path);
      }
      close();
    } catch (err) {
      setError(translateError(err));
      setSending(false);
    }
  };

  return (
    <div className="flex h-screen flex-col bg-base text-fg">
      <header
        // Çerçevesiz değil ama başlık alanı yine de sürüklenebilir olsun.
        data-tauri-drag-region
        className="flex h-11 shrink-0 items-center justify-between border-b border-divider px-4"
      >
        <span className="font-display text-[length:var(--lu-text-body)] font-semibold">
          {t("quick.title")}
        </span>
        <button
          type="button"
          onClick={close}
          aria-label={t("common.close")}
          className="rounded-lu-sm p-1.5 text-fg-secondary hover:bg-hover hover:text-fg"
        >
          <X size={16} />
        </button>
      </header>

      <div className="flex min-h-0 flex-1 flex-col gap-3 p-4">
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
          <ul className="min-h-0 flex-1 space-y-1 overflow-y-auto">
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

        <Button
          variant="accent"
          icon={sending ? <Send size={16} /> : <Paperclip size={16} />}
          disabled={!selected || sending}
          onClick={() => void pickAndSend()}
        >
          {sending ? t("quick.sending") : t("quick.pick")}
        </Button>
      </div>
    </div>
  );
}
