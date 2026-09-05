import { useState } from "react";
import { Plus, Radar, Monitor, X } from "lucide-react";
import { t } from "../../i18n";
import { cn } from "../../lib/cn";
import { Button } from "../../components/Button";
import { useDeviceStore } from "../../stores/deviceStore";
import { ManualAddDialog } from "./ManualAddDialog";

/**
 * Sidebar'ın altındaki "Bulunanlar" bölümü (PLAN.md §3.2): ağda görünen ama
 * henüz eşleşmemiş cihazlar. Eşleştirme akışı Faz 4'te bağlanacak.
 */
export function DiscoveredDevices() {
  const [dialogOpen, setDialogOpen] = useState(false);
  const devices = useDeviceStore((s) => s.discovered);
  const forget = useDeviceStore((s) => s.forget);

  return (
    <div className="mt-auto border-t border-divider px-2 py-3">
      <div className="flex items-center gap-2 px-3 pb-2 text-fg-tertiary">
        <Radar size={14} />
        <span className="text-[length:var(--lu-text-caption)] font-semibold tracking-wide uppercase">
          {t("nav.discovered")}
        </span>
        {devices.length > 0 ? (
          <span className="text-[length:var(--lu-text-caption)]">{devices.length}</span>
        ) : null}
      </div>

      {devices.length === 0 ? (
        <p className="px-3 pb-2 text-[length:var(--lu-text-caption)] text-fg-tertiary">
          {t("nav.discovered.empty")}
        </p>
      ) : (
        <ul className="mb-1 flex max-h-52 flex-col gap-0.5 overflow-y-auto">
          {devices.map((device) => (
            <li key={device.id}>
              <div
                className={cn(
                  "group flex h-[var(--lu-row-h)] items-center gap-2 rounded-lu-sm px-3",
                  "hover:bg-hover",
                )}
                title={`${device.name}\n${device.address ?? ""}\n${device.fingerprint}`}
              >
                <Monitor size={16} className="shrink-0 text-fg-secondary" />
                <span className="min-w-0 flex-1 truncate text-[length:var(--lu-text-body)]">
                  {device.name}
                </span>
                {device.source === "manual" ? (
                  <span className="shrink-0 text-[length:var(--lu-text-caption)] text-fg-tertiary">
                    {t("nav.discovered.manual")}
                  </span>
                ) : null}
                <button
                  type="button"
                  aria-label={t("device.forget")}
                  title={t("device.forget")}
                  onClick={() => void forget(device.id)}
                  className="shrink-0 rounded-lu-sm p-0.5 text-fg-tertiary opacity-0 transition-opacity group-hover:opacity-100 hover:bg-press hover:text-fg"
                >
                  <X size={14} />
                </button>
              </div>
            </li>
          ))}
        </ul>
      )}

      <Button
        variant="subtle"
        icon={<Plus size={16} />}
        className="w-full justify-start"
        onClick={() => setDialogOpen(true)}
      >
        {t("nav.addManually")}
      </Button>

      <ManualAddDialog open={dialogOpen} onClose={() => setDialogOpen(false)} />
    </div>
  );
}
