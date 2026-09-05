import { useState } from "react";
import { MessageSquare, Monitor } from "lucide-react";
import { t } from "../../i18n";
import { cn } from "../../lib/cn";
import { PageHeader, EmptyState } from "../../components/Surface";
import { usePairingStore } from "../../stores/pairingStore";
import { ChatWindow, ChatPlaceholder } from "./ChatWindow";

/**
 * Sohbetler: solda eşleşmiş cihaz listesi, sağda seçili sohbet
 * (PLAN.md §3.2, §3.3).
 */
export function ChatsPage() {
  const devices = usePairingStore((s) => s.trusted);
  const [selectedId, setSelectedId] = useState<string | null>(null);

  if (devices.length === 0) {
    return (
      <>
        <PageHeader title={t("chats.title")} />
        <div className="flex-1 overflow-y-auto px-6 pb-6">
          <EmptyState
            icon={<MessageSquare size={32} strokeWidth={1.5} />}
            title={t("chats.empty.title")}
            body={t("chats.empty.body")}
          />
        </div>
      </>
    );
  }

  const selected = devices.find((device) => device.id === selectedId) ?? null;

  return (
    <div className="flex min-h-0 flex-1">
      <ul className="w-64 shrink-0 overflow-y-auto border-r border-divider py-2">
        {devices.map((device) => (
          <li key={device.id}>
            <button
              type="button"
              onClick={() => setSelectedId(device.id)}
              className={cn(
                "flex w-full items-start gap-2.5 px-4 py-2.5 text-left",
                "transition-colors duration-[var(--lu-dur-fast)]",
                device.id === selectedId ? "bg-selected" : "hover:bg-hover",
              )}
            >
              <span className="relative mt-0.5 shrink-0">
                <Monitor size={18} className="text-fg-secondary" />
                <span
                  aria-hidden
                  className={cn(
                    "absolute -right-0.5 -bottom-0.5 size-2 rounded-full ring-2 ring-[var(--lu-bg-layer)]",
                    device.online ? "bg-success" : "bg-offline",
                  )}
                />
              </span>

              <span className="min-w-0 flex-1">
                <span className="flex items-center gap-2">
                  <span className="min-w-0 flex-1 truncate text-[length:var(--lu-text-body)]">
                    {device.name}
                  </span>
                  {device.unread > 0 ? (
                    <span className="shrink-0 rounded-lu-full bg-accent px-1.5 text-[length:var(--lu-text-caption)] text-on-accent">
                      {device.unread}
                    </span>
                  ) : null}
                </span>
                <span className="block truncate text-[length:var(--lu-text-caption)] text-fg-tertiary">
                  {device.lastMessage ?? t("chats.noMessages")}
                </span>
              </span>
            </button>
          </li>
        ))}
      </ul>

      <div className="flex min-w-0 flex-1 flex-col">
        {selected ? <ChatWindow key={selected.id} device={selected} /> : <ChatPlaceholder />}
      </div>
    </div>
  );
}
