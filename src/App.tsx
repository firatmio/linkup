import { useEffect } from "react";
import { HashRouter, Routes, Route } from "react-router-dom";
import { AppShell } from "./layout/AppShell";
import { Dashboard } from "./features/dashboard/Dashboard";
import { ChatsPage } from "./features/chat/ChatsPage";
import { IncomingFilesPage } from "./features/transfer/IncomingFilesPage";
import { SettingsPage } from "./features/settings/SettingsPage";
import { QuickSendWindow } from "./features/quick/QuickSendWindow";
import { initTheme, useUiStore } from "./stores/uiStore";
import { useAppStore } from "./stores/appStore";
import { useSettingsStore } from "./stores/settingsStore";
import { useUpdateStore } from "./stores/updateStore";
import { ReleaseNotesDialog } from "./features/updates/ReleaseNotesDialog";
import { subscribeToDiscovery, useDeviceStore } from "./stores/deviceStore";
import { subscribeToPairing, usePairingStore } from "./stores/pairingStore";
import { subscribeToChat } from "./stores/chatStore";
import { subscribeToTransfers, useTransferStore } from "./stores/transferStore";
import { useNotificationRouting } from "./features/notifications/useNotificationRouting";
import { disableWebViewInteractions } from "./disabled";

/** Router bağlamı gerektirdiği için ayrı bileşen. */
function NotificationRouting() {
  useNotificationRouting();
  return null;
}

export default function App() {
  disableWebViewInteractions();
  const loadAppInfo = useAppStore((s) => s.load);
  const hydrateTheme = useUiStore((s) => s.hydrate);

  useEffect(() => initTheme(), []);
  useEffect(() => void loadAppInfo(), [loadAppInfo]);
  // Tema önbellekten anında uygulandı; veritabanındaki tercih onu doğrular.
  useEffect(() => void hydrateTheme(), [hydrateTheme]);

  const loadSettings = useSettingsStore((s) => s.load);
  useEffect(() => void loadSettings(), [loadSettings]);

  // Güncelleme kontrolü açılışta bir kez. Periyodik kontrol eklenmedi:
  // uygulama günlerce açık kalabiliyor ama güncelleme yeniden başlatma
  // gerektirdiği için sık sık sormanın faydası yok.
  const checkUpdate = useUpdateStore((s) => s.check);
  useEffect(() => void checkUpdate(), [checkUpdate]);

  // Keşif: önce mevcut liste çekilir, sonra değişiklikler olayla akar.
  const loadDevices = useDeviceStore((s) => s.load);
  useEffect(() => void loadDevices(), [loadDevices]);
  useEffect(() => subscribeToDiscovery(), []);

  // Eşleşmiş cihazlar ve eşleştirme istekleri.
  const loadTrusted = usePairingStore((s) => s.loadTrusted);
  useEffect(() => void loadTrusted(), [loadTrusted]);
  useEffect(() => subscribeToPairing(), []);
  useEffect(() => subscribeToChat(), []);

  // Dosya aktarımları.
  const loadTransfers = useTransferStore((s) => s.load);
  useEffect(() => void loadTransfers(), [loadTransfers]);
  useEffect(() => subscribeToTransfers(), []);

  // Hızlı gönder penceresi aynı bundle'ı yükler ama uygulamanın kabuğunu
  // KULLANMAZ: kenar çubuğu, keşif abonelikleri ve store'lar orada gereksiz
  // iş olurdu. Router'dan önce ayrılıyor.
  if (window.location.hash.startsWith("#/quick-send")) {
    return <QuickSendWindow />;
  }

  return (
    // Masaüstü uygulaması dosya protokolünden servis edildiği için HashRouter.
    <HashRouter>
      <NotificationRouting />
      <ReleaseNotesDialog />
      <AppShell>
        <Routes>
          <Route path="/" element={<Dashboard />} />
          <Route path="/chats" element={<ChatsPage />} />
          <Route path="/files" element={<IncomingFilesPage />} />
          <Route path="/settings" element={<SettingsPage />} />
        </Routes>
      </AppShell>
    </HashRouter>
  );
}
