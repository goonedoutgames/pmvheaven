#!/usr/bin/env bash
# Post-process a Dioxus AppImage for portable Linux hosts.
#
# 1. Bundles WebKitGTK helper processes (Network/Web/GPU) into the AppDir
# 2. Byte-patches libwebkit2gtk hardcoded helper paths → /tmp/.pmvheaven-wk
# 3. Strips bundled libwayland* so rolling compositors use the system ABI
# 4. Installs an AppRun that symlinks helpers, preloads Wayland, prefers host GST
# 5. Repacks with appimagetool when available
#
# Usage:
#   ./scripts/fix-appimage-wayland.sh [path/to/pmvheaven_*.AppImage]

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEFAULT_APPIMAGE="$(ls -1 "$ROOT"/target/dx/pmvheaven/bundle/linux/appimage/pmvheaven_*.AppImage 2>/dev/null | sort | tail -1 || true)"
APPIMAGE="${1:-$DEFAULT_APPIMAGE}"
WK_LINK="/tmp/.pmvheaven-wk"
RELOCATE="$ROOT/scripts/relocate-webkit.py"

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

echo "==> Bundling WebKitGTK helper processes"
WK_SRC=""
for d in \
  /usr/lib/x86_64-linux-gnu/webkit2gtk-4.1 \
  /usr/lib/aarch64-linux-gnu/webkit2gtk-4.1 \
  /usr/lib64/webkit2gtk-4.1 \
  /usr/lib/webkit2gtk-4.1
do
  if [[ -x "$d/WebKitNetworkProcess" ]]; then
    WK_SRC="$d"
    break
  fi
done

if [[ -z "$WK_SRC" ]]; then
  echo "ERROR: WebKitNetworkProcess not found on build host." >&2
  echo "Install webkit2gtk-4.1 (Arch) or libwebkit2gtk-4.1-0 (Debian/Ubuntu)." >&2
  exit 1
fi

# Keep Ubuntu-style layout inside the AppDir (matches CI-built lib paths).
WK_DST="$APPDIR/usr/lib/x86_64-linux-gnu/webkit2gtk-4.1"
# On aarch64 hosts, still use a stable relative layout under usr/lib.
if [[ "$(uname -m)" == "aarch64" ]]; then
  WK_DST="$APPDIR/usr/lib/aarch64-linux-gnu/webkit2gtk-4.1"
fi
mkdir -p "$(dirname "$WK_DST")"
rm -rf "$WK_DST"
cp -a "$WK_SRC" "$WK_DST"
echo "    copied $WK_SRC → ${WK_DST#"$APPDIR"/}"

# Helpers must resolve bundled libs next to them / in usr/lib.
if command -v patchelf >/dev/null 2>&1; then
  for helper in WebKitNetworkProcess WebKitWebProcess WebKitGPUProcess jsc MiniBrowser; do
    f="$WK_DST/$helper"
    [[ -f "$f" && -x "$f" ]] || continue
    # $ORIGIN = webkit2gtk-4.1 dir; ../ = multiarch libdir; ../../ = usr/lib
    patchelf --set-rpath '$ORIGIN:$ORIGIN/..:$ORIGIN/../..' "$f" || true
  done
  echo "    patchelf rpath set on WebKit helpers"
else
  echo "    WARNING: patchelf not found — helpers may fail to load bundled libs"
fi

echo "==> Relocating hardcoded WebKit paths → $WK_LINK"
python3 "$RELOCATE" "$APPDIR" --link-path "$WK_LINK"

echo "==> Removing bundled Wayland client libs (force system ABI)"
REMOVED=0
while IFS= read -r -d '' f; do
  echo "    rm ${f#"$APPDIR"/}"
  rm -f "$f"
  REMOVED=$((REMOVED + 1))
done < <(find "$APPDIR/usr/lib" -maxdepth 3 -name 'libwayland-*.so*' -print0 2>/dev/null || true)
echo "    removed $REMOVED library files"

echo "==> Installing AppRun wrapper"
rm -f "$APPDIR/AppRun"
cat > "$APPDIR/AppRun" <<EOF
#!/usr/bin/env bash
set -euo pipefail
HERE="\$(dirname "\$(readlink -f "\$0")")"
export PATH="\$HERE/usr/bin:\${PATH:-}"
export LD_LIBRARY_PATH="\$HERE/usr/lib:\${LD_LIBRARY_PATH:-}"
# Multiarch lib dirs used by Ubuntu CI bundles.
for libdir in \\
  "\$HERE/usr/lib/x86_64-linux-gnu" \\
  "\$HERE/usr/lib/aarch64-linux-gnu" \\
  "\$HERE/usr/lib64"; do
  if [[ -d "\$libdir" ]]; then
    export LD_LIBRARY_PATH="\$libdir:\${LD_LIBRARY_PATH}"
  fi
done

# WebKitGTK looks for helpers at a compile-time absolute path. We patch the
# bundled libwebkit to use this /tmp symlink, then point it into the AppDir.
WK_LINK="$WK_LINK"
WK_TARGET=""
for d in \\
  "\$HERE/usr/lib/x86_64-linux-gnu/webkit2gtk-4.1" \\
  "\$HERE/usr/lib/aarch64-linux-gnu/webkit2gtk-4.1" \\
  "\$HERE/usr/lib64/webkit2gtk-4.1" \\
  "\$HERE/usr/lib/webkit2gtk-4.1"; do
  if [[ -x "\$d/WebKitNetworkProcess" ]]; then
    WK_TARGET="\$d"
    break
  fi
done
if [[ -z "\$WK_TARGET" ]]; then
  echo "pmvheaven: WebKitNetworkProcess missing from AppImage" >&2
  exit 1
fi
ln -sfn "\$WK_TARGET" "\$WK_LINK"
export WEBKIT_INJECTED_BUNDLE_PATH="\$WK_TARGET/injected-bundle"

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
# Overwrite the release-style name when given; otherwise write -portable alongside.
BASE="$(basename "$APPIMAGE" .AppImage)"
if [[ "$BASE" == PMVHeaven-* ]]; then
  OUT_PATH="$OUT_DIR/${BASE}.AppImage"
else
  OUT_PATH="$OUT_DIR/${BASE}-portable.AppImage"
fi

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
  FALLBACK="$OUT_DIR/${BASE}-portable.AppDir"
  rm -rf "$FALLBACK"
  cp -a "$APPDIR" "$FALLBACK"
  echo "appimagetool not found — wrote extract dir instead:"
  echo "  $FALLBACK"
  echo "Run with: $FALLBACK/AppRun"
  exit 1
fi
