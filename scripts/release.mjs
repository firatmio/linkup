#!/usr/bin/env node
/**
 * Sürüm çıkarır: imzalı kurulum paketini üretir, `latest.json` yazar ve
 * GitHub'da yayın oluşturur.
 *
 * Neden bir betik: güncelleyicinin çalışması üç şeyin AYNI ANDA doğru
 * olmasına bağlı — paketin özel anahtarla imzalanmış olması, `latest.json`
 * içindeki imzanın o pakete ait olması ve sürüm numarasının Cargo/Tauri
 * yapılandırmasıyla eşleşmesi. Elle yapılınca biri kolayca atlanıyor ve hata
 * ancak kullanıcıların güncellemesi sessizce başarısız olduğunda fark ediliyor.
 *
 * Kullanım: bun run release            (mevcut sürümü yayınlar)
 *           bun run release 0.2.0      (önce sürümü yükseltir)
 */

import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync, existsSync, readdirSync } from "node:fs";
import { join, resolve } from "node:path";
import process from "node:process";

const root = resolve(import.meta.dirname, "..");
const KEY_PATH = join(root, ".tauri", "linkup-updater.key");

function fail(message) {
  console.error(`\x1b[1;31m✖ ${message}\x1b[0m`);
  process.exit(1);
}

function run(command, args, options = {}) {
  return execFileSync(command, args, {
    cwd: root,
    encoding: "utf8",
    stdio: options.capture ? "pipe" : "inherit",
    env: options.env ?? process.env,
  });
}

function readJson(path) {
  return JSON.parse(readFileSync(path, "utf8"));
}

// --- sürüm ---------------------------------------------------------------

const requested = process.argv[2];
const tauriConfPath = join(root, "src-tauri", "tauri.conf.json");
const tauriConf = readJson(tauriConfPath);

if (requested) {
  if (!/^\d+\.\d+\.\d+$/.test(requested)) fail(`geçersiz sürüm: ${requested}`);

  tauriConf.version = requested;
  writeFileSync(tauriConfPath, `${JSON.stringify(tauriConf, null, 2)}\n`);

  // Cargo.toml ve package.json aynı sürümü taşımalı: farklı sürüm numaraları
  // "hangisi doğru" sorusunu her seferinde yeniden sordurur.
  for (const [file, pattern] of [
    ["src-tauri/Cargo.toml", /^version = ".*"$/m],
    ["package.json", /"version": ".*"/],
  ]) {
    const path = join(root, file);
    const content = readFileSync(path, "utf8");
    const replacement = file.endsWith(".toml")
      ? `version = "${requested}"`
      : `"version": "${requested}"`;
    writeFileSync(path, content.replace(pattern, replacement));
  }
  console.log(`Sürüm ${requested} olarak güncellendi.`);
}

const version = requested ?? tauriConf.version;
const tag = `v${version}`;

// --- ön koşullar ---------------------------------------------------------

if (!existsSync(KEY_PATH)) {
  fail(
    `imzalama anahtarı yok: ${KEY_PATH}\n` +
      "  Yeni anahtar: bunx tauri signer generate -w .tauri/linkup-updater.key\n" +
      "  DİKKAT: yeni anahtar üretmek, eski sürümlerin güncelleme almasını durdurur.",
  );
}

try {
  run("gh", ["auth", "status"], { capture: true });
} catch {
  fail("gh oturumu yok. `gh auth login` ile giriş yapın.");
}

const status = run("git", ["status", "--porcelain"], { capture: true });
if (status.trim() && !requested) {
  fail("çalışma dizini temiz değil. Önce commit edin.");
}

// --- yapı ----------------------------------------------------------------

console.log(`\nLinkUp ${version} paketleniyor…\n`);
run("bun", ["run", "check"]);
run("bunx", ["tauri", "build"], {
  env: {
    ...process.env,
    TAURI_SIGNING_PRIVATE_KEY_PATH: KEY_PATH,
    TAURI_SIGNING_PRIVATE_KEY_PASSWORD: "",
  },
});

// --- latest.json ---------------------------------------------------------

const bundleDir = join(root, "src-tauri", "target", "release", "bundle", "nsis");
if (!existsSync(bundleDir)) fail(`paket klasörü bulunamadı: ${bundleDir}`);

const files = readdirSync(bundleDir);
const installer = files.find((name) => name.endsWith(".exe"));
const signatureFile = files.find((name) => name.endsWith(".exe.sig"));

if (!installer || !signatureFile) {
  fail(
    "imzalı kurulum paketi bulunamadı.\n" +
      "  Tauri imzayı yalnızca updater yapılandırması varken üretir " +
      "(tauri.conf.json → plugins.updater).",
  );
}

const signature = readFileSync(join(bundleDir, signatureFile), "utf8").trim();
const notesPath = join(root, "RELEASE_NOTES.md");
const notes = existsSync(notesPath) ? readFileSync(notesPath, "utf8").trim() : "";

if (!notes) {
  fail(
    "RELEASE_NOTES.md boş.\n" +
      "  Bu dosyanın içeriği hem GitHub yayınında hem de kullanıcının\n" +
      "  güncelleme sonrası gördüğü 'Yenilikler' penceresinde görünür.",
  );
}

const latest = {
  version,
  notes,
  pub_date: new Date().toISOString(),
  platforms: {
    "windows-x86_64": {
      signature,
      url: `https://github.com/firatmio/linkup/releases/download/${tag}/${installer}`,
    },
  },
};

const latestPath = join(bundleDir, "latest.json");
writeFileSync(latestPath, `${JSON.stringify(latest, null, 2)}\n`);

// --- yayın ---------------------------------------------------------------

console.log(`\nGitHub yayını oluşturuluyor: ${tag}\n`);
run("gh", [
  "release",
  "create",
  tag,
  join(bundleDir, installer),
  latestPath,
  "--title",
  `LinkUp ${version}`,
  "--notes-file",
  notesPath,
]);

console.log(`\n\x1b[1;32m✔ LinkUp ${version} yayınlandı\x1b[0m`);
console.log(
  "\nNOT: Depo PRIVATE olduğu sürece güncelleyici bu dosyaları indiremez —\n" +
    "GitHub özel depolarda yayın varlıklarını kimlik doğrulaması olmadan\n" +
    "sunmaz. Otomatik güncelleme için depo public olmalı.",
);
