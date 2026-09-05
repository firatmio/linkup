import { create } from "zustand";
import {
  api,
  onDiscoveryChanged,
  type DiscoveredDevice,
} from "../lib/tauri";
import { translateError } from "../i18n";

interface DeviceState {
  discovered: DiscoveredDevice[];
  loading: boolean;
  /** Elle ekleme sürerken doğru; formu kilitler. */
  adding: boolean;
  /** Elle ekleme hatası — dialog içinde gösterilir. */
  addError: string | null;

  load: () => Promise<void>;
  addManually: (address: string) => Promise<boolean>;
  forget: (id: string) => Promise<void>;
  clearAddError: () => void;
}

export const useDeviceStore = create<DeviceState>((set, get) => ({
  discovered: [],
  loading: true,
  adding: false,
  addError: null,

  load: async () => {
    try {
      set({ discovered: await api.discoveredDevices(), loading: false });
    } catch {
      // Keşif listesi boş kalabilir; bu, uygulamanın geri kalanını
      // engellemeyecek kadar önemsiz bir hata.
      set({ loading: false });
    }
  },

  addManually: async (address) => {
    set({ adding: true, addError: null });
    try {
      const device = await api.addDeviceManually(address);
      // Backend olay yayınlamıyor (ekleme senkron bir komut); listeyi
      // yerelde güncelleyip tazeliği bir sonraki olaya bırakıyoruz.
      const others = get().discovered.filter((d) => d.id !== device.id);
      set({
        discovered: [...others, device].sort((a, b) => a.name.localeCompare(b.name, "tr")),
        adding: false,
      });
      return true;
    } catch (err) {
      set({ addError: translateError(err), adding: false });
      return false;
    }
  },

  forget: async (id) => {
    await api.forgetDiscoveredDevice(id);
    set({ discovered: get().discovered.filter((d) => d.id !== id) });
  },

  clearAddError: () => set({ addError: null }),
}));

/**
 * Keşif olaylarına abone olur. Uygulama açılışında bir kez çağrılır;
 * döndürdüğü fonksiyon aboneliği bırakır.
 */
export function subscribeToDiscovery(): () => void {
  let unlisten: (() => void) | undefined;
  let cancelled = false;

  void onDiscoveryChanged((devices) => {
    useDeviceStore.setState({ discovered: devices, loading: false });
  }).then((fn) => {
    // Abonelik kurulmadan bileşen söküldüyse hemen bırak.
    if (cancelled) fn();
    else unlisten = fn;
  });

  return () => {
    cancelled = true;
    unlisten?.();
  };
}
