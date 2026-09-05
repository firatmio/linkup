import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

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
  /** Karşı cihazda elle ekleme için kullanılacak adresler. */
  reachableAddresses: string[];
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

/** Cihazın nasıl bulunduğu (PLAN.md §2.4). */
export type DiscoverySource = "mdns" | "manual";

export interface DiscoveredDevice {
  /** Base32 kodlu device_id. */
  id: string;
  /** Kullanıcıya gösterilen gruplanmış fingerprint. */
  fingerprint: string;
  name: string;
  address: string | null;
  protocolVersion: number;
  source: DiscoverySource;
}

/** Backend'in yayınladığı olaylar. */
export const events = {
  discoveryChanged: "discovery:changed",
} as const;

export function onDiscoveryChanged(
  handler: (devices: DiscoveredDevice[]) => void,
): Promise<UnlistenFn> {
  return listen<DiscoveredDevice[]>(events.discoveryChanged, (event) =>
    handler(event.payload),
  );
}

export const api = {
  appInfo: () => invoke<AppInfo>("app_info"),
  identityInfo: () => invoke<IdentityInfo>("identity_info"),
  getSettings: () => invoke<Settings>("get_settings"),
  /** Ayarı yazar ve güncel anlık görüntüyü döndürür. */
  setSetting: (key: SettingKey, value: string) =>
    invoke<Settings>("set_setting", { key, value }),
  discoveredDevices: () => invoke<DiscoveredDevice[]>("discovered_devices"),
  addDeviceManually: (address: string) =>
    invoke<DiscoveredDevice>("add_device_manually", { address }),
  forgetDiscoveredDevice: (id: string) =>
    invoke<boolean>("forget_discovered_device", { id }),
  openLogDir: () => invoke<void>("open_log_dir"),
};
