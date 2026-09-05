import { Palette, Info, FolderOpen, HardDrive, Network, FlaskConical } from "lucide-react";
import { t } from "../../i18n";
import { Card, PageHeader, SectionTitle, SettingRow } from "../../components/Surface";
import { Button } from "../../components/Button";
import { SegmentedControl } from "../../components/SegmentedControl";
import { useUiStore, type ThemePreference } from "../../stores/uiStore";
import { useAppStore } from "../../stores/appStore";
import { api } from "../../lib/tauri";

const themeOptions: readonly { value: ThemePreference; label: string }[] = [
  { value: "system", label: t("settings.theme.system") },
  { value: "light", label: t("settings.theme.light") },
  { value: "dark", label: t("settings.theme.dark") },
];

export function SettingsPage() {
  const themePreference = useUiStore((s) => s.themePreference);
  const setThemePreference = useUiStore((s) => s.setThemePreference);
  const info = useAppStore((s) => s.info);
  const loading = useAppStore((s) => s.loading);
  const error = useAppStore((s) => s.error);

  /** Yüklenirken "Yükleniyor…", hata varsa hata metni, aksi halde değer. */
  const value = (get: (i: NonNullable<typeof info>) => string): string => {
    if (info) return get(info);
    return loading ? t("common.loading") : (error ?? t("error.unknown"));
  };

  return (
    <>
      <PageHeader title={t("settings.title")} />
      <div className="flex-1 space-y-6 overflow-y-auto px-6 pb-6">
        <section>
          <SectionTitle>{t("settings.section.general")}</SectionTitle>
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
                  onChange={setThemePreference}
                />
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
                <Button onClick={() => void api.openLogDir()}>
                  {t("settings.logs.open")}
                </Button>
              }
            />
          </Card>
        </section>
      </div>
    </>
  );
}
