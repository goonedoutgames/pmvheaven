// Assembles the Next.js standalone output into src-tauri/server so the Tauri
// desktop build can bundle and launch it. Run after `next build`.
//
// pnpm's node_modules layout uses symlinks (and can contain a few dangling
// ones from the virtual store). Tauri's resource bundler resolves every file
// and fails on broken links, so we copy with symlinks *dereferenced* and skip
// any that dangle — producing a flat, self-contained server tree.
import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const standalone = path.join(root, ".next", "standalone");
const staticDir = path.join(root, ".next", "static");
const publicDir = path.join(root, "public");
const dest = path.join(root, "src-tauri", "server");

let copied = 0;
let skipped = 0;

/** Recursively copy `src` -> `dst`, dereferencing symlinks, skipping dangling ones. */
function copyTree(src, dst) {
  let stat;
  try {
    // stat (not lstat) follows symlinks; throws if the link target is missing.
    stat = fs.statSync(src);
  } catch {
    skipped++;
    return;
  }

  if (stat.isDirectory()) {
    fs.mkdirSync(dst, { recursive: true });
    for (const name of fs.readdirSync(src)) {
      copyTree(path.join(src, name), path.join(dst, name));
    }
  } else if (stat.isFile()) {
    fs.mkdirSync(path.dirname(dst), { recursive: true });
    fs.copyFileSync(src, dst); // copyFileSync follows symlinks -> copies target
    copied++;
  }
}

if (!fs.existsSync(standalone)) {
  console.error(
    "Missing .next/standalone. Run `next build` first (output: 'standalone').",
  );
  process.exit(1);
}

// Keep the .keep placeholder; clear everything else.
fs.rmSync(dest, { recursive: true, force: true });
fs.mkdirSync(dest, { recursive: true });
fs.writeFileSync(
  path.join(dest, ".keep"),
  "Populated by pnpm desktop:prepare.\n",
);

// 1. The standalone server (server.js + traced node_modules + .next server files)
copyTree(standalone, dest);

// 1b. Next.js's file tracer, combined with pnpm's non-hoisted symlink store,
// omits several of Next's own runtime dependencies (e.g. @swc/helpers, @next/env,
// styled-jsx, postcss...), so the standalone server crashes on boot with
// MODULE_NOT_FOUND. Rather than chase them one at a time, bundle Next's entire
// runtime dependency closure from the pnpm store into the top of the server's
// node_modules (where Node's resolution finds them from any nested path).
function findModuleSource(pkg) {
  const nm = path.join(root, "node_modules");
  const direct = path.join(nm, pkg);
  if (fs.existsSync(path.join(direct, "package.json"))) return direct;

  const pnpmDir = path.join(nm, ".pnpm");
  if (!fs.existsSync(pnpmDir)) return null;
  const flat = pkg.replace(/\//g, "+");
  const entries = fs.readdirSync(pnpmDir);
  // Prefer the package's own store dir (e.g. `@swc+helpers@0.5.15`).
  const ordered = [...entries.filter((e) => e.startsWith(`${flat}@`)), ...entries];
  for (const entry of ordered) {
    const candidate = path.join(pnpmDir, entry, "node_modules", pkg);
    if (fs.existsSync(path.join(candidate, "package.json"))) return candidate;
  }
  return null;
}

let bundledDeps = 0;
function ensureClosure(pkg, seen) {
  if (seen.has(pkg)) return;
  seen.add(pkg);
  const src = findModuleSource(pkg);
  if (!src) return; // optional/absent dep — safe to skip

  const destPkg = path.join(dest, "node_modules", pkg);
  if (!fs.existsSync(path.join(destPkg, "package.json"))) {
    copyTree(src, destPkg);
    bundledDeps++;
  }
  try {
    const pj = JSON.parse(fs.readFileSync(path.join(src, "package.json"), "utf8"));
    for (const dep of Object.keys(pj.dependencies ?? {})) ensureClosure(dep, seen);
  } catch {
    /* ignore malformed package.json */
  }
}

const seen = new Set();
const nextSrc = findModuleSource("next");
if (nextSrc) {
  const nextPj = JSON.parse(
    fs.readFileSync(path.join(nextSrc, "package.json"), "utf8"),
  );
  // Runtime deps only — skip optionalDependencies (platform-specific @next/swc
  // native binaries aren't needed to serve a prebuilt app).
  for (const dep of Object.keys(nextPj.dependencies ?? {})) ensureClosure(dep, seen);
}

// Native packages marked external (serverExternalPackages in next.config.ts) are
// required from node_modules at runtime. Turbopack copies the package + its .node
// binary into .next/node_modules, but the loader (`bindings`) and its deps aren't
// traced under pnpm — bundle their closure so the addon resolves at runtime.
// Keep this list in sync with `serverExternalPackages` in next.config.ts.
const EXTERNAL_PACKAGES = ["better-sqlite3"];
for (const pkg of EXTERNAL_PACKAGES) ensureClosure(pkg, seen);

console.log(`  + bundled ${bundledDeps} runtime dependencies missing from the trace`);

// 2. Static assets are not included in standalone; they must sit at .next/static
copyTree(staticDir, path.join(dest, ".next", "static"));

// 3. Public assets
if (fs.existsSync(publicDir)) {
  copyTree(publicDir, path.join(dest, "public"));
}

console.log(
  `Assembled standalone server -> ${path.relative(root, dest)} ` +
    `(${copied} files, ${skipped} dangling links skipped)`,
);
