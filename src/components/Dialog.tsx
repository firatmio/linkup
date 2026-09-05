import { useEffect, useRef, type ReactNode } from "react";

/**
 * Fluent ContentDialog yorumu.
 *
 * Native `<dialog>` üzerine kurulu: odak tuzağı, Esc ile kapatma ve arka planın
 * etkisizleştirilmesi tarayıcıdan geliyor — elle yeniden yazmaya gerek yok.
 */
export function Dialog({
  open,
  title,
  onClose,
  children,
  footer,
}: {
  open: boolean;
  title: string;
  onClose: () => void;
  children: ReactNode;
  footer?: ReactNode;
}) {
  const ref = useRef<HTMLDialogElement>(null);

  useEffect(() => {
    const dialog = ref.current;
    if (!dialog) return;
    if (open && !dialog.open) dialog.showModal();
    if (!open && dialog.open) dialog.close();
  }, [open]);

  return (
    <dialog
      ref={ref}
      onClose={onClose}
      // Arka plana tıklayınca kapansın: tıklama <dialog> elemanının kendisine
      // düşüyorsa, içerik kutusunun dışına basılmış demektir.
      onClick={(event) => {
        if (event.target === ref.current) onClose();
      }}
      className="m-auto w-[min(28rem,calc(100vw-2rem))] rounded-lu-lg border border-stroke bg-layer p-0 text-fg shadow-flyout backdrop:bg-black/40"
    >
      <div className="px-6 pt-5">
        <h2 className="font-display text-[length:var(--lu-text-subtitle)] font-semibold">
          {title}
        </h2>
      </div>
      <div className="space-y-3 px-6 py-4">{children}</div>
      {footer ? (
        <div className="flex justify-end gap-2 rounded-b-lu-lg border-t border-divider bg-layer-alt px-6 py-4">
          {footer}
        </div>
      ) : null}
    </dialog>
  );
}
