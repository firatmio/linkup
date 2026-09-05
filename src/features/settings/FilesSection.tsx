import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { FolderOpen, ShieldQuestion, Gauge, Layers } from "lucide-react";
import { t } from "../../i18n";
import { Card, SectionTitle, SettingRow } from "../../components/Surface";
import { Button } from "../../components/Button";
import { SegmentedControl } from "../../components/SegmentedControl";
import { NumberField } from "./NumberField";
import { useSettingsStore } from "../../stores/settingsStore";
import { useAppStore } from "../../stores/appStore";

/** Eşik ve hız sınırı bayt tutuluyor; arayüzde MB üzerinden düzenleniyor. */
const MB = 1024 * 1024;

/**
 * Dosya aktarımı ayarları (PLAN.md §3.4).
 *
 * Hepsi backend'de zaten uygulanıyordu ama hiçbirinin arayüzü yoktu; ayarlar
 * veritabanında varsayılan değerleriyle kilitliydi.
 */
export function FilesSection() {
  const settings = useSettingsStore((s) => s.settings);
  const saving = useSettingsStore((s) => s.saving);
  const set = useSettingsStore((s) => s.set);
  const info = useAppStore((s) => s.info);

  const disabled = !settings || saving;
  const policy = settings?.acceptPolicy ?? "always";

  const pickFolder = async () => {
    const chosen = await openDialog({ directory: true, multiple: false });
    if (typeof chosen === "string") await set("downloadDir", chosen);
  };

  return (
    <section className="space-y-2">
      <SectionTitle>{t("settings.section.files")}</SectionTitle>
      <Card>
        <SettingRow
          icon={<FolderOpen size={18} />}
          title={t("settings.downloadDir")}
          // Boş ayar "varsayılan klasör" demek; kullanıcıya boşluk değil,
          // dosyaların gerçekten indiği yer gösterilmeli.
          description={settings?.downloadDir || (info?.downloadsDir ?? t("settings.downloadDir.desc"))}
          control={
            <span className="flex items-center gap-2">
              {settings?.downloadDir ? (
                <Button variant="subtle" disabled={disabled} onClick={() => void set("downloadDir", "")}>
                  {t("settings.downloadDir.reset")}
                </Button>
              ) : null}
              <Button disabled={disabled} onClick={() => void pickFolder()}>
                {t("settings.downloadDir.pick")}
              </Button>
            </span>
          }
        />

        <SettingRow
          icon={<ShieldQuestion size={18} />}
          title={t("settings.acceptPolicy")}
          description={t("settings.acceptPolicy.desc")}
          control={
            <SegmentedControl
              ariaLabel={t("settings.acceptPolicy")}
              value={policy}
              disabled={disabled}
              onChange={(value) => void set("acceptPolicy", value)}
              options={[
                { value: "always", label: t("settings.acceptPolicy.always") },
                { value: "threshold", label: t("settings.acceptPolicy.threshold") },
                { value: "trusted", label: t("settings.acceptPolicy.trusted") },
              ]}
            />
          }
        />

        {/* Eşik yalnızca "büyükse sor" seçiliyken anlamlı; her zaman göstermek
            kullanılmayan bir kutuya bakmak olurdu. */}
        {policy === "threshold" ? (
          <SettingRow
            icon={<ShieldQuestion size={18} />}
            title={t("settings.acceptThreshold")}
            description={t("settings.acceptThreshold.desc")}
            control={
              <NumberField
                ariaLabel={t("settings.acceptThreshold")}
                value={Math.round((settings?.acceptSizeThreshold ?? 0) / MB)}
                suffix="MB"
                max={1024 * 1024}
                disabled={disabled}
                onCommit={(mb) => void set("acceptSizeThreshold", String(mb * MB))}
              />
            }
          />
        ) : null}

        <SettingRow
          icon={<Gauge size={18} />}
          title={t("settings.speedLimit")}
          description={t("settings.speedLimit.desc")}
          control={
            <NumberField
              ariaLabel={t("settings.speedLimit")}
              value={Math.round((settings?.speedLimitBytes ?? 0) / MB)}
              suffix="MB/s"
              max={10_000}
              disabled={disabled}
              onCommit={(mb) => void set("speedLimitBytes", String(mb * MB))}
            />
          }
        />

        <SettingRow
          icon={<Layers size={18} />}
          title={t("settings.concurrency")}
          description={`${t("settings.concurrency.desc")} ${t("settings.restartRequired")}`}
          control={
            <NumberField
              ariaLabel={t("settings.concurrency")}
              value={settings?.maxConcurrentTransfers ?? 3}
              min={1}
              max={16}
              disabled={disabled}
              onCommit={(value) => void set("maxConcurrentTransfers", String(value))}
            />
          }
        />
      </Card>
    </section>
  );
}
