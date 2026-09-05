import { NavLink } from "react-router-dom";
import { House, MessageSquare, FolderDown, Settings } from "lucide-react";
import type { ReactNode } from "react";
import { cn } from "../lib/cn";
import { t, type TranslationKey } from "../i18n";
import { DiscoveredDevices } from "../features/devices/DiscoveredDevices";

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

      <DiscoveredDevices />
    </nav>
  );
}
