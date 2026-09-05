import { create } from "zustand";
import { api, type AppInfo } from "../lib/tauri";
import { translateError } from "../i18n";

interface AppState {
  info: AppInfo | null;
  loading: boolean;
  error: string | null;
  load: () => Promise<void>;
}

export const useAppStore = create<AppState>((set) => ({
  info: null,
  loading: true,
  error: null,

  load: async () => {
    set({ loading: true, error: null });
    try {
      set({ info: await api.appInfo(), loading: false });
    } catch (err) {
      set({ error: translateError(err), loading: false });
    }
  },
}));
