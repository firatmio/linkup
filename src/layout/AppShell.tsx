import type { ReactNode } from "react";
import { AppSidebar } from "./AppSidebar";
import { PairingDialog } from "../features/devices/PairingDialog";

/**
 * Uygulama iskeleti: solda sabit navigasyon, sağda içerik katmanı.
 *
 * Windows 11 deseni — içerik, Mica zeminin üstünde sol üstü yuvarlatılmış
 * ayrı bir katman olarak durur.
 */
export function AppShell({ children }: { children: ReactNode }) {
  return (
    <div className="flex h-full w-full overflow-hidden bg-base">
      <AppSidebar />
      <main className="flex min-w-0 flex-1 flex-col overflow-hidden rounded-tl-lu-lg border-t border-l border-stroke bg-layer">
        {children}
      </main>
      {/* Eşleştirme isteği hangi ekranda olursak olalım öne çıkmalı. */}
      <PairingDialog />
    </div>
  );
}
