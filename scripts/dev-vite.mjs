#!/usr/bin/env node
/**
 * Vite geliştirme sunucusunu ayrılmış (detached) olarak başlatır.
 *
 * `tauri dev`in `beforeDevCommand`ı olarak çalışır. Neden doğrudan `vite`
 * değil: Vite `tauri dev`in alt süreci olduğunda, birinci instance kapanınca
 * Vite de ölüyor ve ikinci instance'ın penceresi kararıyordu — frontend'ini
 * oradan yüklüyor (PLAN.md §6). Ayrılmış başlatınca sunucu, onu başlatan
 * pencereden bağımsız yaşar.
 *
 * Sunucu zaten ayaktaysa hiçbir şey yapmaz: `tauri dev` ikinci kez
 * çalıştırıldığında port çakışmasıyla düşmesin.
 *
 * Sunucuyu durdurmak için: bun run dev:stop
 */

import { spawn } from "node:child_process";
import { openSync, mkdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import process from "node:process";

const URL = "http://localhost:1420";
const READY_TIMEOUT_MS = 60_000;

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");

async function isUp() {
  try {
    const response = await fetch(URL, { signal: AbortSignal.timeout(1000) });
    return response.ok;
  } catch {
    return false;
  }
}

if (await isUp()) {
  console.log(`Vite zaten çalışıyor (${URL})`);
  process.exit(0);
}

const logDir = join(root, "src-tauri", "target", "debug", "instances");
mkdirSync(logDir, { recursive: true });
const out = openSync(join(logDir, "vite.log"), "a");

const child = spawn("bun", ["run", "dev"], {
  cwd: root,
  detached: true,
  stdio: ["ignore", out, out],
  shell: process.platform === "win32",
});
child.unref();

// `tauri dev` bu komut bitince pencereyi açar; sunucu hazır olmadan dönmek
// beyaz/boş bir pencereyle sonuçlanır.
const deadline = Date.now() + READY_TIMEOUT_MS;
while (Date.now() < deadline) {
  if (await isUp()) {
    console.log(`Vite hazır (${URL}) — pid ${child.pid}`);
    process.exit(0);
  }
  await new Promise((done) => setTimeout(done, 250));
}

console.error(`Vite ${READY_TIMEOUT_MS / 1000} saniyede hazır olmadı.`);
console.error(`Günlük: ${join(logDir, "vite.log")}`);
process.exit(1);
