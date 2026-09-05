import type { InputHTMLAttributes } from "react";
import { cn } from "../lib/cn";

interface TextFieldProps extends InputHTMLAttributes<HTMLInputElement> {
  label: string;
  hint?: string;
}

/**
 * Fluent TextBox yorumu: alt kenarı vurgulu çerçeve, odakta accent renginde
 * kalınlaşır.
 */
export function TextField({ label, hint, className, id, ...props }: TextFieldProps) {
  const inputId = id ?? `field-${label.replace(/\s+/g, "-").toLowerCase()}`;
  return (
    <div className="space-y-1.5">
      <label htmlFor={inputId} className="block text-[length:var(--lu-text-body)]">
        {label}
      </label>
      <input
        id={inputId}
        {...props}
        className={cn(
          "lu-selectable h-[var(--lu-control-h)] w-full rounded-lu-sm px-3",
          "border border-stroke-strong bg-layer-alt text-fg",
          "shadow-[inset_0_-1px_0_var(--lu-stroke-strong)]",
          "placeholder:text-fg-tertiary",
          "focus:border-accent focus:shadow-[inset_0_-2px_0_var(--lu-accent)] focus:outline-none",
          "disabled:pointer-events-none disabled:text-fg-disabled",
          className,
        )}
      />
      {hint ? (
        <p className="text-[length:var(--lu-text-caption)] text-fg-secondary">{hint}</p>
      ) : null}
    </div>
  );
}
