# Simple Player 🎵

Un reproductor de música hermoso, dinámico y ultra-ligero desarrollado con **Tauri**, **React** y **GStreamer**. Diseñado para brindar una experiencia estética premium ("liquid-glass") en sistemas Linux, garantizando un rendimiento excelente gracias a su backend en Rust y motor de audio nativo.

## Características Principales ✨

- 🎨 **Diseño "Liquid Glass":** Interfaz moderna y dinámica con colores vibrantes y desenfoques atractivos.
- 🚀 **Ultra Ligero:** Construido sobre Tauri v2, con un consumo de memoria mínimo comparado con aplicaciones Electron.
- 🎧 **Motor GStreamer:** Reproducción de audio robusta, compatible con múltiples formatos (MP3, FLAC, AAC, WAV) a través del potente backend nativo de GStreamer.
- 🎛️ **Controles Precisos:** Busca exactamente el nanosegundo que deseas de tu canción de manera instantánea.
- 📂 **Escaneo Local:** Detección automática de la carpeta de Música y extracción de metadatos (portadas, artista, título).

## Instalación (Linux) 🐧

### Opción 1: AppImage (Universal)
1. Ve a la pestaña de [Releases](../../releases) de este repositorio.
2. Descarga el archivo `.AppImage`.
3. Dale permisos de ejecución: `chmod +x Simple_Player-*.AppImage`
4. ¡Haz doble clic y disfruta!

### Opción 2: Arch Linux / CachyOS (AUR)
Puedes instalar la aplicación de forma nativa a través de AUR usando tu gestor favorito (por ejemplo, `paru` o `yay`):

```bash
paru -S simple-player-bin
```

> **Nota de dependencias:** Si compilas o usas el AppImage, asegúrate de tener los códecs de GStreamer instalados en tu sistema (`gst-plugins-good`, `gst-plugins-bad`, `gst-plugins-ugly`, `gst-libav`).

## Desarrollo 🛠️

Si quieres clonar el proyecto y modificarlo tú mismo, necesitas tener instalado `Node.js`, `Rust`, y las librerías de desarrollo de `GStreamer` y `WebKit2GTK`.

```bash
# Clonar el repositorio
git clone https://github.com/TU_USUARIO_DE_GITHUB/simple-player.git
cd simple-player

# Instalar dependencias web
npm install

# Iniciar servidor de desarrollo
npm run tauri dev

# Construir AppImage
npm run build:appimage
```

---
*Construido con ❤️ usando Tauri y GStreamer.*
