import type { HTMLAttributes, ReactNode } from "react";
import { cn } from "../lib/cn";

/** Fluent "card" katmanı: içerik zemininin üstünde duran yüzey. */
export function Card({ className, children, ...props }: HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      {...props}
      className={cn(
        "rounded-lu-lg border border-stroke bg-layer shadow-card",
        className,
      )}
    >
      {children}
    </div>
  );
}

/** Sayfa başlığı — Fluent'te sayfanın sol üstünde, subtitle ölçeğinde. */
export function PageHeader({ title, action }: { title: string; action?: ReactNode }) {
  return (
    <header className="flex h-[var(--lu-header-h)] shrink-0 items-center justify-between px-6">
      <h1 className="font-display text-[length:var(--lu-text-subtitle)] leading-[var(--lu-leading-tight)] font-semibold">
        {title}
      </h1>
      {action}
    </header>
  );
}

/** Bölüm başlığı (Ayarlar ve Ana Sayfa içindeki gruplar). */
export function SectionTitle({ children }: { children: ReactNode }) {
  return (
    <h2 className="mb-2 text-[length:var(--lu-text-body)] font-semibold text-fg">
      {children}
    </h2>
  );
}

/** Boş durum — her liste ekranının ilk hâli. */
export function EmptyState({
  icon,
  title,
  body,
  action,
}: {
  icon?: ReactNode;
  title: string;
  body: string;
  action?: ReactNode;
}) {
  return (
    <div className="flex flex-col items-center justify-center gap-3 rounded-lu-lg border border-dashed border-stroke-strong px-6 py-14 text-center">
      {icon ? <div className="text-fg-tertiary">{icon}</div> : null}
      <div className="space-y-1">
        <p className="text-[length:var(--lu-text-body)] font-semibold">{title}</p>
        <p className="max-w-sm text-[length:var(--lu-text-body)] text-fg-secondary">
          {body}
        </p>
      </div>
      {action}
    </div>
  );
}

/**
 * Ayarlar satırı — Windows 11 Ayarlar uygulamasındaki kart satırı:
 * solda ikon + başlık/açıklama, sağda kontrol.
 */
export function SettingRow({
  icon,
  title,
  description,
  control,
  children
}: {
  icon?: ReactNode;
  title: string;
  description?: string;
  control?: ReactNode;
  children?: ReactNode;
}) {
  return (
    <div className="last:border-b-0 border-divider border-b">
      <div className="flex items-center gap-4 px-4 py-3">
        {icon ? <div className="text-fg-secondary">{icon}</div> : null}
        <div className="min-w-0 flex-1">
          <p className="truncate text-[length:var(--lu-text-body)]">{title}</p>
          {description ? (
            <p className="lu-selectable truncate text-[length:var(--lu-text-caption)] text-fg-secondary">
              {description}
            </p>
          ) : null}
        </div>
        {control ? <div className="shrink-0">{control}</div> : null}
      </div>
      {children}
    </div>
  );
}
