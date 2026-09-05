#!/usr/bin/env node
/**
 * Çalışan tüm geliştirme süreçlerini kapatır: Vite sunucusu, birincil LinkUp
 * instance'ı ve ikincil instance'lar. `dev-vite.mjs` ve `dev-second.mjs`
 * süreçleri bilerek kabuktan bağımsız başlattığı için Ctrl+C ile kapanmazlar.
 *
 * Birincil instance'ı da kapatmak ŞART: Windows çalışan bir exe'nin üzerine
 * yazdırmaz. Açık kalan bir `linkup.exe`, bir sonraki `bun run dev:a`
 * derlemesini "Erişim engellendi (os error 5)" ile düşürür ve uygulama
 * sessizce ESKİ ikiliyle çalışmaya devam eder — değişikliğin uygulanmadığını
 * fark etmek zor.
 */

import { execSync } from "node:child_process";
import process from "node:process";

const isWindows = process.platform === "win32";

function run(command) {
  try {
    execSync(command, { stdio: "ignore" });
    return true;
  } catch {
    return false;
  }
}

// 1420 portunu dinleyen süreç (Vite).
const viteStopped = isWindows
  ? run(
      'powershell -NoProfile -Command "Get-NetTCPConnection -LocalPort 1420 -State Listen -ErrorAction SilentlyContinue | ' +
        'Select-Object -ExpandProperty OwningProcess -Unique | ForEach-Object { Stop-Process -Id $_ -Force }"',
    )
  : run("pkill -f 'vite'");

// Hem `linkup` hem `linkup-b`/`linkup-c`.
const instancesStopped = isWindows
  ? run(
      'powershell -NoProfile -Command "Get-Process linkup, linkup-* -ErrorAction SilentlyContinue | Stop-Process -Force"',
    )
  : run("pkill -f 'linkup'");

console.log(`Vite: ${viteStopped ? "durduruldu" : "çalışmıyordu"}`);
console.log(`LinkUp instance'ları: ${instancesStopped ? "durduruldu" : "çalışmıyordu"}`);
