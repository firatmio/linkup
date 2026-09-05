import { ArrowDown, ArrowUp, File, FolderOpen, ExternalLink } from "lucide-react";
import { t, type TranslationKey } from "../../i18n";
import { cn } from "../../lib/cn";
import { fileSize, relativeTime, remainingTime, speed } from "../../lib/format";
import { api } from "../../lib/tauri";
import type { Transfer, TransferProgress } from "../../lib/tauri";

/**
 * Bir aktarımın satırı. Hem sürmekte olan aktarımlar hem de tamamlanmış
 * dosyalar için kullanılır: ikisi aynı kayıt, farklı durumdadır.
 */
export function TransferRow({
  transfer,
  progress,
  onSelect,
}: {
  transfer: Transfer;
  progress?: TransferProgress;
  /** Verilirse satır tıklanabilir olur ve dosya bilgi ekranını açar. */
  onSelect?: () => void;
}) {
  const incoming = transfer.direction === "in";
  const done = transfer.status === "done";
  const failed = transfer.status === "failed" || transfer.status === "cancelled";

  // İlerleme olayı veritabanı kaydından tazedir; varsa o kullanılır.
  const bytesDone = progress?.bytesDone ?? transfer.bytesDone;
  const percent =
    transfer.fileSize > 0 ? Math.min(100, (bytesDone / transfer.fileSize) * 100) : 0;

  const remaining = progress
    ? remainingTime(transfer.fileSize - bytesDone, progress.bytesPerSecond)
    : null;

  return (
    <li
      onClick={onSelect}
      // Satır içinde zaten düğmeler var; sarmalayıcıyı `<button>` yapmak
      // iç içe düğme demek olurdu. Klavye erişimi bu yüzden elle veriliyor.
      role={onSelect ? "button" : undefined}
      tabIndex={onSelect ? 0 : undefined}
      onKeyDown={
        onSelect
          ? (event) => {
              if (event.key === "Enter" || event.key === " ") {
                event.preventDefault();
                onSelect();
              }
            }
          : undefined
      }
      className={cn(
        "group flex items-center gap-3 border-b border-divider px-4 py-3 last:border-b-0",
        onSelect &&
          "cursor-pointer transition-colors hover:bg-hover focus-visible:bg-hover focus-visible:outline-none",
      )}
    >
      <span className="relative shrink-0">
        <span className="flex size-9 items-center justify-center rounded-lu-sm bg-layer-alt">
          <File size={18} className="text-fg-secondary" />
        </span>
        <span
          aria-hidden
          className={cn(
            "absolute -right-1 -bottom-1 flex size-4 items-center justify-center rounded-full ring-2 ring-[var(--lu-bg-layer)]",
            incoming ? "bg-accent text-on-accent" : "bg-layer-alt text-fg-secondary",
          )}
        >
          {incoming ? <ArrowDown size={10} /> : <ArrowUp size={10} />}
        </span>
      </span>

      <div className="min-w-0 flex-1">
        <p className="truncate text-[length:var(--lu-text-body)]">{transfer.fileName}</p>

        <p
          className={cn(
            "text-[length:var(--lu-text-caption)]",
            failed ? "text-danger" : "text-fg-secondary",
          )}
        >
          {done ? (
            <>
              {fileSize(transfer.fileSize)} · {relativeTime(transfer.completedAt)}
            </>
          ) : failed ? (
            (transfer.error ?? t(`files.status.${transfer.status}` as TranslationKey))
          ) : (
            <>
              {fileSize(bytesDone)} / {fileSize(transfer.fileSize)}
              {progress && progress.bytesPerSecond > 0
                ? ` · ${speed(progress.bytesPerSecond)}`
                : ""}
              {remaining ? ` · ${t("transfers.remaining", { time: remaining })}` : ""}
            </>
          )}
        </p>

        {!done && !failed ? (
          <div
            role="progressbar"
            aria-valuenow={Math.round(percent)}
            aria-valuemin={0}
            aria-valuemax={100}
            className="mt-1.5 h-1 w-full overflow-hidden rounded-lu-full bg-layer-alt"
          >
            <div
              className="h-full rounded-lu-full bg-accent transition-[width] duration-[var(--lu-dur-normal)]"
              style={{ width: `${percent}%` }}
            />
          </div>
        ) : null}
      </div>

      {done && incoming ? (
        <div className="flex shrink-0 items-center gap-1 opacity-0 transition-opacity group-hover:opacity-100">
          <button
            type="button"
            aria-label={t("files.open")}
            title={t("files.open")}
            onClick={(event) => {
              event.stopPropagation();
              void api.openTransferFile(transfer.transferId);
            }}
            className="rounded-lu-sm p-1.5 text-fg-secondary hover:bg-hover hover:text-fg"
          >
            <ExternalLink size={16} />
          </button>
          <button
            type="button"
            aria-label={t("files.reveal")}
            title={t("files.reveal")}
            onClick={(event) => {
              event.stopPropagation();
              void api.revealTransferFile(transfer.transferId);
            }}
            className="rounded-lu-sm p-1.5 text-fg-secondary hover:bg-hover hover:text-fg"
          >
            <FolderOpen size={16} />
          </button>
        </div>
      ) : null}
    </li>
  );
}
