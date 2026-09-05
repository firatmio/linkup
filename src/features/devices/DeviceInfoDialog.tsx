import { ShieldCheck, Fingerprint, Network, CalendarClock } from "lucide-react";
import { t } from "../../i18n";
import { relativeTime } from "../../lib/format";
import { Dialog } from "../../components/Dialog";
import { Button } from "../../components/Button";
import { Switch } from "../../components/Switch";
import { Callout } from "../../components/Callout";
import { usePairingStore } from "../../stores/pairingStore";
import type { TrustedDevice } from "../../lib/tauri";

/**
 * Cihaz bilgi kartı: kimlik, adres, eşleşme tarihi ve "güvenli cihaz" anahtarı.
 *
 * Anahtar açıkken bu cihazdan gelen dosyalar onay sorulmadan kabul edilir.
 * Karar cihaz bazındadır ve varsayılan kapalıdır — bir cihaza güvenmek
 * hepsine güvenmek değildir.
 */
export function DeviceInfoDialog({
  device,
  open,
  onClose,
}: {
  device: TrustedDevice;
  open: boolean;
  onClose: () => void;
}) {
  const setAutoAccept = usePairingStore((s) => s.setAutoAccept);

  return (
    <Dialog
      open={open}
      title={device.name}
      onClose={onClose}
      footer={<Button onClick={onClose}>{t("common.close")}</Button>}
    >
      <div className="space-y-3">
        <Row
          icon={<Network size={16} />}
          label={t("device.info.address")}
          value={device.lastAddress ?? t("device.info.unknown")}
        />
        <Row
          icon={<CalendarClock size={16} />}
          label={t("device.info.pairedAt")}
          value={relativeTime(device.pairedAt)}
        />
        <div className="space-y-1">
          <p className="flex items-center gap-2 text-[length:var(--lu-text-caption)] text-fg-secondary">
            <Fingerprint size={16} />
            {t("device.info.fingerprint")}
          </p>
          <code className="lu-selectable block rounded-lu-sm border border-stroke bg-layer-alt px-2.5 py-2 font-mono text-[length:var(--lu-text-caption)] leading-relaxed break-all">
            {device.fingerprint}
          </code>
        </div>
      </div>

      <div className="mt-2 flex items-start gap-3 rounded-lu-lg border border-stroke bg-layer-alt px-3 py-3">
        <ShieldCheck size={18} className="mt-0.5 shrink-0 text-fg-secondary" />
        <div className="min-w-0 flex-1">
          <p className="text-[length:var(--lu-text-body)]">{t("device.trusted.title")}</p>
          <p className="text-[length:var(--lu-text-caption)] text-fg-secondary">
            {t("device.trusted.desc")}
          </p>
        </div>
        <Switch
          checked={device.autoAccept}
          onChange={(enabled) => void setAutoAccept(device.id, enabled)}
          label={t("device.trusted.title")}
        />
      </div>

      {device.autoAccept ? (
        <Callout tone="warning">{t("device.trusted.warning")}</Callout>
      ) : null}
    </Dialog>
  );
}

function Row({
  icon,
  label,
  value,
}: {
  icon: React.ReactNode;
  label: string;
  value: string;
}) {
  return (
    <div className="flex items-center justify-between gap-3">
      <span className="flex items-center gap-2 text-[length:var(--lu-text-caption)] text-fg-secondary">
        {icon}
        {label}
      </span>
      <span className="lu-selectable truncate text-[length:var(--lu-text-body)]">{value}</span>
    </div>
  );
}
