import { Check, CheckCheck, Clock, AlertCircle } from "lucide-react";
import { t, type TranslationKey } from "../../i18n";
import { cn } from "../../lib/cn";
import { CodeBlock } from "./CodeBlock";
import type { ChatMessage } from "../../lib/tauri";

/** Mesaj durumu göstergesi — yalnızca giden mesajlarda (PLAN.md §2.8). */
function StatusIcon({ status }: { status: ChatMessage["status"] }) {
  const label = t(`message.status.${status}` as TranslationKey);
  const common = { size: 13, "aria-label": label };

  switch (status) {
    case "sending":
      return <Clock {...common} className="text-fg-tertiary" />;
    case "sent":
      return <Check {...common} className="text-fg-tertiary" />;
    case "delivered":
      return <CheckCheck {...common} className="text-fg-tertiary" />;
    case "read":
      return <CheckCheck {...common} className="text-accent" />;
    case "failed":
      return <AlertCircle {...common} className="text-danger" />;
  }
}

function formatTime(seconds: number): string {
  return new Date(seconds * 1000).toLocaleTimeString("tr-TR", {
    hour: "2-digit",
    minute: "2-digit",
  });
}

export function MessageBubble({ message }: { message: ChatMessage }) {
  const outgoing = message.direction === "out";
  const isCode = message.contentType === "code";

  return (
    <div className={cn("flex px-6 py-0.5", outgoing ? "justify-end" : "justify-start")}>
      <div
        className={cn(
          "max-w-[min(42rem,80%)] rounded-lu-lg px-3 py-2",
          // Kod bloğu kendi çerçevesini taşıdığı için balon dolgusu daralır.
          isCode && "w-full max-w-[min(48rem,90%)] px-2",
          outgoing
            ? "bg-accent-subtle border border-[color-mix(in_srgb,var(--lu-accent)_25%,transparent)]"
            : "border border-stroke bg-layer-alt",
        )}
      >
        {isCode ? (
          <CodeBlock content={message.content} />
        ) : (
          <p className="lu-selectable text-[length:var(--lu-text-body)] whitespace-pre-wrap break-words">
            {message.content}
          </p>
        )}

        <div
          className={cn(
            "mt-1 flex items-center gap-1.5 text-[length:var(--lu-text-caption)] text-fg-tertiary",
            outgoing ? "justify-end" : "justify-start",
          )}
        >
          <span>{formatTime(message.sentAt)}</span>
          {outgoing ? (
            <>
              <StatusIcon status={message.status} />
              {message.status === "failed" ? (
                <span className="text-danger">{t("message.status.failed")}</span>
              ) : null}
            </>
          ) : null}
        </div>
      </div>
    </div>
  );
}
