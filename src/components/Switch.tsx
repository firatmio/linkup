import { cn } from "../lib/cn";

/**
 * Fluent (WinUI) toggle switch.
 *
 * Kapalıyken içi boş bir daire, açıkken accent dolgulu — Windows 11'deki
 * davranışın aynısı: durum yalnızca renkten değil, tutamağın konumundan ve
 * dolgusundan da anlaşılır (renk körlüğü için önemli).
 */
export function Switch({
  checked,
  onChange,
  label,
  disabled = false,
}: {
  checked: boolean;
  onChange: (checked: boolean) => void;
  label: string;
  disabled?: boolean;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className={cn(
        "relative inline-flex h-5 w-10 shrink-0 items-center rounded-full border transition-colors",
        "duration-[var(--lu-dur-fast)] ease-[var(--lu-ease)]",
        "disabled:pointer-events-none disabled:opacity-50",
        checked
          ? "border-accent bg-accent hover:bg-accent-hover"
          : "border-stroke-strong bg-transparent hover:bg-hover",
      )}
    >
      <span
        aria-hidden
        className={cn(
          "absolute size-3 rounded-full transition-all",
          "duration-[var(--lu-dur-fast)] ease-[var(--lu-ease)]",
          checked ? "left-[1.375rem] bg-on-accent" : "left-1 bg-fg-secondary",
        )}
      />
    </button>
  );
}
