// Builds the cross-distro AppImage.
//
// Tauri's built-in AppImage bundling breaks on bleeding-edge distros (Arch /
// CachyOS) for two independent reasons:
//
//   1. linuxdeploy's *bundled* `strip` is too old to parse the modern `.relr.dyn`
//      relocation section (ELF section type 0x13) that current glibc/toolchains
//      emit, so it aborts. Setting NO_STRIP=true skips stripping entirely.
//
//   2. Tauri runs the whole toolchain with APPIMAGE_EXTRACT_AND_RUN=1. Under
//      that mode the linuxdeploy-plugin-appimage process's cwd becomes its own
//      /tmp extraction dir, and it invokes `appimagetool` with a *relative*
//      AppDir path ("PMVHeaven.AppDir"), which then can't be found. Passing the
//      AppDir as an absolute path avoids this.
//
// So we let tauri do what it does well — compile, assemble the AppDir, and bundle
// every GTK/GStreamer dependency via linuxdeploy (with NO_STRIP) — and then
// finalize the .AppImage ourselves with appimagetool using an absolute path.

import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const root = process.cwd();
const bundleDir = path.join(root, "src-tauri/target/release/bundle/appimage");
const toolsDir = path.join(os.homedir(), ".local/share/pmvheaven-build");
const appimagetoolPath = path.join(toolsDir, "appimagetool.AppImage");
const APPIMAGETOOL_URL =
  "https://github.com/AppImage/appimagetool/releases/download/1.9.0/appimagetool-x86_64.AppImage";

function run(cmd, args, extraEnv = {}) {
  const res = spawnSync(cmd, args, {
    stdio: "inherit",
    env: { ...process.env, ...extraEnv },
  });
  return res.status ?? 1;
}

function readVersion() {
  try {
    const conf = JSON.parse(
      fs.readFileSync(path.join(root, "src-tauri/tauri.conf.json"), "utf8"),
    );
    return conf.version ?? "0.0.0";
  } catch {
    return "0.0.0";
  }
}

function ensureAppimagetool() {
  if (fs.existsSync(appimagetoolPath)) return;
  fs.mkdirSync(toolsDir, { recursive: true });
  console.log(`\n> Fetching appimagetool -> ${appimagetoolPath}`);
  const status = run("curl", ["-fSL", "-o", appimagetoolPath, APPIMAGETOOL_URL]);
  if (status !== 0 || !fs.existsSync(appimagetoolPath)) {
    console.error("Failed to download appimagetool.");
    process.exit(1);
  }
  fs.chmodSync(appimagetoolPath, 0o755);
}

// 1. Start from a clean bundle dir so we always package a fresh, complete AppDir.
fs.rmSync(bundleDir, { recursive: true, force: true });

// 2. Let tauri compile + assemble + bundle libs. It is EXPECTED to fail at the
//    final appimagetool step (see header) — we finalize that ourselves.
console.log(
  "\n> Running `tauri build` (NO_STRIP=true). The final 'failed to run linuxdeploy'\n" +
    "  error is expected on Arch-family distros and is handled by this script.\n",
);
run("pnpm", ["exec", "tauri", "build"], { NO_STRIP: "true" });

// 3. Locate the AppDir tauri assembled and confirm it was fully populated.
const appDir = fs.existsSync(bundleDir)
  ? fs
      .readdirSync(bundleDir)
      .map((f) => path.join(bundleDir, f))
      .find((p) => p.endsWith(".AppDir"))
  : null;

const populated =
  appDir && fs.existsSync(path.join(appDir, "usr/lib/libwebkit2gtk-4.1.so.0"));

if (!populated) {
  console.error(
    "\nAppImage build failed before packaging: the AppDir was not fully assembled.\n" +
      "This is a real error (not the known appimagetool path bug). See tauri output above.",
  );
  process.exit(1);
}

// 4. Package the AppDir into a runnable AppImage using an ABSOLUTE path.
ensureAppimagetool();
const version = readVersion();
const outPath = path.join(bundleDir, `PMVHeaven_${version}_amd64.AppImage`);
fs.rmSync(outPath, { force: true });

console.log(`\n> Packaging AppImage -> ${outPath}`);
const status = run(appimagetoolPath, [path.resolve(appDir), outPath], {
  ARCH: "x86_64",
  APPIMAGE_EXTRACT_AND_RUN: "1",
});

if (status !== 0 || !fs.existsSync(outPath)) {
  console.error("\nappimagetool failed to package the AppImage.");
  process.exit(1);
}

fs.chmodSync(outPath, 0o755);
const sizeMb = (fs.statSync(outPath).size / 1024 / 1024).toFixed(0);
console.log(`\n✔ AppImage built: ${outPath} (${sizeMb} MB)`);
