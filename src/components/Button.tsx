import type { ButtonHTMLAttributes, ReactNode } from "react";
import { cn } from "../lib/cn";

type Variant = "standard" | "accent" | "subtle";

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  icon?: ReactNode;
}

/**
 * Fluent (WinUI 3) buton yorumu: 32px yükseklik, 4px yarıçap, alt kenarı
 * bir tık daha koyu kenarlık, basıldığında hafif sönme.
 */
const variants: Record<Variant, string> = {
  standard:
    "bg-layer text-fg border border-stroke-strong shadow-[inset_0_-1px_0_var(--lu-stroke-strong)] hover:bg-layer-alt active:bg-press active:text-fg-secondary active:shadow-none",
  accent:
    "bg-accent text-on-accent border border-transparent hover:bg-accent-hover active:bg-accent-press active:opacity-80",
  subtle:
    "bg-transparent text-fg border border-transparent hover:bg-hover active:bg-press active:text-fg-secondary",
};

export function Button({
  variant = "standard",
  icon,
  className,
  children,
  ...props
}: ButtonProps) {
  return (
    <button
      type="button"
      {...props}
      className={cn(
        "inline-flex h-[var(--lu-control-h)] items-center justify-center gap-2 rounded-lu-sm px-3",
        "text-[length:var(--lu-text-body)] font-normal",
        "transition-colors duration-[var(--lu-dur-fast)] ease-[var(--lu-ease)]",
        "disabled:pointer-events-none disabled:text-fg-disabled disabled:opacity-60",
        variants[variant],
        className,
      )}
    >
      {icon}
      {children}
    </button>
  );
}
