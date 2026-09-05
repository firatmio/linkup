import { create } from "zustand";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { api } from "../lib/tauri";
import { useSettingsStore } from "./settingsStore";

/**
 * Güncelleme durumu.
 *
 * `idle` ile `upToDate` ayrı: ilki "henüz bakılmadı", ikincisi "bakıldı, yeni
 * sürüm yok". Kenar çubuğunda ikisi de bir şey göstermez ama ayrımı korumak,
 * "kontrol hiç çalışmadı mı yoksa çalıştı da mı bir şey bulunamadı" sorusunu
 * yanıtlanabilir kılıyor.
 */
export type UpdatePhase =
  | "idle"
  | "checking"
  | "upToDate"
  | "downloading"
  | "ready"
  | "error";

interface UpdateState {
  phase: UpdatePhase;
  /** Bulunan sürüm (varsa). */
  version: string | null;
  /** İndirme yüzdesi; toplam boyut bilinmiyorsa null. */
  progress: number | null;
  /** Kullanıcıya gösterilmeyen teknik hata; yalnızca konsol/teşhis için. */
  detail: string | null;

  check: () => Promise<void>;
  installAndRestart: () => Promise<void>;
}

/**
 * İndirilmiş güncelleme nesnesi.
 *
 * Store'un dışında tutuluyor: içinde fonksiyonlar var ve Zustand durumuna
 * konursa her render'da taşınan, serileştirilemeyen bir yük olur.
 */
let pending: Update | null = null;

export const useUpdateStore = create<UpdateState>((set, get) => ({
  phase: "idle",
  version: null,
  progress: null,
  detail: null,

  check: async () => {
    if (get().phase === "checking" || get().phase === "downloading") return;
    set({ phase: "checking", detail: null });

    try {
      const update = await check();
      if (!update) {
        set({ phase: "upToDate", version: null });
        return;
      }
      pending = update;
      // İndirme hemen başlar: kullanıcıya "güncelleme var" deyip sonra
      // beklettirmek yerine, yeniden başlat dediğinde hazır olsun.
      set({ phase: "downloading", version: update.version, progress: 0 });

      let downloaded = 0;
      let total = 0;
      await update.download((event) => {
        if (event.event === "Started") {
          total = event.data.contentLength ?? 0;
        } else if (event.event === "Progress") {
          downloaded += event.data.chunkLength;
          set({ progress: total > 0 ? Math.round((downloaded / total) * 100) : null });
        }
      });

      set({ phase: "ready", progress: 100 });
    } catch (err) {
      // Geliştirme yapılarında ve yayın yokken burası normal şekilde
      // başarısız olur; kullanıcıya hata göstermenin bir anlamı yok.
      set({ phase: "error", detail: String(err) });
    }
  },

  installAndRestart: async () => {
    if (!pending) return;

    // Notlar KURULUMDAN ÖNCE yazılır: kurulumdan sonra bu süreç zaten
    // sonlanmış oluyor, yazacak kimse kalmıyor.
    try {
      await api.setSetting(
        "pendingReleaseNotes",
        JSON.stringify({ version: pending.version, notes: pending.body ?? "" }),
      );
      await useSettingsStore.getState().load();
    } catch {
      // Not yazılamadıysa güncelleme yine de kurulmalı; kaybedilen şey
      // yalnızca "yenilikler" penceresi.
    }

    try {
      await pending.install();
      await relaunch();
    } catch (err) {
      set({ phase: "error", detail: String(err) });
    }
  },
}));
