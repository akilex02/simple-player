# Simple Player

Un reproductor de música local para Linux, rápido y con estética "liquid-glass", escrito en **Rust** con **egui/eframe** y **GStreamer**. Biblioteca por canciones, álbumes y artistas, letras sincronizadas, visualizador de espectro y estadísticas de escucha.

## Características

- **Biblioteca local:** escaneo de tu carpeta de música con portadas, artista, álbum y duración; búsqueda con relevancia, orden por columna y pestañas de Canciones, Álbumes y Artistas.
- **Reproducción:** motor GStreamer (MP3, FLAC, AAC, WAV…), seek preciso, aleatorio, repetir, cola visible y control de volumen.
- **Pantalla completa "Ahora suena":** portada grande, fondo desenfocado que sigue a la carátula y acento dinámico.
- **Letras sincronizadas:** desde un `.lrc` junto a la canción o las etiquetas del archivo; la línea activa se resalta y un clic en una línea salta a ese momento.
- **Visualizador de espectro:** barras, radial, resplandor o franja inferior, alineado con el reloj de reproducción.
- **Estadísticas de escucha:** tiempo total, reproducciones, tops de canciones, artistas y álbumes, días activos, rachas, sesiones y línea de tiempo por rango (hoy, semana, mes, año, todo).
- **Integración con el escritorio:** MPRIS2 (teclas multimedia y controles del sistema) y atajos globales.
- **Ligero:** en reposo consume ~0.2 % de CPU; el repintado continuo solo ocurre mientras suena música.

## Atajos de teclado

| Tecla | Acción |
|---|---|
| `Espacio` | Reproducir / pausar |
| `←` / `→` | Retroceder / avanzar 5 s |
| `↑` / `↓` | Subir / bajar el volumen |
| `Alt+←` / `Alt+→` | Canción anterior / siguiente |
| `M` | Silenciar |
| `Q` | Mostrar la cola |
| `F` | Pantalla completa "Ahora suena" |
| `L` | Letra |
| `/` o `Ctrl+K` | Buscar |
| `Esc` | Cerrar la pantalla completa |
| `F11` | Pantalla completa de la ventana |
| `F6` / `F7` / `F8` | Anterior / pausa / siguiente (globales, también con la ventana minimizada) |
| `F3` | Panel de rendimiento (desarrollo) |

## Instalación (Linux)

### Opción 1: AppImage
1. Ve a [Releases](../../releases) y descarga el `.AppImage`.
2. Dale permisos de ejecución: `chmod +x Simple_Player-*.AppImage`

### Opción 2: Arch Linux / CachyOS
```bash
paru -S simple-player-bin
```

### Opción 3: Compilar desde el código

Necesitas **Rust** (edición 2021) y las librerías de desarrollo de **GStreamer** (`gstreamer`, `gst-plugins-base`, `gst-plugins-bad`; en Arch: `gst-plugins-base gst-plugins-bad`).

```bash
git clone https://github.com/akilex02/simple-player.git
cd simple-player
cargo run --release
```

> **Códecs:** para reproducir todos los formatos instala `gst-plugins-good`, `gst-plugins-bad`, `gst-plugins-ugly` y `gst-libav`. El selector de carpeta usa `xdg-desktop-portal`.

## Estructura del proyecto

```
.
├─ Cargo.toml            # crate único (binario `simple-player`)
├─ assets/               # fuentes (GTA Art Deco, Noto Sans), ícono y logo (SVG)
├─ scripts/
│  └─ build-appimage.sh  # empaqueta un AppImage
├─ PKGBUILD              # paquete de Arch (AppImage)
├─ docs/superpowers/     # especificaciones y planes de diseño
└─ src/
   ├─ main.rs            # arranque, ciclo de la UI y atajos
   ├─ state.rs           # estado de la app (cola, reproducción, pestañas)
   ├─ audio/             # reproductor GStreamer, espectro y reloj de reproducción
   ├─ viz/               # motor del visualizador (interpolación, barras, suavizado)
   ├─ stats/             # estadísticas: sesión, almacén SQLite, agregación y servicio
   ├─ library*.rs        # escaneo y vistas cacheadas de la biblioteca
   ├─ lyrics.rs          # lectura y sincronización de letras
   ├─ mpris.rs, hotkeys.rs
   ├─ theme/             # colores, fuentes, íconos y movimiento
   └─ ui/                # shell (sidebar, barra superior, barra inferior),
                         # pantallas, widgets y pantalla completa
```

## Datos que guarda la app

| Qué | Dónde |
|---|---|
| Caché de la biblioteca y carátulas, y el estado de reproducción | `~/.cache/music-player/` |
| Historial de escucha (estadísticas) | `~/.local/share/simple-player/stats.db` (respeta `XDG_DATA_HOME`) |
| Tamaño de la ventana (abre en 1050×750 la primera vez) | `~/.config/simple-player/window.json` (respeta `XDG_CONFIG_HOME`) |

Todo es local; la app no usa la red.

## Desarrollo

```bash
cargo test                      # pruebas (lógica pura: sin audio ni GPU)
cargo run --release             # ejecutar
cargo run --release -- --gallery   # galería del sistema de diseño
```

Opciones útiles (no escriben en tu base de estadísticas real):

| Opción | Efecto |
|---|---|
| `--tab albums\|artists\|stats` | Abre esa pestaña |
| `--stats-demo` | Carga un mes de escucha sintético en memoria |
| `--stats-range today\|week\|month\|year\|all` | Rango de Estadísticas |
| `--stats-db <ruta>` | Usa esa base de estadísticas |
| `--fullscreen`, `--lyrics`, `--queue` | Abre la pantalla completa, las letras o la cola |
| `--shot <ruta.png>` | Guarda una captura de la app y se cierra |
| `--bench <segundos>` | Mide el consumo de CPU y se cierra |
| `--allow-multiple`, `--no-hotkeys` | Permite otra instancia y omite los atajos globales |
| `--window-size AxB` | Abre con ese tamaño (p. ej. `854x658`) sin guardarlo |

Para un AppImage: `scripts/build-appimage.sh` (requiere `linuxdeploy-plugin-appimage`).

## Licencia

MIT
