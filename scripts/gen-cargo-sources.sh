#!/usr/bin/env bash
# Generate packaging/flatpak/cargo-sources.json from Cargo.lock for offline
# flatpak-builder builds (flatpak-cargo-generator).
#
# Usage: ./scripts/gen-cargo-sources.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$ROOT/packaging/flatpak/cargo-sources.json"
GEN_DIR="$(mktemp -d)"
trap 'rm -rf "$GEN_DIR"' EXIT

GENERATOR_URL="https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/master/cargo/flatpak-cargo-generator.py"

echo "==> Fetching flatpak-cargo-generator"
curl -fsSL "$GENERATOR_URL" -o "$GEN_DIR/flatpak-cargo-generator.py"

echo "==> Preparing Python venv with aiohttp + tomlkit"
VENV="$GEN_DIR/venv"
if command -v uv >/dev/null 2>&1; then
  uv venv "$VENV"
  uv pip install --python "$VENV/bin/python" 'aiohttp>=3.9.5,<4' 'tomlkit>=0.13.3'
  PY="$VENV/bin/python"
elif python3 -m venv "$VENV" 2>/dev/null; then
  "$VENV/bin/python" -m pip install -q 'aiohttp>=3.9.5,<4' 'tomlkit>=0.13.3'
  PY="$VENV/bin/python"
elif python3 -c 'import aiohttp, tomlkit' 2>/dev/null; then
  PY="python3"
else
  echo "Need uv, python3-venv, or system aiohttp+tomlkit to generate cargo-sources." >&2
  exit 1
fi

echo "==> Generating $OUT from Cargo.lock"
"$PY" "$GEN_DIR/flatpak-cargo-generator.py" "$ROOT/Cargo.lock" -o "$OUT"
echo "Done: $OUT ($(wc -c < "$OUT") bytes)"
