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
