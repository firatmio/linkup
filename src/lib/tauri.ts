import { invoke } from "@tauri-apps/api/core";

/**
 * Backend API'sinin tip güvenli yüzeyi (PLAN.md §2.1).
 * Bileşenler `invoke("...")` çağırmaz; buradaki fonksiyonları kullanır.
 */

export interface AppInfo {
  version: string;
  profile: string | null;
  dataDir: string;
  logDir: string;
  downloadsDir: string;
  dbPath: string;
  quicPort: number;
  os: string;
}

/** Rust tarafındaki `ErrorPayload` ile eşleşir. */
export interface BackendError {
  code: string;
  detail: string;
}

/** Kimlik anahtarının nerede saklandığı (PLAN.md §2.6). */
export type KeyStorage = "osKeychain" | "plainFile";

export interface IdentityInfo {
  /** 4'erli gruplanmış base32 fingerprint. */
  fingerprint: string;
  storage: KeyStorage;
}

/** Rust tarafındaki `db::settings::Settings` ile eşleşir. */
export interface Settings {
  theme: string;
  deviceName: string;
}

/** Yazılabilir ayar anahtarları — Rust tarafındaki DEFAULTS listesiyle eşleşir. */
export type SettingKey = keyof Settings;

export const api = {
  appInfo: () => invoke<AppInfo>("app_info"),
  identityInfo: () => invoke<IdentityInfo>("identity_info"),
  getSettings: () => invoke<Settings>("get_settings"),
  /** Ayarı yazar ve güncel anlık görüntüyü döndürür. */
  setSetting: (key: SettingKey, value: string) =>
    invoke<Settings>("set_setting", { key, value }),
  openLogDir: () => invoke<void>("open_log_dir"),
};
