import { MonitorSmartphone } from "lucide-react";
import { t } from "../../i18n";
import { PageHeader, EmptyState, SectionTitle } from "../../components/Surface";
import { Callout } from "../../components/Callout";
import { usePairingStore } from "../../stores/pairingStore";
import { useDeviceStore } from "../../stores/deviceStore";
import { DeviceSummaryCard } from "./DeviceSummaryCard";
import { RecentMedia } from "./RecentMedia";

/**
 * Uygulamanın açılış ekranı (PLAN.md §3.2).
 *
 * Şeritler yalnızca gösterecek verileri varken görünür: boş bir başlık,
 * ekranı doldurmaktan başka bir işe yaramaz.
 */
export function Dashboard() {
  const trusted = usePairingStore((s) => s.trusted);
  const message = usePairingStore((s) => s.message);
  const messageIsError = usePairingStore((s) => s.messageIsError);
  const dismissMessage = usePairingStore((s) => s.dismissMessage);
  const discovered = useDeviceStore((s) => s.discovered);

  const online = trusted.filter((device) => device.online).length;
  const unread = trusted.reduce((total, device) => total + device.unread, 0);

  // Henüz eşleşilmemiş, ağda görünen cihazlar.
  const pairable = discovered.filter(
    (device) => !trusted.some((known) => known.id === device.id),
  ).length;

  return (
    <>
      <PageHeader
        title={t("dashboard.title")}
        action={
          trusted.length > 0 ? (
            <span className="flex items-center gap-3 text-[length:var(--lu-text-caption)] text-fg-secondary">
              <span>{t("dashboard.summary.online", { online, total: trusted.length })}</span>
              {unread > 0 ? (
                <span className="text-fg">{t("dashboard.summary.unread", { count: unread })}</span>
              ) : null}
            </span>
          ) : undefined
        }
      />

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
              body={
                // Ağda eşleşilebilecek cihaz varsa kullanıcıyı oraya yönlendir;
                // genel bir "cihaz ekleyin" metni bu durumda işe yaramaz.
                pairable > 0
                  ? t("dashboard.empty.discovered", { count: pairable })
                  : t("dashboard.empty.body")
              }
            />
          ) : (
            <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
              {trusted.map((device) => (
                <DeviceSummaryCard key={device.id} device={device} />
              ))}
            </div>
          )}
        </section>

        <RecentMedia />
      </div>
    </>
  );
}
