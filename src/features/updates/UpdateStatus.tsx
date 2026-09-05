import { RefreshCw, ArrowUpCircle } from "lucide-react";
import { t } from "../../i18n";
import { useUpdateStore } from "../../stores/updateStore";

/**
 * Kenar çubuğundaki güncelleme satırı (kullanıcı isteği).
 *
 * Yalnızca söyleyecek bir şey varken görünür: kontrol sürerken, indirme
 * sürerken ve güncelleme hazırken. "Güncelsiniz" demek için kalıcı bir satır
 * ayırmak, hiçbir şeyin olmadığını duyurmak olurdu.
 */
export function UpdateStatus() {
  const phase = useUpdateStore((s) => s.phase);
  const progress = useUpdateStore((s) => s.progress);
  const version = useUpdateStore((s) => s.version);
  const install = useUpdateStore((s) => s.installAndRestart);

  if (phase === "idle" || phase === "upToDate" || phase === "error") return null;

  if (phase === "ready") {
    return (
      <button
        type="button"
        onClick={() => void install()}
        title={t("update.ready.hint", { version: version ?? "" })}
        className="mx-2 mb-1 flex items-center gap-2 rounded-lu-sm border border-accent bg-accent-subtle px-2.5 py-2 text-left transition-colors hover:bg-hover"
      >
        <ArrowUpCircle size={16} className="shrink-0 text-accent" />
        <span className="min-w-0 flex-1">
          <span className="block truncate text-[length:var(--lu-text-caption)] font-semibold text-accent">
            {t("update.ready")}
          </span>
          <span className="block truncate text-[length:var(--lu-text-caption)] text-fg-secondary">
            {t("update.ready.action")}
          </span>
        </span>
      </button>
    );
  }

  return (
    <p className="mx-2 mb-1 flex items-center gap-2 px-2.5 py-2 text-[length:var(--lu-text-caption)] text-fg-secondary">
      <RefreshCw size={14} className="shrink-0 animate-spin" />
      <span className="min-w-0 truncate">
        {phase === "checking"
          ? t("update.checking")
          : progress === null
            ? t("update.downloading")
            : t("update.downloading.progress", { percent: progress })}
      </span>
    </p>
  );
}
