import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { Send, MessageSquare } from "lucide-react";
import { t } from "../../i18n";
import { cn } from "../../lib/cn";
import { Button } from "../../components/Button";
import { Callout } from "../../components/Callout";
import { MessageBubble } from "./MessageBubble";
import { useChatStore } from "../../stores/chatStore";
import type { TrustedDevice } from "../../lib/tauri";

/** ``` içeren mesaj kod bloğu sayılır (PLAN.md §3.3). */
function looksLikeCode(body: string): boolean {
  return body.trimStart().startsWith("```");
}

export function ChatWindow({ device }: { device: TrustedDevice }) {
  const [draft, setDraft] = useState("");
  const messages = useChatStore((s) => s.messagesOf(device.id));
  const error = useChatStore((s) => s.error);
  const open = useChatStore((s) => s.open);
  const send = useChatStore((s) => s.send);

  const scrollRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    void open(device.id);
  }, [device.id, open]);

  // Yeni mesajda en alta kaydır. Boyama öncesi çalışmalı, yoksa liste bir kare
  // yanlış konumda görünür.
  useLayoutEffect(() => {
    const element = scrollRef.current;
    if (element) element.scrollTop = element.scrollHeight;
  }, [messages.length, device.id]);

  const submit = () => {
    const body = draft.trim();
    if (!body) return;
    setDraft("");
    void send(device.id, body, looksLikeCode(body));
    inputRef.current?.focus();
  };

  return (
    <>
      <header className="flex h-[var(--lu-header-h)] shrink-0 items-center gap-3 border-b border-divider px-6">
        <h1 className="font-display text-[length:var(--lu-text-subtitle)] leading-none font-semibold">
          {device.name}
        </h1>
        <span className="flex items-center gap-1.5 text-[length:var(--lu-text-caption)] text-fg-secondary">
          <span
            aria-hidden
            className={cn(
              "inline-block size-2 rounded-full",
              device.online ? "bg-success" : "bg-offline",
            )}
          />
          {t(device.online ? "status.online" : "status.offline")}
        </span>
      </header>

      <div ref={scrollRef} className="flex-1 overflow-y-auto py-3">
        {messages.length === 0 ? (
          <p className="px-6 py-10 text-center text-[length:var(--lu-text-body)] text-fg-tertiary">
            {t("chats.noMessages")}
          </p>
        ) : (
          messages.map((message) => (
            <MessageBubble key={message.msgId} message={message} />
          ))
        )}
      </div>

      <div className="shrink-0 space-y-2 border-t border-divider px-6 py-3">
        {error ? <Callout tone="warning">{error}</Callout> : null}

        <div className="flex items-end gap-2">
          <textarea
            ref={inputRef}
            value={draft}
            onChange={(event) => setDraft(event.target.value)}
            onKeyDown={(event) => {
              // Enter gönderir, Shift+Enter alt satır açar — sohbet uygulaması
              // beklentisi bu yönde.
              if (event.key === "Enter" && !event.shiftKey) {
                event.preventDefault();
                submit();
              }
            }}
            rows={1}
            placeholder={
              device.online ? t("chats.placeholder") : t("chats.placeholder.offline")
            }
            className="lu-selectable max-h-40 min-h-[var(--lu-control-h)] flex-1 resize-y rounded-lu-sm border border-stroke-strong bg-layer-alt px-3 py-1.5 text-[length:var(--lu-text-body)] shadow-[inset_0_-1px_0_var(--lu-stroke-strong)] placeholder:text-fg-tertiary focus:border-accent focus:shadow-[inset_0_-2px_0_var(--lu-accent)] focus:outline-none"
          />
          <Button
            variant="accent"
            icon={<Send size={16} />}
            onClick={submit}
            disabled={!draft.trim()}
          >
            {t("chats.send")}
          </Button>
        </div>

        <p className="text-[length:var(--lu-text-caption)] text-fg-tertiary">
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
