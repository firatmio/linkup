import { FolderDown } from "lucide-react";
import { t } from "../../i18n";
import { PageHeader, EmptyState, Card, SectionTitle } from "../../components/Surface";
import { Callout } from "../../components/Callout";
import { useTransferStore } from "../../stores/transferStore";
import { TransferRow } from "./TransferRow";

/** Alınan dosyaların tam geçmişi (PLAN.md §3.2). */
export function IncomingFilesPage() {
  const incoming = useTransferStore((s) => s.incoming);
  const active = useTransferStore((s) => s.active);
  const progress = useTransferStore((s) => s.progress);
  const error = useTransferStore((s) => s.error);

  return (
    <>
      <PageHeader
        title={t("files.title")}
        action={
          incoming.length > 0 ? (
            <span className="text-[length:var(--lu-text-caption)] text-fg-secondary">
              {t("files.count", { count: incoming.length })}
            </span>
          ) : undefined
        }
      />

      <div className="flex-1 space-y-6 overflow-y-auto px-6 pb-6">
        {error ? <Callout tone="warning">{error}</Callout> : null}

        {active.length > 0 ? (
          <section className="space-y-2">
            <SectionTitle>{t("transfers.title")}</SectionTitle>
            <Card>
              <ul>
                {active.map((transfer) => (
                  <TransferRow
                    key={transfer.transferId}
                    transfer={transfer}
                    progress={progress[transfer.transferId]}
                  />
                ))}
              </ul>
            </Card>
          </section>
        ) : null}

        {incoming.length === 0 ? (
          <EmptyState
            icon={<FolderDown size={32} strokeWidth={1.5} />}
            title={t("files.empty.title")}
            body={t("files.empty.body")}
          />
        ) : (
          <Card>
            <ul>
              {incoming.map((transfer) => (
                <TransferRow
                  key={transfer.transferId}
                  transfer={transfer}
                  progress={progress[transfer.transferId]}
                />
              ))}
            </ul>
          </Card>
        )}
      </div>
    </>
  );
}
