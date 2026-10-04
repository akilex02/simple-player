#!/usr/bin/env bash
# Instala (o quita con --remove) una entrada de escritorio de USUARIO para la compilación local.
# Plasma y otros escritorios la usan para relacionar la ventana con su ícono y con los controles
# multimedia (anterior / pausa / siguiente) de la miniatura de la barra de tareas.
# Solo escribe en ~/.local/share; no necesita permisos de administrador.
set -euo pipefail
cd "$(dirname "$0")/.."

APPS="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
ICONS="${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor/512x512/apps"
DESKTOP="$APPS/simple-player.desktop"
ICON="$ICONS/simple-player.png"

if [[ "${1:-}" == "--remove" ]]; then
  rm -f "$DESKTOP" "$ICON"
  echo "Entrada de escritorio eliminada."
  exit 0
fi

BIN="$(pwd)/target/release/simple-player"
[ -x "$BIN" ] || { echo "Primero compila: cargo build --release" >&2; exit 1; }

mkdir -p "$APPS" "$ICONS"
cp assets/icons/icon.png "$ICON"
# Misma entrada que usa el AppImage, pero con la ruta absoluta del binario local.
sed "s|^Exec=.*|Exec=$BIN|" assets/simple-player.desktop > "$DESKTOP"
command -v update-desktop-database >/dev/null && update-desktop-database "$APPS" || true
echo "Instalada: $DESKTOP"
echo "Cierra y vuelve a abrir la app para que el escritorio la reconozca."
