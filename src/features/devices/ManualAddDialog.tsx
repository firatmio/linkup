import { useState } from "react";
import { t } from "../../i18n";
import { Dialog } from "../../components/Dialog";
import { Button } from "../../components/Button";
import { TextField } from "../../components/TextField";
import { Callout } from "../../components/Callout";
import { useDeviceStore } from "../../stores/deviceStore";

/**
 * Elle cihaz ekleme (PLAN.md §2.4, §10-K7).
 *
 * Adres yalnızca listeye yazılmaz — gerçekten bağlanılır. Kullanıcı yanlış
 * adres girdiyse bunu bir sonraki adımda değil, burada öğrenmeli.
 */
export function ManualAddDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  const [address, setAddress] = useState("");
  const adding = useDeviceStore((s) => s.adding);
  const addError = useDeviceStore((s) => s.addError);
  const addManually = useDeviceStore((s) => s.addManually);
  const clearAddError = useDeviceStore((s) => s.clearAddError);

  const close = () => {
    setAddress("");
    clearAddError();
    onClose();
  };

  const submit = async () => {
    if (!address.trim() || adding) return;
    if (await addManually(address)) close();
  };

  return (
    <Dialog
      open={open}
      title={t("addDevice.title")}
      onClose={close}
      footer={
        <>
          <Button onClick={close} disabled={adding}>
            {t("addDevice.cancel")}
          </Button>
          <Button variant="accent" onClick={() => void submit()} disabled={adding || !address.trim()}>
            {adding ? t("addDevice.connecting") : t("addDevice.submit")}
          </Button>
        </>
      }
    >
      <p className="text-[length:var(--lu-text-body)] text-fg-secondary">
        {t("addDevice.body")}
      </p>

      <TextField
        label={t("addDevice.label")}
        hint={t("addDevice.hint")}
        placeholder={t("addDevice.placeholder")}
        value={address}
        disabled={adding}
        autoFocus
        onChange={(event) => {
          setAddress(event.target.value);
          if (addError) clearAddError();
        }}
        onKeyDown={(event) => {
          if (event.key === "Enter") void submit();
        }}
      />

      {addError ? <Callout tone="warning">{addError}</Callout> : null}
    </Dialog>
  );
}
