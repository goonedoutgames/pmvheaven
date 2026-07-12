// Assembles the Next.js standalone output into src-tauri/server so the Tauri
// desktop build can bundle and launch it. Run after `next build`.
import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const standalone = path.join(root, ".next", "standalone");
const staticDir = path.join(root, ".next", "static");
const publicDir = path.join(root, "public");
const dest = path.join(root, "src-tauri", "server");

if (!fs.existsSync(standalone)) {
  console.error(
    "Missing .next/standalone. Run `next build` first (output: 'standalone').",
  );
  process.exit(1);
}

fs.rmSync(dest, { recursive: true, force: true });
fs.mkdirSync(dest, { recursive: true });

// 1. The standalone server (server.js + traced node_modules + .next server files)
fs.cpSync(standalone, dest, { recursive: true });

// 2. Static assets are not included in standalone; they must sit at .next/static
fs.cpSync(staticDir, path.join(dest, ".next", "static"), { recursive: true });

// 3. Public assets
if (fs.existsSync(publicDir)) {
  fs.cpSync(publicDir, path.join(dest, "public"), { recursive: true });
}

console.log(`Assembled standalone server -> ${path.relative(root, dest)}`);
