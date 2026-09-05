import { useState } from "react";
import { Monitor, Trash2 } from "lucide-react";
import { useNavigate } from "react-router-dom";
import { t } from "../../i18n";
import { cn } from "../../lib/cn";
import { relativeTime } from "../../lib/format";
import { Card } from "../../components/Surface";
import { Button } from "../../components/Button";
import { Dialog } from "../../components/Dialog";
import { usePairingStore } from "../../stores/pairingStore";
import { useChatStore } from "../../stores/chatStore";
import type { TrustedDevice } from "../../lib/tauri";

/**
 * Eşleşmiş bir cihazın özet kartı (PLAN.md §3.2).
 *
 * Karta tıklamak o cihazın sohbetini açar: özet kartın işe yaraması için
 * kullanıcıyı bir sonraki adıma götürmesi gerekir, yalnızca bilgi göstermesi
 * değil.
 */
export function DeviceSummaryCard({ device }: { device: TrustedDevice }) {
  const [confirmOpen, setConfirmOpen] = useState(false);
  const navigate = useNavigate();
  const forget = usePairingStore((s) => s.forget);
  const select = useChatStore((s) => s.select);

  const openChat = () => {
    select(device.id);
    navigate("/chats");
  };

  return (
    <Card className="group overflow-hidden">
      <div
        role="button"
        tabIndex={0}
        aria-label={t("dashboard.openChat")}
        onClick={openChat}
        onKeyDown={(event) => {
          if (event.key === "Enter" || event.key === " ") {
            event.preventDefault();
            openChat();
          }
        }}
        className="flex cursor-pointer items-start gap-3 p-4 transition-colors hover:bg-hover active:bg-press"
      >
        <span className="relative shrink-0">
          <span className="flex size-9 items-center justify-center rounded-lu-full bg-layer-alt">
            <Monitor size={18} className="text-fg-secondary" />
          </span>
          <span
            aria-hidden
            className={cn(
              "absolute -right-0.5 -bottom-0.5 size-2.5 rounded-full ring-2 ring-[var(--lu-bg-layer)]",
              device.online ? "bg-success" : "bg-offline",
            )}
          />
        </span>

        <div className="min-w-0 flex-1">
          <div className="flex items-baseline gap-2">
            <p className="min-w-0 flex-1 truncate font-semibold">{device.name}</p>
            <span className="shrink-0 text-[length:var(--lu-text-caption)] text-fg-tertiary">
              {relativeTime(device.lastMessageAt)}
            </span>
          </div>

          <p className="text-[length:var(--lu-text-caption)] text-fg-secondary">
            {t(device.online ? "status.online" : "status.offline")}
          </p>

          {device.lastMessage ? (
            <p
              className={cn(
                "mt-1.5 truncate text-[length:var(--lu-text-caption)]",
                device.unread > 0 ? "text-fg" : "text-fg-secondary",
              )}
            >
              {device.lastMessage}
            </p>
          ) : null}
        </div>

        <div className="flex shrink-0 items-center gap-1">
          {device.unread > 0 ? (
            <span className="rounded-lu-full bg-accent px-1.5 text-[length:var(--lu-text-caption)] leading-[1.4] text-on-accent">
              {device.unread}
            </span>
          ) : null}

          <button
            type="button"
            aria-label={t("devices.forget")}
            title={t("devices.forget")}
            onClick={(event) => {
              // Kart tıklamasını tetiklemesin.
              event.stopPropagation();
              setConfirmOpen(true);
            }}
            className="rounded-lu-sm p-1 text-fg-tertiary opacity-0 transition-opacity group-hover:opacity-100 hover:bg-press hover:text-danger"
          >
            <Trash2 size={16} />
          </button>
        </div>
      </div>

      <Dialog
        open={confirmOpen}
        title={t("devices.forget")}
        onClose={() => setConfirmOpen(false)}
        footer={
          <>
            <Button onClick={() => setConfirmOpen(false)}>{t("devices.forget.cancel")}</Button>
            <Button
              variant="accent"
              onClick={() => {
                setConfirmOpen(false);
                void forget(device.id);
              }}
            >
              {t("devices.forget")}
            </Button>
          </>
        }
      >
        <p className="text-[length:var(--lu-text-body)] text-fg-secondary">
          {t("devices.forget.confirm", { device: device.name })}
        </p>
      </Dialog>
    </Card>
  );
}
