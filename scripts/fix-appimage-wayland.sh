#!/usr/bin/env bash
# Post-process a Dioxus AppImage for rolling-release Wayland hosts.
#
# 1. Strips bundled libwayland*.so* so the dynamic linker uses the system ones
# 2. Replaces the AppRun symlink with a thin wrapper that preloads system
#    libwayland-client (belt-and-suspenders with the in-binary PMV_GFX logic)
# 3. Repacks with appimagetool when available
#
# Usage:
#   ./scripts/fix-appimage-wayland.sh [path/to/pmvheaven_*.AppImage]
#
# Compare graphics modes after fix:
#   PMV_GFX=wayland   ./pmvheaven_….AppImage   # default, GPU/DMABUF on
#   PMV_GFX=dmabuf-off ./pmvheaven_….AppImage
#   PMV_GFX=soft      ./pmvheaven_….AppImage
#   PMV_GFX=stock     ./pmvheaven_….AppImage   # no fixes

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEFAULT_APPIMAGE="$(ls -1 "$ROOT"/target/dx/pmvheaven/bundle/linux/appimage/pmvheaven_*.AppImage 2>/dev/null | sort | tail -1 || true)"
APPIMAGE="${1:-$DEFAULT_APPIMAGE}"

if [[ -z "$APPIMAGE" || ! -f "$APPIMAGE" ]]; then
  echo "AppImage not found. Build first:"
  echo "  dx bundle --platform desktop --release"
  echo "Then: $0 path/to/pmvheaven_*.AppImage"
  exit 1
fi

APPIMAGE="$(cd "$(dirname "$APPIMAGE")" && pwd)/$(basename "$APPIMAGE")"
WORKDIR="$(mktemp -d)"
trap 'rm -rf "$WORKDIR"' EXIT

echo "==> Extracting $(basename "$APPIMAGE")"
cd "$WORKDIR"
"$APPIMAGE" --appimage-extract >/dev/null

APPDIR="$WORKDIR/squashfs-root"
BIN_NAME="pmvheaven"
if [[ ! -x "$APPDIR/usr/bin/$BIN_NAME" ]]; then
  BIN_NAME="$(basename "$(find "$APPDIR/usr/bin" -type f -executable | head -1)")"
fi

echo "==> Removing bundled Wayland client libs (force system ABI)"
REMOVED=0
for f in "$APPDIR"/usr/lib/libwayland-*.so*; do
  if [[ -e "$f" ]]; then
    echo "    rm $(basename "$f")"
    rm -f "$f"
    REMOVED=$((REMOVED + 1))
  fi
done
echo "    removed $REMOVED library files"

echo "==> Installing AppRun wrapper"
rm -f "$APPDIR/AppRun"
cat > "$APPDIR/AppRun" <<EOF
#!/usr/bin/env bash
set -euo pipefail
HERE="\$(dirname "\$(readlink -f "\$0")")"
export PATH="\$HERE/usr/bin:\${PATH:-}"
export LD_LIBRARY_PATH="\$HERE/usr/lib:\${LD_LIBRARY_PATH:-}"

# Prefer host Wayland client over anything still resolved from the AppDir.
if [[ -z "\${LD_PRELOAD:-}" ]]; then
  for lib in \\
    /usr/lib/libwayland-client.so.0 \\
    /usr/lib/libwayland-client.so \\
    /usr/lib64/libwayland-client.so.0 \\
    /usr/lib/x86_64-linux-gnu/libwayland-client.so.0 \\
    /usr/lib/aarch64-linux-gnu/libwayland-client.so.0; do
    if [[ -f "\$lib" ]]; then
      export LD_PRELOAD="\$lib"
      break
    fi
  done
fi

# Prefer host GStreamer plugins (VAAPI / NVDEC) over the AppImage's partial set.
for gst in \\
  /usr/lib/gstreamer-1.0 \\
  /usr/lib64/gstreamer-1.0 \\
  /usr/lib/x86_64-linux-gnu/gstreamer-1.0; do
  if [[ -d "\$gst" ]]; then
    export GST_PLUGIN_SYSTEM_PATH_1_0="\$gst\${GST_PLUGIN_SYSTEM_PATH_1_0:+:\$GST_PLUGIN_SYSTEM_PATH_1_0}"
    export GST_PLUGIN_PATH="\$gst\${GST_PLUGIN_PATH:+:\$GST_PLUGIN_PATH}"
    break
  fi
done

# Default graphics mode: system Wayland + keep DMABUF (best video FPS).
export PMV_GFX="\${PMV_GFX:-wayland}"
export PMV_WAYLAND_PRELOADED="\${PMV_WAYLAND_PRELOADED:-1}"
# Keep GPU compositing on unless the user overrode PMV_GFX.
if [[ "\${PMV_GFX}" == "wayland" ]]; then
  unset WEBKIT_DISABLE_DMABUF_RENDERER || true
  unset WEBKIT_DISABLE_COMPOSITING_MODE || true
  export GDK_GL="\${GDK_GL:-gles}"
  export GST_GL_PLATFORM="\${GST_GL_PLATFORM:-egl}"
  export GST_GL_API="\${GST_GL_API:-gles2}"
fi

exec "\$HERE/usr/bin/$BIN_NAME" "\$@"
EOF
chmod +x "$APPDIR/AppRun"

OUT_DIR="$(dirname "$APPIMAGE")"
OUT_NAME="$(basename "$APPIMAGE" .AppImage)-wayland.AppImage"
OUT_PATH="$OUT_DIR/$OUT_NAME"

if command -v appimagetool >/dev/null 2>&1; then
  echo "==> Repacking with appimagetool → $OUT_PATH"
  ARCH="${ARCH:-x86_64}" appimagetool "$APPDIR" "$OUT_PATH"
  chmod +x "$OUT_PATH"
  echo "Done: $OUT_PATH"
elif command -v appimagetool.AppImage >/dev/null 2>&1; then
  echo "==> Repacking with appimagetool.AppImage → $OUT_PATH"
  ARCH="${ARCH:-x86_64}" appimagetool.AppImage "$APPDIR" "$OUT_PATH"
  chmod +x "$OUT_PATH"
  echo "Done: $OUT_PATH"
else
  # Fallback: leave an extract dir next to the AppImage for manual run / later pack.
  FALLBACK="$OUT_DIR/$(basename "$APPIMAGE" .AppImage)-wayland.AppDir"
  rm -rf "$FALLBACK"
  cp -a "$APPDIR" "$FALLBACK"
  echo "appimagetool not found — wrote extract dir instead:"
  echo "  $FALLBACK"
  echo "Run with:"
  echo "  $FALLBACK/AppRun"
  echo "Or install appimagetool and re-run this script to produce a single .AppImage."
fi

echo
echo "A/B performance modes:"
echo "  PMV_GFX=wayland   $OUT_PATH   # default — preload + DMABUF on"
echo "  PMV_GFX=dmabuf-off $OUT_PATH"
echo "  PMV_GFX=soft      $OUT_PATH"
echo "  PMV_GFX=stock     $OUT_PATH   # no fixes"
