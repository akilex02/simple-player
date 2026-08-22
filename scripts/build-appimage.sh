#!/bin/sh
set -e

export WEBKIT_DISABLE_COMPOSITING_MODE=1
export GDK_BACKEND=x11

echo "1/4 Building release binary..."
npx tauri build --no-bundle

APPDIR="src-tauri/target/release/bundle/appimage/simple-player.AppDir"
mkdir -p "$APPDIR/usr/bin"
mkdir -p "$APPDIR/usr/lib/gstreamer-1.0"
mkdir -p "$APPDIR/usr/share/applications"
mkdir -p "$APPDIR/usr/share/icons/hicolor/512x512/apps"

echo "2/4 Setting up AppDir files & launcher..."
cp src-tauri/target/release/simple-player "$APPDIR/usr/bin/simple-player-bin"

cat << 'EOF' > "$APPDIR/usr/bin/simple-player"
#!/bin/sh
export WEBKIT_DISABLE_COMPOSITING_MODE=1
export GDK_BACKEND=x11
HERE="$(dirname "$(readlink -f "${0}")")"
exec "${HERE}/simple-player-bin" "$@"
EOF
chmod +x "$APPDIR/usr/bin/simple-player"

if [ -f "src-tauri/icons/icon.png" ]; then
    cp "src-tauri/icons/icon.png" "$APPDIR/simple-player.png"
    cp "src-tauri/icons/icon.png" "$APPDIR/.DirIcon"
    cp "src-tauri/icons/icon.png" "$APPDIR/usr/share/icons/hicolor/512x512/apps/simple-player.png"
fi

cat << 'EOF' > "$APPDIR/simple-player.desktop"
[Desktop Entry]
Name=Simple Player
Exec=simple-player
Icon=simple-player
Type=Application
Categories=Audio;Music;Player;AudioVideo;
Comment=Simple Player Music Player
EOF
cp "$APPDIR/simple-player.desktop" "$APPDIR/usr/share/applications/simple-player.desktop"

# Create AppRun symlink which is REQUIRED for the AppImage runtime to execute
ln -sf usr/bin/simple-player "$APPDIR/AppRun"

echo "3/4 Bundling GStreamer audio plugins..."
(cp -r /usr/lib64/gstreamer-1.0/* "$APPDIR/usr/lib/gstreamer-1.0/" 2>/dev/null || true)
(cp -r /usr/lib/gstreamer-1.0/* "$APPDIR/usr/lib/gstreamer-1.0/" 2>/dev/null || true)

echo "4/4 Packaging AppImage binary..."
/home/akilex/.cache/tauri/linuxdeploy-plugin-appimage.AppImage --appdir "$APPDIR"
cp Simple_Player-x86_64.AppImage src-tauri/target/release/bundle/appimage/simple-player_0.1.0_amd64.AppImage

echo "SUCCESS: Standalone AppImage created at src-tauri/target/release/bundle/appimage/simple-player_0.1.0_amd64.AppImage"
