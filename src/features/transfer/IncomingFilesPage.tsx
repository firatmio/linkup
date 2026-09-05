import { useEffect, useMemo, useState } from "react";
import { FolderDown, Trash2, Search } from "lucide-react";
import { t } from "../../i18n";
import { PageHeader, EmptyState, Card, SectionTitle } from "../../components/Surface";
import { Callout } from "../../components/Callout";
import { Button } from "../../components/Button";
import { SegmentedControl } from "../../components/SegmentedControl";
import { api, type Transfer } from "../../lib/tauri";
import { useTransferStore } from "../../stores/transferStore";
import { TransferRow } from "./TransferRow";
import { FileInfoDialog } from "./FileInfoDialog";

/** Dosya türü filtresi. Kategoriler uzantıdan çıkarılır. */
type Kind = "all" | "image" | "document" | "other";

const IMAGE_EXTENSIONS = ["png", "jpg", "jpeg", "gif", "webp", "bmp", "svg", "heic", "avif"];
const DOCUMENT_EXTENSIONS = [
  "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx",
  "txt", "md", "csv", "json", "xml", "rtf", "odt",
];

function kindOf(fileName: string): Exclude<Kind, "all"> {
  const extension = fileName.split(".").pop()?.toLowerCase() ?? "";
  if (IMAGE_EXTENSIONS.includes(extension)) return "image";
  if (DOCUMENT_EXTENSIONS.includes(extension)) return "document";
  return "other";
}

/**
 * Alınan dosyaların tam geçmişi (PLAN.md §3.2).
 *
 * Arama backend'de yapılıyor: Türkçe'nin noktalı/noktasız i ayrımı orada tek
 * bir yerde çözüldü (bkz. `db::transfers::list_incoming`). Tür filtresi ise
 * burada, çünkü kategoriler tamamen bir sunum kararı — veritabanının dosya
 * uzantılarını sınıflandırması için bir sebep yok.
 */
export function IncomingFilesPage() {
  const incoming = useTransferStore((s) => s.incoming);
  const active = useTransferStore((s) => s.active);
  const progress = useTransferStore((s) => s.progress);
  const error = useTransferStore((s) => s.error);
  const reload = useTransferStore((s) => s.load);
  const search = useTransferStore((s) => s.search);
  const setQuery = useTransferStore((s) => s.setQuery);

  const [kind, setKind] = useState<Kind>("all");
  const [selected, setSelected] = useState<Transfer | null>(null);

  // Yazarken her tuşta sorgu atmamak için kısa bir bekleme.
  const [draft, setDraft] = useState(search);
  useEffect(() => {
    if (draft === search) return;
    const timer = setTimeout(() => void setQuery(draft), 200);
    return () => clearTimeout(timer);
  }, [draft, search, setQuery]);

  const visible = useMemo(
    () => (kind === "all" ? incoming : incoming.filter((item) => kindOf(item.fileName) === kind)),
    [incoming, kind],
  );

  // Seçili kayıt silinmiş olabilir; kopya yerine güncel hâli gösterilir.
  const detail = selected
    ? (incoming.find((item) => item.transferId === selected.transferId) ?? null)
    : null;

  const clearFinished = async () => {
    await api.clearFinishedTransfers();
    await reload();
  };

  const filtering = search.trim().length > 0 || kind !== "all";

  return (
    <>
      <PageHeader
        title={t("files.title")}
        action={
          <span className="flex items-center gap-3">
            <span className="text-[length:var(--lu-text-caption)] text-fg-secondary">
              {t("files.count", { count: visible.length })}
            </span>
            <Button
              variant="subtle"
              icon={<Trash2 size={16} />}
              onClick={() => void clearFinished()}
            >
              {t("transfers.clear")}
            </Button>
          </span>
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

        <div className="flex flex-wrap items-center gap-3">
          <label className="relative min-w-56 flex-1">
            <Search
              size={16}
              aria-hidden
              className="pointer-events-none absolute top-1/2 left-3 -translate-y-1/2 text-fg-tertiary"
            />
            <input
              type="search"
              value={draft}
              onChange={(event) => setDraft(event.target.value)}
              aria-label={t("files.search")}
              placeholder={t("files.search.placeholder")}
              className="lu-selectable h-[var(--lu-control-h)] w-full rounded-lu-sm border border-stroke-strong bg-layer-alt pr-3 pl-9 text-fg shadow-[inset_0_-1px_0_var(--lu-stroke-strong)] placeholder:text-fg-tertiary focus:border-accent focus:shadow-[inset_0_-2px_0_var(--lu-accent)] focus:outline-none"
            />
          </label>

          <SegmentedControl
            value={kind}
            onChange={setKind}
            ariaLabel={t("files.title")}
            options={[
              { value: "all", label: t("files.filter.all") },
              { value: "image", label: t("files.filter.image") },
              { value: "document", label: t("files.filter.document") },
              { value: "other", label: t("files.filter.other") },
            ]}
          />
        </div>

        {visible.length === 0 ? (
          <EmptyState
            icon={<FolderDown size={32} strokeWidth={1.5} />}
            title={filtering ? t("files.noResults.title") : t("files.empty.title")}
            body={filtering ? t("files.noResults.body") : t("files.empty.body")}
          />
        ) : (
          <Card>
            <ul>
              {visible.map((transfer) => (
                <TransferRow
                  key={transfer.transferId}
                  transfer={transfer}
                  progress={progress[transfer.transferId]}
                  onSelect={() => setSelected(transfer)}
                />
              ))}
            </ul>
          </Card>
        )}
      </div>

      <FileInfoDialog transfer={detail} onClose={() => setSelected(null)} />
    </>
  );
}
