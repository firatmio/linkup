import { create } from "zustand";
import {
  api,
  onTransferChanged,
  onTransferProgress,
  type Transfer,
  type TransferProgress,
} from "../lib/tauri";
import { translateError } from "../i18n";

interface TransferState {
  /** Sürmekte olan aktarımlar. */
  active: Transfer[];
  /** Alınan dosyaların geçmişi. */
  incoming: Transfer[];
  /** Anlık ilerleme — veritabanına her saniye yazmak yerine bellekte tutulur. */
  progress: Record<string, TransferProgress>;
  error: string | null;

  load: () => Promise<void>;
  send: (deviceId: string, path: string) => Promise<boolean>;
  clearError: () => void;
}

export const useTransferStore = create<TransferState>((set) => ({
  active: [],
  incoming: [],
  progress: {},
  error: null,

  load: async () => {
    try {
      const [active, incoming] = await Promise.all([
        api.activeTransfers(),
        api.incomingFiles(),
      ]);
      // İlerleme kayıtları sonlanan aktarımlarla birlikte atılır: kalan bir
      // kayıt, veritabanında artık var olmayan bir aktarımı %100 dolu bir
      // çubukla ekranda tutabiliyordu.
      set((state) => ({
        active,
        incoming,
        progress: Object.fromEntries(
          active
            .map((item) => [item.transferId, state.progress[item.transferId]] as const)
            .filter((entry): entry is [string, TransferProgress] => Boolean(entry[1])),
        ),
      }));
    } catch (err) {
      set({ error: translateError(err) });
    }
  },

  send: async (deviceId, path) => {
    try {
      await api.sendFile(deviceId, path);
      return true;
    } catch (err) {
      set({ error: translateError(err) });
      return false;
    }
  },

  clearError: () => set({ error: null }),
}));

/** Transfer olaylarına abone olur. */
export function subscribeToTransfers(): () => void {
  const unlisteners: Array<() => void> = [];
  let cancelled = false;

  const track = (fn: () => void) => {
    if (cancelled) fn();
    else unlisteners.push(fn);
  };

  void onTransferProgress((event) => {
    // İlerleme saniyede iki kez gelir; listeyi yeniden çekmek yerine
    // yalnızca ilgili kaydın anlık değeri güncellenir.
    useTransferStore.setState((state) => ({
      progress: { ...state.progress, [event.transferId]: event },
    }));
  }).then(track);

  void onTransferChanged(() => {
    void useTransferStore.getState().load();
  }).then(track);

  return () => {
    cancelled = true;
    unlisteners.forEach((fn) => fn());
  };
}
