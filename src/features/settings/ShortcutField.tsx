import { useState } from "react";
import { t } from "../../i18n";
import { cn } from "../../lib/cn";
import { Button } from "../../components/Button";

/** Tuş adını Tauri'nin hızlandırıcı sözdizimine çevirir. */
function keyName(event: KeyboardEvent | React.KeyboardEvent): string | null {
  const key = event.key;

  // Yalnızca değiştirici tuşa basılmışsa henüz bir kombinasyon yok.
  if (["Control", "Shift", "Alt", "Meta"].includes(key)) return null;

  if (key === " ") return "Space";
  if (key.length === 1) return key.toUpperCase();

  // F1..F24, oklar, Enter gibi adlandırılmış tuşlar zaten uygun biçimde.
  return key;
}

/**
 * Kısayol yakalama alanı.
 *
 * Kombinasyon serbest metinle yazılmıyor: kullanıcının "Ctrl + Shift + L"
 * ile "CmdOrCtrl+Shift+L" arasındaki farkı bilmesi gerekmemeli. Kutuya
 * tıklanır ve tuşlanır; biçimi bu bileşen üretir.
 *
 * En az bir değiştirici zorunlu: değiştiricisiz bir global kısayol, o tuşu
 * her uygulamada ele geçirir.
 */
export function ShortcutField({
  value,
  disabled,
  onChange,
}: {
  value: string;
  disabled?: boolean;
  onChange: (accelerator: string) => void;
}) {
  const [recording, setRecording] = useState(false);

  const capture = (event: React.KeyboardEvent<HTMLButtonElement>) => {
    event.preventDefault();

    if (event.key === "Escape") {
      setRecording(false);
      return;
    }

    const key = keyName(event);
    if (!key) return;

    const parts: string[] = [];
    if (event.ctrlKey || event.metaKey) parts.push("CmdOrCtrl");
    if (event.shiftKey) parts.push("Shift");
    if (event.altKey) parts.push("Alt");
    if (parts.length === 0) return;

    parts.push(key);
    setRecording(false);
    onChange(parts.join("+"));
  };

  return (
    <span className="flex items-center gap-2">
      <button
        type="button"
        disabled={disabled}
        onClick={() => setRecording(true)}
        onBlur={() => setRecording(false)}
        onKeyDown={recording ? capture : undefined}
        className={cn(
          "h-[var(--lu-control-h)] min-w-40 rounded-lu-sm border px-3 font-mono",
          "text-[length:var(--lu-text-caption)]",
          "disabled:pointer-events-none disabled:text-fg-disabled",
          recording
            ? "border-accent bg-layer-alt text-accent"
            : "border-stroke-strong bg-layer-alt text-fg",
        )}
      >
        {recording
          ? t("settings.shortcut.recording")
          : (value || t("settings.shortcut.empty"))}
      </button>
      {value ? (
        <Button variant="subtle" disabled={disabled} onClick={() => onChange("")}>
          {t("settings.shortcut.clear")}
        </Button>
      ) : null}
    </span>
  );
}
