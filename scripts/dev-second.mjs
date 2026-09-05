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
 * derlenmiş binary'nin profile özel bir kopyasını çalıştırır. Böylece
 * - ikinci instance saniyeler içinde açılır,
 * - iki pencere de aynı Vite sunucusundan beslenir, HMR ikisinde de çalışır,
 * - kopya ayrı bir dosya olduğu için `dev:a` arka planda serbestçe yeniden
 *   derleyebilir.
 *
 * Rust tarafı değiştiğinde bu instance'ı elle yeniden başlatın (yeni binary'yi
 * kopyalaması için).
 */

import { spawn } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import process from "node:process";

const profile = process.argv[2];
if (!profile || !/^[a-z0-9_-]+$/i.test(profile)) {
  console.error("Kullanım: node scripts/dev-second.mjs <profil>   (örn. b)");
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

console.log(`LinkUp (${profile.toUpperCase()}) başlatılıyor — ${target}`);

const child = spawn(target, ["--profile", profile], { stdio: "inherit" });
child.on("exit", (code) => process.exit(code ?? 0));
