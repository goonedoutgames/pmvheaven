import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  reactCompiler: true,
  // better-sqlite3 is a native module and must not be bundled by webpack/turbopack.
  serverExternalPackages: ["better-sqlite3"],
  // Produce a self-contained server bundle (.next/standalone) so the Tauri
  // desktop build can ship and launch the Next.js server as a child process.
  output: "standalone",
};

export default nextConfig;
