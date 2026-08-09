#!/usr/bin/env bash
# Post-process a Dioxus AppImage for portable Linux hosts.
#
# COMPARISON / DEV ONLY — released Linux builds are Flatpak
# (packaging/flatpak/com.pmvheaven.Desktop.yml). Keep this script for the
# linux-packaging CI AppImage job and local experiments; do not treat
# AppImage as the supported distribution format.
#
# 1. Bundles WebKitGTK helper processes (Network/Web/GPU) into the AppDir
# 2. Byte-patches libwebkit2gtk hardcoded helper paths → /tmp/.pmvheaven-wk
# 3. Strips bundled libwayland* so rolling compositors use the system ABI
# 4. Bundles GStreamer plugins matching the AppImage's libgstreamer ABI
# 5. Installs an AppRun that symlinks helpers, preloads Wayland, uses AppDir GST only
# 6. Repacks with appimagetool when available
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

echo "==> Bundling GStreamer plugins (same ABI as AppImage libgstreamer)"
# linuxdeploy only pulls core libgst*.so — not the plugin modules. Mixing host
# plugins (e.g. Arch 1.28) with Ubuntu 1.24 libs causes undefined-symbol storms.
GST_SRC=""
for d in \
  /usr/lib/x86_64-linux-gnu/gstreamer-1.0 \
  /usr/lib/aarch64-linux-gnu/gstreamer-1.0 \
  /usr/lib64/gstreamer-1.0 \
  /usr/lib/gstreamer-1.0
do
  if [[ -d "$d" ]] && compgen -G "$d/libgst*.so" >/dev/null 2>&1; then
    GST_SRC="$d"
    break
  fi
done

if [[ -z "$GST_SRC" ]]; then
  echo "ERROR: no gstreamer-1.0 plugin dir on build host." >&2
  echo "Install gstreamer1.0-plugins-base/good (Debian) or gst-plugins-base/good (Arch)." >&2
  exit 1
fi

GST_DST_PARENT="$(dirname "$WK_DST")"  # e.g. .../usr/lib/x86_64-linux-gnu
GST_DST="$GST_DST_PARENT/gstreamer-1.0"
mkdir -p "$GST_DST"
# Prefer cp (always available). Avoid rsync --delete wiping a partial copy on error.
cp -a "$GST_SRC"/. "$GST_DST"/
PLUGIN_COUNT="$(find "$GST_DST" -maxdepth 1 -name 'libgst*.so' | wc -l)"
echo "    copied $PLUGIN_COUNT plugins from $GST_SRC → ${GST_DST#"$APPDIR"/}"
if [[ "$PLUGIN_COUNT" -lt 10 ]]; then
  echo "ERROR: too few GStreamer plugins copied ($PLUGIN_COUNT)" >&2
  exit 1
fi
for need in libgstplayback.so libgstautodetect.so; do
  if [[ ! -f "$GST_DST/$need" ]]; then
    echo "ERROR: required plugin missing after copy: $need" >&2
    ls -la "$GST_DST" | head -40 >&2 || true
    exit 1
  fi
done
# Drop empty/partial leftover plugin dirs that confuse loaders & CI checks.
for leftover in \
  "$APPDIR/usr/lib/gstreamer-1.0" \
  "$APPDIR/usr/lib64/gstreamer-1.0"
do
  if [[ -d "$leftover" && "$leftover" != "$GST_DST" ]]; then
    if [[ ! -f "$leftover/libgstplayback.so" ]]; then
      echo "    removing leftover plugin dir ${leftover#"$APPDIR"/}"
      rm -rf "$leftover"
    fi
  fi
done
# gst-plugin-scanner lives next to plugins on some distros / in libexec on others
for scanner in \
  "$GST_SRC/gst-plugin-scanner" \
  /usr/libexec/gstreamer-1.0/gst-plugin-scanner \
  /usr/lib/x86_64-linux-gnu/gstreamer1.0/gstreamer-1.0/gst-plugin-scanner \
  /usr/lib/gstreamer-1.0/gst-plugin-scanner
do
  if [[ -x "$scanner" ]]; then
    mkdir -p "$APPDIR/usr/libexec/gstreamer-1.0"
    cp -a "$scanner" "$APPDIR/usr/libexec/gstreamer-1.0/gst-plugin-scanner"
    if command -v patchelf >/dev/null 2>&1; then
      patchelf --set-rpath '$ORIGIN/../../lib:$ORIGIN/../../lib/x86_64-linux-gnu' \
        "$APPDIR/usr/libexec/gstreamer-1.0/gst-plugin-scanner" 2>/dev/null || true
    fi
    echo "    bundled gst-plugin-scanner"
    break
  fi
done

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

# Use ONLY AppImage GStreamer plugins — same ABI as bundled libgstreamer.
# Mixing host plugins (newer Arch) with Ubuntu libs → undefined symbol freezes.
GST_PLUGINS=""
PLAYBACK=""
while IFS= read -r -d '' f; do
  PLAYBACK="\$f"
  break
done < <(find "\$HERE/usr/lib" -type f -name 'libgstplayback.so' -print0 2>/dev/null || true)
if [[ -n "\$PLAYBACK" ]]; then
  GST_PLUGINS="\$(dirname "\$PLAYBACK")"
fi
if [[ -z "\$GST_PLUGINS" ]]; then
  for d in \\
    "\$HERE/usr/lib/x86_64-linux-gnu/gstreamer-1.0" \\
    "\$HERE/usr/lib/aarch64-linux-gnu/gstreamer-1.0" \\
    "\$HERE/usr/lib64/gstreamer-1.0" \\
    "\$HERE/usr/lib/gstreamer-1.0"; do
    if [[ -d "\$d" ]]; then
      GST_PLUGINS="\$d"
      break
    fi
  done
fi
if [[ -z "\$GST_PLUGINS" || ! -f "\$GST_PLUGINS/libgstplayback.so" ]]; then
  echo "pmvheaven: gstreamer-1.0 plugins missing from AppImage" >&2
  exit 1
fi
export GST_PLUGIN_SYSTEM_PATH_1_0="\$GST_PLUGINS"
export GST_PLUGIN_PATH="\$GST_PLUGINS"
if [[ -x "\$HERE/usr/libexec/gstreamer-1.0/gst-plugin-scanner" ]]; then
  export GST_PLUGIN_SCANNER="\$HERE/usr/libexec/gstreamer-1.0/gst-plugin-scanner"
fi
# Prevent registry from picking up incompatible host plugins.
export GST_REGISTRY="\$HOME/.cache/pmvheaven/gst-registry.bin"
mkdir -p "\$HOME/.cache/pmvheaven" 2>/dev/null || true

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
