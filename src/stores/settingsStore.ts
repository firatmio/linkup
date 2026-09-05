import { create } from "zustand";
import { api, type Settings, type SettingKey } from "../lib/tauri";
import { translateError } from "../i18n";

/**
 * Ayarların tamamı (tema hariç).
 *
 * Tema `uiStore`da ayrı duruyor çünkü ilk boyamadan ÖNCE gerekiyor ve
 * localStorage önbelleğiyle çalışıyor; buradaki ayarların böyle bir aciliyeti
 * yok, doğrudan veritabanından okunuyorlar.
 */
interface SettingsState {
  settings: Settings | null;
  /** Bir yazma sürüyor; ilgili kontroller bu sırada devre dışı kalır. */
  saving: boolean;
  error: string | null;

  load: () => Promise<void>;
  setCloseToTray: (enabled: boolean) => Promise<void>;
  setAutostart: (enabled: boolean) => Promise<void>;
  setGlobalShortcut: (accelerator: string) => Promise<void>;
  /** Basit anahtar yazma; değeri olduğu gibi gönderir. */
  set: (key: SettingKey, value: string) => Promise<void>;
  setFlag: (key: SettingKey, enabled: boolean) => Promise<void>;
  setReadSelection: (enabled: boolean) => Promise<void>;
}

export const useSettingsStore = create<SettingsState>((set) => ({
  settings: null,
  saving: false,
  error: null,

  load: async () => {
    try {
      set({ settings: await api.getSettings(), error: null });
    } catch (err) {
      set({ error: translateError(err) });
    }
  },

  set: async (key, value) => {
    set({ saving: true, error: null });
    try {
      set({ settings: await api.setSetting(key, value), saving: false });
    } catch (err) {
      set({ error: translateError(err), saving: false });
    }
  },

  setFlag: async (key, enabled) => {
    await useSettingsStore.getState().set(key, enabled ? "1" : "0");
  },

  setCloseToTray: async (enabled) => {
    set({ saving: true, error: null });
    try {
      const settings = await api.setSetting("closeToTray", enabled ? "1" : "0");
      set({ settings, saving: false });
    } catch (err) {
      set({ error: translateError(err), saving: false });
    }
  },

  setReadSelection: async (enabled) => {
    set({ saving: true, error: null });
    try {
      const settings = await api.setSetting("quickSendReadSelection", enabled ? "1" : "0");
      set({ settings, saving: false });
    } catch (err) {
      set({ error: translateError(err), saving: false });
    }
  },

  setAutostart: async (enabled) => {
    set({ saving: true, error: null });
    try {
      // İşletim sistemi kaydı ile ayar birlikte yazılır; kayıt başarısız
      // olursa ayar da yazılmaz ve anahtar eski hâlinde kalır.
      const settings = await api.setAutostart(enabled);
      set({ settings, saving: false });
    } catch (err) {
      set({ error: translateError(err), saving: false });
    }
  },

  setGlobalShortcut: async (accelerator) => {
    set({ saving: true, error: null });
    try {
      // Kayıt başarısızsa backend ayarı YAZMIYOR; hata mesajı kullanıcıya
      // kombinasyonun alınmış olduğunu söylüyor ve eski kısayol duruyor.
      const settings = await api.setGlobalShortcut(accelerator);
      set({ settings, saving: false });
    } catch (err) {
      set({ error: translateError(err), saving: false });
    }
  },
}));
