import { useEffect, useState } from "react";
import { cn } from "../../lib/cn";

/**
 * Sayı girişi olan ayarlar için küçük alan.
 *
 * Yazarken DEĞİL, odak kaybında veya Enter'da kaydeder: her tuş vuruşunda
 * yazmak, "1024" yazarken önce 1, sonra 10, sonra 102 değerlerini kalıcı
 * hâle getirirdi.
 *
 * Geçersiz giriş kaydedilmez ve alan son geçerli değere döner: kullanıcıya
 * kabul edilmiş gibi görünen bir çöp değer bırakmak yanıltıcı olur.
 */
export function NumberField({
  value,
  suffix,
  min = 0,
  max = 65535,
  disabled,
  ariaLabel,
  onCommit,
}: {
  value: number;
  suffix?: string;
  min?: number;
  max?: number;
  disabled?: boolean;
  ariaLabel: string;
  onCommit: (value: number) => void;
}) {
  const [draft, setDraft] = useState(String(value));

  // Dışarıdan değişirse (kaydetme sonrası tazeleme) alan da güncellenmeli.
  useEffect(() => setDraft(String(value)), [value]);

  const commit = () => {
    const parsed = Number(draft.trim());
    if (!Number.isFinite(parsed) || parsed < min || parsed > max) {
      setDraft(String(value));
      return;
    }
    const rounded = Math.round(parsed);
    if (rounded !== value) onCommit(rounded);
    setDraft(String(rounded));
  };

  return (
    <span className="flex items-center gap-2">
      <input
        type="text"
        inputMode="numeric"
        value={draft}
        disabled={disabled}
        aria-label={ariaLabel}
        onChange={(event) => setDraft(event.target.value)}
        onBlur={commit}
        onKeyDown={(event) => {
          if (event.key === "Enter") event.currentTarget.blur();
        }}
        className={cn(
          "lu-selectable h-[var(--lu-control-h)] w-24 rounded-lu-sm px-3 text-right",
          "border border-stroke-strong bg-layer-alt text-fg",
          "shadow-[inset_0_-1px_0_var(--lu-stroke-strong)]",
          "focus:border-accent focus:shadow-[inset_0_-2px_0_var(--lu-accent)] focus:outline-none",
          "disabled:pointer-events-none disabled:text-fg-disabled",
        )}
      />
      {suffix ? (
        <span className="text-[length:var(--lu-text-caption)] text-fg-secondary">{suffix}</span>
      ) : null}
    </span>
  );
}
