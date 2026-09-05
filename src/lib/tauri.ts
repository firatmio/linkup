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

/** Eşleşmiş, güvenilir cihaz (PLAN.md §2.12). */
export interface TrustedDevice {
  id: string;
  fingerprint: string;
  name: string;
  lastAddress: string | null;
  pairedAt: number;
  online: boolean;
  /** Açıkken bu cihazdan gelen dosyalar onay sorulmadan kabul edilir. */
  autoAccept: boolean;
  /** Sohbet listesinde gösterilen son mesaj özeti. */
  lastMessage: string | null;
  lastMessageAt: number | null;
  unread: number;
}

/** Sohbet mesajı (PLAN.md §2.8). */
export interface ChatMessage {
  msgId: string;
  direction: "in" | "out";
  contentType: "text" | "code" | "image" | "file_ref";
  content: string;
  sentAt: number;
  status: "sending" | "sent" | "delivered" | "read" | "failed";
}

export interface IncomingMessageEvent {
  deviceId: string;
  message: ChatMessage;
}

export interface MessageStatusEvent {
  deviceId: string;
  msgId: string;
  status: ChatMessage["status"];
}

/** Eşleştirme onayı istendiğinde gelen olay (PLAN.md §2.5). */
export interface PairingRequest {
  sessionId: string;
  deviceId: string;
  deviceName: string;
  fingerprint: string;
  /** Karşı ekranla karşılaştırılacak 6 haneli kod. */
  code: string;
  initiatedByUs: boolean;
}

export interface PairingFinished {
  sessionId: string;
  ok: boolean;
  /** Başarısızsa i18n anahtarı. */
  reason: string | null;
}


/** Dosya transferi kaydı (PLAN.md §2.7). */
export interface Transfer {
  transferId: string;
  deviceId: string;
  direction: "in" | "out";
  fileName: string;
  fileSize: number;
  mime: string | null;
  savePath: string | null;
  bytesDone: number;
  status: "pending" | "active" | "paused" | "done" | "failed" | "cancelled";
  error: string | null;
  startedAt: number;
  completedAt: number | null;
}

/** Gelen dosya onayı istendiğinde (PLAN.md §2.13.3). */
export interface TransferRequest {
  transferId: string;
  deviceId: string;
  deviceName: string;
  fileName: string;
  fileSize: number;
}

export interface TransferProgress {
  transferId: string;
  bytesDone: number;
  total: number;
  bytesPerSecond: number;
}

/** Bildirime tıklandığında nereye gidileceği. */
export type NotificationAction =
  | { kind: "openChat"; deviceId: string }
  | { kind: "openFiles" };

/** Backend'in yayınladığı olaylar. */
export const events = {
  discoveryChanged: "discovery:changed",
  devicesChanged: "devices:changed",
  devicesPresence: "devices:presence",
  pairingRequested: "pairing:requested",
  pairingFinished: "pairing:finished",
  chatMessage: "chat:message",
  chatStatus: "chat:status",
  transferProgress: "transfer:progress",
  transferChanged: "transfer:changed",
  transferRequested: "transfer:requested",
  transferResolved: "transfer:resolved",
  notificationActivated: "notification:activated",
} as const;

export function onNotificationActivated(
  handler: (action: NotificationAction) => void,
): Promise<UnlistenFn> {
  return listen<NotificationAction>(events.notificationActivated, (e) => handler(e.payload));
}

export function onTransferRequested(
  handler: (request: TransferRequest) => void,
): Promise<UnlistenFn> {
  return listen<TransferRequest>(events.transferRequested, (e) => handler(e.payload));
}

/** İstek başka bir yolla sonuçlandı (süre doldu veya iptal edildi). */
export function onTransferResolved(
  handler: (transferId: string) => void,
): Promise<UnlistenFn> {
  return listen<string>(events.transferResolved, (e) => handler(e.payload));
}

export function onTransferProgress(
  handler: (event: TransferProgress) => void,
): Promise<UnlistenFn> {
  return listen<TransferProgress>(events.transferProgress, (e) => handler(e.payload));
}

export function onTransferChanged(handler: () => void): Promise<UnlistenFn> {
  return listen(events.transferChanged, () => handler());
}

export function onChatMessage(
  handler: (event: IncomingMessageEvent) => void,
): Promise<UnlistenFn> {
  return listen<IncomingMessageEvent>(events.chatMessage, (e) => handler(e.payload));
}

export function onChatStatus(
  handler: (event: MessageStatusEvent) => void,
): Promise<UnlistenFn> {
  return listen<MessageStatusEvent>(events.chatStatus, (e) => handler(e.payload));
}

export function onPairingRequested(
  handler: (request: PairingRequest) => void,
): Promise<UnlistenFn> {
  return listen<PairingRequest>(events.pairingRequested, (e) => handler(e.payload));
}

export function onPairingFinished(
  handler: (result: PairingFinished) => void,
): Promise<UnlistenFn> {
  return listen<PairingFinished>(events.pairingFinished, (e) => handler(e.payload));
}

/** Güvenilir cihaz listesi veya çevrimiçilik değiştiğinde tetiklenir. */
export function onDevicesChanged(handler: () => void): Promise<UnlistenFn[]> {
  return Promise.all([
    listen(events.devicesChanged, () => handler()),
    listen(events.devicesPresence, () => handler()),
  ]);
}

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
  trustedDevices: () => invoke<TrustedDevice[]>("trusted_devices"),
  /** Eşleştirmeyi başlatır; akış bitene kadar (en fazla 90 sn) bekler. */
  startPairing: (id: string) => invoke<void>("start_pairing", { id }),
  respondToPairing: (sessionId: string, accept: boolean) =>
    invoke<boolean>("respond_to_pairing", { sessionId, accept }),
  forgetDevice: (id: string) => invoke<boolean>("forget_device", { id }),
  setDeviceAutoAccept: (id: string, enabled: boolean) =>
    invoke<void>("set_device_auto_accept", { id, enabled }),
  clearFinishedTransfers: () => invoke<number>("clear_finished_transfers"),
  chatHistory: (id: string, limit?: number) =>
    invoke<ChatMessage[]>("chat_history", { id, limit }),
  sendMessage: (id: string, body: string, isCode?: boolean) =>
    invoke<ChatMessage>("send_message", { id, body, isCode }),
  markConversationRead: (id: string) =>
    invoke<number>("mark_conversation_read", { id }),
  sendFile: (id: string, path: string) => invoke<string>("send_file", { id, path }),
  respondToTransfer: (transferId: string, accept: boolean) =>
    invoke<boolean>("respond_to_transfer", { transferId, accept }),
  incomingFiles: (limit?: number) => invoke<Transfer[]>("incoming_files", { limit }),
  activeTransfers: () => invoke<Transfer[]>("active_transfers"),
  openTransferFile: (transferId: string) =>
    invoke<void>("open_transfer_file", { transferId }),
  revealTransferFile: (transferId: string) =>
    invoke<void>("reveal_transfer_file", { transferId }),
  openLogDir: () => invoke<void>("open_log_dir"),
};
