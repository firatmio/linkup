import { useEffect, useRef, useState } from "react";
import { Send, MessageSquare, Monitor, Paperclip, MoreHorizontal } from "lucide-react";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { t } from "../../i18n";
import { cn } from "../../lib/cn";
import { Callout } from "../../components/Callout";
import { MessageList } from "./MessageList";
import { useChatStore } from "../../stores/chatStore";
import { useTransferStore } from "../../stores/transferStore";
import { DeviceInfoDialog } from "../devices/DeviceInfoDialog";
import type { TrustedDevice } from "../../lib/tauri";

/** ``` içeren mesaj kod bloğu sayılır (PLAN.md §3.3). */
function looksLikeCode(body: string): boolean {
  return body.trimStart().startsWith("```");
}

export function ChatWindow({ device }: { device: TrustedDevice }) {
  const [draft, setDraft] = useState("");
  const messages = useChatStore((s) => s.messagesOf(device.id));
  const error = useChatStore((s) => s.error);
  const send = useChatStore((s) => s.send);

  const [dropActive, setDropActive] = useState(false);
  const [infoOpen, setInfoOpen] = useState(false);
  const sendFile = useTransferStore((s) => s.send);

  const inputRef = useRef<HTMLTextAreaElement>(null);

  // Sürükle-bırak (PLAN.md §3.3). Tarayıcının drop olayı Tauri'de dosya YOLU
  // vermez; yolu yalnızca webview'ın kendi olayı taşır.
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;

    void getCurrentWebview()
      .onDragDropEvent((event) => {
        if (event.payload.type === "over") setDropActive(true);
        else if (event.payload.type === "leave") setDropActive(false);
        else if (event.payload.type === "drop") {
          setDropActive(false);
          for (const path of event.payload.paths) {
            void sendFile(device.id, path);
          }
        }
      })
      .then((fn) => {
        if (cancelled) fn();
        else unlisten = fn;
      });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [device.id, sendFile]);

  const pickFile = async () => {
    const selected = await openFileDialog({ multiple: true });
    if (!selected) return;
    for (const path of Array.isArray(selected) ? selected : [selected]) {
      await sendFile(device.id, path);
    }
  };

  const submit = () => {
    const body = draft.trim();
    if (!body) return;
    setDraft("");
    void send(device.id, body, looksLikeCode(body));
    inputRef.current?.focus();
  };

  return (
    <>
      <header className="flex h-[var(--lu-header-h)] shrink-0 items-center gap-2 border-b border-divider px-5">
        <span className="flex size-8 shrink-0 items-center justify-center rounded-full bg-layer-alt">
          <Monitor size={16} className="text-fg-secondary" />
        </span>
        <span className="min-w-0">
          <span className="block truncate font-display text-[length:var(--lu-text-body)] font-semibold">
            {device.name}
          </span>
          <span className="flex items-center gap-1.5 text-[length:var(--lu-text-caption)] text-fg-secondary">
            <span
              aria-hidden
              className={cn(
                "inline-block size-1.5 rounded-full",
                device.online ? "bg-success" : "bg-offline",
              )}
            />
            {t(device.online ? "status.online" : "status.offline")}
          </span>
        </span>

        <button
          type="button"
          onClick={() => setInfoOpen(true)}
          aria-label={t("device.info")}
          title={t("device.info")}
          className="ml-auto shrink-0 rounded-lu-sm p-1.5 text-fg-secondary transition-colors hover:bg-hover hover:text-fg active:bg-press"
        >
          <MoreHorizontal size={18} />
        </button>
      </header>

      <DeviceInfoDialog device={device} open={infoOpen} onClose={() => setInfoOpen(false)} />

      <div
        className="relative min-h-0 flex-1"
        // Sürükle-bırak göstergesi listenin ÜSTÜNDE durmalı; sanallaştırılmış
        // liste kendi kaydırma kabını yönettiği için sarmalayıcıya konuyor.
      >
        {dropActive ? (
          <div className="pointer-events-none absolute inset-3 z-10 flex items-center justify-center rounded-lu-lg border-2 border-dashed border-accent bg-accent-subtle text-[length:var(--lu-text-body)] font-semibold text-accent">
            {t("chats.dropHere")}
          </div>
        ) : null}
        {messages.length === 0 ? (
          <p className="px-6 py-10 text-center text-[length:var(--lu-text-body)] text-fg-tertiary">
            {t("chats.noMessages")}
          </p>
        ) : (
          <MessageList deviceId={device.id} messages={messages} />
        )}
      </div>

      <div className="shrink-0 space-y-2 px-5 pt-1 pb-4">
        {error ? <Callout tone="warning">{error}</Callout> : null}

        {/* Windows 11 deseni: giriş alanı ve gönder düğmesi tek yüzeyde. */}
        <div className="flex items-end gap-1 rounded-lu-lg border border-stroke-strong bg-layer-alt p-1 shadow-[inset_0_-1px_0_var(--lu-stroke-strong)] focus-within:border-accent focus-within:shadow-[inset_0_-2px_0_var(--lu-accent)]">
          <button
            type="button"
            onClick={() => void pickFile()}
            aria-label={t("chats.attach")}
            title={t("chats.attach")}
            className="mb-0.5 flex size-8 shrink-0 items-center justify-center rounded-lu-sm text-fg-secondary transition-colors hover:bg-hover hover:text-fg"
          >
            <Paperclip size={16} />
          </button>
          <textarea
            ref={inputRef}
            value={draft}
            onChange={(event) => setDraft(event.target.value)}
            onKeyDown={(event) => {
              // Enter gönderir, Shift+Enter alt satır açar.
              if (event.key === "Enter" && !event.shiftKey) {
                event.preventDefault();
                submit();
              }
            }}
            rows={1}
            placeholder={device.online ? t("chats.placeholder") : t("chats.placeholder.offline")}
            className="lu-selectable max-h-40 min-h-8 flex-1 resize-none bg-transparent px-2.5 py-1.5 text-[length:var(--lu-text-body)] placeholder:text-fg-tertiary focus:outline-none"
          />
          <button
            type="button"
            onClick={submit}
            disabled={!draft.trim()}
            aria-label={t("chats.send")}
            title={t("chats.send")}
            className={cn(
              "mb-0.5 flex size-8 shrink-0 items-center justify-center rounded-lu-sm transition-colors",
              draft.trim()
                ? "bg-accent text-on-accent hover:bg-accent-hover active:bg-accent-press"
                : "text-fg-disabled",
            )}
          >
            <Send size={16} />
          </button>
        </div>

        <p className="px-1 text-[length:var(--lu-text-caption)] text-fg-tertiary">
          {t("chats.sendHint")}
        </p>
      </div>
    </>
  );
}

export function ChatPlaceholder() {
  return (
    <div className="flex flex-1 flex-col items-center justify-center gap-3 text-fg-tertiary">
      <MessageSquare size={32} strokeWidth={1.5} />
      <p className="text-[length:var(--lu-text-body)]">{t("chats.select")}</p>
    </div>
  );
}
