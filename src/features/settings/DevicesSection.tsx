import { useState } from "react";
import { Monitor, Trash2 } from "lucide-react";
import { t } from "../../i18n";
import { Card, SectionTitle } from "../../components/Surface";
import { Button } from "../../components/Button";
import { api, type TrustedDevice } from "../../lib/tauri";
import { usePairingStore } from "../../stores/pairingStore";

/**
 * Eşleşmiş cihazlar (PLAN.md §3.4).
 *
 * Takma ad yalnızca yerel: karşı tarafa gitmiyor, ağda ilan edilmiyor.
 * Kullanıcının "Ofis bilgisayarı" demesi karşı tarafın kendi adını
 * değiştirmemeli.
 */
export function DevicesSection() {
  const devices = usePairingStore((s) => s.trusted);
  const forget = usePairingStore((s) => s.forget);

  return (
    <section className="space-y-2">
      <SectionTitle>{t("settings.section.devices")}</SectionTitle>
      <Card>
        {devices.length === 0 ? (
          <p className="px-4 py-6 text-center text-[length:var(--lu-text-body)] text-fg-tertiary">
            {t("settings.devices.empty")}
          </p>
        ) : (
          <ul>
            {devices.map((device) => (
              <DeviceRow key={device.id} device={device} onForget={() => void forget(device.id)} />
            ))}
          </ul>
        )}
      </Card>
      <p className="px-1 text-[length:var(--lu-text-caption)] text-fg-secondary">
        {t("settings.devices.desc")}
      </p>
    </section>
  );
}

function DeviceRow({ device, onForget }: { device: TrustedDevice; onForget: () => void }) {
  const loadTrusted = usePairingStore((s) => s.loadTrusted);
  const [alias, setAlias] = useState(device.alias ?? "");
  const [confirming, setConfirming] = useState(false);

  // Yazarken değil, odak kaybında kaydedilir: her tuşta veritabanına yazmak
  // ve listeyi yeniden çekmek, yazarken imlecin zıplamasına yol açardı.
  const commit = async () => {
    if (alias.trim() === (device.alias ?? "").trim()) return;
    await api.setDeviceAlias(device.id, alias);
    await loadTrusted();
  };

  return (
    <li className="flex items-center gap-3 border-b border-divider px-4 py-3 last:border-b-0">
      <span className="relative shrink-0">
        <span className="flex size-9 items-center justify-center rounded-lu-full bg-layer-alt">
          <Monitor size={18} className="text-fg-secondary" />
        </span>
        <span
          aria-hidden
          className={`absolute -right-0.5 -bottom-0.5 size-2.5 rounded-full ring-2 ring-[var(--lu-bg-layer)] ${
            device.online ? "bg-success" : "bg-offline"
          }`}
        />
      </span>

      <span className="min-w-0 flex-1">
        <span className="block truncate text-[length:var(--lu-text-body)]">
          {device.deviceName}
        </span>
        <span className="block truncate font-mono text-[length:var(--lu-text-caption)] text-fg-tertiary">
          {device.fingerprint.slice(0, 29)}…
        </span>
      </span>

      <input
        type="text"
        value={alias}
        aria-label={t("settings.devices.alias")}
        placeholder={t("settings.devices.aliasPlaceholder")}
        onChange={(event) => setAlias(event.target.value)}
        onBlur={() => void commit()}
        onKeyDown={(event) => {
          if (event.key === "Enter") event.currentTarget.blur();
        }}
        className="lu-selectable h-[var(--lu-control-h)] w-48 shrink-0 rounded-lu-sm border border-stroke-strong bg-layer-alt px-3 text-fg shadow-[inset_0_-1px_0_var(--lu-stroke-strong)] placeholder:text-fg-tertiary focus:border-accent focus:shadow-[inset_0_-2px_0_var(--lu-accent)] focus:outline-none"
      />

      {confirming ? (
        <span className="flex shrink-0 items-center gap-1">
          <Button variant="subtle" onClick={() => setConfirming(false)}>
            {t("common.cancel")}
          </Button>
          <Button variant="danger" onClick={onForget}>
            {t("settings.devices.forget")}
          </Button>
        </span>
      ) : (
        <Button
          variant="subtle"
          className="shrink-0"
          icon={<Trash2 size={16} />}
          onClick={() => setConfirming(true)}
        >
          {t("settings.devices.forget")}
        </Button>
      )}
    </li>
  );
}
