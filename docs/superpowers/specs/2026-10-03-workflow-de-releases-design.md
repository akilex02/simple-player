# Simple Player — Workflow de GitHub para compilar y publicar el AppImage

> **Estado:** diseño aprobado; pendiente de plan e implementación.
> **Rama de trabajo:** `nueva-version` (`master` queda como legado/referencia de la versión anterior; no se toca).
> **Fecha:** 2026-10-03

---

## 1. Objetivo y alcance

**Qué se quiere:** que GitHub Actions compile Simple Player y publique, con cada push a `nueva-version`, un release con los AppImage listos para descargar y un resumen de los commits que llegaron (a modo de changelog).

**Dentro del alcance**

- Un workflow `.github/workflows/release.yml` que prueba, compila, empaqueta y publica.
- Dos AppImage por release: **autocontenido** (incluye GStreamer y las bibliotecas de apoyo) y **ligero** (usa las bibliotecas del sistema).
- Un script del repo que genera las notas del release (`scripts/release-notes.sh`), con pruebas locales.
- Ajustes a `scripts/build-appimage.sh` para que lo use el workflow y siga sirviendo en local.
- Una sección "Descargas" en el README.

**Fuera del alcance (YAGNI)**

- Actualizar el `PKGBUILD` (apunta al tag `simpleplayer` y a un hash del AppImage antiguo de Tauri); se hace después del primer release.
- Compilar para otras arquitecturas o sistemas (solo `x86_64` Linux).
- Firmar los AppImage, publicar en Flathub/AUR automáticamente o crear releases "estables" (`v1.0.0`, "Latest").
- Cambiar la rama por defecto del repositorio o tocar `master`.
- Actualización automática dentro de la app.

---

## 2. Decisiones tomadas

| Tema | Decisión |
|---|---|
| Cuándo se publica | **Un release por push** a `nueva-version` (tag propio, pre-release). Además, ejecución manual. |
| Qué se publica | **Los dos AppImage**: autocontenido y ligero, más `SHA256SUMS`. |
| Changelog | Los commits desde el release `dev` anterior, generados por un script del repo. |
| Docs | Los pushes que solo cambian documentos no disparan release. |
| `master` | Sin workflow; queda como referencia. |
| Release "Latest" actual (Tauri) | No se toca: los nuevos son pre-releases. |

---

## 3. Disparadores

```yaml
on:
  push:
    branches: [nueva-version]
    paths-ignore: ['docs/**', '**/*.md', '.claude/**']
  workflow_dispatch:
concurrency:
  group: release-nueva-version
  cancel-in-progress: false      # cada push conserva su release; se encolan, no se cancelan
```

- `paths-ignore` solo omite la ejecución si **todos** los archivos del push coinciden; un push que mezcla código y docs sí publica.
- **Ejecución manual:** `workflow_dispatch` permite lanzarlo desde la pestaña Actions o con `gh workflow run release.yml --ref nueva-version`. GitHub a veces muestra el botón "Run workflow" solo cuando el archivo existe también en la rama por defecto (`master`); se comprueba tras el primer push. Plan B: `gh workflow run` desde el terminal o "Re-run all jobs" sobre una ejecución anterior. No se agrega el workflow a `master`.
- Una ejecución manual repetida sobre el mismo commit publica otro release (con otro número) cuyas notas dicen "Sin cambios desde el release anterior".

---

## 4. Estructura del workflow

Todo en `ubuntu-22.04` (glibc 2.35: compatible con distros más nuevas; un AppImage compilado en una glibc nueva no corre en una vieja).

### 4.1 Job `build` — probar y compilar

1. `actions/checkout@v4`.
2. Instalar las dependencias de compilación con `apt` (GStreamer y su `-dev`, `libgstreamer-plugins-base1.0-dev`, `libgstreamer-plugins-bad1.0-dev` para `gstreamer-player`, `libdbus-1-dev` para MPRIS, `pkg-config` y herramientas de compilación). La lista exacta se fija en el plan comprobándola contra un contenedor limpio de Ubuntu 22.04 cuando haya Docker/Podman, y si no, con la primera ejecución.
3. Rust estable con `rustup` (ya presente en el runner) y caché con `actions/cache@v4` sobre `~/.cargo/registry`, `~/.cargo/git` y `target`, con clave basada en `Cargo.lock`.
4. `cargo test --locked` — **si falla, no se publica nada**.
5. `cargo build --release --locked`.
6. Sube el binario (`target/release/simple-player`) como artefacto.

### 4.2 Job `package` — dos AppImage en paralelo (matriz `full` / `light`)

- Depende de `build`; descarga el binario (no recompila) y llama a `scripts/build-appimage.sh <full|light> --binary <ruta>`.
- Sube el AppImage resultante como artefacto.

### 4.3 Job `publish` — crear el release

- Depende de `package`; es el **único** con `permissions: contents: write`.
- Pasos: checkout con `fetch-depth: 0` (historial y tags), descargar artefactos, calcular versión y tag, generar `SHA256SUMS`, generar las notas, crear el release **en borrador**, subir los archivos, publicarlo (quitar el borrador) y podar los viejos. Si algo falla antes de publicar, no queda un release a medias a la vista.

