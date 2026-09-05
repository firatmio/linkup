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
 * Hızlandırıcıyı okunabilir tuş adlarına böler.
 *
 * `CmdOrCtrl` platforma göre çözülür: kullanıcıya Tauri'nin iç gösterimini
 * değil, klavyesinde yazan tuşu göstermek gerekir.
 */
function keyCaps(accelerator: string): string[] {
  const isMac = navigator.platform.toLowerCase().includes("mac");
  return accelerator.split("+").map((part) => {
    switch (part) {
      case "CmdOrCtrl":
      case "CommandOrControl":
        return isMac ? "⌘" : "Ctrl";
      case "Cmd":
      case "Command":
        return "⌘";
      case "Control":
        return "Ctrl";
      case "Alt":
        return isMac ? "⌥" : "Alt";
      case "Shift":
        return isMac ? "⇧" : "Shift";
      default:
        return part;
    }
  });
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
          "flex h-[var(--lu-control-h)] min-w-44 items-center justify-center gap-1 rounded-lu-sm border px-3",
          "text-[length:var(--lu-text-caption)]",
          "disabled:pointer-events-none disabled:text-fg-disabled",
          recording
            ? "border-accent bg-layer-alt text-accent"
            : "border-stroke-strong bg-layer-alt text-fg hover:border-accent",
        )}
      >
        {recording ? (
          t("settings.shortcut.recording")
        ) : value ? (
          // Tuş başına bir kapak: "CmdOrCtrl+Shift+L" tek parça bir dize
          // olarak okunmuyordu, klavyede aranacak şey üç ayrı tuş.
          keyCaps(value).map((cap, index) => (
            <span key={`${cap}-${index}`} className="flex items-center gap-1">
              {index > 0 ? <span className="text-fg-tertiary">+</span> : null}
              <kbd className="rounded-[3px] border border-stroke-strong bg-layer px-1.5 py-0.5 font-sans text-[length:var(--lu-text-caption)] leading-none shadow-[inset_0_-1px_0_var(--lu-stroke-strong)]">
                {cap}
              </kbd>
            </span>
          ))
        ) : (
          <span className="text-fg-tertiary">{t("settings.shortcut.empty")}</span>
        )}
      </button>
      {value ? (
        <Button variant="subtle" disabled={disabled} onClick={() => onChange("")}>
          {t("settings.shortcut.clear")}
        </Button>
      ) : null}
    </span>
  );
}
