import { useEffect } from "react";
import { HashRouter, Routes, Route } from "react-router-dom";
import { AppShell } from "./layout/AppShell";
import { Dashboard } from "./features/dashboard/Dashboard";
import { ChatsPage } from "./features/chat/ChatsPage";
import { IncomingFilesPage } from "./features/transfer/IncomingFilesPage";
import { SettingsPage } from "./features/settings/SettingsPage";
import { initTheme, useUiStore } from "./stores/uiStore";
import { useAppStore } from "./stores/appStore";
import { subscribeToDiscovery, useDeviceStore } from "./stores/deviceStore";
import { subscribeToPairing, usePairingStore } from "./stores/pairingStore";
import { subscribeToChat } from "./stores/chatStore";

export default function App() {
  const loadAppInfo = useAppStore((s) => s.load);
  const hydrateTheme = useUiStore((s) => s.hydrate);

  useEffect(() => initTheme(), []);
  useEffect(() => void loadAppInfo(), [loadAppInfo]);
  // Tema önbellekten anında uygulandı; veritabanındaki tercih onu doğrular.
  useEffect(() => void hydrateTheme(), [hydrateTheme]);

  // Keşif: önce mevcut liste çekilir, sonra değişiklikler olayla akar.
  const loadDevices = useDeviceStore((s) => s.load);
  useEffect(() => void loadDevices(), [loadDevices]);
  useEffect(() => subscribeToDiscovery(), []);

  // Eşleşmiş cihazlar ve eşleştirme istekleri.
  const loadTrusted = usePairingStore((s) => s.loadTrusted);
  useEffect(() => void loadTrusted(), [loadTrusted]);
  useEffect(() => subscribeToPairing(), []);
  useEffect(() => subscribeToChat(), []);

  return (
    // Masaüstü uygulaması dosya protokolünden servis edildiği için HashRouter.
    <HashRouter>
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
