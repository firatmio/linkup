import {
  Palette,
  Info,
  FolderOpen,
  HardDrive,
  Network,
  FlaskConical,
  Fingerprint,
  KeyRound,
  PanelsTopLeft,
  Power,
} from "lucide-react";
import { t } from "../../i18n";
import { Card, PageHeader, SectionTitle, SettingRow } from "../../components/Surface";
import { Button } from "../../components/Button";
import { Callout } from "../../components/Callout";
import { SegmentedControl } from "../../components/SegmentedControl";
import { Switch } from "../../components/Switch";
import { useUiStore, type ThemePreference } from "../../stores/uiStore";
import { useAppStore } from "../../stores/appStore";
import { useSettingsStore } from "../../stores/settingsStore";
import { api } from "../../lib/tauri";

const themeOptions: readonly { value: ThemePreference; label: string }[] = [
  { value: "system", label: t("settings.theme.system") },
  { value: "light", label: t("settings.theme.light") },
  { value: "dark", label: t("settings.theme.dark") },
];

export function SettingsPage() {
  const themePreference = useUiStore((s) => s.themePreference);
  const setThemePreference = useUiStore((s) => s.setThemePreference);
  const savingTheme = useUiStore((s) => s.savingTheme);
  const themeError = useUiStore((s) => s.error);

  const settings = useSettingsStore((s) => s.settings);
  const savingSettings = useSettingsStore((s) => s.saving);
  const settingsError = useSettingsStore((s) => s.error);
  const setCloseToTray = useSettingsStore((s) => s.setCloseToTray);
  const setAutostart = useSettingsStore((s) => s.setAutostart);

  const info = useAppStore((s) => s.info);
  const identity = useAppStore((s) => s.identity);
  const loading = useAppStore((s) => s.loading);
  const error = useAppStore((s) => s.error);

  /** Yüklenirken "Yükleniyor…", hata varsa hata metni, aksi halde değer. */
  const value = (get: (i: NonNullable<typeof info>) => string): string => {
    if (info) return get(info);
    return loading ? t("common.loading") : (error ?? t("error.unknown"));
  };

  const keyInFile = identity?.storage === "plainFile";

  return (
    <>
      <PageHeader title={t("settings.title")} />
      <div className="flex-1 space-y-6 overflow-y-auto px-6 pb-6">
        <section className="space-y-2">
          <SectionTitle>{t("settings.section.general")}</SectionTitle>
          {themeError ? <Callout tone="warning">{themeError}</Callout> : null}
          <Card>
            <SettingRow
              icon={<Palette size={18} />}
              title={t("settings.theme")}
              description={t("settings.theme.desc")}
              control={
                <SegmentedControl
                  ariaLabel={t("settings.theme")}
                  value={themePreference}
                  options={themeOptions}
                  onChange={(preference) => void setThemePreference(preference)}
                  disabled={savingTheme}
                />
              }
            />
          </Card>
        </section>

        <section className="space-y-2">
          <SectionTitle>{t("settings.section.window")}</SectionTitle>
          {settingsError ? <Callout tone="warning">{settingsError}</Callout> : null}
          <Card>
            <SettingRow
              icon={<PanelsTopLeft size={18} />}
              title={t("settings.closeToTray")}
              description={t("settings.closeToTray.desc")}
              control={
                <Switch
                  checked={settings?.closeToTray ?? true}
                  onChange={(enabled) => void setCloseToTray(enabled)}
                  label={t("settings.closeToTray")}
                  disabled={!settings || savingSettings}
                />
              }
            />
            <SettingRow
              icon={<Power size={18} />}
              title={t("settings.autostart")}
              description={t("settings.autostart.desc")}
              control={
                <Switch
                  checked={settings?.autostart ?? false}
                  onChange={(enabled) => void setAutostart(enabled)}
                  label={t("settings.autostart")}
                  disabled={!settings || savingSettings}
                />
              }
            />
          </Card>
        </section>

        <section className="space-y-2">
          <SectionTitle>{t("settings.section.security")}</SectionTitle>
          {keyInFile ? (
            <Callout tone="warning">{t("settings.keyStorage.plainFile.warning")}</Callout>
          ) : null}
          <Card>
            <SettingRow
              icon={<Fingerprint size={18} />}
              title={t("settings.fingerprint")}
              description={t("settings.fingerprint.desc")}
            >
              {identity ? (
                <div className="px-4 pb-4">
                  <code className="lu-selectable block rounded-lu-sm border border-stroke bg-layer-alt px-3 py-2 font-mono text-[length:var(--lu-text-caption)] leading-relaxed break-all">
                    {identity.fingerprint}
                  </code>
                </div>
              ) : null}
            </SettingRow>
            <SettingRow
              icon={<KeyRound size={18} />}
              title={t("settings.keyStorage")}
              description={
                identity
                  ? t(
                    identity.storage === "osKeychain"
                      ? "settings.keyStorage.osKeychain"
                      : "settings.keyStorage.plainFile",
                  )
                  : t("common.loading")
              }
            />
          </Card>
        </section>

        <section>
          <SectionTitle>{t("settings.section.advanced")}</SectionTitle>
          <Card>
            <SettingRow
              icon={<Info size={18} />}
              title={t("settings.version")}
              description={value((i) => i.version)}
            />
            <SettingRow
              icon={<FlaskConical size={18} />}
              title={t("settings.profile")}
              description={value((i) => i.profile ?? t("settings.profile.none"))}
            />
            <SettingRow
              icon={<Network size={18} />}
              title={t("settings.address")}
              description={value((i) =>
                i.reachableAddresses.length > 0
                  ? i.reachableAddresses.join("  ·  ")
                  : t("settings.address.none"),
              )}
            />
            <SettingRow
              icon={<Network size={18} />}
              title={t("settings.quicPort")}
              description={value((i) => String(i.quicPort))}
            />
            <SettingRow
              icon={<HardDrive size={18} />}
              title={t("settings.dataDir")}
              description={value((i) => i.dataDir)}
            />
            <SettingRow
              icon={<FolderOpen size={18} />}
              title={t("settings.downloadsDir")}
              description={value((i) => i.downloadsDir)}
            />
            <SettingRow
              icon={<FolderOpen size={18} />}
              title={t("settings.logs")}
              description={t("settings.logs.desc")}
              control={
                <Button onClick={() => void api.openLogDir()}>{t("settings.logs.open")}</Button>
              }
            />
          </Card>
        </section>
      </div>
    </>
  );
}
