import { defineConfig, globalIgnores } from "eslint/config";
import nextVitals from "eslint-config-next/core-web-vitals";
import nextTs from "eslint-config-next/typescript";

const eslintConfig = defineConfig([
  ...nextVitals,
  ...nextTs,
  // Override default ignores of eslint-config-next.
  globalIgnores([
    // Default ignores of eslint-config-next:
    ".next/**",
    "out/**",
    "build/**",
    "next-env.d.ts",
    // Tauri: Rust build artifacts and the assembled standalone server bundle.
    "src-tauri/**",
  ]),
  {
    rules: {
      // The app intentionally initializes client-only state on mount
      // (localStorage hydration, session/feed fetches). Keep as guidance.
      "react-hooks/set-state-in-effect": "warn",
    },
  },
]);

export default eslintConfig;
