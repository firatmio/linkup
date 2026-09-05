import { Bell, Moon } from "lucide-react";
import { t } from "../../i18n";
import { Card, SectionTitle, SettingRow } from "../../components/Surface";
import { Switch } from "../../components/Switch";
import { useSettingsStore } from "../../stores/settingsStore";

/** Sessiz saatler kapalıyken açılırsa kullanılan aralık. */
const DEFAULT_RANGE = "23:00-07:00";

function split(range: string): [string, string] | null {
  const parts = range.split("-");
  return parts.length === 2 ? [parts[0].trim(), parts[1].trim()] : null;
}

/** Bildirim ayarları (PLAN.md §3.4). */
export function NotificationsSection() {
  const settings = useSettingsStore((s) => s.settings);
  const saving = useSettingsStore((s) => s.saving);
  const set = useSettingsStore((s) => s.set);
  const setFlag = useSettingsStore((s) => s.setFlag);

  const disabled = !settings || saving;
  const enabled = settings?.notificationsEnabled ?? true;
  const range = split(settings?.quietHours ?? "");

  const setBound = (index: 0 | 1, value: string) => {
    const current = range ?? split(DEFAULT_RANGE)!;
    const next: [string, string] = [...current] as [string, string];
    next[index] = value;
    void set("quietHours", `${next[0]}-${next[1]}`);
  };

  return (
    <section className="space-y-2">
      <SectionTitle>{t("settings.section.notifications")}</SectionTitle>
      <Card>
        <SettingRow
          icon={<Bell size={18} />}
          title={t("settings.notifications")}
          description={t("settings.notifications.desc")}
          control={
            <Switch
              checked={enabled}
              disabled={disabled}
              onChange={(value) => void setFlag("notificationsEnabled", value)}
              label={t("settings.notifications")}
            />
          }
        />

        <SettingRow
          icon={<Moon size={18} />}
          title={t("settings.quietHours")}
          description={t("settings.quietHours.desc")}
          control={
            <span className="flex items-center gap-2">
              {range ? (
                <>
                  <TimeInput
                    label={t("settings.quietHours.from")}
                    value={range[0]}
                    disabled={disabled || !enabled}
                    onChange={(value) => setBound(0, value)}
                  />
                  <span className="text-fg-tertiary">–</span>
                  <TimeInput
                    label={t("settings.quietHours.to")}
                    value={range[1]}
                    disabled={disabled || !enabled}
                    onChange={(value) => setBound(1, value)}
                  />
                </>
              ) : null}
              <Switch
                checked={range !== null}
                disabled={disabled || !enabled}
                onChange={(on) => void set("quietHours", on ? DEFAULT_RANGE : "")}
                label={t("settings.quietHours.enable")}
              />
            </span>
          }
        />
      </Card>
    </section>
  );
}

function TimeInput({
  label,
  value,
  disabled,
  onChange,
}: {
  label: string;
  value: string;
  disabled?: boolean;
  onChange: (value: string) => void;
}) {
  return (
    <input
      type="time"
      value={value}
      aria-label={label}
      disabled={disabled}
      onChange={(event) => onChange(event.target.value)}
      className="h-[var(--lu-control-h)] rounded-lu-sm border border-stroke-strong bg-layer-alt px-2 text-fg shadow-[inset_0_-1px_0_var(--lu-stroke-strong)] focus:border-accent focus:outline-none disabled:pointer-events-none disabled:text-fg-disabled"
    />
  );
}
