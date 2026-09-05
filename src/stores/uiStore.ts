import { create } from "zustand";
import { api } from "../lib/tauri";
import { translateError } from "../i18n";

export type ThemePreference = "system" | "light" | "dark";
export type ResolvedTheme = "light" | "dark";

/**
 * Tema tercihinin kaynağı `settings` tablosudur (PLAN.md §2.12).
 *
 * localStorage yalnızca bir ÖNBELLEK: veritabanı okuması asenkron olduğu için,
 * ilk boyamada yanlış temayla yanıp sönmeyi (flash) önler. Çelişki hâlinde
 * veritabanı kazanır ve önbellek güncellenir.
 */
const CACHE_KEY = "linkup.theme";

function isPreference(value: unknown): value is ThemePreference {
  return value === "system" || value === "light" || value === "dark";
}

function readCache(): ThemePreference {
  try {
    const raw = localStorage.getItem(CACHE_KEY);
    if (isPreference(raw)) return raw;
  } catch {
    // Depolama erişilemiyorsa sessizce varsayılana düş.
  }
  return "system";
}

function writeCache(preference: ThemePreference) {
  try {
    localStorage.setItem(CACHE_KEY, preference);
  } catch {
    // Önbellek yazılamazsa da tema oturum içinde uygulanır.
  }
}

function systemTheme(): ResolvedTheme {
  return window.matchMedia?.("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

function resolve(preference: ThemePreference): ResolvedTheme {
  return preference === "system" ? systemTheme() : preference;
}

function applyToDocument(theme: ResolvedTheme) {
  document.documentElement.setAttribute("data-theme", theme);
}

interface UiState {
  themePreference: ThemePreference;
  resolvedTheme: ResolvedTheme;
  /** Tema yazılırken doğru; UI kontrolü bu sırada devre dışı kalır. */
  savingTheme: boolean;
  error: string | null;

  /** Veritabanındaki tercihi okur ve uygular. */
  hydrate: () => Promise<void>;
  setThemePreference: (preference: ThemePreference) => Promise<void>;
  /** Sistem teması değiştiğinde çağrılır; tercih "system" ise yeniden çözer. */
  syncWithSystem: () => void;
}

export const useUiStore = create<UiState>((set, get) => ({
  themePreference: readCache(),
  resolvedTheme: resolve(readCache()),
  savingTheme: false,
  error: null,

  hydrate: async () => {
    try {
      const settings = await api.getSettings();
      const preference = isPreference(settings.theme) ? settings.theme : "system";
      const resolved = resolve(preference);
      writeCache(preference);
      applyToDocument(resolved);
      set({ themePreference: preference, resolvedTheme: resolved, error: null });
    } catch (err) {
      // Önbellekten uygulanan tema yerinde kalır; kullanıcı karanlıkta kalmaz.
      set({ error: translateError(err) });
    }
  },

  setThemePreference: async (preference) => {
    const previous = get().themePreference;
    const resolved = resolve(preference);

    // İyimser uygulama: tema anında değişir, yazma arkada tamamlanır.
    applyToDocument(resolved);
    set({ themePreference: preference, resolvedTheme: resolved, savingTheme: true, error: null });

    try {
      await api.setSetting("theme", preference);
      writeCache(preference);
      set({ savingTheme: false });
    } catch (err) {
      // Kalıcı olmadıysa göstermek yanıltıcı olur — eski tercihe geri dönülür.
      const revertedResolved = resolve(previous);
      applyToDocument(revertedResolved);
      set({
        themePreference: previous,
        resolvedTheme: revertedResolved,
        savingTheme: false,
        error: translateError(err),
      });
    }
  },

  syncWithSystem: () => {
    if (get().themePreference !== "system") return;
    const resolved = systemTheme();
    applyToDocument(resolved);
    set({ resolvedTheme: resolved });
  },
}));

/** Uygulama açılışında bir kez çağrılır; sistem teması dinleyicisini kurar. */
export function initTheme(): () => void {
  applyToDocument(useUiStore.getState().resolvedTheme);

  const media = window.matchMedia?.("(prefers-color-scheme: dark)");
  if (!media) return () => {};

  const listener = () => useUiStore.getState().syncWithSystem();
  media.addEventListener("change", listener);
  return () => media.removeEventListener("change", listener);
}
