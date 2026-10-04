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

# Plugins de GStreamer que se incluyen (lista curada: copiar todos arrastra decenas de dependencias
# gráficas que no hacen falta para reproducir audio). Los REQUIRED hacen fallar el build si faltan.
GST_REQUIRED="coreelements typefindfunctions playback audioconvert audioresample volume spectrum app"
GST_OPTIONAL="audiorate audioparsers id3demux apetag flac wavparse ogg vorbis opus isomp4 matroska mpg123 libav faad autodetect pulseaudio alsa pipewire audiofx level equalizer"
# Bibliotecas que el sistema anfitrión aporta (controladores de gráficos y de pantalla): no se exigen dentro del AppImage.
HOST_LIBS_RE='libGL|libEGL|libGLX|libGLESv2|libOpenGL|libX11|libXext|libXv|libxcb|libwayland|libvulkan|libdrm|libasound'

# Primer directorio que exista de una lista (Debian/Ubuntu y Arch tienen rutas distintas).
first_dir() { for d in "$@"; do [ -d "$d" ] && { echo "$d"; return; }; done; return 1; }

# Copia solo los plugins elegidos a un directorio nuevo y devuelve su ruta.
curated_plugins_dir() { # directorio_origen
  local src="$1" dst="$WORK/gst-plugins" name
  mkdir -p "$dst"
  for name in $GST_REQUIRED; do
    [ -f "$src/libgst${name}.so" ] || { echo "Falta el plugin de GStreamer requerido: libgst${name}.so (en $src)" >&2; exit 1; }
    cp "$src/libgst${name}.so" "$dst/"
  done
  for name in $GST_OPTIONAL; do
    if [ -f "$src/libgst${name}.so" ]; then cp "$src/libgst${name}.so" "$dst/"; else echo "Aviso: no está libgst${name}.so; se omite" >&2; fi
  done
  echo "$dst"
}

# Falla si alguna biblioteca incluida (o el binario) depende de algo que no está dentro ni lo aporta el sistema.
verify_bundle() {
  local missing=0 f out
  while IFS= read -r f; do
    out="$(LD_LIBRARY_PATH="$APPDIR/usr/lib" ldd "$f" 2>/dev/null | grep 'not found' | grep -Ev "$HOST_LIBS_RE" || true)"
    if [ -n "$out" ]; then
      echo "Faltan bibliotecas para ${f#"$APPDIR"/}:" >&2
      echo "$out" >&2
      missing=1
    fi
  done < <(find "$APPDIR/usr" -type f \( -name '*.so' -o -name '*.so.*' -o -name 'simple-player-bin' -o -name 'gst-plugin-scanner' \))
  [ "$missing" -eq 0 ] || { echo "El AppImage autocontenido tiene bibliotecas sin resolver" >&2; exit 1; }
}

build_full() {
  command -v patchelf >/dev/null || { echo "Falta patchelf (lo necesita el plugin de GStreamer)" >&2; exit 1; }
  command -v file >/dev/null || { echo "Falta el comando file" >&2; exit 1; }
  local plugins_src helpers_dir
  plugins_src="$(first_dir /usr/lib/x86_64-linux-gnu/gstreamer-1.0 /usr/lib/gstreamer-1.0 /usr/lib64/gstreamer-1.0)" \
    || { echo "No encuentro los plugins de GStreamer del sistema" >&2; exit 1; }
  helpers_dir="$(first_dir /usr/lib/x86_64-linux-gnu/gstreamer1.0/gstreamer-1.0 /usr/lib/gstreamer-1.0 /usr/libexec/gstreamer-1.0)" \
    || { echo "No encuentro los ayudantes de GStreamer (gst-plugin-scanner)" >&2; exit 1; }

  export PATH="$ROOT_DIR/$TOOLS_DIR:$PATH"
  export LINUXDEPLOY="$ROOT_DIR/$TOOLS_DIR/$LINUXDEPLOY_NAME"
  GSTREAMER_PLUGINS_DIR="$(curated_plugins_dir "$plugins_src")"
  export GSTREAMER_PLUGINS_DIR
  export GSTREAMER_HELPERS_DIR="$helpers_dir"

  # AppRun propio: define las rutas de GStreamer (hooks del plugin) y arranca el binario.
  cat > "$WORK/AppRun" <<'APPRUN_EOF'
#!/bin/sh
HERE="$(dirname "$(readlink -f "$0")")"
export APPDIR="${APPDIR:-$HERE}"
export LD_LIBRARY_PATH="$HERE/usr/lib:${LD_LIBRARY_PATH:-}"
for hook in "$HERE"/apprun-hooks/*.sh; do
  [ -f "$hook" ] && . "$hook"
done
exec "$HERE/usr/bin/simple-player-bin" "$@"
APPRUN_EOF
  chmod +x "$WORK/AppRun"

  (cd "$WORK" && "$LINUXDEPLOY" --appdir "$APPDIR" \
    --executable "$APPDIR/usr/bin/simple-player-bin" \
    --desktop-file "$APPDIR/simple-player.desktop" \
    --icon-file "$ROOT_DIR/assets/icons/icon.png" \
    --custom-apprun "$WORK/AppRun" \
    --plugin gstreamer)
  verify_bundle
  (cd "$WORK" && "$LINUXDEPLOY" --appdir "$APPDIR" --output appimage)
  local made
  made="$(ls "$WORK"/*.AppImage 2>/dev/null | head -n1 || true)"
  [ -n "$made" ] || { echo "linuxdeploy no generó ningún AppImage" >&2; exit 1; }
  mv "$made" "$OUTPUT"
}

case "$VARIANT" in
  light)
    ln -sf usr/bin/simple-player "$APPDIR/AppRun"
    "$TOOLS_DIR/$APPIMAGETOOL_NAME" --no-appstream "$APPDIR" "$OUTPUT"
    ;;
  full)
    build_full
    ;;
esac

echo "LISTO: $OUTPUT"
