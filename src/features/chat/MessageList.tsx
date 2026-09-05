import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { t } from "../../i18n";
import { useChatStore } from "../../stores/chatStore";
import { MessageBubble } from "./MessageBubble";
import type { ChatMessage } from "../../lib/tauri";

/** Aynı gruba giren ardışık mesajlar arasındaki en fazla süre. */
const GROUP_WINDOW_SECONDS = 120;

/** Ölçülmeden önce bir mesaj için varsayılan yükseklik. */
const ESTIMATED_HEIGHT = 56;

/** Kullanıcı en alta bu kadar yakınsa yeni mesajda otomatik kaydırılır. */
const STICKY_THRESHOLD_PX = 120;

/** Listenin başına bu kadar kalınca bir sonraki sayfa istenir. */
const LOAD_MORE_THRESHOLD_PX = 200;

function sameDay(a: number, b: number): boolean {
  return new Date(a * 1000).toDateString() === new Date(b * 1000).toDateString();
}

function isSameGroup(previous: ChatMessage | undefined, current: ChatMessage): boolean {
  if (!previous) return false;
  return (
    previous.direction === current.direction &&
    current.sentAt - previous.sentAt < GROUP_WINDOW_SECONDS &&
    sameDay(previous.sentAt, current.sentAt)
  );
}

function formatDay(seconds: number): string {
  const date = new Date(seconds * 1000);
  const today = new Date();
  if (date.toDateString() === today.toDateString()) return t("chats.today");

  const yesterday = new Date(today);
  yesterday.setDate(today.getDate() - 1);
  if (date.toDateString() === yesterday.toDateString()) return t("chats.yesterday");

  return date.toLocaleDateString("tr-TR", { day: "numeric", month: "long", year: "numeric" });
}

/**
 * Sanallaştırılmış mesaj listesi (PLAN.md §3.3).
 *
 * Uzun bir geçmişte her mesajı DOM'a basmak kaydırmayı gözle görülür şekilde
 * ağırlaştırır; burada yalnızca görünen aralık basılıyor. Baloncuk yükseklikleri
 * değişken olduğu için tahminle başlanıp gerçek yükseklik ölçülüyor.
 *
 * İki kaydırma davranışı elle yönetiliyor, çünkü ikisi de otomatik değil:
 *
 * 1. **Alta yapışma.** Yeni mesajda en alta inilir — ama yalnızca kullanıcı
 *    zaten alttaysa. Geçmişi okuyan birini yeni mesaj yüzünden aşağı fırlatmak
 *    okuduğu yeri kaybettirir.
 * 2. **Geçmiş yüklendiğinde konum koruma.** Listenin başına mesaj eklemek
 *    içeriği aşağı iter; hiçbir şey yapılmazsa kullanıcının baktığı yer kayar.
 *    Piksel farkı yerine İNDEKS kullanılıyor: eklenen sayı kadar ileri
 *    kaydırmak, ölçümler henüz tamamlanmamışken bile doğru sonucu verir.
 */
export function MessageList({
  deviceId,
  messages,
}: {
  deviceId: string;
  messages: ChatMessage[];
}) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const loadOlder = useChatStore((s) => s.loadOlder);
  const loadingOlder = useChatStore((s) => s.loadingOlder);
  const atStart = useChatStore((s) => s.atStart[deviceId] ?? false);

  const [stickToBottom, setStickToBottom] = useState(true);

  const virtualizer = useVirtualizer({
    count: messages.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => ESTIMATED_HEIGHT,
    // Anahtar indeks değil mesaj kimliği: başa ekleme yapıldığında indeksler
    // kayar ve ölçümler yanlış satıra bağlanırdı.
    getItemKey: (index) => messages[index]?.msgId ?? index,
    overscan: 8,
  });

  // Sohbet değiştiğinde her zaman en altta başlanır.
  useLayoutEffect(() => {
    setStickToBottom(true);
    const element = scrollRef.current;
    if (element) element.scrollTop = element.scrollHeight;
  }, [deviceId]);

  // Yeni mesaj geldiğinde alta in — yalnızca kullanıcı zaten alttaysa.
  useLayoutEffect(() => {
    if (!stickToBottom || messages.length === 0) return;
    virtualizer.scrollToIndex(messages.length - 1, { align: "end" });
    // `virtualizer` her render'da yeni bir nesne; bağımlılığa eklemek etkiyi
    // sürekli tetiklerdi.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [messages.length, stickToBottom]);

  const onScroll = useCallback(() => {
    const element = scrollRef.current;
    if (!element) return;

    const distanceToBottom = element.scrollHeight - element.scrollTop - element.clientHeight;
    setStickToBottom(distanceToBottom < STICKY_THRESHOLD_PX);

    if (element.scrollTop < LOAD_MORE_THRESHOLD_PX && !atStart && !loadingOlder) {
      void loadOlder(deviceId).then((added) => {
        // Eklenen kadar ileri kaydırılır: kullanıcının baktığı mesaj yerinde
        // kalır, liste altından değil üstünden büyümüş gibi görünür.
        if (added > 0) virtualizer.scrollToIndex(added, { align: "start" });
      });
    }
  }, [atStart, deviceId, loadOlder, loadingOlder, virtualizer]);

  // Kısa bir geçmiş listeyi doldurmayabilir; o durumda kaydırma olayı hiç
  // tetiklenmez ve sonraki sayfa istenmez.
  useEffect(() => {
    const element = scrollRef.current;
    if (!element || atStart || loadingOlder) return;
    if (element.scrollHeight <= element.clientHeight) void loadOlder(deviceId);
  }, [atStart, deviceId, loadOlder, loadingOlder, messages.length]);

  const items = virtualizer.getVirtualItems();

  return (
    <div ref={scrollRef} onScroll={onScroll} className="relative h-full overflow-y-auto py-4">
      {loadingOlder ? (
        <p className="py-2 text-center text-[length:var(--lu-text-caption)] text-fg-tertiary">
          {t("common.loading")}
        </p>
      ) : null}

      <div className="relative w-full" style={{ height: virtualizer.getTotalSize() }}>
        {items.map((item) => {
          const message = messages[item.index];
          const previous = messages[item.index - 1];
          const next = messages[item.index + 1];
          const newDay = !previous || !sameDay(previous.sentAt, message.sentAt);

          return (
            <div
              key={item.key}
              data-index={item.index}
              ref={virtualizer.measureElement}
              className="absolute top-0 left-0 w-full"
              style={{ transform: `translateY(${item.start}px)` }}
            >
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
        })}
      </div>
    </div>
  );
}
