# Simple Player

Un reproductor de música local para Linux, rápido y con estética "liquid-glass", escrito en **Rust** con **egui/eframe** y **GStreamer**. Biblioteca por canciones, álbumes y artistas, letras sincronizadas, visualizador de espectro y estadísticas de escucha.

## Características

- **Biblioteca local:** escaneo de tu carpeta de música con portadas, artista, álbum y duración; búsqueda con relevancia, orden por columna y pestañas de Canciones, Álbumes y Artistas.
- **Reproducción:** motor GStreamer (MP3, FLAC, AAC, WAV…), seek preciso, aleatorio, repetir, cola visible y control de volumen.
- **Pantalla completa "Ahora suena":** portada grande, fondo desenfocado que sigue a la carátula y acento dinámico.
- **Letras sincronizadas:** desde un `.lrc` junto a la canción o las etiquetas del archivo; la línea activa se resalta y un clic en una línea salta a ese momento.
- **Retoma donde te quedaste:** al cerrar guarda la cola y el segundo en que ibas; al abrir queda en pausa en ese punto y continúa cuando das play (también con las teclas multimedia o los controles de la barra de tareas).
- **Configuración:** varias carpetas de música, visualizador por defecto, recordar el tamaño de la ventana, exportar/importar/borrar el historial y restablecer de fábrica.
- **Visualizadores:** barras, anillo alrededor de la portada, partículas, constelación, osciloscopio o franja inferior, alineados con el reloj de reproducción; con un clic mantenido las partículas siguen al cursor.
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
| `Ctrl+Q` | Cerrar la aplicación |
| `F` | Pantalla completa "Ahora suena" |
| `L` | Letra |
| `/` o `Ctrl+K` | Buscar |
| `Esc` | Cerrar la pantalla completa |
| `G` | Atraer / repeler las partículas con el clic (pantalla completa) |
| `F11` | Pantalla completa de la ventana |
| `F6` / `F7` / `F8` | Anterior / pausa / siguiente (globales, también con la ventana minimizada) |
| `F3` | Panel de rendimiento (desarrollo) |

## Instalación (Linux)

### Opción 1: AppImage (descargas automáticas)

Cada cambio de código en la rama `nueva-version` publica un *pre-release* en [Releases](../../releases) con su resumen de cambios y dos archivos:

| Archivo | Para quién |
|---|---|
| `Simple_Player-…-x86_64.AppImage` | **Autocontenido:** funciona en casi cualquier distro; incluye GStreamer y sus bibliotecas (pesa más). |
| `Simple_Player-…-x86_64-light.AppImage` | **Ligero:** usa las bibliotecas del sistema (Arch/CachyOS o distros con GStreamer instalado). |

```bash
chmod +x Simple_Player-*.AppImage
./Simple_Player-*.AppImage
sha256sum -c SHA256SUMS --ignore-missing   # opcional: verificar la descarga
```

Para que Plasma muestre los controles multimedia en la miniatura de la barra de tareas, registra el AppImage en el menú (por ejemplo con AppImageLauncher).

> La rama `master` conserva la versión anterior (Tauri) como referencia; los releases nuevos son de la versión nativa.

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

> **Controles multimedia en la barra de tareas (Plasma y similares):** el escritorio relaciona la ventana con su reproductor por medio del archivo `.desktop`. El AppImage y el paquete ya lo traen; si ejecutas la compilación local ejecuta una vez `scripts/install-desktop-entry.sh` (solo escribe en `~/.local/share`; `--remove` lo quita) y vuelve a abrir la app.

> **Códecs:** para reproducir todos los formatos instala `gst-plugins-good`, `gst-plugins-bad`, `gst-plugins-ugly` y `gst-libav`. El selector de carpeta usa `xdg-desktop-portal`.

## Estructura del proyecto

```
.
├─ Cargo.toml            # crate único (binario `simple-player`)
├─ assets/               # fuentes (GTA Art Deco en toda la interfaz; Noto Sans de respaldo), ícono y logo (SVG)
├─ scripts/
│  └─ build-appimage.sh  # empaqueta un AppImage
├─ PKGBUILD              # paquete de Arch (AppImage)
├─ docs/superpowers/     # especificaciones y planes de diseño
└─ src/
   ├─ main.rs            # arranque, ciclo de la UI y atajos
   ├─ state.rs           # estado de la app (cola, reproducción, pestañas)
   ├─ audio/             # reproductor GStreamer, espectro y reloj de reproducción
   ├─ viz/               # motor del visualizador (interpolación, bandas, física de partículas)
   ├─ stats/             # estadísticas: sesión, almacén SQLite, agregación, servicio y exportación
   ├─ settings.rs        # ajustes del usuario (carpetas, preferencias) y su migración
   ├─ reset.rs           # restablecer de fábrica
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
| Caché de la biblioteca y carátulas, estado de reproducción y posición de la canción | `~/.cache/music-player/` |
| Historial de escucha (estadísticas) | `~/.local/share/simple-player/stats.db` (respeta `XDG_DATA_HOME`) |
| Tamaño de la ventana (abre en 1050×750; mínimo 1000×650) | `~/.config/simple-player/window.json` (respeta `XDG_CONFIG_HOME`) |
| Ajustes (carpetas de música, visualizador por defecto, recordar tamaño) | `~/.config/simple-player/settings.json` (respeta `XDG_CONFIG_HOME`) |

Todo es local; la app no usa la red. En Configuración → Acerca de puedes abrir cada una de estas carpetas.

## Desarrollo

```bash
cargo test                      # pruebas (lógica pura: sin audio ni GPU)
cargo run --release             # ejecutar
cargo run --release -- --gallery   # galería del sistema de diseño
bash scripts/tests/run-all.sh   # pruebas de los scripts de release y del workflow
```

Opciones útiles (no escriben en tu base de estadísticas real):

| Opción | Efecto |
|---|---|
| `--tab albums\|artists\|stats\|settings` | Abre esa pestaña |
| `--music-folder <ruta>` | Siembra una carpeta en los ajustes en memoria (repetible) |
| `--stats-demo` | Carga un mes de escucha sintético en memoria |
| `--stats-range today\|week\|month\|year\|all` | Rango de Estadísticas |
| `--stats-db <ruta>` | Usa esa base de estadísticas |
| `--fullscreen`, `--lyrics`, `--queue` | Abre la pantalla completa, las letras o la cola |
| `--viz barras\|anillo\|particulas\|constelacion\|osciloscopio\|franja\|apagado` | Abre la pantalla completa con ese visualizador |
| `--shot <ruta.png>` | Guarda una captura de la app y se cierra |
| `--bench <segundos>` | Mide CPU, GPU (vía `nvidia-smi` o sysfs) y vértices por frame, y se cierra |
| `--allow-multiple`, `--no-hotkeys` | Permite otra instancia y omite los atajos globales |
| `--window-size AxB` | Abre con ese tamaño (p. ej. `854x658`) sin guardarlo |

Para medir CPU y GPU de cada visualizador: `scripts/bench-visualizers.sh` (variables `SECS`, `CPU_MAX`, `MS_MAX`, `GPU_MAX`); falla si algún modo rebasa los límites.

Para un AppImage: `scripts/build-appimage.sh` (requiere `linuxdeploy-plugin-appimage`).

## Licencia

MIT
