import { Check, CheckCheck, Clock, AlertCircle } from "lucide-react";
import { t, type TranslationKey } from "../../i18n";
import { cn } from "../../lib/cn";
import { CodeBlock } from "./CodeBlock";
import { FileBubble } from "./FileBubble";
import { useTransferStore } from "../../stores/transferStore";
import type { ChatMessage } from "../../lib/tauri";

/** Mesaj durumu göstergesi — yalnızca giden mesajlarda (PLAN.md §2.8). */
function StatusIcon({
  status,
  onAccent,
}: {
  status: ChatMessage["status"];
  onAccent: boolean;
}) {
  const label = t(`message.status.${status}` as TranslationKey);
  const muted = onAccent ? "opacity-70" : "text-fg-tertiary";

  switch (status) {
    case "sending":
      return <Clock size={13} aria-label={label} className={muted} />;
    case "sent":
      return <Check size={13} aria-label={label} className={muted} />;
    case "delivered":
      return <CheckCheck size={13} aria-label={label} className={muted} />;
    case "read":
      return (
        <CheckCheck
          size={13}
          aria-label={label}
          className={onAccent ? "opacity-100" : "text-accent"}
        />
      );
    case "failed":
      return <AlertCircle size={13} aria-label={label} className="text-danger" />;
  }
}

function formatTime(seconds: number): string {
  return new Date(seconds * 1000).toLocaleTimeString("tr-TR", {
    hour: "2-digit",
    minute: "2-digit",
  });
}

/**
 * Windows 11 sohbet balonu.
 *
 * Giden mesajlar accent dolgulu, gelenler yüzey katmanında. Ardışık mesajlar
 * gruplanır: grubun içindeki köşeler küçülür ve zaman damgası yalnızca grubun
 * SONUNDA gösterilir — her balonda saat tekrar etmesi Windows'un sohbet
 * uygulamalarında da yapılmaz, gürültü yaratır.
 */
export function MessageBubble({
  message,
  firstOfGroup,
  lastOfGroup,
}: {
  message: ChatMessage;
  firstOfGroup: boolean;
  lastOfGroup: boolean;
}) {
  const outgoing = message.direction === "out";
  const isCode = message.contentType === "code";
  const isFile = message.contentType === "file_ref";
  const failed = message.status === "failed";

  // Anlık ilerleme veritabanına yazılmaz; baloncuk onu bellekten okur.
  const progress = useTransferStore((s) =>
    message.transferId ? s.progress[message.transferId] : undefined,
  );

  return (
    <div
      className={cn(
        "flex px-5",
        outgoing ? "justify-end" : "justify-start",
        lastOfGroup ? "mb-2.5" : "mb-0.5",
      )}
    >
      <div
        className={cn(
          "flex max-w-[min(38rem,78%)] flex-col",
          isCode && "w-full max-w-[min(48rem,92%)]",
          isFile && "w-[min(22rem,78%)]",
        )}
      >
        <div
          className={cn(
            "px-3 py-2 text-[length:var(--lu-text-body)]",
            isCode && "px-2",
            // Köşeler: grubun dış kenarları yuvarlak, iç kenarlar daralır.
            "rounded-lu-lg",
            outgoing
              ? cn(!firstOfGroup && "rounded-tr-lu-sm", !lastOfGroup && "rounded-br-lu-sm")
              : cn(!firstOfGroup && "rounded-tl-lu-sm", !lastOfGroup && "rounded-bl-lu-sm"),
            outgoing
              ? failed
                ? "border border-danger bg-[color-mix(in_srgb,var(--lu-danger)_12%,transparent)] text-fg"
                : "bg-accent text-on-accent"
              : "border border-stroke bg-layer-alt text-fg",
          )}
        >
          {isFile ? (
            <FileBubble
              transfer={message.transfer}
              fileName={message.content}
              progress={progress}
              outgoing={outgoing}
            />
          ) : isCode ? (
            <CodeBlock content={message.content} />
          ) : (
            <p className="lu-selectable break-words whitespace-pre-wrap">{message.content}</p>
          )}
        </div>

        {lastOfGroup ? (
          <div
            className={cn(
              "mt-1 flex items-center gap-1.5 px-1 text-[length:var(--lu-text-caption)] text-fg-tertiary",
              outgoing ? "justify-end" : "justify-start",
            )}
          >
            <span>{formatTime(message.sentAt)}</span>
            {outgoing && !isFile ? (
              <>
                <StatusIcon status={message.status} onAccent={false} />
                {failed ? <span className="text-danger">{t("message.status.failed")}</span> : null}
              </>
            ) : null}
          </div>
        ) : null}
      </div>
    </div>
  );
}
