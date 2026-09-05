#!/usr/bin/env node
/**
 * Doğrulama: tip kontrolü, build, fmt, clippy, testler.
 *
 * `.githooks/pre-push` bunu her push'tan önce otomatik çalıştırır — projenin
 * CI'ı budur (PLAN.md §6). Ayrı bir CI servisi yok: tek geliştirici, tek
 * makine, Windows-öncelikli bir uygulamada uzak koşucunun getirdiği değer
 * (temiz oda, çapraz platform) henüz maliyetini karşılamıyor.
 *
 * Elle çalıştırmak için: bun run check
 * Hook'u atlamak için:  git push --no-verify
 */

import { spawnSync } from "node:child_process";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import process from "node:process";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const srcTauri = join(root, "src-tauri");

const steps = [
  { name: "TypeScript tip kontrolü", cmd: "bun", args: ["run", "typecheck"], cwd: root },
  // Tauri'nin generate_context! makrosu dist/ bekler — backend'den önce gelmeli.
  { name: "Frontend build", cmd: "bun", args: ["run", "build"], cwd: root },
  {
    name: "Rust biçimlendirme",
    cmd: "cargo",
    args: ["fmt", "--all", "--check"],
    cwd: srcTauri,
    hint: "düzeltmek için: cd src-tauri && cargo fmt --all",
  },
  {
    name: "Rust clippy",
    cmd: "cargo",
    args: ["clippy", "--all-targets", "--", "-D", "warnings"],
    cwd: srcTauri,
  },
  { name: "Rust testleri", cmd: "cargo", args: ["test"], cwd: srcTauri },
];

for (const step of steps) {
  process.stdout.write(`\n\x1b[1;36m▶ ${step.name}\x1b[0m\n`);
  // shell:true kullanılmıyor — bun ve cargo Windows'ta da .exe, doğrudan
  // spawn edilebilirler; kabuk argümanları kaçırmadan birleştirdiği için risk.
  const result = spawnSync(step.cmd, step.args, {
    stdio: "inherit",
    cwd: step.cwd,
  });
  const status = result.status ?? 1;
  if (status !== 0) {
    process.stdout.write(`\n\x1b[1;31m✖ Başarısız: ${step.name}\x1b[0m\n`);
    if (step.hint) process.stdout.write(`  ${step.hint}\n`);
    process.exit(status);
  }
}

process.stdout.write("\n\x1b[1;32m✔ Tüm kontroller geçti\x1b[0m\n");
