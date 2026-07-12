import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  reactCompiler: true,
  // better-sqlite3 is a native module and must not be bundled by webpack/turbopack.
  serverExternalPackages: ["better-sqlite3"],
};

export default nextConfig;
