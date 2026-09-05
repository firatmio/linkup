import { ArrowLeft, Monitor, MessageSquare } from "lucide-react";
import { useNavigate } from "react-router-dom";
import { t } from "../../i18n";
import { cn } from "../../lib/cn";
import { usePairingStore } from "../../stores/pairingStore";
import { useChatStore } from "../../stores/chatStore";

/** Gün içindeyse saat, değilse kısa tarih. */
function formatStamp(seconds: number | null): string {
  if (!seconds) return "";
  const date = new Date(seconds * 1000);
  const sameDay = new Date().toDateString() === date.toDateString();
  return sameDay
    ? date.toLocaleTimeString("tr-TR", { hour: "2-digit", minute: "2-digit" })
    : date.toLocaleDateString("tr-TR", { day: "2-digit", month: "2-digit" });
}

/**
 * Sohbetler bölümündeyken navigasyon sidebar'ının yerini alan liste
 * (kullanıcı isteği): sol sütun sohbetlere ayrılır, içerik alanı yalnızca
 * sohbet ekranı olur. Üstteki geri düğmesi ana navigasyona döndürür.
 */
export function ConversationList() {
  const navigate = useNavigate();
  const devices = usePairingStore((s) => s.trusted);
  const activeId = useChatStore((s) => s.activeDeviceId);
  const select = useChatStore((s) => s.select);

  return (
    <nav className="flex w-[var(--lu-sidebar-w)] shrink-0 flex-col bg-sidebar">
      <div className="flex h-[var(--lu-header-h)] shrink-0 items-center gap-1 px-2">
        <button
          type="button"
          aria-label={t("chats.back")}
          title={t("chats.back")}
          onClick={() => navigate("/")}
          className="rounded-lu-sm p-1.5 text-fg-secondary transition-colors hover:bg-hover hover:text-fg active:bg-press"
        >
          <ArrowLeft size={18} />
        </button>
        <span className="font-display text-[length:var(--lu-text-body)] font-semibold">
          {t("chats.title")}
        </span>
      </div>

      {devices.length === 0 ? (
        <div className="flex flex-1 flex-col items-center justify-center gap-2 px-6 text-center text-fg-tertiary">
          <MessageSquare size={26} strokeWidth={1.5} />
          <p className="text-[length:var(--lu-text-caption)]">{t("chats.empty.body")}</p>
        </div>
      ) : (
        <ul className="flex-1 overflow-y-auto px-2 pb-2">
          {devices.map((device) => {
            const active = device.id === activeId;
            return (
              <li key={device.id}>
                <button
                  type="button"
                  onClick={() => select(device.id)}
                  className={cn(
                    "relative flex w-full items-center gap-2.5 rounded-lu-sm px-2.5 py-2 text-left",
                    "transition-colors duration-[var(--lu-dur-fast)] ease-[var(--lu-ease)]",
                    active ? "bg-selected" : "hover:bg-hover active:bg-press",
                  )}
                >
                  <span
                    aria-hidden
                    className={cn(
                      "absolute left-0 w-[3px] rounded-lu-sm bg-accent transition-all duration-[var(--lu-dur-normal)]",
                      active ? "h-5 opacity-100" : "h-0 opacity-0",
                    )}
                  />

                  <span className="relative shrink-0">
                    <span className="flex size-8 items-center justify-center rounded-lu-full bg-layer-alt">
                      <Monitor size={16} className="text-fg-secondary" />
                    </span>
                    <span
                      aria-hidden
                      className={cn(
                        "absolute -right-0.5 -bottom-0.5 size-2.5 rounded-full ring-2 ring-[var(--lu-bg-sidebar)]",
                        device.online ? "bg-success" : "bg-offline",
                      )}
                    />
                  </span>

                  <span className="min-w-0 flex-1">
                    <span className="flex items-baseline gap-2">
                      <span
                        className={cn(
                          "min-w-0 flex-1 truncate text-[length:var(--lu-text-body)]",
                          device.unread > 0 && "font-semibold",
                        )}
                      >
                        {device.name}
                      </span>
                      <span className="shrink-0 text-[length:var(--lu-text-caption)] text-fg-tertiary">
                        {formatStamp(device.lastMessageAt)}
                      </span>
                    </span>

                    <span className="flex items-center gap-2">
                      <span
                        className={cn(
                          "min-w-0 flex-1 truncate text-[length:var(--lu-text-caption)]",
                          device.unread > 0 ? "text-fg-secondary" : "text-fg-tertiary",
                        )}
                      >
                        {device.lastMessage ?? "—"}
                      </span>
                      {device.unread > 0 ? (
                        <span className="shrink-0 rounded-full bg-accent px-1.5 text-[length:var(--lu-text-caption)] leading-[1.4] text-on-accent">
                          {device.unread}
                        </span>
                      ) : null}
                    </span>
                  </span>
                </button>
              </li>
            );
          })}
        </ul>
      )}
    </nav>
  );
}
