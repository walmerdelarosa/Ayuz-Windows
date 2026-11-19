#!/usr/bin/env bash
# Builds a portable Windows bundle of Ayuz from an MSYS2 UCRT64 shell:
#
#   dist/ayuz/bin/ayuz.exe            + every GTK/libadwaita DLL it needs
#   dist/ayuz/lib/gdk-pixbuf-2.0/...  image loaders (SVG/PNG icons)
#   dist/ayuz/share/glib-2.0/schemas  compiled GSettings schemas
#   dist/ayuz/share/icons/...         Adwaita + hicolor icon themes
#   dist/ayuz/share/ayuz/anime        AniMatrix GIF gallery
#
# and zips it to dist/ayuz-windows-x86_64.zip.
#
# Usage (after `cargo build --release`):  bash packaging/windows/bundle.sh
set -euo pipefail

PREFIX="${MINGW_PREFIX:-/ucrt64}"
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
DIST="$ROOT/dist"
OUT="$DIST/ayuz"
EXE="$ROOT/target/release/ayuz.exe"

[ -f "$EXE" ] || { echo "missing $EXE - run 'cargo build --release' first" >&2; exit 1; }

rm -rf "$DIST"
mkdir -p "$OUT/bin" "$OUT/lib" "$OUT/share/glib-2.0/schemas" "$OUT/share/icons" "$OUT/share/ayuz"

cp "$EXE" "$OUT/bin/"

# Copies every DLL from $PREFIX/bin that the given binaries depend on.
copy_deps() {
    for bin in "$@"; do
        ldd "$bin" | awk '{print $3}' | grep -i "^${PREFIX}/bin/" | sort -u | while read -r dll; do
            [ -f "$OUT/bin/$(basename "$dll")" ] || cp "$dll" "$OUT/bin/"
        done
    done
}

copy_deps "$OUT/bin/ayuz.exe"

# gdk-pixbuf loaders are loaded at runtime, so ldd does not see them.
PIXBUF_DIR="lib/gdk-pixbuf-2.0/2.10.0"
mkdir -p "$OUT/$PIXBUF_DIR/loaders"
cp "$PREFIX/$PIXBUF_DIR/loaders/"*.dll "$OUT/$PIXBUF_DIR/loaders/" 2>/dev/null || true
if compgen -G "$OUT/$PIXBUF_DIR/loaders/*.dll" >/dev/null; then
    copy_deps "$OUT/$PIXBUF_DIR/loaders/"*.dll
    # Regenerate the cache and make every module path relative to the bundle
    # root (whatever slash style the tool printed), so the zip is relocatable.
    gdk-pixbuf-query-loaders "$OUT/$PIXBUF_DIR/loaders/"*.dll \
        | sed -E 's#^"[^"]*[/\\]+dist[/\\]+ayuz[/\\]+#"#' \
        > "$OUT/$PIXBUF_DIR/loaders.cache"
fi

# GSettings schemas used by GTK (file chooser, settings, ...).
cp "$PREFIX/share/glib-2.0/schemas/"*.xml "$OUT/share/glib-2.0/schemas/" 2>/dev/null || true
glib-compile-schemas "$OUT/share/glib-2.0/schemas"

# Icon themes: Ayuz forces "Adwaita" and uses its symbolic icons.
for theme in Adwaita hicolor; do
    if [ -d "$PREFIX/share/icons/$theme" ]; then
        cp -r "$PREFIX/share/icons/$theme" "$OUT/share/icons/"
    fi
done

# AniMatrix GIFs (resolved as <bin>/../share/ayuz/anime).
cp -r "$ROOT/assets/anime" "$OUT/share/ayuz/anime"

cp "$ROOT/LICENSE" "$OUT/LICENSE.txt"
cp "$ROOT/docs/WINDOWS.md" "$OUT/README-Windows.md"

(cd "$DIST" && zip -qr ayuz-windows-x86_64.zip ayuz)
echo "bundle: $DIST/ayuz-windows-x86_64.zip"
