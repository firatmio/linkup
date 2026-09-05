import { useEffect } from "react";
import { usePairingStore } from "../../stores/pairingStore";
import { useChatStore } from "../../stores/chatStore";
import { ChatWindow, ChatPlaceholder } from "./ChatWindow";

/**
 * Sohbet ekranı. Sohbet listesi burada değil, sidebar'ın yerini alan
 * `ConversationList`te — içerik alanı yalnızca konuşmaya ayrılmıştır.
 */
export function ChatsPage() {
  const devices = usePairingStore((s) => s.trusted);
  const activeId = useChatStore((s) => s.activeDeviceId);
  const select = useChatStore((s) => s.select);
  const close = useChatStore((s) => s.close);

  // Sohbetlerden çıkınca seçim bırakılır; geri dönünce liste temiz açılır.
  useEffect(() => close, [close]);

  // Tek cihaz varsa seçtirmeye gerek yok, doğrudan açılır.
  useEffect(() => {
    if (!activeId && devices.length === 1) select(devices[0].id);
  }, [activeId, devices, select]);

  const active = devices.find((device) => device.id === activeId) ?? null;
  return active ? <ChatWindow key={active.id} device={active} /> : <ChatPlaceholder />;
}
