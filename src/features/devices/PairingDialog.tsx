import { ShieldCheck } from "lucide-react";
import { t } from "../../i18n";
import { Dialog } from "../../components/Dialog";
import { Button } from "../../components/Button";
import { Callout } from "../../components/Callout";
import { usePairingStore } from "../../stores/pairingStore";

/**
 * Karşılıklı doğrulama kodu diyaloğu (PLAN.md §2.5, §10-K2).
 *
 * Kod GİRİLMEZ, karşılaştırılır: iki cihazda da aynı kod gösterilir ve
 * kullanıcı "aynı mı?" sorusunu yanıtlar. Kodun MITM'e karşı koruması buna
 * dayandığı için uyarı metni öne çıkarılmıştır — kullanıcı farkı görmezse
 * koruma çalışmaz.
 */
export function PairingDialog() {
  const request = usePairingStore((s) => s.request);
  const waitingForPeer = usePairingStore((s) => s.waitingForPeer);
  const respond = usePairingStore((s) => s.respond);

  const open = request !== null;

  return (
    <Dialog
      open={open}
      title={t("pairing.title")}
      // Esc ile kapatma reddetme sayılır: yanıtsız bırakmak karşı tarafı
      // 90 saniye bekletirdi.
      onClose={() => void respond(false)}
      footer={
        waitingForPeer ? (
          <span className="text-[length:var(--lu-text-body)] text-fg-secondary">
            {t("pairing.waiting")}
          </span>
        ) : (
          <>
            <Button onClick={() => void respond(false)}>{t("pairing.reject")}</Button>
            <Button variant="accent" onClick={() => void respond(true)}>
              {t("pairing.accept")}
            </Button>
          </>
        )
      }
    >
      {request ? (
        <>
          <p className="text-[length:var(--lu-text-body)] text-fg-secondary">
            {t(request.initiatedByUs ? "pairing.outgoing" : "pairing.incoming", {
              device: request.deviceName,
            })}
          </p>

          <div className="flex justify-center py-2">
            <span className="lu-selectable font-mono text-[2.5rem] leading-none font-semibold tracking-[0.3em] tabular-nums">
              {request.code}
            </span>
          </div>

          <Callout tone="warning">{t("pairing.warning")}</Callout>

          <p className="flex items-center gap-2 text-[length:var(--lu-text-caption)] text-fg-tertiary">
            <ShieldCheck size={14} className="shrink-0" />
            <span className="lu-selectable break-all">{request.fingerprint}</span>
          </p>
        </>
      ) : null}
    </Dialog>
  );
}
