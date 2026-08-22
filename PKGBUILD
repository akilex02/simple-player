# Maintainer: AkiLeX raydercontacto@gmail.com
pkgname=simple-player-bin
pkgver=0.1.0
pkgrel=1
pkgdesc="Un reproductor de música hermoso y dinámico con motor GStreamer."
arch=('x86_64')
url="https://github.com/akilex02/simple-player"
license=('MIT')

# Dependencias nativas en CachyOS/Arch para que funcione perfectamente
depends=('webkit2gtk-4.1' 'gst-plugins-good' 'gst-plugins-bad' 'gst-plugins-ugly' 'gst-libav' 'glib2' 'gtk3')

provides=('simple-player')
conflicts=('simple-player')

# URL de descarga apuntando a tu Release de GitHub (AppImage)
source=("${pkgname}-${pkgver}.AppImage::https://github.com/akilex02/simple-player/releases/download/simpleplayer/Simple_Player-x86_64.appimage")

# Por seguridad, reemplaza 'SKIP' con el hash sha256 de tu AppImage usando el comando: sha256sum tu_archivo.AppImage
sha256sums=('d083f7ccbd0136f6d5d64775f7dc74ae01f6d9cf06c7732344fb0abb138f00b3')

prepare() {
    chmod +x "${srcdir}/${pkgname}-${pkgver}.AppImage"
    # Extraemos el contenido del AppImage
    "${srcdir}/${pkgname}-${pkgver}.AppImage" --appimage-extract
}

package() {
    # Instalar el binario
    install -Dm755 "${srcdir}/squashfs-root/usr/bin/simple-player-bin" "${pkgdir}/usr/bin/simple-player"
    
    # Instalar el archivo .desktop para el menú de aplicaciones
    install -Dm644 "${srcdir}/squashfs-root/usr/share/applications/simple-player.desktop" "${pkgdir}/usr/share/applications/simple-player.desktop"
    
    # Instalar el icono
    install -Dm644 "${srcdir}/squashfs-root/usr/share/icons/hicolor/512x512/apps/simple-player.png" "${pkgdir}/usr/share/icons/hicolor/512x512/apps/simple-player.png"
}
