import { create } from "zustand";
import {
  api,
  onPairingFinished,
  onPairingRequested,
  onDevicesChanged,
  type PairingRequest,
  type TrustedDevice,
} from "../lib/tauri";
import { t, translateError, type TranslationKey } from "../i18n";

interface PairingState {
  /** Ekranda gösterilen eşleştirme isteği; yoksa dialog kapalı. */
  request: PairingRequest | null;
  /** Kullanıcı karar verdi, karşı taraf bekleniyor. */
  waitingForPeer: boolean;
  /** Eşleştirme başlatılıyor (bağlantı kuruluyor). */
  starting: boolean;
  /** Son sonucun kullanıcıya gösterilecek metni. */
  message: string | null;
  messageIsError: boolean;

  trusted: TrustedDevice[];

  loadTrusted: () => Promise<void>;
  start: (deviceId: string) => Promise<void>;
  respond: (accept: boolean) => Promise<void>;
  forget: (deviceId: string) => Promise<void>;
  dismissMessage: () => void;
}

/** Backend'den gelen hata kodunu sözlükten çevirir. */
function translateReason(reason: string | null): string {
  if (!reason) return t("error.unknown");
  return t(reason as TranslationKey);
}

export const usePairingStore = create<PairingState>((set, get) => ({
  request: null,
  waitingForPeer: false,
  starting: false,
  message: null,
  messageIsError: false,
  trusted: [],

  loadTrusted: async () => {
    try {
      set({ trusted: await api.trustedDevices() });
    } catch (err) {
      set({ message: translateError(err), messageIsError: true });
    }
  },

  start: async (deviceId) => {
    set({ starting: true, message: null });
    try {
      // Bu çağrı eşleştirme bitene kadar bekler; dialog bu sırada
      // `pairing:requested` olayıyla açılır.
      await api.startPairing(deviceId);
    } catch (err) {
      // Sonuç olayı da gelir; burada yalnızca bağlanamama gibi erken
      // hatalar yakalanır.
      set({ message: translateError(err), messageIsError: true });
    } finally {
      set({ starting: false });
    }
  },

  respond: async (accept) => {
    const request = get().request;
    if (!request) return;
    await api.respondToPairing(request.sessionId, accept);
    if (accept) {
      // Dialog açık kalır: eşleşme ancak karşı taraf da onaylayınca biter.
      set({ waitingForPeer: true });
    } else {
      set({ request: null, waitingForPeer: false });
    }
  },

  forget: async (deviceId) => {
    await api.forgetDevice(deviceId);
    await get().loadTrusted();
  },

  dismissMessage: () => set({ message: null, messageIsError: false }),
}));

/** Eşleştirme ve cihaz olaylarına abone olur. */
export function subscribeToPairing(): () => void {
  const unlisteners: Array<() => void> = [];
  let cancelled = false;

  const track = (fn: () => void) => {
    if (cancelled) fn();
    else unlisteners.push(fn);
  };

  void onPairingRequested((request) => {
    usePairingStore.setState({
      request,
      waitingForPeer: false,
      message: null,
      messageIsError: false,
    });
  }).then(track);

  void onPairingFinished((result) => {
    usePairingStore.setState({
      request: null,
      waitingForPeer: false,
      message: result.ok ? t("pairing.success") : translateReason(result.reason),
      messageIsError: !result.ok,
    });
    void usePairingStore.getState().loadTrusted();
  }).then(track);

  void onDevicesChanged(() => {
    void usePairingStore.getState().loadTrusted();
  }).then((fns) => fns.forEach(track));

  return () => {
    cancelled = true;
    unlisteners.forEach((fn) => fn());
  };
}
