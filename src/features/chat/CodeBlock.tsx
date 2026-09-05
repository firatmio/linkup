import { useEffect, useState } from "react";
import { Check, Copy } from "lucide-react";
import { t } from "../../i18n";
import { useUiStore } from "../../stores/uiStore";

/**
 * Kod bloğu (PLAN.md §3.3).
 *
 * Shiki tembel (lazy) yüklenir ve tek bir örnek paylaşılır: vurgulayıcı ve
 * gramerler birkaç yüz kilobayt tutar, her mesaj için yeniden yüklemek
 * masaüstü uygulamasında bile fark edilir bir maliyet olurdu. Yüklenene kadar
 * düz metin gösterilir — kod hiçbir zaman gecikmeli görünmez.
 */
const LANGUAGES = [
  "typescript",
  "javascript",
  "tsx",
  "jsx",
  "rust",
  "python",
  "json",
  "bash",
  "sql",
  "html",
  "css",
  "yaml",
  "markdown",
] as const;

type Highlighter = {
  codeToHtml: (code: string, options: { lang: string; theme: string }) => string;
};

let highlighterPromise: Promise<Highlighter> | null = null;

function loadHighlighter(): Promise<Highlighter> {
  highlighterPromise ??= import("shiki").then((shiki) =>
    shiki.createHighlighter({
      themes: ["github-light", "github-dark"],
      langs: [...LANGUAGES],
    }),
  ) as Promise<Highlighter>;
  return highlighterPromise;
}

/** ```lang ... ``` biçimini ayrıştırır; fence yoksa içeriği olduğu gibi alır. */
export function parseFenced(content: string): { language: string; code: string } {
  const match = content.match(/^```([\w+-]*)\r?\n([\s\S]*?)\r?\n?```\s*$/);
  if (!match) return { language: "text", code: content };

  const language = match[1].toLowerCase();
  return {
    language: (LANGUAGES as readonly string[]).includes(language) ? language : "text",
    code: match[2],
  };
}

export function CodeBlock({ content }: { content: string }) {
  const { language, code } = parseFenced(content);
  const theme = useUiStore((s) => s.resolvedTheme);
  const [html, setHtml] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    if (language === "text") return;

    let active = true;
    void loadHighlighter()
      .then((highlighter) => {
        if (!active) return;
        setHtml(
          highlighter.codeToHtml(code, {
            lang: language,
            theme: theme === "dark" ? "github-dark" : "github-light",
          }),
        );
      })
      .catch(() => {
        // Vurgulama başarısızsa düz metin gösterilir; mesaj kaybolmaz.
      });

    return () => {
      active = false;
    };
  }, [code, language, theme]);

  const copy = () => {
    void navigator.clipboard.writeText(code).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    });
  };

  return (
    <div className="group/code relative">
      <button
        type="button"
        onClick={copy}
        title={copied ? t("chats.copied") : t("chats.copyCode")}
        aria-label={copied ? t("chats.copied") : t("chats.copyCode")}
        className="absolute top-1.5 right-1.5 rounded-lu-sm border border-stroke bg-layer p-1 text-fg-secondary opacity-0 transition-opacity group-hover/code:opacity-100 hover:text-fg"
      >
        {copied ? <Check size={13} className="text-success" /> : <Copy size={13} />}
      </button>

      {html ? (
        <div
          className="lu-selectable overflow-x-auto rounded-lu-sm border border-stroke [&_pre]:m-0 [&_pre]:bg-transparent! [&_pre]:p-3 [&_pre]:text-[length:var(--lu-text-caption)]"
          // Shiki'nin ürettiği HTML güvenilirdir: girdi kullanıcı metni olsa da
          // shiki onu kaçırarak (escape) işler, dışarıdan gelen ham HTML değildir.
          dangerouslySetInnerHTML={{ __html: html }}
        />
      ) : (
        <pre className="lu-selectable overflow-x-auto rounded-lu-sm border border-stroke bg-layer-alt p-3 font-mono text-[length:var(--lu-text-caption)]">
          {code}
        </pre>
      )}
    </div>
  );
}
