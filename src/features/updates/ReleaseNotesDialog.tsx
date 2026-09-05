import { useEffect, useState } from "react";
import { Sparkles } from "lucide-react";
import { t } from "../../i18n";
import { Dialog } from "../../components/Dialog";
import { Button } from "../../components/Button";
import { api } from "../../lib/tauri";
import { useSettingsStore } from "../../stores/settingsStore";
import { useAppStore } from "../../stores/appStore";

interface StoredNotes {
  version: string;
  notes: string;
}

function parse(raw: string): StoredNotes | null {
  if (!raw.trim()) return null;
  try {
    const parsed: unknown = JSON.parse(raw);
    if (
      typeof parsed === "object" &&
      parsed !== null &&
      typeof (parsed as StoredNotes).version === "string"
    ) {
      return parsed as StoredNotes;
    }
  } catch {
    // Bozuk kayıt: gösterilecek bir şey yok, kayıt aşağıda temizlenir.
  }
  return null;
}

/**
 * Güncellemeden sonraki ilk açılışta çıkan "yenilikler" penceresi.
 *
 * Notlar güncelleme kurulmadan ÖNCE yazılıyor; burada yalnızca kayıtlı
 * sürümün çalışan sürümle aynı olup olmadığına bakılıyor. Aynıysa güncelleme
 * gerçekten uygulanmış demektir ve notlar gösterilir. Farklıysa (kurulum
 * yarıda kaldı, kullanıcı eski sürüme döndü) kayıt sessizce temizlenir —
 * yüklemediği bir sürümün yeniliklerini göstermek yanıltıcı olurdu.
 */
export function ReleaseNotesDialog() {
  const settings = useSettingsStore((s) => s.settings);
  const info = useAppStore((s) => s.info);
  const [dismissed, setDismissed] = useState(false);

  const stored = settings ? parse(settings.pendingReleaseNotes) : null;
  const matches = stored && info ? stored.version === info.version : false;

  // Sürüm tutmuyorsa kayıt hemen temizlenir; pencere hiç açılmaz.
  useEffect(() => {
    if (stored && info && !matches) void clear();
  }, [stored, info, matches]);

  if (!stored || !matches || dismissed) return null;

  const close = () => {
    setDismissed(true);
    void clear();
  };

  return (
    <Dialog
      open
      title={t("update.notes.title", { version: stored.version })}
      onClose={close}
      footer={<Button variant="accent" onClick={close}>{t("common.close")}</Button>}
    >
      <p className="flex items-center gap-2 text-[length:var(--lu-text-caption)] text-fg-secondary">
        <Sparkles size={16} />
        {t("update.notes.subtitle")}
      </p>
      {stored.notes.trim() ? (
        <div className="lu-selectable max-h-72 overflow-y-auto rounded-lu-sm border border-stroke bg-layer-alt px-3 py-2.5 text-[length:var(--lu-text-body)] whitespace-pre-wrap">
          {stored.notes}
        </div>
      ) : (
        <p className="text-[length:var(--lu-text-body)] text-fg-secondary">
          {t("update.notes.empty")}
        </p>
      )}
    </Dialog>
  );
}

/** Kaydı temizler; aynı notlar bir daha gösterilmez. */
async function clear(): Promise<void> {
  try {
    await api.setSetting("pendingReleaseNotes", "");
    await useSettingsStore.getState().load();
  } catch {
    // Temizlenemezse pencere bir sonraki açılışta tekrar çıkar; can sıkıcı
    // ama veri kaybı değil.
  }
}
