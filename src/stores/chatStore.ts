import { create } from "zustand";
import {
  api,
  onChatMessage,
  onChatStatus,
  onTransferChanged,
  type ChatMessage,
} from "../lib/tauri";
import { translateError } from "../i18n";
import { usePairingStore } from "./pairingStore";

/** Bir sohbet açılırken çekilen mesaj sayısı. */
const PAGE_SIZE = 60;

interface ChatState {
  /** Cihaz kimliğine göre mesaj listeleri. */
  byDevice: Record<string, ChatMessage[]>;
  /** Geçmişin başına ulaşılan sohbetler; boşuna sorgu atılmaz. */
  atStart: Record<string, boolean>;
  /** Açık olan sohbet; okundu bildirimi buna göre gönderilir. */
  activeDeviceId: string | null;
  loading: boolean;
  /** Geçmişe kaydırma sürüyor. */
  loadingOlder: boolean;
  error: string | null;

  open: (deviceId: string) => Promise<void>;
  /** Listenin başına gelindiğinde bir sayfa daha geçmiş yükler. */
  loadOlder: (deviceId: string) => Promise<number>;
  close: () => void;
  /** Sohbet listesinden seçim; sidebar ile içerik bu değeri paylaşır. */
  select: (deviceId: string) => void;
  send: (deviceId: string, body: string, isCode: boolean) => Promise<void>;
  messagesOf: (deviceId: string) => ChatMessage[];
}

const EMPTY: ChatMessage[] = [];

/** Durum ilerleme sırası; geriye düşüşü engeller. */
const STATUS_RANK: Record<ChatMessage["status"], number> = {
  sending: 0,
  sent: 1,
  delivered: 2,
  read: 3,
  failed: 0,
};

/**
 * Henüz listede olmayan mesajlar için gelen durumlar.
 *
 * Gerekli çünkü backend, mesaj `invoke` yanıtı arayüze dönmeden önce durum
 * olayı yayınlayabilir: bağlantı döngüsü çerçeveyi milisaniyeler içinde yazar.
 * Tamponlanmazsa o güncelleme sessizce kaybolur ve mesaj "gönderiliyor"da
 * takılı görünür.
 */
const pendingStatus = new Map<string, ChatMessage["status"]>();

/** İki durumdan ileride olanı seçer. */
function laterStatus(
  current: ChatMessage["status"],
  incoming: ChatMessage["status"],
): ChatMessage["status"] {
  if (incoming === "failed") return current === "sending" ? "failed" : current;
  if (current === "failed") return incoming;
  return STATUS_RANK[incoming] > STATUS_RANK[current] ? incoming : current;
}