---

## 5. Empaquetado (`scripts/build-appimage.sh`)

Uso: `scripts/build-appimage.sh [full|light] [--binary RUTA]`. Sin `--binary` compila con `cargo build --release` (uso local, como hoy). La salida va a `dist/`.

### 5.1 `light` (ligero)

- Es el flujo actual: AppDir con el binario, el lanzador, `assets/simple-player.desktop` y el ícono, empaquetado con `appimagetool`/`linuxdeploy-plugin-appimage`. **No** incluye bibliotecas: exige que la distro tenga GStreamer (base, good, bad, libav), `dbus`, `libxkbcommon` y `libglvnd`. Es lo que espera el `PKGBUILD`.
- Se elimina la copia de los plugins de GStreamer del sistema (no aportaba nada y fallaba en silencio: `/usr/lib64` y `/usr/lib/gstreamer-1.0` no existen en Debian/Ubuntu).

### 5.2 `full` (autocontenido)

- `linuxdeploy` con el plugin de GStreamer (`linuxdeploy-plugin-gstreamer`) y el de AppImage; incluye GStreamer, sus plugins (base, good, bad, libav si está disponible) y las bibliotecas de apoyo. Las bibliotecas de gráficos y de pantalla (OpenGL/EGL, Wayland, X11) se toman del sistema, como hace `linuxdeploy` por defecto con su lista de exclusión.
- Se ejecuta con extracción automática (`APPIMAGE_EXTRACT_AND_RUN=1`), porque el runner no tiene FUSE.
- **Herramientas con versión fija y hash:** las URL y los SHA-256 de `linuxdeploy` y sus plugins viven en un archivo versionado del repo (`scripts/appimage-tools.lock`) y el script verifica el hash antes de ejecutarlas. Actualizarlos es un commit explícito.
- **Verificación obligatoria:** tras empaquetar, el script extrae el AppImage y comprueba con `ldd` que ni el binario ni los plugins de GStreamer incluidos tienen bibliotecas "not found"; si hay alguna, el build falla.
- El lanzador usa el `AppRun` que genera el plugin de GStreamer (define las rutas de plugins y del escáner de GStreamer dentro del AppImage).

### 5.3 Ambos

- Nombre del archivo final: ver §6.
- El `.desktop` es `assets/simple-player.desktop` (con `StartupWMClass=simple-player`), como ya usa el resto del repo.

---

## 6. El release

| Campo | Valor |
|---|---|
| Versión | `version` de `Cargo.toml` (hoy `0.1.0`) |
| Tag | `v{version}-dev.{N}`, donde `N` es `github.run_number` (p. ej. `v0.1.0-dev.14`). Los números pueden tener huecos si hay ejecuciones fallidas. |
| Título | `Simple Player v{version}-dev.{N}` |
| Tipo | **pre-release** (el release "Latest" actual no cambia) |
| Commit | el del push (`--target $GITHUB_SHA`) |
| Archivos | `Simple_Player-{version}-dev.{N}-x86_64.AppImage` (autocontenido), `Simple_Player-{version}-dev.{N}-x86_64-light.AppImage` (ligero), `SHA256SUMS` |

**Limpieza:** se conservan los últimos **10** releases `dev` (por fecha de creación) y se borran los anteriores junto con su tag (`gh release delete --cleanup-tag`). El número es una variable del workflow (`KEEP_RELEASES`). Solo se consideran releases cuyo tag cumple `^v.+-dev\.[0-9]+$`; nunca se toca el release `simpleplayer` ni ningún otro.

---

## 7. Notas del release (`scripts/release-notes.sh`)

Script de shell que imprime Markdown. Entradas (argumentos o variables): repositorio (`owner/nombre`), SHA actual, SHA del release anterior (vacío si no hay), tag, versión y la carpeta de los archivos (para tamaños y hashes). Se prueba en local contra un repositorio temporal (`scripts/tests/release-notes.test.sh`).

Estructura:

1. **Encabezado:** "Compilación automática de `nueva-version` · commit `abc1234` · fecha UTC" y, si hay release anterior, el enlace "Comparar con el release anterior" (`prev...sha`).
2. **Cambios:** `- Título del commit ([abc1234](enlace))`, del más nuevo al más viejo.
   - Solo la primera línea de cada commit; sin merges; sin líneas de `Co-Authored-By`.
   - El título se escapa para que no rompa el Markdown ni mencione a nadie (se neutraliza `@`, se escapan `<` y `>`).
   - **Tope de 40** commits; si hay más: "… y N cambios más (ver la comparación completa)".
   - **Sin release anterior** (primera ejecución): los últimos 20 commits y una nota que lo explica.
   - **Sin commits nuevos** (ejecución manual repetida): "Sin cambios desde el release anterior."
