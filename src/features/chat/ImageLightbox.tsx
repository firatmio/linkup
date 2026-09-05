import { useEffect } from "react";
import { createPortal } from "react-dom";
import { X, FolderOpen, ExternalLink } from "lucide-react";
import { t } from "../../i18n";
import { api } from "../../lib/tauri";
import { useTransferPreview } from "./useTransferPreview";

/** Büyük önizlemenin en uzun kenarı. */
const LARGE_EDGE = 1600;

/**
 * Görsele tıklayınca açılan tam ekran önizleme (PLAN.md §3.3).
 *
 * Küçük resim varken bile büyüğü ayrıca isteniyor: sohbetteki 320 piksellik
 * kopyayı ekrana yaymak bulanık bir sonuç verir. Büyük kopya gelene kadar
 * küçüğü gösteriliyor, böylece açılış anında boş bir kutu görünmüyor.
 */
export function ImageLightbox({
  transferId,
  fileName,
  fallback,
  onClose,
}: {
  transferId: string;
  fileName: string;
  fallback: string | null;
  onClose: () => void;
}) {
  const large = useTransferPreview(transferId, true, LARGE_EDGE);
  const source = large ? `data:${large.mime};base64,${large.data}` : fallback;

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  return createPortal(
    <div
      role="dialog"
      aria-modal="true"
      aria-label={fileName}
      onClick={onClose}
      className="fixed inset-0 z-50 flex flex-col bg-[color-mix(in_srgb,black_78%,transparent)] backdrop-blur-sm"
    >
      <div
        className="flex h-[var(--lu-header-h)] shrink-0 items-center gap-2 px-3 text-white"
        onClick={(event) => event.stopPropagation()}
      >
        <span className="min-w-0 flex-1 truncate text-[length:var(--lu-text-body)]">
          {fileName}
        </span>
        <IconButton
          label={t("files.open")}
          onClick={() => void api.openTransferFile(transferId)}
          icon={<ExternalLink size={18} />}
        />
        <IconButton
          label={t("files.reveal")}
          onClick={() => void api.revealTransferFile(transferId)}
          icon={<FolderOpen size={18} />}
        />
        <IconButton label={t("common.close")} onClick={onClose} icon={<X size={18} />} />
      </div>

      <div className="flex min-h-0 flex-1 items-center justify-center p-6">
        {source ? (
          <img
            src={source}
            alt={fileName}
            // Tıklama kapatmayı tetiklemesin: kullanıcı görselin üstünde
            // gezinirken yanlışlıkla kapatmak can sıkıcı olur.
            onClick={(event) => event.stopPropagation()}
            className="max-h-full max-w-full rounded-lu-lg object-contain shadow-2xl"
          />
        ) : null}
      </div>
    </div>,
    document.body,
  );
}

function IconButton({
  label,
  icon,
  onClick,
}: {
  label: string;
  icon: React.ReactNode;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      onClick={onClick}
      className="rounded-lu-sm p-2 text-white/80 transition-colors hover:bg-white/15 hover:text-white"
    >
      {icon}
    </button>
  );
}
