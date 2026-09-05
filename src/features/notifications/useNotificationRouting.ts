import { useEffect } from "react";
import { useNavigate } from "react-router-dom";
import { onNotificationActivated } from "../../lib/tauri";
import { useChatStore } from "../../stores/chatStore";

/**
 * Bildirime tıklandığında ilgili ekrana yönlendirir (PLAN.md §2.10).
 *
 * Pencereyi öne getirme işi backend'e ait: webview görünmezken JS
 * çalışmayabilir, dolayısıyla odaklama tıklamanın geldiği yerde yapılmalı.
 * Burada yalnızca yönlendirme kalıyor.
 */
export function useNotificationRouting() {
  const navigate = useNavigate();
  const select = useChatStore((s) => s.select);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;

    void onNotificationActivated((action) => {
      if (action.kind === "openChat") {
        select(action.deviceId);
        navigate("/chats");
      } else {
        navigate("/files");
      }
    }).then((fn) => {
      if (cancelled) fn();
      else unlisten = fn;
    });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [navigate, select]);
}
