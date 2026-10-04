#!/usr/bin/env bash
# Instala los paquetes de apt que necesita cada job (ubuntu-22.04).
#   build    compilar y probar
#   package  empaquetar el AppImage autocontenido (plugins de GStreamer y herramientas)
set -euo pipefail

BUILD_PKGS="build-essential pkg-config libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev libgstreamer-plugins-bad1.0-dev libdbus-1-dev libglib2.0-dev libxkbcommon-dev libx11-dev gstreamer1.0-plugins-base gstreamer1.0-plugins-good"
# (los plugins base y good los necesitan las pruebas de AppState, que crean un reproductor real: playbin)
PACKAGE_PKGS="$BUILD_PKGS patchelf file desktop-file-utils gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-plugins-ugly gstreamer1.0-libav gstreamer1.0-pulseaudio gstreamer1.0-alsa gstreamer1.0-pipewire"

case "${1:-}" in
  build) PKGS="$BUILD_PKGS" ;;
  package) PKGS="$PACKAGE_PKGS" ;;
  *) echo "Uso: $0 build|package" >&2; exit 2 ;;
esac

sudo apt-get update
# El runner trae libunwind-14-dev (de LLVM), que choca con el libunwind-dev que exige libgstreamer1.0-dev.
sudo apt-get remove -y libunwind-14-dev || true
# shellcheck disable=SC2086
sudo apt-get install -y --no-install-recommends $PKGS
