#!/usr/bin/env bash
# Genera un AppImage del binario nativo (eframe/egui + GStreamer).
#
#   scripts/build-appimage.sh [full|light] [--binary RUTA]
#
#   light  Solo el binario con su lanzador; usa las bibliotecas del sistema (lo que espera el PKGBUILD).
#   full   Autocontenido: incluye GStreamer, sus plugins y las bibliotecas de apoyo.
#
# Variables: APP_VERSION (por defecto, la de Cargo.toml), TOOLS_DIR, DIST_DIR, LOCK_FILE.
# Sin --binary compila con `cargo build --release --locked`.
set -euo pipefail
cd "$(dirname "$0")/.."
ROOT_DIR="$PWD"

VARIANT="${1:-light}"
[ $# -gt 0 ] && shift
BINARY=""
while [ $# -gt 0 ]; do
  case "$1" in
    --binary) BINARY="${2:?falta la ruta del binario}"; shift 2 ;;
    *) echo "Opción desconocida: $1" >&2; exit 2 ;;
  esac
done
case "$VARIANT" in
  full | light) ;;
  *) echo "Uso: $0 [full|light] [--binary RUTA]" >&2; exit 2 ;;
esac

VERSION="${APP_VERSION:-$(sed -n 's/^version *= *"\(.*\)"/\1/p' Cargo.toml | head -1)}"
TOOLS_DIR="${TOOLS_DIR:-.cache/appimage-tools}"
DIST_DIR="${DIST_DIR:-dist}"
LOCK_FILE="${LOCK_FILE:-scripts/appimage-tools.lock}"
SUFFIX=""
[ "$VARIANT" = "light" ] && SUFFIX="-light"
OUTPUT="$DIST_DIR/Simple_Player-${VERSION}-x86_64${SUFFIX}.AppImage"

# El runner no tiene FUSE: las AppImage se ejecutan extrayéndose.
export APPIMAGE_EXTRACT_AND_RUN=1
export ARCH=x86_64

# shellcheck disable=SC1090
. "$LOCK_FILE"

# Descarga una herramienta y verifica su SHA-256 antes de dejarla ejecutable.
fetch_tool() { # nombre url sha256
  local name="$1" url="$2" sha="$3" dest="$TOOLS_DIR/$1" got
  mkdir -p "$TOOLS_DIR"
  if [ -f "$dest" ] && [ "$(sha256sum "$dest" | cut -d' ' -f1)" = "$sha" ]; then
    chmod +x "$dest"
    return
  fi
  echo "Descargando $name..."
  curl -fsSL --retry 3 -o "$dest.tmp" "$url"
  got="$(sha256sum "$dest.tmp" | cut -d' ' -f1)"
  if [ "$got" != "$sha" ]; then
    rm -f "$dest.tmp"
    echo "El hash de $name no coincide (esperado $sha, obtenido $got)" >&2
    exit 1
  fi
  mv "$dest.tmp" "$dest"
  chmod +x "$dest"
}

fetch_tool "$APPIMAGETOOL_NAME" "$APPIMAGETOOL_URL" "$APPIMAGETOOL_SHA256"
if [ "$VARIANT" = "full" ]; then
  fetch_tool "$LINUXDEPLOY_NAME" "$LINUXDEPLOY_URL" "$LINUXDEPLOY_SHA256"
  fetch_tool "$GSTREAMER_PLUGIN_NAME" "$GSTREAMER_PLUGIN_URL" "$GSTREAMER_PLUGIN_SHA256"
  fetch_tool "$APPIMAGE_PLUGIN_NAME" "$APPIMAGE_PLUGIN_URL" "$APPIMAGE_PLUGIN_SHA256"
fi
[ "${FETCH_ONLY:-0}" = "1" ] && exit 0

if [ -z "$BINARY" ]; then
  echo "Compilando el binario release..."
  cargo build --release --locked
  BINARY="target/release/simple-player"
fi
[ -x "$BINARY" ] || { echo "No existe el binario: $BINARY" >&2; exit 1; }

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
APPDIR="$WORK/simple-player.AppDir"
mkdir -p "$DIST_DIR"

# AppDir común: binario real + lanzador (el PKGBUILD extrae `simple-player-bin`).
prepare_appdir() {
  mkdir -p "$APPDIR/usr/bin" "$APPDIR/usr/share/applications" "$APPDIR/usr/share/icons/hicolor/512x512/apps"
  install -m755 "$BINARY" "$APPDIR/usr/bin/simple-player-bin"
  printf '%s\n' '#!/bin/sh' 'HERE="$(dirname "$(readlink -f "${0}")")"' 'exec "${HERE}/simple-player-bin" "$@"' > "$APPDIR/usr/bin/simple-player"
  chmod +x "$APPDIR/usr/bin/simple-player"
  cp assets/icons/icon.png "$APPDIR/simple-player.png"
  cp assets/icons/icon.png "$APPDIR/.DirIcon"
  cp assets/icons/icon.png "$APPDIR/usr/share/icons/hicolor/512x512/apps/simple-player.png"
  cp assets/simple-player.desktop "$APPDIR/simple-player.desktop"
  cp assets/simple-player.desktop "$APPDIR/usr/share/applications/simple-player.desktop"
}

prepare_appdir

case "$VARIANT" in
  light)
    ln -sf usr/bin/simple-player "$APPDIR/AppRun"
    "$TOOLS_DIR/$APPIMAGETOOL_NAME" --no-appstream "$APPDIR" "$OUTPUT"
    ;;
  full)
    echo "La variante full todavía no está implementada" >&2
    exit 3
    ;;
esac

echo "LISTO: $OUTPUT"
