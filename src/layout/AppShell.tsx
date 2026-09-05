import type { ReactNode } from "react";
import { useLocation } from "react-router-dom";
import { AppSidebar } from "./AppSidebar";
import { TitleBar } from "./TitleBar";
import { ConversationList } from "../features/chat/ConversationList";
import { PairingDialog } from "../features/devices/PairingDialog";
import { TransferRequestDialog } from "../features/transfer/TransferRequestDialog";

/**
 * Uygulama iskeleti: solda sabit sütun, sağda içerik katmanı.
 *
 * Windows 11 deseni — içerik, Mica zeminin üstünde sol üstü yuvarlatılmış
 * ayrı bir katman olarak durur.
 *
 * Sohbetler bölümünde sol sütun navigasyondan sohbet listesine devrolur:
 * konuşurken ekranın tamamı konuşmaya ayrılır, listeyi ikinci bir sütuna
 * sıkıştırmak yerine.
 */
export function AppShell({ children }: { children: ReactNode }) {
  const inChat = useLocation().pathname.startsWith("/chats");

  return (
    // Pencere `decorations: false`: başlık çubuğu bize ait, o yüzden dikey
    // yığın en üstte onunla başlıyor.
    <div className="flex h-full w-full flex-col overflow-hidden bg-base">
      <TitleBar />
      <div className="flex min-h-0 flex-1 overflow-hidden">
        {inChat ? <ConversationList /> : <AppSidebar />}
        <main className="flex min-w-0 flex-1 flex-col overflow-hidden rounded-tl-lu-lg border-t border-l border-stroke bg-layer">
          {children}
        </main>
      </div>
      {/* Eşleştirme ve dosya istekleri hangi ekranda olursak olalım öne çıkmalı. */}
      <PairingDialog />
      <TransferRequestDialog />
    </div>
  );
}
