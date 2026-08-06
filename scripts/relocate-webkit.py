#!/usr/bin/env python3
"""Patch hardcoded WebKitGTK helper paths inside bundled libwebkit*.so*.

WebKitGTK embeds compile-time paths like:
  /usr/lib/x86_64-linux-gnu/webkit2gtk-4.1
  /usr/lib/webkit2gtk-4.1

Those absolute host paths break AppImages on other distros. WEBKIT_EXEC_PATH
is not honored on production builds, so we rewrite the C strings in-place to a
stable /tmp symlink that AppRun creates at launch.

Usage:
  relocate-webkit.py <AppDir> [--link-path /tmp/.pmvheaven-wk]
"""

from __future__ import annotations

import argparse
import pathlib
import re
import sys


DEFAULT_LINK = "/tmp/.pmvheaven-wk"

# Paths WebKitGTK commonly bakes in (exact directory of WebKitNetworkProcess).
PATH_PATTERNS = [
    re.compile(rb"/usr/lib/x86_64-linux-gnu/webkit2gtk-4\.1(?:/injected-bundle)?/?"),
    re.compile(rb"/usr/lib64/webkit2gtk-4\.1(?:/injected-bundle)?/?"),
    re.compile(rb"/usr/lib/aarch64-linux-gnu/webkit2gtk-4\.1(?:/injected-bundle)?/?"),
    re.compile(rb"/usr/lib/webkit2gtk-4\.1(?:/injected-bundle)?/?"),
]


def nul_pad(replacement: bytes, length: int) -> bytes:
    if len(replacement) > length:
        raise ValueError(
            f"replacement longer than original ({len(replacement)} > {length}): {replacement!r}"
        )
    return replacement + (b"\0" * (length - len(replacement)))


def patch_blob(blob: bytes, link_path: str) -> tuple[bytes, int]:
    """Replace every known WebKit helper prefix with link_path (+ suffix)."""
    link = link_path.encode("ascii")
    total = 0
    out = blob

    # Collect unique exact matches first so overlapping replaces are stable.
    matches: list[bytes] = []
    for pat in PATH_PATTERNS:
        for m in pat.finditer(blob):
            s = m.group(0)
            if s not in matches:
                matches.append(s)

    # Prefer longer strings first so we don't partially clobber suffixes.
    matches.sort(key=len, reverse=True)

    for old in matches:
        # Preserve trailing /injected-bundle/ if present.
        if old.endswith(b"/injected-bundle/"):
            new = link + b"/injected-bundle/"
        elif old.endswith(b"/injected-bundle"):
            new = link + b"/injected-bundle"
        elif old.endswith(b"/"):
            new = link + b"/"
        else:
            new = link

        padded = nul_pad(new, len(old))
        count = out.count(old)
        if count:
            out = out.replace(old, padded)
            total += count
            print(f"  patched {count}× {old!r} → {new!r} (nul-padded to {len(old)})")

    return out, total


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("appdir", type=pathlib.Path)
    ap.add_argument("--link-path", default=DEFAULT_LINK)
    args = ap.parse_args()

    appdir: pathlib.Path = args.appdir
    libs = sorted(appdir.glob("usr/lib/**/libwebkit2gtk-4.1.so*")) + sorted(
        appdir.glob("usr/lib/libwebkit2gtk-4.1.so*")
    )
    # Also scan nested lib dirs linuxdeploy may use.
    libs = sorted({p.resolve() for p in appdir.rglob("libwebkit2gtk-4.1.so*") if p.is_file()})

    if not libs:
        print("relocate-webkit: no libwebkit2gtk-4.1.so* under AppDir", file=sys.stderr)
        return 2

    patched_any = 0
    for lib in libs:
        blob = lib.read_bytes()
        new_blob, n = patch_blob(blob, args.link_path)
        if n == 0:
            print(f"relocate-webkit: no hardcoded paths in {lib}")
            continue
        lib.write_bytes(new_blob)
        patched_any += n
        print(f"relocate-webkit: wrote {lib}")

    if patched_any == 0:
        print(
            "relocate-webkit: WARNING — found webkit libs but no known path strings to patch",
            file=sys.stderr,
        )
        return 3

    print(f"relocate-webkit: {patched_any} replacements; AppRun must symlink {args.link_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
