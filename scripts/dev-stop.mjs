#!/usr/bin/env node
/**
 * Ayrılmış olarak başlatılan geliştirme süreçlerini kapatır: Vite sunucusu ve
 * ikincil LinkUp instance'ları. `dev-vite.mjs` ve `dev-second.mjs` süreçleri
 * bilerek kabuktan bağımsız başlattığı için Ctrl+C ile kapanmazlar.
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

const instancesStopped = isWindows
  ? run('powershell -NoProfile -Command "Get-Process linkup-* -ErrorAction SilentlyContinue | Stop-Process -Force"')
  : run("pkill -f 'linkup-'");

console.log(`Vite: ${viteStopped ? "durduruldu" : "çalışmıyordu"}`);
console.log(`İkincil instance'lar: ${instancesStopped ? "durduruldu" : "çalışmıyordu"}`);
