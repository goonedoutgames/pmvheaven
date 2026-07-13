import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  // React Compiler (still experimental at v1.0.0) transformed the production
  // build differently from the dev runtime, breaking context-driven re-renders
  // (e.g. the "Watched" badge not updating while browsing, player button quirks)
  // — i.e. release behaved differently from dev. Disabled for prod/dev parity.
  // better-sqlite3 is a native module and must not be bundled by webpack/turbopack.
  serverExternalPackages: ["better-sqlite3"],
  // Produce a self-contained server bundle (.next/standalone) so the Tauri
  // desktop build can ship and launch the Next.js server as a child process.
  output: "standalone",
};

export default nextConfig;
