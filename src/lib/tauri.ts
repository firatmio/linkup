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

export const api = {
  appInfo: () => invoke<AppInfo>("app_info"),
  openLogDir: () => invoke<void>("open_log_dir"),
};
