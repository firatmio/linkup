import { cn } from "../lib/cn";

interface Option<T extends string> {
  value: T;
  label: string;
}

/** Fluent segmented control — Ayarlar'daki küçük seçimler için. */
export function SegmentedControl<T extends string>({
  value,
  options,
  onChange,
  ariaLabel,
}: {
  value: T;
  options: readonly Option<T>[];
  onChange: (value: T) => void;
  ariaLabel: string;
}) {
  return (
    <div
      role="radiogroup"
      aria-label={ariaLabel}
      className="inline-flex rounded-lu-sm border border-stroke-strong bg-layer p-0.5"
    >
      {options.map((option) => {
        const selected = option.value === value;
        return (
          <button
            key={option.value}
            type="button"
            role="radio"
            aria-checked={selected}
            onClick={() => onChange(option.value)}
            className={cn(
              "h-7 rounded-[3px] px-3 text-[length:var(--lu-text-body)]",
              "transition-colors duration-[var(--lu-dur-fast)] ease-[var(--lu-ease)]",
              selected
                ? "bg-accent text-on-accent"
                : "text-fg-secondary hover:bg-hover active:bg-press",
            )}
          >
            {option.label}
          </button>
        );
      })}
    </div>
  );
}
