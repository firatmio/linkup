import { create } from "zustand";
import {
  api,
  onChatMessage,
  onChatStatus,
  type ChatMessage,
} from "../lib/tauri";
import { translateError } from "../i18n";
import { usePairingStore } from "./pairingStore";

interface ChatState {
  /** Cihaz kimliğine göre mesaj listeleri. */
  byDevice: Record<string, ChatMessage[]>;
  /** Açık olan sohbet; okundu bildirimi buna göre gönderilir. */
  activeDeviceId: string | null;
  loading: boolean;
  error: string | null;

  open: (deviceId: string) => Promise<void>;
  close: () => void;
  send: (deviceId: string, body: string, isCode: boolean) => Promise<void>;
  messagesOf: (deviceId: string) => ChatMessage[];
}

const EMPTY: ChatMessage[] = [];

export const useChatStore = create<ChatState>((set, get) => ({
  byDevice: {},
  activeDeviceId: null,
  loading: false,
  error: null,

  open: async (deviceId) => {
    set({ activeDeviceId: deviceId, loading: true, error: null });
    try {
      const messages = await api.chatHistory(deviceId);
      set((state) => ({
        byDevice: { ...state.byDevice, [deviceId]: messages },
        loading: false,
      }));
      // Sohbet açıldı: gelen mesajlar okundu sayılır ve karşı tarafa bildirilir.
      await api.markConversationRead(deviceId);
    } catch (err) {
      set({ loading: false, error: translateError(err) });
    }
  },

  close: () => set({ activeDeviceId: null }),

  send: async (deviceId, body, isCode) => {
    try {
      const message = await api.sendMessage(deviceId, body, isCode);
      void usePairingStore.getState().loadTrusted();
      set((state) => ({
        byDevice: {
          ...state.byDevice,
          [deviceId]: [...(state.byDevice[deviceId] ?? []), message],
        },
        error: null,
      }));
    } catch (err) {
      set({ error: translateError(err) });
    }
  },

  messagesOf: (deviceId) => get().byDevice[deviceId] ?? EMPTY,
}));

/** Gelen mesaj ve durum olaylarına abone olur. */
export function subscribeToChat(): () => void {
  const unlisteners: Array<() => void> = [];
  let cancelled = false;

  const track = (fn: () => void) => {
    if (cancelled) fn();
    else unlisteners.push(fn);
  };

  void onChatMessage(({ deviceId, message }) => {
    const state = useChatStore.getState();
    const existing = state.byDevice[deviceId] ?? [];

    // Aynı mesaj iki kez gelebilir (yeniden bağlanma sonrası tekrar gönderim);
    // arayüzde ikilenmesin.
    if (existing.some((m) => m.msgId === message.msgId)) return;

    useChatStore.setState({
      byDevice: { ...state.byDevice, [deviceId]: [...existing, message] },
    });

    // Sohbet açıksa mesaj anında okunmuş sayılır.
    if (state.activeDeviceId === deviceId) {
      void api.markConversationRead(deviceId);
    }

    // Cihaz listesindeki "son mesaj" ve okunmamış sayısı da tazelenmeli.
    void usePairingStore.getState().loadTrusted();
  }).then(track);

  void onChatStatus(({ deviceId, msgId, status }) => {
    const state = useChatStore.getState();
    const messages = state.byDevice[deviceId];
    if (!messages) return;

    useChatStore.setState({
      byDevice: {
        ...state.byDevice,
        [deviceId]: messages.map((m) => (m.msgId === msgId ? { ...m, status } : m)),
      },
    });
  }).then(track);

  return () => {
    cancelled = true;
    unlisteners.forEach((fn) => fn());
  };
}
