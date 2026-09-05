import { MonitorSmartphone } from "lucide-react";
import { t } from "../../i18n";
import { PageHeader, EmptyState, SectionTitle } from "../../components/Surface";
import { Button } from "../../components/Button";

/**
 * Uygulamanın açılış ekranı (PLAN.md §3.2).
 * Faz 6'da cihaz özet kartları, aktif transferler ve son medyalar gerçek
 * verilerle dolacak; şimdilik iskelet ve boş durum.
 */
export function Dashboard() {
  return (
    <>
      <PageHeader title={t("dashboard.title")} />
      <div className="flex-1 space-y-6 overflow-y-auto px-6 pb-6">
        <section>
          <SectionTitle>{t("dashboard.devices")}</SectionTitle>
          <EmptyState
            icon={<MonitorSmartphone size={32} strokeWidth={1.5} />}
            title={t("dashboard.empty.title")}
            body={t("dashboard.empty.body")}
            action={
              <Button variant="accent" disabled>
                {t("dashboard.empty.action")}
              </Button>
            }
          />
        </section>
      </div>
    </>
  );
}
