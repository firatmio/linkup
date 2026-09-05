import { useState } from "react";
import { Trash2, FolderOpen, ExternalLink } from "lucide-react";
import { t } from "../../i18n";
import { Dialog } from "../../components/Dialog";
import { Button } from "../../components/Button";
import { Callout } from "../../components/Callout";
import { duration, fileSize, relativeTime } from "../../lib/format";
import { api, type Transfer } from "../../lib/tauri";
import { usePairingStore } from "../../stores/pairingStore";
import { useTransferStore } from "../../stores/transferStore";
import { useTransferPreview, forgetPreview } from "../chat/useTransferPreview";

/**
 * Dosya bilgi ekranı (PLAN.md §3.3).
 *
 * "Tekrar indir" bilerek yok: teklifi yeniden gönderen bir mekanizma henüz
 * yok (Faz 11), dolayısıyla düğme koymak çalışmayan bir söz vermek olurdu.
 */
export function FileInfoDialog({
  transfer,
  onClose,
}: {
  transfer: Transfer | null;
  onClose: () => void;
}) {
  const devices = usePairingStore((s) => s.trusted);
  const reload = useTransferStore((s) => s.load);
  const [confirming, setConfirming] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const preview = useTransferPreview(
    transfer?.transferId ?? null,
    transfer?.status === "done",
    480,
  );

  if (!transfer) return null;

  const sender =
    devices.find((device) => device.id === transfer.deviceId)?.name ??
    t("files.info.unknownDevice");
  const took = duration(transfer.startedAt, transfer.completedAt);

  const remove = async () => {
    try {
      await api.deleteTransferFile(transfer.transferId);
      forgetPreview(transfer.transferId);
      await reload();
      onClose();
    } catch {
      // Dosya taşınmış veya kilitli olabilir; kayıt yerinde bırakılır.
      setError(t("error.unknown"));
      setConfirming(false);
    }
  };

  return (
    <Dialog
      open
      title={confirming ? t("files.delete.confirm.title") : t("files.info.title")}
      onClose={onClose}
      footer={
        confirming ? (
          <>
            <Button variant="subtle" onClick={() => setConfirming(false)}>
              {t("common.cancel")}
            </Button>
            <Button variant="danger" icon={<Trash2 size={16} />} onClick={() => void remove()}>
              {t("files.delete")}
            </Button>
          </>
        ) : (
          <Button onClick={onClose}>{t("common.close")}</Button>
        )
      }
    >
      {confirming ? (
        <p className="text-[length:var(--lu-text-body)]">
          {t("files.delete.confirm.body", { name: transfer.fileName })}
        </p>
      ) : (
        <>
          {preview ? (
            <img
              src={`data:${preview.mime};base64,${preview.data}`}
              alt={transfer.fileName}
              className="max-h-56 w-full rounded-lu-sm object-contain"
            />
          ) : null}

          <p className="lu-selectable text-[length:var(--lu-text-body)] font-semibold break-all">
            {transfer.fileName}
          </p>

          <dl className="space-y-2">
            <Row label={t("files.info.sender")} value={sender} />
            <Row label={t("files.info.size")} value={fileSize(transfer.fileSize)} />
            <Row label={t("files.info.received")} value={relativeTime(transfer.completedAt)} />
            {took ? <Row label={t("files.info.duration")} value={took} /> : null}
          </dl>

          {transfer.savePath ? (
            <div className="space-y-1">
              <p className="text-[length:var(--lu-text-caption)] text-fg-secondary">
                {t("files.info.path")}
              </p>
              <code className="lu-selectable block rounded-lu-sm border border-stroke bg-layer-alt px-2.5 py-2 font-mono text-[length:var(--lu-text-caption)] break-all">
                {transfer.savePath}
              </code>
            </div>
          ) : null}

          {error ? <Callout tone="warning">{error}</Callout> : null}

          <div className="flex flex-wrap gap-2 pt-1">
            <Button
              variant="subtle"
              icon={<ExternalLink size={16} />}
              onClick={() => void api.openTransferFile(transfer.transferId)}
            >
              {t("files.open")}
            </Button>
            <Button
              variant="subtle"
              icon={<FolderOpen size={16} />}
              onClick={() => void api.revealTransferFile(transfer.transferId)}
            >
              {t("files.reveal")}
            </Button>
            <Button
              variant="subtle"
              icon={<Trash2 size={16} />}
              onClick={() => setConfirming(true)}
            >
              {t("files.delete")}
            </Button>
          </div>
        </>
      )}
    </Dialog>
  );
}

function Row({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-baseline justify-between gap-3">
      <dt className="text-[length:var(--lu-text-caption)] text-fg-secondary">{label}</dt>
      <dd className="lu-selectable truncate text-[length:var(--lu-text-body)]">{value}</dd>
    </div>
  );
}
