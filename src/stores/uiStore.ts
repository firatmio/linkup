import { create } from "zustand";

export type ThemePreference = "system" | "light" | "dark";
export type ResolvedTheme = "light" | "dark";

/**
 * Faz 0'da tema tercihi localStorage'da tutulur. Faz 1'de `settings` tablosuna
 * taşınacak (PLAN.md §2.12) — store arayüzü aynı kalacağı için bileşenler
 * etkilenmez.
 */
const STORAGE_KEY = "linkup.theme";

function readStoredPreference(): ThemePreference {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw === "light" || raw === "dark" || raw === "system") return raw;
  } catch {
    // Depolama erişilemiyorsa sessizce varsayılana düş.
  }
  return "system";
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
  setThemePreference: (preference: ThemePreference) => void;
  /** Sistem teması değiştiğinde çağrılır; tercih "system" ise yeniden çözer. */
  syncWithSystem: () => void;
}

export const useUiStore = create<UiState>((set, get) => ({
  themePreference: readStoredPreference(),
  resolvedTheme: resolve(readStoredPreference()),

  setThemePreference: (preference) => {
    const resolved = resolve(preference);
    try {
      localStorage.setItem(STORAGE_KEY, preference);
    } catch {
      // Tercih kalıcı olmasa da oturum içinde uygulanmalı.
    }
    applyToDocument(resolved);
    set({ themePreference: preference, resolvedTheme: resolved });
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
