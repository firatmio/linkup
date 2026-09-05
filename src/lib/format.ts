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

const UNITS = ["B", "KB", "MB", "GB", "TB"];

/** 1536 → "1,5 KB" */
export function fileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;

  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < UNITS.length - 1) {
    value /= 1024;
    unit += 1;
  }
  // Büyük değerlerde ondalık gürültüdür: "1,5 GB" ama "512 MB".
  const digits = value >= 100 ? 0 : 1;
  return `${value.toFixed(digits).replace(".", ",")} ${UNITS[unit]}`;
}

/** Saniyedeki bayt → "12,4 MB/s" */
export function speed(bytesPerSecond: number): string {
  return `${fileSize(bytesPerSecond)}/s`;
}

/**
 * Kalan süre. Hız sıfırsa tahmin yapılmaz — "sonsuz" göstermek yerine
 * hiçbir şey göstermek daha dürüst.
 */
export function remainingTime(bytesLeft: number, bytesPerSecond: number): string | null {
  if (bytesPerSecond <= 0) return null;

  const seconds = Math.ceil(bytesLeft / bytesPerSecond);
  if (seconds < 60) return `${seconds} sn`;
  if (seconds < HOUR) return `${Math.ceil(seconds / MINUTE)} dk`;
  return `${Math.round((seconds / HOUR) * 10) / 10} sa`.replace(".", ",");
}

/**
 * Aktarımın ne kadar sürdüğü. Başlangıç ve bitiş aynı saniyeye düşerse
 * "0 sn" yerine "1 sn'den kısa" denir: sıfır süre yanlış bir kesinlik iddiası.
 */
export function duration(startedAt: number, completedAt: number | null): string | null {
  if (!completedAt) return null;

  const seconds = completedAt - startedAt;
  if (seconds < 0) return null;
  if (seconds < 1) return "1 sn'den kısa";
  if (seconds < 60) return `${seconds} sn`;
  if (seconds < HOUR) return `${Math.round(seconds / MINUTE)} dk`;
  return `${Math.round((seconds / HOUR) * 10) / 10} sa`.replace(".", ",");
}