export const useChatStore = create<ChatState>((set, get) => ({
  byDevice: {},
  atStart: {},
  activeDeviceId: null,
  loading: false,
  loadingOlder: false,
  error: null,

  open: async (deviceId) => {
    set({ activeDeviceId: deviceId, loading: true, error: null });
    try {
      const messages = await api.chatHistory(deviceId, PAGE_SIZE);
      set((state) => ({
        byDevice: { ...state.byDevice, [deviceId]: messages },
        // Bir sayfadan az geldiyse geçmişin tamamı elimizde.
        atStart: { ...state.atStart, [deviceId]: messages.length < PAGE_SIZE },
        loading: false,
      }));
      // Sohbet açıldı: gelen mesajlar okundu sayılır ve karşı tarafa bildirilir.
      await api.markConversationRead(deviceId);
    } catch (err) {
      set({ loading: false, error: translateError(err) });
    }
  },

  loadOlder: async (deviceId) => {
    const state = get();
    const current = state.byDevice[deviceId];
    if (state.loadingOlder || state.atStart[deviceId] || !current?.length) return 0;

    set({ loadingOlder: true });
    try {
      const older = await api.chatHistory(deviceId, PAGE_SIZE, current[0].id);
      set((inner) => ({
        byDevice: {
          ...inner.byDevice,
          // Araya yeni mesaj girmiş olabilir; birleştirme mevcut listeyi
          // yeniden okuyarak yapılır, `current` üzerinden değil.
          [deviceId]: [...older, ...(inner.byDevice[deviceId] ?? [])],
        },
        atStart: { ...inner.atStart, [deviceId]: older.length < PAGE_SIZE },
        loadingOlder: false,
      }));
      return older.length;
    } catch (err) {
      set({ error: translateError(err), loadingOlder: false });
      return 0;
    }
  },

  close: () => set({ activeDeviceId: null }),

  select: (deviceId) => {
    // Kimlik dışarıdan (bildirim yönlendirmesi) da gelebiliyor; boş bir
    // kimlikle sohbet açmaya çalışmak kullanıcıya anlamsız bir hata gösterir.
    if (!deviceId || get().activeDeviceId === deviceId) return;
    void get().open(deviceId);
  },

  send: async (deviceId, body, isCode) => {
    try {
      const message = await api.sendMessage(deviceId, body, isCode);
      void usePairingStore.getState().loadTrusted();

      // Mesaj listeye girmeden önce bir durum olayı gelmiş olabilir.
      const buffered = pendingStatus.get(message.msgId);
      if (buffered) {
        pendingStatus.delete(message.msgId);
        message.status = laterStatus(message.status, buffered);
      }

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

/**
 * Açık sohbetteki dosya baloncuklarının aktarım kayıtlarını tazeler.
 *
 * Mesajların kendisi yeniden yüklenmez: kullanıcının kaydırdığı geçmiş
 * silinmemeli. Yalnızca son sayfadaki `file_ref` kayıtlarının aktarım
 * bilgisi güncellenir.
 */
async function refreshTransfers(deviceId: string): Promise<void> {
  const messages = useChatStore.getState().byDevice[deviceId];
  if (!messages?.some((m) => m.transferId)) return;

  try {
    const fresh = await api.chatHistory(deviceId, PAGE_SIZE);
    const byTransfer = new Map(
      fresh.filter((m) => m.transferId).map((m) => [m.transferId, m.transfer] as const),
    );

    useChatStore.setState((state) => ({
      byDevice: {
        ...state.byDevice,
        [deviceId]: (state.byDevice[deviceId] ?? []).map((m) =>
          m.transferId && byTransfer.has(m.transferId)
            ? { ...m, transfer: byTransfer.get(m.transferId) ?? m.transfer }
            : m,
        ),
      },
    }));
  } catch {
    // Tazeleme başarısızsa baloncuk eski hâlinde kalır; kullanıcıya
    // gösterilecek bir hata yok.
  }
}

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

  // Aktarım ilerledikçe sohbetteki dosya baloncuğu da değişmeli. Baloncuk
  // aktarımın durumunu kopyalamadığı için tek yol kaydı yeniden okumaktır;
  // yalnızca AÇIK sohbet tazelenir, arka plandakiler zaten görünmüyor.
  void onTransferChanged(() => {
    const { activeDeviceId } = useChatStore.getState();
    if (!activeDeviceId) return;
    void refreshTransfers(activeDeviceId);
  }).then(track);

  void onChatStatus(({ deviceId, msgId, status }) => {
    const state = useChatStore.getState();
    const messages = state.byDevice[deviceId];

    if (!messages?.some((m) => m.msgId === msgId)) {
      // Mesaj henüz listede değil; durumu sakla, eklenince uygulanır.
      pendingStatus.set(msgId, status);
      return;
    }

    useChatStore.setState({
      byDevice: {
        ...state.byDevice,
        [deviceId]: messages.map((m) =>
          m.msgId === msgId ? { ...m, status: laterStatus(m.status, status) } : m,
        ),
      },
    });
  }).then(track);

  return () => {
    cancelled = true;
    unlisteners.forEach((fn) => fn());
  };
}
