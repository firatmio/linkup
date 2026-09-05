import type { ReactNode } from "react";
import { AlertTriangle, Info } from "lucide-react";
import { cn } from "../lib/cn";

type Tone = "info" | "warning";

/** Fluent InfoBar yorumu — kullanıcıya durum bildiren satır. */
export function Callout({ tone = "info", children }: { tone?: Tone; children: ReactNode }) {
  const Icon = tone === "warning" ? AlertTriangle : Info;
  return (
    <div
      role="status"
      className={cn(
        "flex items-start gap-3 rounded-lu-lg border px-4 py-3",
        "text-[length:var(--lu-text-body)]",
        tone === "warning"
          ? "border-[color-mix(in_srgb,var(--lu-warning)_40%,transparent)] bg-[color-mix(in_srgb,var(--lu-warning)_10%,transparent)]"
          : "border-stroke bg-layer-alt",
      )}
    >
      <Icon
        size={18}
        className={cn("mt-0.5 shrink-0", tone === "warning" ? "text-warning" : "text-accent")}
      />
      <p className="text-fg-secondary">{children}</p>
    </div>
  );
}
