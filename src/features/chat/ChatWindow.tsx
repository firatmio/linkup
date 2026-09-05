import { useLayoutEffect, useRef, useState } from "react";
import { Send, MessageSquare, Monitor } from "lucide-react";
import { t } from "../../i18n";
import { cn } from "../../lib/cn";
import { Callout } from "../../components/Callout";
import { MessageBubble } from "./MessageBubble";
import { useChatStore } from "../../stores/chatStore";
import type { ChatMessage, TrustedDevice } from "../../lib/tauri";

/** ``` içeren mesaj kod bloğu sayılır (PLAN.md §3.3). */
function looksLikeCode(body: string): boolean {
  return body.trimStart().startsWith("```");
}

/** Aynı gün mü? Gün ayracı buna göre basılır. */
function sameDay(a: number, b: number): boolean {
  return new Date(a * 1000).toDateString() === new Date(b * 1000).toDateString();
}

function formatDay(seconds: number): string {
  const date = new Date(seconds * 1000);
  const today = new Date();
  const yesterday = new Date(today.getTime() - 86_400_000);

  if (date.toDateString() === today.toDateString()) return t("chats.today");
  if (date.toDateString() === yesterday.toDateString()) return t("chats.yesterday");
  return date.toLocaleDateString("tr-TR", { day: "numeric", month: "long", year: "numeric" });
}

/** Ardışık aynı yönlü ve yakın zamanlı mesajlar tek grup sayılır. */
const GROUP_WINDOW_SECONDS = 120;

function isSameGroup(previous: ChatMessage | undefined, current: ChatMessage): boolean {
  if (!previous) return false;
  return (
    previous.direction === current.direction &&
    current.sentAt - previous.sentAt < GROUP_WINDOW_SECONDS &&
    sameDay(previous.sentAt, current.sentAt)
  );
}

export function ChatWindow({ device }: { device: TrustedDevice }) {
  const [draft, setDraft] = useState("");
  const messages = useChatStore((s) => s.messagesOf(device.id));
  const error = useChatStore((s) => s.error);
  const send = useChatStore((s) => s.send);

  const scrollRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);

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
      <header className="flex h-[var(--lu-header-h)] shrink-0 items-center gap-3 border-b border-divider px-5">
        <span className="flex size-8 shrink-0 items-center justify-center rounded-lu-full bg-layer-alt">
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
      </header>

      <div ref={scrollRef} className="flex-1 overflow-y-auto py-4">
        {messages.length === 0 ? (
          <p className="px-6 py-10 text-center text-[length:var(--lu-text-body)] text-fg-tertiary">
            {t("chats.noMessages")}
          </p>
        ) : (
          messages.map((message, index) => {
            const previous = messages[index - 1];
            const next = messages[index + 1];
            const newDay = !previous || !sameDay(previous.sentAt, message.sentAt);

            return (
              <div key={message.msgId}>
                {newDay ? (
                  <div className="my-3 flex items-center gap-3 px-5">
                    <span className="h-px flex-1 bg-divider" />
                    <span className="text-[length:var(--lu-text-caption)] text-fg-tertiary">
                      {formatDay(message.sentAt)}
                    </span>
                    <span className="h-px flex-1 bg-divider" />
                  </div>
                ) : null}
                <MessageBubble
                  message={message}
                  firstOfGroup={newDay || !isSameGroup(previous, message)}
                  lastOfGroup={!next || !isSameGroup(message, next)}
                />
              </div>
            );
          })
        )}
      </div>

      <div className="shrink-0 space-y-2 px-5 pt-1 pb-4">
        {error ? <Callout tone="warning">{error}</Callout> : null}

        {/* Windows 11 deseni: giriş alanı ve gönder düğmesi tek yüzeyde. */}
        <div className="flex items-end gap-1 rounded-lu-lg border border-stroke-strong bg-layer-alt p-1 shadow-[inset_0_-1px_0_var(--lu-stroke-strong)] focus-within:border-accent focus-within:shadow-[inset_0_-2px_0_var(--lu-accent)]">
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
