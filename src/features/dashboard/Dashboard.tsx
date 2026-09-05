import { MonitorSmartphone } from "lucide-react";
import { t } from "../../i18n";
import { PageHeader, EmptyState, SectionTitle } from "../../components/Surface";
import { Callout } from "../../components/Callout";
import { usePairingStore } from "../../stores/pairingStore";
import { DeviceSummaryCard } from "./DeviceSummaryCard";

/**
 * Uygulamanın açılış ekranı (PLAN.md §3.2).
 *
 * Faz 6'da kartlara son mesaj ve aktif transfer özeti eklenecek; şimdilik
 * eşleşmiş cihazlar ve çevrimiçilik durumu gösteriliyor.
 */
export function Dashboard() {
  const trusted = usePairingStore((s) => s.trusted);
  const message = usePairingStore((s) => s.message);
  const messageIsError = usePairingStore((s) => s.messageIsError);
  const dismissMessage = usePairingStore((s) => s.dismissMessage);

  return (
    <>
      <PageHeader title={t("dashboard.title")} />
      <div className="flex-1 space-y-6 overflow-y-auto px-6 pb-6">
        {message ? (
          <button type="button" onClick={dismissMessage} className="block w-full text-left">
            <Callout tone={messageIsError ? "warning" : "info"}>{message}</Callout>
          </button>
        ) : null}

        <section className="space-y-2">
          <SectionTitle>{t("dashboard.devices")}</SectionTitle>
          {trusted.length === 0 ? (
            <EmptyState
              icon={<MonitorSmartphone size={32} strokeWidth={1.5} />}
              title={t("dashboard.empty.title")}
              body={t("dashboard.empty.body")}
            />
          ) : (
            <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
              {trusted.map((device) => (
                <DeviceSummaryCard key={device.id} device={device} />
              ))}
            </div>
          )}
        </section>
      </div>
    </>
  );
}
