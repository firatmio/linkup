import { useEffect } from "react";
import { HashRouter, Routes, Route } from "react-router-dom";
import { AppShell } from "./layout/AppShell";
import { Dashboard } from "./features/dashboard/Dashboard";
import { ChatsPage } from "./features/chat/ChatsPage";
import { IncomingFilesPage } from "./features/transfer/IncomingFilesPage";
import { SettingsPage } from "./features/settings/SettingsPage";
import { initTheme } from "./stores/uiStore";
import { useAppStore } from "./stores/appStore";

export default function App() {
  const loadAppInfo = useAppStore((s) => s.load);

  useEffect(() => initTheme(), []);
  useEffect(() => void loadAppInfo(), [loadAppInfo]);

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
