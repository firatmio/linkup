/** Boyut, hız, süre ve zaman biçimlendirme yardımcıları. */

const MINUTE = 60;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

/**
 * "az önce" / "5 dk önce" / "3 sa önce" / "dün" / tarih.
 *
 * Mutlak saat yerine göreli süre: kullanıcı "son ne zaman konuştuk"
 * sorusuna bakıyor, tam saate değil.
 */
export function relativeTime(seconds: number | null): string {
  if (!seconds) return "";

  const elapsed = Math.floor(Date.now() / 1000) - seconds;
  // Karşı tarafın saati ileri olabilir; negatif süreyi "az önce" say.
  if (elapsed < MINUTE) return "az önce";
  if (elapsed < HOUR) return `${Math.floor(elapsed / MINUTE)} dk önce`;
  if (elapsed < DAY) return `${Math.floor(elapsed / HOUR)} sa önce`;
  if (elapsed < 2 * DAY) return "dün";
  if (elapsed < 7 * DAY) return `${Math.floor(elapsed / DAY)} gün önce`;

  return new Date(seconds * 1000).toLocaleDateString("tr-TR", {
    day: "numeric",
    month: "short",
  });
}
