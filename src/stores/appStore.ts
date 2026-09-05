import { create } from "zustand";
import { api, type AppInfo, type IdentityInfo } from "../lib/tauri";
import { translateError } from "../i18n";

interface AppState {
  info: AppInfo | null;
  identity: IdentityInfo | null;
  loading: boolean;
  error: string | null;
  load: () => Promise<void>;
}

export const useAppStore = create<AppState>((set) => ({
  info: null,
  identity: null,
  loading: true,
  error: null,

  load: async () => {
    set({ loading: true, error: null });
    try {
      const [info, identity] = await Promise.all([api.appInfo(), api.identityInfo()]);
      set({ info, identity, loading: false });
    } catch (err) {
      set({ error: translateError(err), loading: false });
    }
  },
}));
