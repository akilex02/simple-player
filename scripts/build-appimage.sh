#!/bin/sh
# Genera un AppImage del binario nativo (eframe/egui + GStreamer).
# Requiere `linuxdeploy-plugin-appimage`: en el PATH o indicado con
# LINUXDEPLOY_PLUGIN_APPIMAGE=/ruta/al/plugin. Los plugins de GStreamer se
# copian del sistema, así que deben estar instalados.
set -e

cd "$(dirname "$0")/.."

PLUGIN="${LINUXDEPLOY_PLUGIN_APPIMAGE:-linuxdeploy-plugin-appimage}"
VERSION="$(sed -n 's/^version *= *"\(.*\)"/\1/p' Cargo.toml | head -1)"
BUNDLE_DIR="target/release/bundle/appimage"
APPDIR="$BUNDLE_DIR/simple-player.AppDir"
OUTPUT="$BUNDLE_DIR/simple-player_${VERSION}_amd64.AppImage"

echo "1/4 Compilando el binario release..."
cargo build --release

rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr/bin"
mkdir -p "$APPDIR/usr/lib/gstreamer-1.0"
mkdir -p "$APPDIR/usr/share/applications"
mkdir -p "$APPDIR/usr/share/icons/hicolor/512x512/apps"

echo "2/4 Preparando el AppDir y el lanzador..."
cp target/release/simple-player "$APPDIR/usr/bin/simple-player-bin"

cat << 'EOF2' > "$APPDIR/usr/bin/simple-player"
#!/bin/sh
HERE="$(dirname "$(readlink -f "${0}")")"
exec "${HERE}/simple-player-bin" "$@"
EOF2
chmod +x "$APPDIR/usr/bin/simple-player"

cp assets/icons/icon.png "$APPDIR/simple-player.png"
cp assets/icons/icon.png "$APPDIR/.DirIcon"
cp assets/icons/icon.png "$APPDIR/usr/share/icons/hicolor/512x512/apps/simple-player.png"

cat << 'EOF2' > "$APPDIR/simple-player.desktop"
[Desktop Entry]
Name=Simple Player
Exec=simple-player
Icon=simple-player
Type=Application
Categories=Audio;Music;Player;AudioVideo;
Comment=Simple Player Music Player
EOF2
cp "$APPDIR/simple-player.desktop" "$APPDIR/usr/share/applications/simple-player.desktop"

# AppRun es obligatorio para que el runtime del AppImage ejecute la app.
ln -sf usr/bin/simple-player "$APPDIR/AppRun"

echo "3/4 Incluyendo los plugins de audio de GStreamer..."
(cp -r /usr/lib64/gstreamer-1.0/* "$APPDIR/usr/lib/gstreamer-1.0/" 2>/dev/null || true)
(cp -r /usr/lib/gstreamer-1.0/* "$APPDIR/usr/lib/gstreamer-1.0/" 2>/dev/null || true)

echo "4/4 Empaquetando el AppImage..."
"$PLUGIN" --appdir "$APPDIR"
cp Simple_Player-x86_64.AppImage "$OUTPUT"

echo "LISTO: AppImage creado en $OUTPUT"
