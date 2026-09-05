import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Minus, Square, Copy, X } from "lucide-react";
import { t } from "../i18n";
import { cn } from "../lib/cn";
import { useAppStore } from "../stores/appStore";

/**
 * Windows 11 tarzı özel başlık çubuğu.
 *
 * Pencere `decorations: false` ile açılıyor; sürükleme, çift tıkla büyütme ve
 * pencere düğmeleri buradan geliyor. Sürüklemeyi Tauri'nin
 * `data-tauri-drag-region` özniteliği yapıyor: yalnızca özniteliğin BULUNDUĞU
 * eleman sürükler, dolayısıyla üstündeki düğmeler normal şekilde tıklanır.
 *
 * Ölçüler Windows'un kendi ölçüleri: 32 px yükseklik, 46×32 px başlık
 * düğmeleri, kapatmada kırmızı vurgu.
 */
export function TitleBar() {
  const [maximized, setMaximized] = useState(false);
  const info = useAppStore((s) => s.info);

  // Başlık, pencere başlığıyla aynı olmalı: profil çalıştırmalarında hangi
  // instance'a baktığını ayırt etmenin tek yolu bu.
  const title = info?.profile ? `LinkUp (${info.profile.toUpperCase()})` : "LinkUp";

  useEffect(() => {
    const window = getCurrentWindow();
    let unlisten: (() => void) | undefined;
    let cancelled = false;

    void window.isMaximized().then((value) => {
      if (!cancelled) setMaximized(value);
    });

    // Büyütme kenarlıktan sürükleyerek de değişebilir; düğmeye basılmasını
    // beklemek simgeyi yanlış durumda bırakırdı.
    void window.onResized(() => {
      void window.isMaximized().then((value) => {
        if (!cancelled) setMaximized(value);
      });
    }).then((fn) => {
      if (cancelled) fn();
      else unlisten = fn;
    });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  const window = getCurrentWindow();

  return (
    <header
      data-tauri-drag-region
      className="flex h-8 shrink-0 items-center justify-between bg-base pl-3 select-none"
    >
      <span
        data-tauri-drag-region
        className="pointer-events-none text-[length:var(--lu-text-caption)] text-fg-secondary"
      >
        {title}
      </span>

      <div className="flex">
        <CaptionButton label={t("window.minimize")} onClick={() => void window.minimize()}>
          <Minus size={14} />
        </CaptionButton>
        <CaptionButton
          label={t(maximized ? "window.restore" : "window.maximize")}
          onClick={() => void window.toggleMaximize()}
        >
          {maximized ? <Copy size={12} /> : <Square size={12} />}
        </CaptionButton>
        <CaptionButton label={t("window.close")} danger onClick={() => void window.close()}>
          <X size={14} />
        </CaptionButton>
      </div>
    </header>
  );
}

function CaptionButton({
  label,
  onClick,
  danger = false,
  children,
}: {
  label: string;
  onClick: () => void;
  danger?: boolean;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      onClick={onClick}
      className={cn(
        "flex h-8 w-[46px] items-center justify-center text-fg-secondary",
        "transition-colors duration-[var(--lu-dur-fast)]",
        danger ? "hover:bg-[#c42b1c] hover:text-white" : "hover:bg-hover hover:text-fg",
      )}
    >
      {children}
    </button>
  );
}
