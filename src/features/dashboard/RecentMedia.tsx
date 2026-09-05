import { useMemo, useState } from "react";
import { t } from "../../i18n";
import { SectionTitle } from "../../components/Surface";
import { useTransferStore } from "../../stores/transferStore";
import { useTransferPreview } from "../chat/useTransferPreview";
import { FileInfoDialog } from "../transfer/FileInfoDialog";
import type { Transfer } from "../../lib/tauri";

/** Şeritte gösterilecek en fazla kayıt. */
const LIMIT = 12;

/**
 * Son gelen medyalar şeridi (PLAN.md §3.2).
 *
 * Hiç görsel yoksa bölüm tamamen gizlenir — boş bir başlık ekranı doldurmaktan
 * başka işe yaramaz.
 */
export function RecentMedia() {
  const incoming = useTransferStore((s) => s.incoming);
  const [selected, setSelected] = useState<Transfer | null>(null);

  const media = useMemo(
    () =>
      incoming
        .filter((item) => item.status === "done" && isImage(item.fileName))
        .slice(0, LIMIT),
    [incoming],
  );

  if (media.length === 0) return null;

  const detail = selected
    ? (incoming.find((item) => item.transferId === selected.transferId) ?? null)
    : null;

  return (
    <section className="space-y-2">
      <SectionTitle>{t("files.recent")}</SectionTitle>
      <ul className="flex gap-3 overflow-x-auto pb-1">
        {media.map((transfer) => (
          <li key={transfer.transferId}>
            <Thumbnail transfer={transfer} onSelect={() => setSelected(transfer)} />
          </li>
        ))}
      </ul>
      <FileInfoDialog transfer={detail} onClose={() => setSelected(null)} />
    </section>
  );
}

function Thumbnail({ transfer, onSelect }: { transfer: Transfer; onSelect: () => void }) {
  const preview = useTransferPreview(transfer.transferId, true, 240);

  return (
    <button
      type="button"
      onClick={onSelect}
      title={transfer.fileName}
      aria-label={transfer.fileName}
      className="size-24 overflow-hidden rounded-lu-lg border border-stroke bg-layer-alt transition-colors hover:border-accent"
    >
      {preview ? (
        <img
          src={`data:${preview.mime};base64,${preview.data}`}
          alt=""
          className="size-full object-cover"
        />
      ) : null}
    </button>
  );
}

const IMAGE_EXTENSIONS = ["png", "jpg", "jpeg", "gif", "webp", "bmp"];

function isImage(fileName: string): boolean {
  const extension = fileName.split(".").pop()?.toLowerCase() ?? "";
  return IMAGE_EXTENSIONS.includes(extension);
}