3. **Descargas:** una tabla con los dos AppImage, para quién es cada uno, tamaño y SHA-256:
   - *Autocontenido:* "para la mayoría de las distros; incluye GStreamer".
   - *Ligero:* "para Arch/CachyOS y distros con GStreamer instalado (base, good, bad, libav), dbus, libxkbcommon y libglvnd".
4. **Cómo usarlo:** `chmod +x archivo.AppImage && ./archivo.AppImage` y cómo verificar el hash (`sha256sum -c SHA256SUMS`). Nota sobre los controles multimedia en Plasma: el AppImage suelto necesita registrarse en el menú (p. ej. AppImageLauncher) para que el escritorio lo relacione con su `.desktop`.
5. **Pie:** una línea que indica que es la versión nativa (Rust/egui) y que `master` conserva la versión anterior como referencia.

---

## 8. Seguridad y permisos

- Solo `GITHUB_TOKEN`; el workflow declara `permissions: contents: read` por defecto y **`contents: write` únicamente en el job `publish`**.
- Sin acciones de terceros para publicar: se usa la CLI `gh` del runner. Acciones usadas: `actions/checkout`, `actions/cache`, `actions/upload-artifact` y `actions/download-artifact` (oficiales de GitHub, fijadas a su versión mayor).
- Las herramientas descargadas para el AppImage se verifican por hash (§5.2).
- Las notas se generan a partir de textos de commits: se escapan para evitar Markdown/menciones inesperadas (§7). Nada de entradas del evento se interpola directamente en comandos de shell; se pasan por variables de entorno.
- Depende de que **Actions esté habilitado** en `akilex02/simple-player` y de que la política de permisos del repositorio no impida que el workflow declare `contents: write` (no se puede consultar sin permisos de administrador; se comprueba con el primer run).

---

## 9. Verificación

- **Antes del primer push:** pruebas de `release-notes.sh` en local (repositorio temporal con y sin release anterior, con merges, con `@` y `<` en los títulos, más de 40 commits y sin commits nuevos); `bash -n` y `shellcheck` (si está instalado) sobre los scripts; el YAML se valida con un parser; el script `build-appimage.sh light` se prueba localmente.
- **Primer push:** se sigue la ejecución con `gh run watch` y se leen los logs de cada job; se itera hasta que el release salga completo. Se comprueba: asset names, `SHA256SUMS`, notas, que el release sea pre-release y que el release antiguo siga siendo "Latest".
- **Descarga real:** se baja cada AppImage y se prueba (el ligero en esta máquina; el autocontenido con `--appimage-extract` y `ldd`, y abriéndolo con extracción automática). La prueba en una distro distinta la hace el usuario.
- **Segundo push:** se verifica que las notas muestran solo los commits nuevos y que la limpieza respeta el límite (se prueba con `KEEP_RELEASES` bajo en un release de prueba antes de dejarlo en 10).

---

## 10. Riesgos conocidos

- **El AppImage autocontenido suele requerir varias iteraciones** hasta quedar estable (plugins de GStreamer que faltan, rutas, bibliotecas excluidas). El primer run puede fallar.
- **`gstreamer-player`** viene de `gst-plugins-bad`; en Ubuntu 22.04 (GStreamer 1.20) debe existir el paquete `-dev` correspondiente; si no, se pasa a `ubuntu-24.04`.
- **Tamaño y tiempo:** el autocontenido pesa ~100–150 MB y el empaquetado tarda más; con 10 releases conservados el repositorio ocupa ~2 GB de assets (dentro de los límites de GitHub para releases, pero no despreciable).
- **Huecos en la numeración** `dev.N` cuando una ejecución falla.
- **`master` y el botón manual:** puede que el botón "Run workflow" no aparezca (ver §3).

---

## 11. Criterios de aceptación

1. Un push a `nueva-version` con cambios de código publica un pre-release `v0.1.0-dev.N` con los dos AppImage y `SHA256SUMS`.
2. Un push que solo cambia documentos no publica nada.
3. El release "Latest" antiguo (Tauri) sigue como "Latest" y el tag `simpleplayer` no se toca.
4. Las notas muestran los commits nuevos desde el release anterior (título y enlace), con el tope y los casos especiales de §7.
5. Si falla una prueba o el empaquetado, no se publica ningún release.
6. El AppImage ligero abre y reproduce en CachyOS; el autocontenido tiene todas sus bibliotecas resueltas y abre.
7. Solo se conservan los últimos 10 releases `dev`.
8. Se puede lanzar manualmente (botón o `gh workflow run`).
9. README con la sección "Descargas".

---

## 12. Orden de implementación

1. `scripts/release-notes.sh` y sus pruebas (se verifica sin GitHub).
2. `scripts/build-appimage.sh` con `full`/`light`, `--binary` y el archivo de herramientas con hash; prueba local del `light`.
3. `.github/workflows/release.yml` (jobs `build`, `package`, `publish`) y validación del YAML.
4. Primer push, seguimiento con `gh run watch` y las iteraciones necesarias hasta un release correcto.
5. README (sección Descargas) y verificación del segundo push y la limpieza.
