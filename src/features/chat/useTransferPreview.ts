import { useEffect, useState } from "react";
import { api, type TransferPreview } from "../../lib/tauri";

/**
 * Aynı aktarımın önizlemesi bir kez üretilir.
 *
 * Küçültme işi backend'de yüz milisaniyeler sürebiliyor; sohbette yukarı
 * aşağı kaydırmak her seferinde yeniden üretime yol açmamalı. `null`, "bu
 * dosya görsel değil" demektir ve o da önbelleğe alınır — aksi hâlde her PDF
 * için tekrar tekrar sorulurdu.
 */
const cache = new Map<string, TransferPreview | null>();
const inFlight = new Map<string, Promise<TransferPreview | null>>();

function load(transferId: string, maxEdge: number): Promise<TransferPreview | null> {
  const key = `${transferId}@${maxEdge}`;
  const cached = cache.get(key);
  if (cached !== undefined) return Promise.resolve(cached);

  const existing = inFlight.get(key);
  if (existing) return existing;

  const request = api
    .transferPreview(transferId, maxEdge)
    .then((preview) => {
      cache.set(key, preview);
      return preview;
    })
    .catch(() => null)
    .finally(() => inFlight.delete(key));

  inFlight.set(key, request);
  return request;
}

/**
 * Tamamlanmış bir aktarımın önizlemesini getirir.
 *
 * `enabled` yanlışken hiç istek atılmaz: aktarım sürerken dosya henüz diskte
 * tamamlanmamıştır ve yarım bir görseli çözmeye çalışmak boşuna iştir.
 */
export function useTransferPreview(
  transferId: string | null,
  enabled: boolean,
  maxEdge = 320,
): TransferPreview | null {
  const [preview, setPreview] = useState<TransferPreview | null>(null);

  useEffect(() => {
    if (!transferId || !enabled) {
      setPreview(null);
      return;
    }

    let cancelled = false;
    void load(transferId, maxEdge).then((result) => {
      if (!cancelled) setPreview(result);
    });

    return () => {
      cancelled = true;
    };
  }, [transferId, enabled, maxEdge]);

  return preview;
}

/** Dosya silinip yeniden indirildiğinde eski önizleme geçersizdir. */
export function forgetPreview(transferId: string): void {
  for (const key of [...cache.keys()]) {
    if (key.startsWith(`${transferId}@`)) cache.delete(key);
  }
}
