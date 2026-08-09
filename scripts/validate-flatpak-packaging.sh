#!/usr/bin/env bash
# Static checks for Flatpak packaging (no flatpak-builder required).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "==> Required files"
for f in \
  packaging/flatpak/com.pmvheaven.Desktop.yml \
  packaging/flatpak/com.pmvheaven.Desktop.desktop \
  packaging/flatpak/com.pmvheaven.Desktop.metainfo.xml \
  scripts/gen-cargo-sources.sh \
  .github/workflows/linux-packaging.yml \
  .github/workflows/release.yml
do
  test -f "$f" || { echo "missing $f" >&2; exit 1; }
  echo "  ok $f"
done

echo "==> Manifest keys"
grep -q 'app-id: com.pmvheaven.Desktop' packaging/flatpak/com.pmvheaven.Desktop.yml
grep -q 'runtime: org.gnome.Platform' packaging/flatpak/com.pmvheaven.Desktop.yml
grep -q 'cargo-sources.json' packaging/flatpak/com.pmvheaven.Desktop.yml
grep -q 'org.freedesktop.Sdk.Extension.rust-stable' packaging/flatpak/com.pmvheaven.Desktop.yml

echo "==> Desktop Exec"
grep -q '^Exec=pmvheaven$' packaging/flatpak/com.pmvheaven.Desktop.desktop

echo "==> Generate cargo-sources"
./scripts/gen-cargo-sources.sh >/dev/null
test -s packaging/flatpak/cargo-sources.json
python3 -c 'import json; json.load(open("packaging/flatpak/cargo-sources.json"))'
echo "  cargo-sources.json OK ($(wc -c < packaging/flatpak/cargo-sources.json) bytes)"

echo "==> All packaging static checks passed"
echo "Next: push ci/linux-packaging (or workflow_dispatch Linux packaging),"
echo "      download the .flatpak artifact, then:"
echo "  flatpak install --user ./PMVHeaven-*-x86_64.flatpak"
echo "  flatpak run com.pmvheaven.Desktop"
