#!/usr/bin/env node
/**
 * İkinci (üçüncü, dördüncü…) LinkUp instance'ını başlatır — PLAN.md §6.
 *
 * Neden ayrı bir script:
 * `tauri dev`i iki kez çalıştırmak Windows'ta çalışmaz; ikinci cargo derlemesi
 * birincinin çalıştırdığı `linkup.exe`yi silemez (os error 5). Ayrı `target/`
 * dizinleri vermek de her profil için sıfırdan bir derleme demektir.
 *
 * Bunun yerine: `dev:a` tek derlemeyi ve tek Vite sunucusunu yönetir; bu script
 * derlenmiş binary'nin profile özel bir kopyasını çalıştırır. Böylece ikinci
 * instance saniyeler içinde açılır, iki pencere de aynı Vite sunucusundan
 * beslenir ve `dev:a` arka planda serbestçe yeniden derleyebilir.
 *
 * Uygulama AYRILMIŞ (detached) olarak başlatılır: aksi hâlde bu script'i
 * çalıştıran kabuk kapandığında süreç ağacıyla birlikte uygulama da ölür.
 * Bu, geliştirme sırasında pencerenin sebepsiz kapanması olarak görülüyordu.
 *
 * Rust tarafı değiştiğinde bu instance'ı elle yeniden başlatın (yeni binary'yi
 * kopyalaması için).
 */

import { spawn } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, openSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import process from "node:process";

const DEV_SERVER = "http://localhost:1420";

const args = process.argv.slice(2);
const profile = args.find((arg) => !arg.startsWith("-"));
/** Çıktıyı bu terminale bağla ve süreci burada tut (Ctrl+C ile kapatmak için). */
const attach = args.includes("--attach");

if (!profile || !/^[a-z0-9_-]+$/i.test(profile)) {
  console.error("Kullanım: node scripts/dev-second.mjs <profil> [--attach]   (örn. b)");
  process.exit(1);
}

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const isWindows = process.platform === "win32";
const ext = isWindows ? ".exe" : "";
const debugDir = join(root, "src-tauri", "target", "debug");
const source = join(debugDir, `linkup${ext}`);

if (!existsSync(source)) {
  console.error(
    [
      `Derlenmiş binary bulunamadı: ${source}`,
      "",
      "Önce birinci instance'ı başlatın (bir kez derlemesi gerekiyor):",
      "  bun run dev:a",
      "",
      "Derleme bitip pencere açıldıktan sonra bu komutu tekrar çalıştırın.",
    ].join("\n"),
  );
  process.exit(1);
}

// Bu instance frontend'i `dev:a`nın Vite sunucusundan yükler. Sunucu ayakta
// değilse pencere siyah açılır; sebebi baştan söylemek, sonradan aramaktan iyi.
try {
  const response = await fetch(DEV_SERVER, { signal: AbortSignal.timeout(2000) });
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
} catch {
  console.error(
    [
      `Vite geliştirme sunucusu yanıt vermiyor (${DEV_SERVER}).`,
      "",
      "Bu instance frontend'i oradan yükler; sunucu olmadan pencere boş açılır.",
      "Önce birinci instance'ı başlatın ve açılmasını bekleyin:",
      "  bun run dev:a",
    ].join("\n"),
  );
  process.exit(1);
}

// Kopya ayrı bir dosya: dev:a arka planda yeniden derlerken bu instance
// çalışmaya devam edebilir.
const copiesDir = join(debugDir, "instances");
mkdirSync(copiesDir, { recursive: true });
const target = join(copiesDir, `linkup-${profile}${ext}`);

try {
  copyFileSync(source, target);
} catch (err) {
  console.error(`Binary kopyalanamadı: ${err.message}`);
  console.error(`"${profile}" profili zaten çalışıyor olabilir — önce onu kapatın.`);
  process.exit(1);
}

const spawnArgs = [target, ["--profile", profile]];

if (attach) {
  const child = spawn(...spawnArgs, { stdio: "inherit" });
  child.on("exit", (code) => process.exit(code ?? 0));
} else {
  // Ayrılmış başlatma: script biter, uygulama yaşamaya devam eder.
  const logPath = join(copiesDir, `linkup-${profile}.out.log`);
  const out = openSync(logPath, "a");
  const child = spawn(...spawnArgs, {
    detached: true,
    stdio: ["ignore", out, out],
    windowsHide: false,
  });
  child.unref();

  console.log(`LinkUp (${profile.toUpperCase()}) başlatıldı — pid ${child.pid}`);
  console.log(`Konsol çıktısı: ${logPath}`);
  console.log("Bu pencere kapansa da uygulama çalışmaya devam eder.");
}
