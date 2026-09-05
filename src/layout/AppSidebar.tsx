import { NavLink } from "react-router-dom";
import { House, MessageSquare, FolderDown, Settings, Plus, Radar } from "lucide-react";
import type { ReactNode } from "react";
import { cn } from "../lib/cn";
import { t, type TranslationKey } from "../i18n";
import { Button } from "../components/Button";

interface NavItem {
  to: string;
  labelKey: TranslationKey;
  icon: ReactNode;
}

const ICON = 18;

const items: NavItem[] = [
  { to: "/", labelKey: "nav.dashboard", icon: <House size={ICON} /> },
  { to: "/chats", labelKey: "nav.chats", icon: <MessageSquare size={ICON} /> },
  { to: "/files", labelKey: "nav.files", icon: <FolderDown size={ICON} /> },
  { to: "/settings", labelKey: "nav.settings", icon: <Settings size={ICON} /> },
];

/**
 * WinUI NavigationView yorumu: sabit genişlik, seçili öğede sol tarafta
 * accent renkli hap gösterge, hover'da subtle dolgu.
 */
export function AppSidebar() {
  return (
    <nav className="flex w-[var(--lu-sidebar-w)] shrink-0 flex-col bg-sidebar">
      <div className="flex h-[var(--lu-header-h)] items-center px-5">
        <span className="font-display text-[length:var(--lu-text-body)] font-semibold tracking-tight">
          {t("app.name")}
        </span>
      </div>

      <ul className="flex flex-col gap-0.5 px-2">
        {items.map((item) => (
          <li key={item.to}>
            <NavLink to={item.to} end={item.to === "/"} className="block">
              {({ isActive }) => (
                <span
                  className={cn(
                    "relative flex h-[var(--lu-row-h)] items-center gap-3 rounded-lu-sm px-3",
                    "transition-colors duration-[var(--lu-dur-fast)] ease-[var(--lu-ease)]",
                    isActive ? "bg-selected" : "hover:bg-hover active:bg-press",
                  )}
                >
                  <span
                    aria-hidden
                    className={cn(
                      "absolute left-0 w-[3px] rounded-lu-sm bg-accent transition-all duration-[var(--lu-dur-normal)] ease-[var(--lu-ease)]",
                      isActive ? "h-4 opacity-100" : "h-0 opacity-0",
                    )}
                  />
                  <span className={cn(isActive ? "text-fg" : "text-fg-secondary")}>
                    {item.icon}
                  </span>
                  <span
                    className={cn(
                      "text-[length:var(--lu-text-body)]",
                      isActive && "font-semibold",
                    )}
                  >
                    {t(item.labelKey)}
                  </span>
                </span>
              )}
            </NavLink>
          </li>
        ))}
      </ul>

      {/* Eşleşmemiş, ağda görünen cihazlar (PLAN.md §3.2). Faz 3'te canlanacak. */}
      <div className="mt-auto border-t border-divider px-2 py-3">
        <div className="flex items-center gap-2 px-3 pb-2 text-fg-tertiary">
          <Radar size={14} />
          <span className="text-[length:var(--lu-text-caption)] font-semibold uppercase tracking-wide">
            {t("nav.discovered")}
          </span>
        </div>
        <p className="px-3 pb-2 text-[length:var(--lu-text-caption)] text-fg-tertiary">
          {t("nav.discovered.empty")}
        </p>
        <Button variant="subtle" icon={<Plus size={16} />} className="w-full justify-start" disabled>
          {t("nav.addManually")}
        </Button>
      </div>
    </nav>
  );
}
