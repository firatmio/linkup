import { useEffect, useState } from "react";
import { File } from "lucide-react";
import { t } from "../../i18n";
import { fileSize } from "../../lib/format";
import { Dialog } from "../../components/Dialog";
import { Button } from "../../components/Button";
import {
  api,
  onTransferRequested,
  onTransferResolved,
  type TransferRequest,
} from "../../lib/tauri";

/**
 * Gelen dosya onayı (PLAN.md §2.13.3).
 *
 * Karşı taraf güvenilir olsa bile gönderdiği dosya sessizce diske
 * yazılmamalı; kullanıcı ne aldığını bilmeli.
 *
 * Birden fazla istek aynı anda gelebilir; kuyruğa alınıp sırayla sorulur —
 * diyalogların üst üste binmesi hangi dosyayı onayladığını belirsizleştirir.
 */
export function TransferRequestDialog() {
  const [queue, setQueue] = useState<TransferRequest[]>([]);
  const current = queue[0] ?? null;

  useEffect(() => {
    const unlisteners: Array<() => void> = [];
    let cancelled = false;

    const track = (fn: () => void) => {
      if (cancelled) fn();
      else unlisteners.push(fn);
    };

    void onTransferRequested((request) => {
      setQueue((existing) =>
        existing.some((item) => item.transferId === request.transferId)
          ? existing
          : [...existing, request],
      );
    }).then(track);

    // Süre dolduysa veya iptal edildiyse diyalog kapanmalı: kullanıcı artık
    // etkisi olmayan bir soruya bakmamalı.
    void onTransferResolved((transferId) => {
      setQueue((existing) => existing.filter((item) => item.transferId !== transferId));
    }).then(track);

    return () => {
      cancelled = true;
      unlisteners.forEach((fn) => fn());
    };
  }, []);

  const respond = (accept: boolean) => {
    if (!current) return;
    void api.respondToTransfer(current.transferId, accept);
    setQueue((existing) => existing.slice(1));
  };

  return (
    <Dialog
      open={current !== null}
      title={t("transfers.request.title")}
      // Esc reddetme sayılır: yanıtsız bırakmak karşı tarafı bekletir.
      onClose={() => respond(false)}
      footer={
        <>
          <Button onClick={() => respond(false)}>{t("transfers.request.reject")}</Button>
          <Button variant="accent" onClick={() => respond(true)}>
            {t("transfers.request.accept")}
          </Button>
        </>
      }
    >
      {current ? (
        <>
          <p className="text-[length:var(--lu-text-body)] text-fg-secondary">
            {t("transfers.request.body", { device: current.deviceName })}
          </p>

          <div className="flex items-center gap-3 rounded-lu-lg border border-stroke bg-layer-alt px-3 py-2.5">
            <span className="flex size-9 shrink-0 items-center justify-center rounded-lu-sm bg-layer">
              <File size={18} className="text-fg-secondary" />
            </span>
            <span className="min-w-0">
              <span className="block truncate text-[length:var(--lu-text-body)]">
                {current.fileName}
              </span>
              <span className="text-[length:var(--lu-text-caption)] text-fg-secondary">
                {fileSize(current.fileSize)}
              </span>
            </span>
          </div>

          <p className="text-[length:var(--lu-text-caption)] text-fg-tertiary">
            {t("transfers.request.hint")}
          </p>
        </>
      ) : null}
    </Dialog>
  );
}
