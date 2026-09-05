import { tr, type TranslationKey } from "./tr";

/**
 * Tip güvenli çeviri.
 *
 * `t("nav.chats")` derlenir; `t("nav.chatz")` derlenmez — sözlükte olmayan
 * anahtar TypeScript hatası verir. v1'de tek dil yüklenir (PLAN.md §10-K8).
 */
const dictionary: Record<TranslationKey, string> = tr;

export function t(key: TranslationKey, vars?: Record<string, string | number>): string {
  const template = dictionary[key];
  if (!vars) return template;
  return template.replace(/\{(\w+)\}/g, (match, name: string) =>
    name in vars ? String(vars[name]) : match,
  );
}

/**
 * Backend'den gelen hata kodunu kullanıcıya gösterilecek metne çevirir.
 * Backend asla hazır metin göndermez, yalnızca kod (PLAN.md §2.14).
 */
export function translateError(error: unknown): string {
  if (
    typeof error === "object" &&
    error !== null &&
    "code" in error &&
    typeof (error as { code: unknown }).code === "string"
  ) {
    const code = (error as { code: string }).code;
    if (code in dictionary) return dictionary[code as TranslationKey];
  }
  return dictionary["error.unknown"];
}

export type { TranslationKey };
