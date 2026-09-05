import { useState } from "react";
import { Monitor, Trash2 } from "lucide-react";
import { t } from "../../i18n";
import { cn } from "../../lib/cn";
import { Card } from "../../components/Surface";
import { Button } from "../../components/Button";
import { Dialog } from "../../components/Dialog";
import { usePairingStore } from "../../stores/pairingStore";
import type { TrustedDevice } from "../../lib/tauri";

/**
 * Eşleşmiş bir cihazın özet kartı (PLAN.md §3.2).
 * Faz 5/6'da son mesaj ve zaman bilgisi eklenecek.
 */
export function DeviceSummaryCard({ device }: { device: TrustedDevice }) {
  const [confirmOpen, setConfirmOpen] = useState(false);
  const forget = usePairingStore((s) => s.forget);

  return (
    <Card className="group p-4">
      <div className="flex items-start gap-3">
        <Monitor size={20} className="mt-0.5 shrink-0 text-fg-secondary" />

        <div className="min-w-0 flex-1">
          <p className="truncate font-semibold">{device.name}</p>
          <p className="flex items-center gap-1.5 text-[length:var(--lu-text-caption)] text-fg-secondary">
            <span
              aria-hidden
              className={cn(
                "inline-block size-2 rounded-full",
                device.online ? "bg-success" : "bg-offline",
              )}
            />
            {t(device.online ? "status.online" : "status.offline")}
          </p>
        </div>

        <button
          type="button"
          aria-label={t("devices.forget")}
          title={t("devices.forget")}
          onClick={() => setConfirmOpen(true)}
          className="shrink-0 rounded-lu-sm p-1 text-fg-tertiary opacity-0 transition-opacity group-hover:opacity-100 hover:bg-press hover:text-danger"
        >
          <Trash2 size={16} />
        </button>
      </div>

      <Dialog
        open={confirmOpen}
        title={t("devices.forget")}
        onClose={() => setConfirmOpen(false)}
        footer={
          <>
            <Button onClick={() => setConfirmOpen(false)}>{t("devices.forget.cancel")}</Button>
            <Button
              variant="accent"
              onClick={() => {
                setConfirmOpen(false);
                void forget(device.id);
              }}
            >
              {t("devices.forget")}
            </Button>
          </>
        }
      >
        <p className="text-[length:var(--lu-text-body)] text-fg-secondary">
          {t("devices.forget.confirm", { device: device.name })}
        </p>
      </Dialog>
    </Card>
  );
}
