import { File, FolderOpen, ExternalLink, ArrowDown, ArrowUp } from "lucide-react";
import { t, type TranslationKey } from "../../i18n";
import { cn } from "../../lib/cn";
import { fileSize, remainingTime, speed } from "../../lib/format";
import { api } from "../../lib/tauri";
import type { Transfer, TransferProgress } from "../../lib/tauri";

/**
 * Sohbet akışındaki dosya baloncuğu.
 *
 * Aktarımlar önce yalnızca sohbetin altındaki ayrı bir şeritte görünüyordu;
 * dosya bitince oradan kayboluyor ve konuşmada hiçbir izi kalmıyordu. Artık
 * her aktarımın akışta kendi baloncuğu var: sürerken ilerleme çubuğu,
 * bitince açma ve klasörde gösterme düğmeleri.
 *
 * Durum `transfer` kaydından okunur; ilerleme (varsa) bellekteki anlık
 * olaydan. İkisi ayrıdır çünkü ilerleme saniyede iki kez gelir ve
 * veritabanına o sıklıkta yazılmaz.
 */
export function FileBubble({
  transfer,
  fileName,
  progress,
  outgoing,
}: {
  transfer: Transfer | null;
  fileName: string;
  progress?: TransferProgress;
  outgoing: boolean;
}) {
  const status = transfer?.status ?? "failed";
  const done = status === "done";
  const failed = status === "failed" || status === "cancelled";
  const total = transfer?.fileSize ?? 0;
  const bytesDone = progress?.bytesDone ?? transfer?.bytesDone ?? 0;
  const percent = total > 0 ? Math.min(100, (bytesDone / total) * 100) : 0;
  const remaining = progress ? remainingTime(total - bytesDone, progress.bytesPerSecond) : null;

  return (
    <div className="flex min-w-0 items-center gap-3">
      <span className="relative shrink-0">
        <span className="flex size-10 items-center justify-center rounded-lu-sm bg-layer-alt">
          <File size={20} className="text-fg-secondary" />
        </span>
        <span
          aria-hidden
          className={cn(
            "absolute -right-1 -bottom-1 flex size-4 items-center justify-center rounded-full",
            "bg-layer text-fg-secondary ring-2 ring-[var(--lu-bg-layer)]",
          )}
        >
          {outgoing ? <ArrowUp size={10} /> : <ArrowDown size={10} />}
        </span>
      </span>

      <div className="min-w-0 flex-1">
        <p className="truncate">{fileName}</p>
        <p
          className={cn(
            "text-[length:var(--lu-text-caption)]",
            failed ? "text-danger" : "opacity-80",
          )}
        >
          {done ? (
            fileSize(total)
          ) : failed ? (
            (transfer?.error ??
              t(`files.status.${transfer?.status ?? "failed"}` as TranslationKey))
          ) : (
            <>
              {fileSize(bytesDone)} / {fileSize(total)}
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
            className="mt-1.5 h-1 w-full overflow-hidden rounded-lu-full bg-[color-mix(in_srgb,currentColor_20%,transparent)]"
          >
            <div
              className="h-full rounded-lu-full bg-current transition-[width] duration-[var(--lu-dur-normal)]"
              style={{ width: `${percent}%` }}
            />
          </div>
        ) : null}
      </div>

      {done && transfer ? (
        <div className="flex shrink-0 items-center gap-0.5">
          <button
            type="button"
            aria-label={t("files.open")}
            title={t("files.open")}
            onClick={() => void api.openTransferFile(transfer.transferId)}
            className="rounded-lu-sm p-1.5 opacity-70 hover:bg-[color-mix(in_srgb,currentColor_14%,transparent)] hover:opacity-100"
          >
            <ExternalLink size={16} />
          </button>
          <button
            type="button"
            aria-label={t("files.reveal")}
            title={t("files.reveal")}
            onClick={() => void api.revealTransferFile(transfer.transferId)}
            className="rounded-lu-sm p-1.5 opacity-70 hover:bg-[color-mix(in_srgb,currentColor_14%,transparent)] hover:opacity-100"
          >
            <FolderOpen size={16} />
          </button>
        </div>
      ) : null}
    </div>
  );
}
