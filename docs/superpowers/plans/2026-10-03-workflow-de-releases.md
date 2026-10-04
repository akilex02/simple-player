# Workflow de releases — Plan de implementación

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Que cada push de código a `nueva-version` compile Simple Player en GitHub Actions y publique un pre-release con dos AppImage (autocontenido y ligero), `SHA256SUMS` y notas con los commits nuevos.

**Architecture:** Lógica en scripts del repo, probados en local sin GitHub (`scripts/release-notes.sh`, `scripts/ci/select-prune.sh`, `scripts/build-appimage.sh`), y un workflow delgado (`.github/workflows/release.yml`) con tres jobs: `build` (pruebas + binario), `package` (matriz `full`/`light`) y `publish` (único con permiso de escritura). Una prueba de política en Python valida la forma del workflow (permisos, disparadores, acciones permitidas, orden de publicación).

**Tech Stack:** GitHub Actions (`ubuntu-22.04`), Bash, Python 3 + PyYAML (solo pruebas), `gh` CLI, `linuxdeploy` + plugin de GStreamer + `appimagetool` (versiones fijas con SHA-256).

**Spec:** `docs/superpowers/specs/2026-10-03-workflow-de-releases-design.md`

## Global Constraints

- Disparadores: `push` a `nueva-version` con `paths-ignore: ['docs/**', '**/*.md', '.claude/**']`, y `workflow_dispatch` (con entrada opcional `keep_releases`, por defecto `10`). `concurrency: group: release-nueva-version`, `cancel-in-progress: false`. Sin workflow en `master`.
- Todos los jobs en `ubuntu-22.04`. Permisos: `contents: read` a nivel del workflow; `contents: write` **solo** en el job `publish`.
- Acciones permitidas (solo estas): `actions/checkout@v4`, `actions/cache@v4`, `actions/upload-artifact@v4`, `actions/download-artifact@v4`. Publicar con la CLI `gh`. Nada de `${{ ... }}` dentro de bloques `run:` (todo por `env:`).
- Versión: `version` de `Cargo.toml` (hoy `0.1.0`). Tag `v{version}-dev.{github.run_number}`; título `Simple Player v{version}-dev.{N}`; **pre-release**; `--target $GITHUB_SHA`; `--latest=false`.
- Archivos: `Simple_Player-{version}-dev.{N}-x86_64.AppImage` (autocontenido), `Simple_Player-{version}-dev.{N}-x86_64-light.AppImage` (ligero), `SHA256SUMS`.
- Se crea el release en **borrador**, se suben los archivos y solo después se publica (`--draft=false`); si falla algo antes de publicar, se borra el borrador.
- Limpieza: se conservan los últimos `KEEP_RELEASES` (10) releases cuyo tag cumple `^v.+-dev\.[0-9]+$`; nunca se tocan otros (p. ej. `simpleplayer`).
- Herramientas de empaquetado con URL y SHA-256 fijos en `scripts/appimage-tools.lock`; el hash se verifica antes de ejecutar.
- Notas: Markdown generado por `scripts/release-notes.sh`; títulos escapados (`&`, `<`, `>`, `[`, `]`, `@`); tope 40 commits; primer release 20; sin merges; sin `Co-Authored-By`.
- El `PKGBUILD` no se toca en este plan (queda anotado para después del primer release).
- Todo texto de interfaz y de las notas en español. Sin push ni merge sin que el usuario lo pida: **los pasos marcados «PUSH» requieren pedir permiso al usuario** (el flujo depende de empujar a `nueva-version`).
- Commits terminan con `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`.
- Comandos de prueba (desde la raíz): `bash scripts/tests/run-all.sh`.

**Aclaraciones al spec (decididas al planificar):**
- El AppImage autocontenido lleva una **lista curada de plugins de GStreamer** (en vez de copiar todos los del sistema): el plugin de `linuxdeploy` copia cada plugin que encuentre y arrastra decenas de dependencias gráficas (OpenGL, Qt, etc.) que no se necesitan para reproducir audio y que harían fallar la verificación `ldd`. La lista se ajusta en las iteraciones de CI.
- El `AppRun` del autocontenido es propio (carga `apprun-hooks/*.sh`, que es donde el plugin de GStreamer define sus rutas) y se pasa con `--custom-apprun`, para no depender de cómo genere `linuxdeploy` el suyo.
- Se conserva el diseño actual del AppDir (`usr/bin/simple-player-bin` + lanzador `usr/bin/simple-player`) en ambas variantes para no romper el `PKGBUILD`, que extrae `simple-player-bin`.
- Las notas se prueban con scripts de shell (no `cargo test`); `scripts/tests/run-all.sh` los ejecuta junto con la prueba de política del workflow.
- No hay `shellcheck`, `actionlint`, Docker ni Podman en esta máquina: la validación del workflow es la prueba de política en Python y, sobre todo, la ejecución real en GitHub (Tarea 8).

## Review Focus

1. **Títulos de commit hostiles** (`@usuario`, `[clic](http://…)`, `<script>`, `&`): las notas no deben mencionar a nadie, crear enlaces ni inyectar HTML. → Tarea 2.
2. **Release anterior inalcanzable** (historial reescrito o tag borrado a mano): las notas caen al modo "primer release" en vez de fallar o publicar un rango vacío engañoso. → Tarea 2.
3. **Poda segura:** nunca borra `simpleplayer`, releases sin patrón `-dev.N` ni más de los necesarios; conserva exactamente `KEEP`; valida que `KEEP` sea un número. → Tarea 3 y 6.
4. **Release a medias:** un fallo entre crear el borrador y publicar no deja nada visible y limpia el borrador. → Tarea 6 (prueba de política).
5. **Herramienta descargada alterada:** un hash que no coincide aborta el empaquetado antes de ejecutar nada. → Tarea 4.

---

### Task 1: Comprobación previa de Actions y permisos

**Files:**
- Create: `.github/workflows/ci-check.yml` (temporal; se borra en la Tarea 9)

**Interfaces:**
- Consumes: nada.
- Produces: la certeza de que Actions está habilitado y de que `contents: write` permite crear y borrar un release (o el mensaje para pedir el ajuste al propietario).

- [ ] **Step 1: Crear el workflow mínimo**

```yaml
name: Comprobación de CI (temporal)

on:
  push:
    branches: [nueva-version]
    paths: ['.github/workflows/ci-check.yml']
  workflow_dispatch:

permissions:
  contents: read

jobs:
  hola:
    runs-on: ubuntu-22.04
    steps:
      - name: Entorno
        run: |
          echo "Actions funciona"
          uname -a
          gcc --version | head -1
          rustc --version || true
          gh --version | head -1

  permiso-de-release:
    runs-on: ubuntu-22.04
    permissions:
      contents: write
    steps:
      - uses: actions/checkout@v4
      - name: Crear y borrar un release de prueba en borrador
        env:
          GH_TOKEN: ${{ github.token }}
        run: |
          set -euo pipefail
          gh release create v0.0.0-ci-check --draft --target "$GITHUB_SHA" \
            --title "Comprobación de CI" --notes "Release temporal de comprobación; se borra solo."
          gh release list --limit 5
          gh release delete v0.0.0-ci-check --yes --cleanup-tag
          echo "contents: write funciona"
```

- [ ] **Step 2: Validar el YAML y commitear**

Run: `python3 -c "import yaml,sys; d=yaml.safe_load(open('.github/workflows/ci-check.yml')); print(sorted(d['jobs']))"`
Expected: `['hola', 'permiso-de-release']`

```bash
git add .github/workflows/ci-check.yml
git commit -m "CI: comprobación temporal de Actions y de permisos de release

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

- [ ] **Step 3: PUSH (pedir permiso al usuario) y observar**

Pedir al usuario permiso para empujar a `nueva-version` (con el método de siempre: verificar fast-forward y empujar a `https://github.com/akilex02/simple-player.git` con el helper de `gh`). Después:

```bash
sleep 20; gh run list -R akilex02/simple-player --workflow ci-check.yml --limit 3
gh run watch -R akilex02/simple-player "$(gh run list -R akilex02/simple-player --workflow ci-check.yml --limit 1 --json databaseId --jq '.[0].databaseId')" --exit-status
```

Expected (éxito): los dos jobs en verde; el log de `permiso-de-release` termina con `contents: write funciona`.

- [ ] **Step 4: Si falla o no arranca, pedir el ajuste al propietario**

- **No aparece ninguna ejecución:** Actions está deshabilitado. Mensaje para `akilex02`:
  > Hola, para que GitHub compile y publique el AppImage de `nueva-version` necesito que habilites Actions en el repo: *Settings → Actions → General → Actions permissions → Allow all actions and reusable workflows* (o "Allow … select actions" incluyendo las de GitHub). ¡Gracias!
- **`permiso-de-release` falla con «Resource not accessible by integration» o HTTP 403:**
  > Hola, el workflow necesita crear releases: en *Settings → Actions → General → Workflow permissions* elige **Read and write permissions** (o confirma que se permite a los workflows solicitar `contents: write`). ¡Gracias!

Detener el plan hasta que el propietario lo haga y repetir el Step 3 con `gh workflow run ci-check.yml --ref nueva-version`.

---

### Task 2: Notas del release (`scripts/release-notes.sh`)

**Files:**
- Create: `scripts/release-notes.sh`, `scripts/tests/release-notes.test.sh`, `scripts/tests/run-all.sh`
- Test: `scripts/tests/release-notes.test.sh`

**Interfaces:**
- Consumes: un repositorio git (se ejecuta dentro de él).
- Produces: `bash scripts/release-notes.sh` imprime Markdown. Entradas por variables de entorno: `REPO`, `SHA`, `TAG`, `VERSION` (obligatorias); `PREV_SHA`, `ASSETS_DIR`, `DATE_UTC`, `MAX_COMMITS` (40), `FIRST_RUN_COMMITS` (20) (opcionales). La sección de cambios usa líneas que empiezan con `- `; ninguna otra sección usa viñetas con `- `.

- [ ] **Step 1: Write the failing tests**

`scripts/tests/run-all.sh`:

```bash
#!/usr/bin/env bash
# Ejecuta todas las pruebas de shell y la prueba de política del workflow.
set -uo pipefail
cd "$(dirname "$0")"
status=0
for t in *.test.sh; do
  echo "== $t"
  bash "$t" || status=1
done
if [ -f check-workflow.py ] && [ -f ../../.github/workflows/release.yml ]; then
  echo "== check-workflow.py"
  python3 check-workflow.py || status=1
fi
exit $status
```

`scripts/tests/release-notes.test.sh`:

```bash
#!/usr/bin/env bash
# Pruebas de scripts/release-notes.sh con repositorios git temporales.
set -uo pipefail
SCRIPT="$(cd "$(dirname "$0")/.." && pwd)/release-notes.sh"
ORIGIN="$(pwd)"
fails=0

ok()  { echo "ok   - $1"; }
bad() { echo "FAIL - $1"; fails=$((fails + 1)); }
has() { grep -qF -- "$2" <<<"$1" && ok "$3" || bad "$3 (falta: $2)"; }
lacks() { grep -qF -- "$2" <<<"$1" && bad "$3 (sobra: $2)" || ok "$3"; }
count_bullets() { grep -c '^- ' <<<"$1" || true; }

new_repo() {
  TMP="$(mktemp -d)"
  cd "$TMP" && git init -q -b main
  git config user.email t@t && git config user.name tester
}
commit() { git commit -q --allow-empty -m "$1"; git rev-parse HEAD; }
notes() {
  env REPO=o/r TAG=v0.1.0-dev.5 VERSION=0.1.0-dev.5 DATE_UTC="2026-10-03 12:00 UTC" "$@" bash "$SCRIPT"
}
done_repo() { cd "$ORIGIN" && rm -rf "$TMP"; }

# 1. Lista solo los commits nuevos, del más nuevo al más viejo.
new_repo
a=$(commit "Primero"); b=$(commit "Segundo"); c=$(commit "Tercero")
out=$(notes SHA="$c" PREV_SHA="$a")
has "$out" "- Tercero" "incluye el commit más nuevo"
has "$out" "- Segundo" "incluye el commit intermedio"
lacks "$out" "Primero" "no incluye lo anterior al release previo"
[ "$(grep -n 'Tercero' <<<"$out" | head -1 | cut -d: -f1)" -lt "$(grep -n 'Segundo' <<<"$out" | head -1 | cut -d: -f1)" ] \
  && ok "orden: más nuevo primero" || bad "orden: más nuevo primero"
has "$out" "/commit/$c" "enlaza al commit completo"
has "$out" "/compare/${a:0:7}...${c:0:7}" "enlaza a la comparación"
done_repo

# 2. Solo el título: sin cuerpo ni Co-Authored-By.
new_repo
a=$(commit "Base")
git commit -q --allow-empty -m "Título

Detalle largo

Co-Authored-By: Alguien <a@b.c>"
c=$(git rev-parse HEAD)
out=$(notes SHA="$c" PREV_SHA="$a")
has "$out" "- Título" "usa el título"
lacks "$out" "Co-Authored-By" "sin Co-Authored-By"
lacks "$out" "Detalle largo" "sin el cuerpo del commit"
done_repo

# 3. Sin merges.
new_repo
a=$(commit "Base")
git checkout -q -b rama; commit "En la rama" >/dev/null
git checkout -q main; commit "En main" >/dev/null
git merge -q --no-ff rama -m "Merge de la rama" >/dev/null 2>&1
c=$(git rev-parse HEAD)
out=$(notes SHA="$c" PREV_SHA="$a")
has "$out" "- En la rama" "incluye los commits de la rama"
lacks "$out" "Merge de la rama" "omite el merge"
done_repo

# 4. Escapa HTML, enlaces y menciones.
new_repo
a=$(commit "Base")
c=$(commit 'Arregla @octocat <b>x</b> [clic](http://evil.test) & más')
out=$(notes SHA="$c" PREV_SHA="$a")
has "$out" '@&#8203;octocat' "neutraliza la mención"
has "$out" '&lt;b&gt;x&lt;/b&gt;' "escapa el HTML"
has "$out" '\[clic\]' "escapa los corchetes"
lacks "$out" '[clic](' "no deja un enlace activo"
has "$out" '&amp; más' "escapa el ampersand"
done_repo

# 5. Tope de 40 con aviso.
new_repo
a=$(commit "Base")
for i in $(seq 1 45); do c=$(commit "Cambio $i"); done
out=$(notes SHA="$c" PREV_SHA="$a")
[ "$(count_bullets "$out")" = "40" ] && ok "muestra 40 cambios" || bad "muestra 40 cambios (hay $(count_bullets "$out"))"
has "$out" "y 5 cambios más" "avisa de los 5 restantes"
done_repo

# 6. Primer release (sin PREV_SHA): últimos 20 y una nota.
new_repo
for i in $(seq 1 25); do c=$(commit "Cambio $i"); done
out=$(notes SHA="$c")
[ "$(count_bullets "$out")" = "20" ] && ok "primer release: 20 cambios" || bad "primer release: 20 cambios (hay $(count_bullets "$out"))"
has "$out" "Primer release" "explica que es el primero"
lacks "$out" "Comparar con el release anterior" "sin enlace de comparación"
done_repo

# 7. Release anterior inalcanzable (historial reescrito): cae al modo primer release.
new_repo
for i in $(seq 1 3); do c=$(commit "Cambio $i"); done
out=$(notes SHA="$c" PREV_SHA="0123456789abcdef0123456789abcdef01234567")
has "$out" "Primer release" "SHA anterior inexistente: modo primer release"
has "$out" "- Cambio 3" "sigue listando commits"
done_repo

# 8. Sin cambios nuevos.
new_repo
c=$(commit "Único")
out=$(notes SHA="$c" PREV_SHA="$c")
has "$out" "Sin cambios desde el release anterior." "sin commits nuevos"
done_repo

# 9. Tabla de descargas con tamaño y hash.
new_repo
c=$(commit "Algo")
assets="$(mktemp -d)"
head -c 2097152 /dev/zero > "$assets/Simple_Player-0.1.0-dev.5-x86_64.AppImage"
head -c 1048576 /dev/zero > "$assets/Simple_Player-0.1.0-dev.5-x86_64-light.AppImage"
sum_full=$(sha256sum "$assets/Simple_Player-0.1.0-dev.5-x86_64.AppImage" | cut -d' ' -f1)
out=$(notes SHA="$c" ASSETS_DIR="$assets")
has "$out" "Simple_Player-0.1.0-dev.5-x86_64.AppImage" "lista el autocontenido"
has "$out" "Simple_Player-0.1.0-dev.5-x86_64-light.AppImage" "lista el ligero"
has "$out" "$sum_full" "incluye el SHA-256"
has "$out" "2.0 MB" "incluye el tamaño"
has "$out" "chmod +x" "explica cómo usarlo"
has "$out" "master" "menciona que master es el legado"
rm -rf "$assets"; done_repo

echo
[ "$fails" -eq 0 ] && echo "TODO BIEN" || { echo "$fails prueba(s) fallaron"; exit 1; }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `bash scripts/tests/release-notes.test.sh 2>&1 | tail -8`
Expected: fallan todas (el script no existe: `bash: .../release-notes.sh: No existe el archivo`) y termina con `prueba(s) fallaron` (RED).

- [ ] **Step 3: Write minimal implementation**

`scripts/release-notes.sh`:

```bash
#!/usr/bin/env bash
# Genera las notas de un release (Markdown) con los commits nuevos y los archivos publicados.
# Variables de entorno: REPO, SHA, TAG, VERSION (obligatorias); PREV_SHA, ASSETS_DIR, DATE_UTC,
# MAX_COMMITS (40) y FIRST_RUN_COMMITS (20) (opcionales). Se ejecuta dentro del repositorio.
set -euo pipefail

: "${REPO:?falta REPO}" "${SHA:?falta SHA}" "${TAG:?falta TAG}" "${VERSION:?falta VERSION}"
PREV_SHA="${PREV_SHA:-}"
ASSETS_DIR="${ASSETS_DIR:-}"
MAX_COMMITS="${MAX_COMMITS:-40}"
FIRST_RUN_COMMITS="${FIRST_RUN_COMMITS:-20}"
DATE_UTC="${DATE_UTC:-$(date -u '+%Y-%m-%d %H:%M UTC')}"
URL="https://github.com/${REPO}"
SHORT="${SHA:0:7}"

# Un título de commit no debe poder romper el Markdown, crear enlaces ni mencionar a nadie.
# (sed y no ${t//x/y}: en bash 5.2 un `&` sin comillas en el reemplazo significa «lo encontrado».)
escape_title() {
  printf '%s' "$1" | sed -e 's/&/\&amp;/g' -e 's/</\&lt;/g' -e 's/>/\&gt;/g' -e 's/\[/\\[/g' -e 's/\]/\\]/g' -e 's/@/@\&#8203;/g'
}

human_size() { awk -v b="$1" 'BEGIN { printf "%.1f MB", b / 1048576 }'; }

# ¿Hay un release anterior alcanzable? Si no (primer release o historial reescrito), se muestran los últimos N.
first_run=0
compare_url=""
limit_args=()
if [ -n "$PREV_SHA" ] && git cat-file -e "${PREV_SHA}^{commit}" 2>/dev/null; then
  range="${PREV_SHA}..${SHA}"
  compare_url="${URL}/compare/${PREV_SHA:0:7}...${SHORT}"
else
  range="$SHA"
  limit_args=(-n "$FIRST_RUN_COMMITS")
  first_run=1
fi

lines=()
while IFS= read -r line; do lines+=("$line"); done < <(git log --no-merges ${limit_args[@]+"${limit_args[@]}"} --format='%H%x1f%s' "$range")
total=${#lines[@]}

echo "## Compilación automática de \`nueva-version\`"
echo
echo "Commit [\`${SHORT}\`](${URL}/commit/${SHA}) · ${DATE_UTC}"
if [ -n "$compare_url" ]; then
  echo
  echo "[Comparar con el release anterior](${compare_url})"
fi
echo
echo "### Cambios"
echo
if [ "$total" -eq 0 ]; then
  echo "Sin cambios desde el release anterior."
else
  if [ "$first_run" -eq 1 ]; then
    echo "Primer release automático: se muestran los últimos ${total} commits."
    echo
  fi
  shown=0
  for line in "${lines[@]}"; do
    [ "$shown" -ge "$MAX_COMMITS" ] && break
    hash="${line%%$'\x1f'*}"
    subject="${line#*$'\x1f'}"
    echo "- $(escape_title "$subject") ([${hash:0:7}](${URL}/commit/${hash}))"
    shown=$((shown + 1))
  done
  if [ "$total" -gt "$shown" ]; then
    more=$((total - shown))
    if [ -n "$compare_url" ]; then
      echo
      echo "… y ${more} cambios más ([ver la comparación completa](${compare_url}))."
    else
      echo
      echo "… y ${more} cambios más."
    fi
  fi
fi

if [ -n "$ASSETS_DIR" ] && ls "$ASSETS_DIR"/*.AppImage >/dev/null 2>&1; then
  echo
  echo "### Descargas"
  echo
  echo "| Archivo | Para quién | Tamaño | SHA-256 |"
  echo "|---|---|---|---|"
  # Primero el autocontenido y después el ligero.
  for f in $(ls "$ASSETS_DIR"/*.AppImage | grep -v -- '-light\.AppImage$') $(ls "$ASSETS_DIR"/*.AppImage | grep -- '-light\.AppImage$' || true); do
    name="$(basename "$f")"
    sum="$(sha256sum "$f" | cut -d' ' -f1)"
    size="$(human_size "$(stat -c %s "$f")")"
    case "$name" in
      *-light.AppImage) who="Arch/CachyOS y distros con GStreamer (base, good, bad, libav), dbus, libxkbcommon y libglvnd instalados" ;;
      *) who="La mayoría de las distros; incluye GStreamer y sus bibliotecas" ;;
    esac
    echo "| \`${name}\` | ${who} | ${size} | \`${sum}\` |"
  done
fi

cat <<EOF

### Cómo usarlo

\`\`\`
chmod +x Simple_Player-*.AppImage
./Simple_Player-*.AppImage
sha256sum -c SHA256SUMS --ignore-missing
\`\`\`

Para que Plasma muestre los controles multimedia en la miniatura de la barra de tareas, el AppImage suelto debe registrarse en el menú (por ejemplo con AppImageLauncher).

---

Esta es la versión nativa (Rust + egui) de Simple Player. La rama \`master\` conserva la versión anterior como referencia.
EOF
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `bash scripts/tests/release-notes.test.sh 2>&1 | tail -15`
Expected: todas las líneas `ok` y `TODO BIEN`. Si el caso 4 falla por el escape de `[` o `@`, corregir `escape_title` (las pruebas mandan), no las pruebas.

- [ ] **Step 5: Commit**

```bash
chmod +x scripts/release-notes.sh scripts/tests/run-all.sh scripts/tests/release-notes.test.sh
git add scripts/release-notes.sh scripts/tests
git commit -m "Notas de release: changelog con los commits nuevos, escapado y tabla de descargas

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Selección de releases a podar (`scripts/ci/select-prune.sh`)

**Files:**
- Create: `scripts/ci/select-prune.sh`, `scripts/tests/select-prune.test.sh`
- Test: `scripts/tests/select-prune.test.sh`

**Interfaces:**
- Consumes: por la entrada estándar, un tag por línea, **del más nuevo al más viejo**.
- Produces: `KEEP=10 bash scripts/ci/select-prune.sh < tags` imprime, uno por línea, los tags a borrar: los de patrón `^v.+-dev\.[0-9]+$` que quedan después de los primeros `KEEP`. `KEEP` debe ser un entero ≥ 0; si no, sale con error (código 2) sin imprimir nada.

- [ ] **Step 1: Write the failing tests**

`scripts/tests/select-prune.test.sh`:

```bash
#!/usr/bin/env bash
set -uo pipefail
SCRIPT="$(cd "$(dirname "$0")/.." && pwd)/ci/select-prune.sh"
fails=0
ok()  { echo "ok   - $1"; }
bad() { echo "FAIL - $1"; fails=$((fails + 1)); }
eq()  { [ "$1" = "$2" ] && ok "$3" || bad "$3 (esperado «$2», obtenido «$1»)"; }

tags() { printf '%s\n' "$@"; }

# 1. Conserva los KEEP más nuevos y borra el resto.
out=$(tags v0.1.0-dev.5 v0.1.0-dev.4 v0.1.0-dev.3 v0.1.0-dev.2 v0.1.0-dev.1 | KEEP=3 bash "$SCRIPT" | tr '\n' ' ')
eq "$out" "v0.1.0-dev.2 v0.1.0-dev.1 " "borra los más viejos"

# 2. Nunca toca tags que no son dev (p. ej. el release de Tauri).
out=$(tags v0.1.0-dev.3 simpleplayer v0.1.0 v0.1.0-dev.2 v0.1.0-dev.1 | KEEP=1 bash "$SCRIPT" | tr '\n' ' ')
eq "$out" "v0.1.0-dev.2 v0.1.0-dev.1 " "ignora simpleplayer y v0.1.0"

# 3. Si hay menos que KEEP no borra nada.
out=$(tags v0.1.0-dev.2 v0.1.0-dev.1 | KEEP=10 bash "$SCRIPT")
eq "$out" "" "no borra si no se supera el límite"

# 4. KEEP=0 borra todos los dev.
out=$(tags v0.1.0-dev.2 simpleplayer v0.1.0-dev.1 | KEEP=0 bash "$SCRIPT" | tr '\n' ' ')
eq "$out" "v0.1.0-dev.2 v0.1.0-dev.1 " "KEEP=0 borra todos los dev"

# 5. KEEP inválido: error y nada impreso.
out=$(tags v0.1.0-dev.2 | KEEP='1; rm -rf /' bash "$SCRIPT" 2>/dev/null); code=$?
eq "$code" "2" "KEEP no numérico sale con 2"
eq "$out" "" "KEEP no numérico no imprime nada"
out=$(tags v0.1.0-dev.2 | KEEP='' bash "$SCRIPT" 2>/dev/null); code=$?
eq "$code" "2" "KEEP vacío sale con 2"

# 6. Tags de otras versiones con el mismo patrón también cuentan como dev.
out=$(tags v0.2.0-dev.3 v0.1.0-dev.9 v0.1.0-dev.8 | KEEP=2 bash "$SCRIPT" | tr '\n' ' ')
eq "$out" "v0.1.0-dev.8 " "el patrón no depende de la versión"

echo
[ "$fails" -eq 0 ] && echo "TODO BIEN" || { echo "$fails prueba(s) fallaron"; exit 1; }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `bash scripts/tests/select-prune.test.sh 2>&1 | tail -6`
Expected: fallan (el script no existe) (RED).

- [ ] **Step 3: Write minimal implementation**

`scripts/ci/select-prune.sh`:

```bash
#!/usr/bin/env bash
# Entrada: tags de releases, uno por línea, del más nuevo al más viejo.
# Salida: los tags de releases «dev» que sobran (después de los primeros KEEP).
# Solo se consideran tags con la forma vX.Y.Z-dev.N; cualquier otro nunca se imprime.
set -euo pipefail

KEEP="${KEEP-}"
if ! [[ "$KEEP" =~ ^[0-9]+$ ]]; then
  echo "KEEP debe ser un número entero (recibido: «${KEEP}»)" >&2
  exit 2
fi

{ grep -E '^v.+-dev\.[0-9]+$' || true; } | awk -v keep="$KEEP" 'NR > keep'
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `bash scripts/tests/select-prune.test.sh 2>&1 | tail -12`
Expected: todo `ok` y `TODO BIEN`.

- [ ] **Step 5: Commit**

```bash
chmod +x scripts/ci/select-prune.sh scripts/tests/select-prune.test.sh
git add scripts/ci scripts/tests/select-prune.test.sh
git commit -m "CI: selección segura de los releases dev que se podan

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 4: Herramientas fijas y empaquetado ligero (`scripts/build-appimage.sh light`)

**Files:**
- Create: `scripts/appimage-tools.lock`, `scripts/tests/build-appimage.test.sh`
- Modify: `scripts/build-appimage.sh` (reescritura), `.gitignore`
- Test: `scripts/tests/build-appimage.test.sh`

**Interfaces:**
- Consumes: `assets/icons/icon.png`, `assets/simple-player.desktop`.
- Produces: `scripts/build-appimage.sh [full|light] [--binary RUTA]`. Variables: `APP_VERSION` (por defecto la de `Cargo.toml`), `TOOLS_DIR` (`.cache/appimage-tools`), `DIST_DIR` (`dist`), `LOCK_FILE` (`scripts/appimage-tools.lock`). Salida: `$DIST_DIR/Simple_Player-${APP_VERSION}-x86_64[-light].AppImage`. Un hash que no coincide aborta con «no coincide» y código ≠ 0 antes de ejecutar nada.

- [ ] **Step 1: Write the failing tests**

`scripts/appimage-tools.lock` (valores reales verificados):

```bash
# Herramientas de empaquetado con versión y hash fijos (se verifican antes de ejecutarlas).
# Para actualizar una: cambiar la URL, descargar el archivo y poner el nuevo `sha256sum`; un commit explícito.
LINUXDEPLOY_NAME=linuxdeploy-x86_64.AppImage
LINUXDEPLOY_URL=https://github.com/linuxdeploy/linuxdeploy/releases/download/1-alpha-20251107-1/linuxdeploy-x86_64.AppImage
LINUXDEPLOY_SHA256=c20cd71e3a4e3b80c3483cef793cda3f4e990aca14014d23c544ca3ce1270b4d

GSTREAMER_PLUGIN_NAME=linuxdeploy-plugin-gstreamer.sh
GSTREAMER_PLUGIN_URL=https://raw.githubusercontent.com/linuxdeploy/linuxdeploy-plugin-gstreamer/2a2e67491c32995a3f279ad0ecbe77abd512b42a/linuxdeploy-plugin-gstreamer.sh
GSTREAMER_PLUGIN_SHA256=c107b49d84edbffc6ab226ed1007e0626a4f7aa2c3a36b7782bef62351d49e94

APPIMAGE_PLUGIN_NAME=linuxdeploy-plugin-appimage-x86_64.AppImage
APPIMAGE_PLUGIN_URL=https://github.com/linuxdeploy/linuxdeploy-plugin-appimage/releases/download/1-alpha-20250213-1/linuxdeploy-plugin-appimage-x86_64.AppImage
APPIMAGE_PLUGIN_SHA256=992d502a248e14ab185448ddf6f6e7d25558cb84d4623c354c3af350c25fccb3

APPIMAGETOOL_NAME=appimagetool-x86_64.AppImage
APPIMAGETOOL_URL=https://github.com/AppImage/appimagetool/releases/download/1.9.1/appimagetool-x86_64.AppImage
APPIMAGETOOL_SHA256=ed4ce84f0d9caff66f50bcca6ff6f35aae54ce8135408b3fa33abfc3cb384eb0
```

`scripts/tests/build-appimage.test.sh`:

```bash
#!/usr/bin/env bash
# Pruebas de la verificación de herramientas de scripts/build-appimage.sh (sin red: URLs file://).
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
fails=0
ok()  { echo "ok   - $1"; }
bad() { echo "FAIL - $1"; fails=$((fails + 1)); }

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
printf '#!/bin/sh\necho herramienta falsa\n' > "$TMP/falsa"
good="$(sha256sum "$TMP/falsa" | cut -d' ' -f1)"

# Un .lock con todas las herramientas apuntando al mismo archivo local.
make_lock() { # hash
  : > "$TMP/lock"
  for p in LINUXDEPLOY GSTREAMER_PLUGIN APPIMAGE_PLUGIN APPIMAGETOOL; do
    printf '%s_NAME=%s.bin\n%s_URL=file://%s\n%s_SHA256=%s\n' "$p" "$p" "$p" "$TMP/falsa" "$p" "$1" >> "$TMP/lock"
  done
}

# 1. Hash incorrecto: aborta y lo dice.
make_lock "0000000000000000000000000000000000000000000000000000000000000000"
out=$(cd "$ROOT" && LOCK_FILE="$TMP/lock" TOOLS_DIR="$TMP/t1" DIST_DIR="$TMP/d1" FETCH_ONLY=1 bash scripts/build-appimage.sh light 2>&1); code=$?
[ "$code" -ne 0 ] && ok "hash incorrecto: sale con error" || bad "hash incorrecto: sale con error"
grep -q "no coincide" <<<"$out" && ok "hash incorrecto: mensaje claro" || bad "hash incorrecto: mensaje claro"
[ ! -e "$TMP/t1/APPIMAGETOOL.bin" ] && ok "hash incorrecto: no deja la herramienta" || bad "hash incorrecto: no deja la herramienta"

# 2. Hash correcto: descarga y deja ejecutable.
make_lock "$good"
out=$(cd "$ROOT" && LOCK_FILE="$TMP/lock" TOOLS_DIR="$TMP/t2" DIST_DIR="$TMP/d2" FETCH_ONLY=1 bash scripts/build-appimage.sh light 2>&1); code=$?
[ "$code" -eq 0 ] && ok "hash correcto: termina bien" || bad "hash correcto: termina bien ($out)"
[ -x "$TMP/t2/APPIMAGETOOL.bin" ] && ok "hash correcto: herramienta ejecutable" || bad "hash correcto: herramienta ejecutable"

# 3. Una variante desconocida se rechaza.
out=$(cd "$ROOT" && bash scripts/build-appimage.sh enorme 2>&1); code=$?
[ "$code" -eq 2 ] && ok "variante desconocida: código 2" || bad "variante desconocida: código 2 ($code)"

echo
[ "$fails" -eq 0 ] && echo "TODO BIEN" || { echo "$fails prueba(s) fallaron"; exit 1; }
```

Nota: `FETCH_ONLY=1` es una variable de prueba del script (solo descarga y verifica las herramientas y termina).

- [ ] **Step 2: Run tests to verify they fail**

Run: `bash scripts/tests/build-appimage.test.sh 2>&1 | tail -8`
Expected: fallan (el script actual no conoce `light`, `LOCK_FILE` ni `FETCH_ONLY`) (RED).

- [ ] **Step 3: Write minimal implementation**

Reescribir `scripts/build-appimage.sh` (esta tarea implementa `light`; `full` se añade en la Tarea 5 — hasta entonces `full` termina con «todavía no implementado» y código 3):

```bash
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

case "$VARIANT" in
  light)
    ln -sf usr/bin/simple-player "$APPDIR/AppRun"
    "$TOOLS_DIR/$APPIMAGETOOL_NAME" --no-appstream "$APPDIR" "$OUTPUT"
    ;;
  full)
    echo "La variante full todavía no está implementada" >&2
    exit 3
    ;;
esac

echo "LISTO: $OUTPUT"
```

Añadir a `.gitignore`, en la sección de empaquetado: `/dist/` y `/.cache/`.

- [ ] **Step 4: Run tests and build a real `light`**

Run: `bash scripts/tests/build-appimage.test.sh 2>&1 | tail -12`
Expected: todo `ok` y `TODO BIEN`.

Luego la prueba real con el binario ya compilado (descarga las herramientas verificadas):

```bash
cargo build --release --locked 2>&1 | tail -1
APP_VERSION=0.1.0-local scripts/build-appimage.sh light --binary target/release/simple-player
ls -la dist/
APPIMAGE_EXTRACT_AND_RUN=1 dist/Simple_Player-0.1.0-local-x86_64-light.AppImage --appimage-extract >/dev/null 2>&1; ls squashfs-root/usr/bin squashfs-root/*.desktop; rm -rf squashfs-root
```
Expected: `dist/Simple_Player-0.1.0-local-x86_64-light.AppImage` existe (~10–15 MB); el contenido extraído tiene `usr/bin/simple-player-bin`, `usr/bin/simple-player` y `simple-player.desktop`.

- [ ] **Step 5: Commit**

```bash
chmod +x scripts/build-appimage.sh scripts/tests/build-appimage.test.sh
git add scripts/build-appimage.sh scripts/appimage-tools.lock scripts/tests/build-appimage.test.sh .gitignore
git commit -m "AppImage: herramientas con hash fijo y variante ligera con --binary

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 5: Empaquetado autocontenido (`scripts/build-appimage.sh full`)

**Files:**
- Modify: `scripts/build-appimage.sh`

**Interfaces:**
- Consumes: las herramientas de la Tarea 4; `patchelf` y `file` instalados; los plugins de GStreamer del sistema.
- Produces: la variante `full`: AppImage con GStreamer y bibliotecas dentro, `AppRun` propio que carga `apprun-hooks/*.sh`, y verificación `ldd` que falla el build si queda alguna biblioteca sin resolver (salvo las del sistema anfitrión, lista `HOST_LIBS_RE`).

Esta variante no puede probarse al completo sin ejecutarla de verdad: se prueba aquí en esta máquina (Arch/CachyOS) para validar la mecánica, y su portabilidad se valida en CI (Tarea 8). Las dos verificaciones (lista de plugins requeridos y `ldd`) son el control automático.

- [ ] **Step 1: Implementar la variante `full`**

Sustituir el caso `full)` del `case "$VARIANT"` y añadir las funciones antes de él:

```bash
# Plugins de GStreamer que se incluyen (lista curada: copiar todos arrastra decenas de dependencias
# gráficas que no hacen falta para reproducir audio). Los REQUIRED hacen fallar el build si faltan.
GST_REQUIRED="coreelements typefindfunctions playback audioconvert audioresample volume spectrum app"
GST_OPTIONAL="audiorate audioparsers id3demux apetag flac wavparse ogg vorbis opus isomp4 matroska mpg123 mpegaudioparse libav faad autodetect pulseaudio alsa pipewire audiofx level equalizer"
# Bibliotecas que el sistema anfitrión aporta (controladores de gráficos y de pantalla): no se exigen dentro del AppImage.
HOST_LIBS_RE='libGL|libEGL|libGLX|libGLESv2|libOpenGL|libX11|libXext|libXv|libxcb|libwayland|libvulkan|libdrm|libasound'

# Primer directorio que exista de una lista (Debian/Ubuntu y Arch tienen rutas distintas).
first_dir() { for d in "$@"; do [ -d "$d" ] && { echo "$d"; return; }; done; return 1; }

curated_plugins_dir() { # directorio_origen -> directorio con solo los plugins elegidos
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
  command -v patchelf >/dev/null || { echo "Falta patchelf" >&2; exit 1; }
  command -v file >/dev/null || { echo "Falta el comando file" >&2; exit 1; }
  local plugins_src helpers_dir
  plugins_src="$(first_dir /usr/lib/x86_64-linux-gnu/gstreamer-1.0 /usr/lib/gstreamer-1.0 /usr/lib64/gstreamer-1.0)" \
    || { echo "No encuentro los plugins de GStreamer del sistema" >&2; exit 1; }
  helpers_dir="$(first_dir /usr/lib/x86_64-linux-gnu/gstreamer1.0/gstreamer-1.0 /usr/lib/gstreamer-1.0 /usr/libexec/gstreamer-1.0)" \
    || { echo "No encuentro los ayudantes de GStreamer (gst-plugin-scanner)" >&2; exit 1; }

  export PATH="$PWD/$TOOLS_DIR:$PATH"
  export LINUXDEPLOY="$PWD/$TOOLS_DIR/$LINUXDEPLOY_NAME"
  export GSTREAMER_PLUGINS_DIR; GSTREAMER_PLUGINS_DIR="$(curated_plugins_dir "$plugins_src")"
  export GSTREAMER_HELPERS_DIR="$helpers_dir"

  # AppRun propio: define las rutas de GStreamer (hooks del plugin) y arranca el binario.
  cat > "$WORK/AppRun" <<'APPRUN'
#!/bin/sh
HERE="$(dirname "$(readlink -f "$0")")"
export APPDIR="${APPDIR:-$HERE}"
export LD_LIBRARY_PATH="$HERE/usr/lib:${LD_LIBRARY_PATH:-}"
for hook in "$HERE"/apprun-hooks/*.sh; do
  [ -f "$hook" ] && . "$hook"
done
exec "$HERE/usr/bin/simple-player-bin" "$@"
APPRUN
  chmod +x "$WORK/AppRun"

  (cd "$WORK" && "$LINUXDEPLOY" --appdir "$APPDIR" \
    --executable "$APPDIR/usr/bin/simple-player-bin" \
    --desktop-file "$APPDIR/simple-player.desktop" \
    --icon-file "$ROOT_DIR/assets/icons/icon.png" \
    --custom-apprun "$WORK/AppRun" \
    --plugin gstreamer)
  verify_bundle
  (cd "$WORK" && "$LINUXDEPLOY" --appdir "$APPDIR" --output appimage)
  local made; made="$(ls "$WORK"/*.AppImage 2>/dev/null | head -n1)"
  [ -n "$made" ] || { echo "linuxdeploy no generó ningún AppImage" >&2; exit 1; }
  mv "$made" "$OUTPUT"
}
```

y en el `case`:

```bash
  full)
    build_full
    ;;
```


- [ ] **Step 2: Probar en esta máquina**

```bash
command -v patchelf file || echo "faltan patchelf/file: instalar con pacman -S patchelf file (o saltar al Step 4)"
APP_VERSION=0.1.0-local scripts/build-appimage.sh full --binary target/release/simple-player 2>&1 | tail -25
ls -la dist/
```
Expected: se genera `dist/Simple_Player-0.1.0-local-x86_64.AppImage` (decenas de MB). Si falla, el mensaje dice por qué (plugin requerido ausente, biblioteca sin resolver, etc.); ajustar `GST_REQUIRED`/`GST_OPTIONAL`/`HOST_LIBS_RE` y repetir. Si faltan `patchelf`/`file` y el usuario no quiere instalarlos, este paso se valida directamente en CI (Tarea 8) y se anota en el ledger.

- [ ] **Step 3: Comprobar el contenido del AppImage generado**

```bash
cd /tmp && rm -rf sp_check && mkdir sp_check && cd sp_check
APPIMAGE_EXTRACT_AND_RUN=1 "$OLDPWD/dist/Simple_Player-0.1.0-local-x86_64.AppImage" --appimage-extract >/dev/null 2>&1
ls squashfs-root/usr/lib | head -5; ls squashfs-root/usr/lib/gstreamer-1.0 | head -8; ls squashfs-root/apprun-hooks
cd - >/dev/null; rm -rf /tmp/sp_check
```
Expected: hay `libgst*` y `gstreamer-1.0/libgstplayback.so`, `apprun-hooks/linuxdeploy-plugin-gstreamer.sh`.

- [ ] **Step 4: Commit**

```bash
git add scripts/build-appimage.sh
git commit -m "AppImage autocontenido: GStreamer curado, AppRun propio y verificación de bibliotecas

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 6: El workflow `release.yml` y su prueba de política

**Files:**
- Create: `.github/workflows/release.yml`, `scripts/ci/install-deps.sh`, `scripts/tests/check-workflow.py`
- Test: `scripts/tests/check-workflow.py`

**Interfaces:**
- Consumes: `scripts/build-appimage.sh`, `scripts/release-notes.sh`, `scripts/ci/select-prune.sh`.
- Produces: el workflow con los jobs `build` (salidas `app_version` y `tag`), `package` (matriz `variant: [full, light]`) y `publish`; `scripts/ci/install-deps.sh build|package` instala los paquetes de `apt`.

- [ ] **Step 1: Write the failing policy test**

`scripts/tests/check-workflow.py`:

```python
#!/usr/bin/env python3
"""Prueba de política de .github/workflows/release.yml (sin red): permisos, disparadores,
acciones permitidas y el orden de publicación."""
import re
import sys
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[2]
PATH = ROOT / ".github/workflows/release.yml"
ALLOWED_ACTIONS = {
    "actions/checkout@v4",
    "actions/cache@v4",
    "actions/upload-artifact@v4",
    "actions/download-artifact@v4",
}
failures = []


def check(cond, msg):
    print(("ok   - " if cond else "FAIL - ") + msg)
    if not cond:
        failures.append(msg)


wf = yaml.safe_load(PATH.read_text())
on = wf.get("on", wf.get(True))  # PyYAML interpreta `on:` como True
jobs = wf["jobs"]

# Disparadores
check(on["push"]["branches"] == ["nueva-version"], "solo se dispara con push a nueva-version")
ignored = on["push"].get("paths-ignore", [])
check("docs/**" in ignored and "**/*.md" in ignored, "los pushes solo de documentos se ignoran")
check("workflow_dispatch" in on, "se puede lanzar a mano")
check("pull_request" not in on and "pull_request_target" not in on, "no se dispara con pull requests")
check(
    wf["concurrency"]["group"] == "release-nueva-version" and wf["concurrency"]["cancel-in-progress"] is False,
    "los pushes se encolan (no se cancelan)",
)

# Permisos
check(wf["permissions"] == {"contents": "read"}, "permisos por defecto: solo lectura")
writers = [n for n, j in jobs.items() if (j.get("permissions") or {}).get("contents") == "write"]
check(writers == ["publish"], f"solo publish puede escribir (escriben: {writers})")

# Runner y acciones
check(all(j["runs-on"] == "ubuntu-22.04" for j in jobs.values()), "todos los jobs usan ubuntu-22.04")
uses = [s["uses"] for j in jobs.values() for s in j["steps"] if "uses" in s]
check(all(u in ALLOWED_ACTIONS for u in uses), f"solo acciones permitidas (usadas: {sorted(set(uses))})")
runs = [s["run"] for j in jobs.values() for s in j["steps"] if "run" in s]
check(not any("${{" in r for r in runs), "ningún bloque run interpola ${{ }} (todo va por env)")

# Etapas
check(set(jobs) == {"build", "package", "publish"}, "jobs: build, package, publish")
check(jobs["package"]["needs"] == "build" and set(jobs["publish"]["needs"]) == {"build", "package"}, "build -> package -> publish")
check(jobs["package"]["strategy"]["matrix"]["variant"] == ["full", "light"], "se empaquetan full y light")

build = "\n".join(s.get("run", "") for s in jobs["build"]["steps"])
check("cargo test" in build and build.index("cargo test") < build.index("cargo build"), "las pruebas corren antes de compilar")

# Orden de publicación
steps = jobs["publish"]["steps"]
text = [(s.get("name", ""), s.get("run", "")) for s in steps]
def idx(pred):
    return next((i for i, (_, r) in enumerate(text) if pred(r)), -1)
i_create = idx(lambda r: "gh release create" in r and "--draft" in r)
i_publish = idx(lambda r: "--draft=false" in r)
i_prune = idx(lambda r: "select-prune.sh" in r)
check(i_create >= 0, "el release se crea en borrador")
check(i_create < i_publish < i_prune, "orden: crear borrador -> publicar -> podar")
create_run = text[i_create][1]
check("--prerelease" in create_run and "--latest=false" in create_run, "es pre-release y no es 'Latest'")
check("--target" in create_run, "apunta al commit del push")
cleanup = [s for s in steps if s.get("if") == "failure()" and "gh release delete" in s.get("run", "")]
check(len(cleanup) == 1, "si falla, se borra el borrador")
prune_step = steps[i_prune]
check(
    "KEEP" in prune_step.get("env", {}) and "cleanup-tag" in prune_step["run"],
    "la poda valida KEEP por entorno y borra también el tag",
)

print()
if failures:
    print(f"{len(failures)} comprobación(es) fallaron")
    sys.exit(1)
print("TODO BIEN")
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `python3 scripts/tests/check-workflow.py 2>&1 | tail -5`
Expected: falla con `FileNotFoundError: .github/workflows/release.yml` (RED).

- [ ] **Step 3: Write the implementation**

`scripts/ci/install-deps.sh`:

```bash
#!/usr/bin/env bash
# Instala los paquetes de apt que necesita cada job (ubuntu-22.04).
#   build    compilar y probar
#   package  empaquetar el AppImage autocontenido (plugins de GStreamer y herramientas)
set -euo pipefail

BUILD_PKGS="build-essential pkg-config libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev libgstreamer-plugins-bad1.0-dev libdbus-1-dev libglib2.0-dev libxkbcommon-dev libx11-dev"
PACKAGE_PKGS="$BUILD_PKGS patchelf file desktop-file-utils gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-plugins-ugly gstreamer1.0-libav gstreamer1.0-pulseaudio gstreamer1.0-alsa gstreamer1.0-pipewire"

case "${1:-}" in
  build) PKGS="$BUILD_PKGS" ;;
  package) PKGS="$PACKAGE_PKGS" ;;
  *) echo "Uso: $0 build|package" >&2; exit 2 ;;
esac

sudo apt-get update
# shellcheck disable=SC2086
sudo apt-get install -y --no-install-recommends $PKGS
```

`.github/workflows/release.yml`:

```yaml
name: Release

on:
  push:
    branches: [nueva-version]
    paths-ignore: ['docs/**', '**/*.md', '.claude/**']
  workflow_dispatch:
    inputs:
      keep_releases:
        description: "Cuántos releases dev conservar (los más nuevos)"
        required: false
        default: "10"

concurrency:
  group: release-nueva-version
  cancel-in-progress: false

permissions:
  contents: read

env:
  CARGO_TERM_COLOR: always

jobs:
  build:
    runs-on: ubuntu-22.04
    outputs:
      app_version: ${{ steps.meta.outputs.app_version }}
      tag: ${{ steps.meta.outputs.tag }}
    steps:
      - uses: actions/checkout@v4

      - name: Versión y tag
        id: meta
        run: |
          set -euo pipefail
          base="$(sed -n 's/^version *= *"\(.*\)"/\1/p' Cargo.toml | head -1)"
          echo "app_version=${base}-dev.${GITHUB_RUN_NUMBER}" >> "$GITHUB_OUTPUT"
          echo "tag=v${base}-dev.${GITHUB_RUN_NUMBER}" >> "$GITHUB_OUTPUT"

      - name: Dependencias del sistema
        run: bash scripts/ci/install-deps.sh build

      - name: Rust estable
        run: rustup toolchain install stable --profile minimal && rustup default stable

      - uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: cargo-${{ runner.os }}-${{ hashFiles('Cargo.lock') }}
          restore-keys: cargo-${{ runner.os }}-

      - name: Pruebas
        run: cargo test --locked

      - name: Compilar
        run: cargo build --release --locked

      - uses: actions/upload-artifact@v4
        with:
          name: simple-player-bin
          path: target/release/simple-player
          if-no-files-found: error

  package:
    needs: build
    runs-on: ubuntu-22.04
    strategy:
      fail-fast: true
      matrix:
        variant: [full, light]
    steps:
      - uses: actions/checkout@v4

      - name: Dependencias del sistema
        if: matrix.variant == 'full'
        run: bash scripts/ci/install-deps.sh package

      - uses: actions/download-artifact@v4
        with:
          name: simple-player-bin
          path: bin

      - name: Empaquetar
        env:
          VARIANT: ${{ matrix.variant }}
          APP_VERSION: ${{ needs.build.outputs.app_version }}
        run: |
          set -euo pipefail
          chmod +x bin/simple-player
          bash scripts/build-appimage.sh "$VARIANT" --binary bin/simple-player

      - uses: actions/upload-artifact@v4
        with:
          name: appimage-${{ matrix.variant }}
          path: dist/*.AppImage
          if-no-files-found: error

  publish:
    needs: [build, package]
    runs-on: ubuntu-22.04
    permissions:
      contents: write
    env:
      GH_TOKEN: ${{ github.token }}
      TAG: ${{ needs.build.outputs.tag }}
      APP_VERSION: ${{ needs.build.outputs.app_version }}
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - uses: actions/download-artifact@v4
        with:
          pattern: appimage-*
          path: artifacts
          merge-multiple: true

      - name: Archivos del release
        run: |
          set -euo pipefail
          mkdir release
          mv artifacts/*.AppImage release/
          (cd release && sha256sum ./*.AppImage | sed 's|\./||' > SHA256SUMS)
          ls -la release

      - name: Release anterior
        id: prev
        run: |
          set -euo pipefail
          prev_tag="$(gh release list --limit 200 --json tagName,createdAt --jq 'sort_by(.createdAt) | reverse | .[].tagName' | grep -E '^v.+-dev\.[0-9]+$' | head -n1 || true)"
          prev_sha=""
          if [ -n "$prev_tag" ] && git rev-parse -q --verify "refs/tags/${prev_tag}^{commit}" >/dev/null; then
            prev_sha="$(git rev-list -n1 "$prev_tag")"
          fi
          echo "sha=${prev_sha}" >> "$GITHUB_OUTPUT"

      - name: Notas del release
        env:
          REPO: ${{ github.repository }}
          SHA: ${{ github.sha }}
          PREV_SHA: ${{ steps.prev.outputs.sha }}
          VERSION: ${{ needs.build.outputs.app_version }}
          ASSETS_DIR: release
        run: bash scripts/release-notes.sh > notes.md

      - name: Crear el release en borrador y subir los archivos
        run: |
          set -euo pipefail
          gh release create "$TAG" --draft --prerelease --latest=false --target "$GITHUB_SHA" \
            --title "Simple Player ${TAG}" --notes-file notes.md release/*

      - name: Publicar
        run: gh release edit "$TAG" --draft=false

      - name: Borrar el borrador si algo falló
        if: failure()
        run: gh release delete "$TAG" --yes --cleanup-tag || true

      - name: Podar los releases dev antiguos
        env:
          KEEP: ${{ inputs.keep_releases || '10' }}
        run: |
          set -euo pipefail
          gh release list --limit 200 --json tagName,createdAt --jq 'sort_by(.createdAt) | reverse | .[].tagName' \
            | bash scripts/ci/select-prune.sh \
            | while read -r old; do
                echo "Borrando $old"
                gh release delete "$old" --yes --cleanup-tag
              done
```

- [ ] **Step 4: Run all tests**

Run: `bash scripts/tests/run-all.sh 2>&1 | tail -30`
Expected: las pruebas de shell y `check-workflow.py` terminan con `TODO BIEN`. Si la prueba de política marca algo, corregir el workflow (no la prueba), salvo que el criterio mismo esté mal justificado.

Run: `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml')); print('yaml ok')"`
Expected: `yaml ok`

- [ ] **Step 5: Commit**

```bash
chmod +x scripts/ci/install-deps.sh scripts/tests/check-workflow.py
git add .github/workflows/release.yml scripts/ci/install-deps.sh scripts/tests/check-workflow.py
git commit -m "Workflow de releases: pruebas, dos AppImage y publicación con changelog

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 7: README (sección Descargas)

**Files:**
- Modify: `README.md`

**Interfaces:**
- Consumes: el formato de los nombres de archivo y la política de releases (Global Constraints).
- Produces: la sección «Descargas» y la nota sobre `master` como legado.

- [ ] **Step 1: Editar el README**

Sustituir la subsección «Opción 1: AppImage» de «Instalación (Linux)» por:

```markdown
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
```

Añadir en «Desarrollo» una línea: `` `bash scripts/tests/run-all.sh` — pruebas de los scripts de release y del workflow ``.

- [ ] **Step 2: Commit**

```bash
git add README.md
git commit -m "README: descargas automáticas de los AppImage

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 8: Primer push real e iteración en GitHub

**Files:**
- Modify (según lo que muestren los logs): `scripts/build-appimage.sh`, `scripts/ci/install-deps.sh`, `.github/workflows/release.yml`

**Interfaces:**
- Consumes: todo lo anterior y los resultados de la Tarea 1.
- Produces: un release real `v0.1.0-dev.N` completo (los dos AppImage, `SHA256SUMS` y notas).

- [ ] **Step 1: PUSH (pedir permiso al usuario)**

Con todo commiteado y `bash scripts/tests/run-all.sh` en verde, pedir permiso y empujar a `nueva-version` (fast-forward, helper de `gh`). El push contiene código, así que dispara el workflow.

- [ ] **Step 2: Seguir la ejecución**

```bash
sleep 20
RUN=$(gh run list -R akilex02/simple-player --workflow release.yml --limit 1 --json databaseId --jq '.[0].databaseId')
gh run watch -R akilex02/simple-player "$RUN" --exit-status || true
gh run view -R akilex02/simple-player "$RUN" --json jobs --jq '.jobs[] | {name, conclusion}'
```

- [ ] **Step 3: Si un job falla, leer el log y corregir**

```bash
gh run view -R akilex02/simple-player "$RUN" --log-failed | tail -80
```

Tabla de diagnóstico habitual (corregir el script o el workflow, repetir Step 1–2 con un push nuevo por cada corrección):

| Síntoma en el log | Causa probable | Corrección |
|---|---|---|
| `build`: `Package gstreamer-player-1.0 was not found` | falta `-dev` de GStreamer bad | añadir el paquete a `BUILD_PKGS` en `install-deps.sh`; si 22.04 no lo trae, cambiar los jobs a `ubuntu-24.04` (y actualizar la prueba de política) |
| `build`: error de `rustc` por versión | el Rust del runner es viejo | confirmar que `rustup toolchain install stable` corrió antes de compilar |
| `package full`: `Falta el plugin de GStreamer requerido` | el paquete no está instalado o cambia el nombre del `.so` | instalar el paquete en `PACKAGE_PKGS`; ajustar `GST_REQUIRED` |
| `package full`: `Faltan bibliotecas para …` | una dependencia de un plugin | quitar ese plugin de `GST_OPTIONAL` o añadir la biblioteca anfitriona a `HOST_LIBS_RE` si es del sistema |
| `package full`: `linuxdeploy` no encuentra el plugin | el nombre del script no coincide con `linuxdeploy-plugin-gstreamer.sh` en `PATH` | revisar `GSTREAMER_PLUGIN_NAME` y que `TOOLS_DIR` esté en `PATH` |
| `publish`: `Resource not accessible by integration` | política de permisos del repo | pedir el ajuste al propietario (mensaje de la Tarea 1) |
| `publish`: `release.yml` falla en `Notas` | `PREV_SHA` o `git log` | reproducir en local con `ASSETS_DIR` y `PREV_SHA` reales |

- [ ] **Step 4: Verificar el release resultante**

```bash
TAG=$(gh release list -R akilex02/simple-player --limit 1 --json tagName --jq '.[0].tagName'); echo "$TAG"
gh release view "$TAG" -R akilex02/simple-player --json isPrerelease,isDraft,assets,body --jq '{prerelease: .isPrerelease, draft: .isDraft, assets: [.assets[].name], body: .body[0:600]}'
gh release list -R akilex02/simple-player --limit 5
```
Expected: `prerelease: true`, `draft: false`, tres archivos (`…-x86_64.AppImage`, `…-x86_64-light.AppImage`, `SHA256SUMS`), notas con «Primer release automático» y los commits; el release `SimplePlayer v0.1.0` sigue marcado `Latest`.

- [ ] **Step 5: Probar los AppImage descargados**

```bash
mkdir -p /tmp/sp_release && cd /tmp/sp_release
gh release download "$TAG" -R akilex02/simple-player
sha256sum -c SHA256SUMS
chmod +x ./*.AppImage
APPIMAGE_EXTRACT_AND_RUN=1 ./Simple_Player-*-x86_64-light.AppImage --appimage-extract >/dev/null 2>&1 && echo "ligero: extrae bien"
rm -rf squashfs-root
APPIMAGE_EXTRACT_AND_RUN=1 ./Simple_Player-*-x86_64.AppImage --appimage-extract >/dev/null 2>&1 && ls squashfs-root/usr/lib | head -3
cd - >/dev/null
```
Expected: `SHA256SUMS: correcto` en ambos; ambos extraen. Pedir al usuario que abra el ligero y el autocontenido (la prueba real de reproducción en una máquina distinta la hace él).

- [ ] **Step 6: Commit de las correcciones (si hubo)**

Cada corrección se commitea con un mensaje que diga qué falló y por qué (`git commit -m "AppImage full: …"`). No hay commit si la primera ejecución salió bien.

---

### Task 9: Disparo manual, poda y limpieza

**Files:**
- Delete: `.github/workflows/ci-check.yml`

**Interfaces:**
- Consumes: el release de la Tarea 8.
- Produces: la verificación del botón/`gh workflow run`, de las notas incrementales y de la poda; el workflow temporal borrado.

- [ ] **Step 1: Lanzar a mano (PUSH no necesario; pedir permiso para ejecutar el workflow)**

```bash
gh workflow run release.yml -R akilex02/simple-player --ref nueva-version
sleep 15
RUN=$(gh run list -R akilex02/simple-player --workflow release.yml --limit 1 --json databaseId --jq '.[0].databaseId')
gh run watch -R akilex02/simple-player "$RUN" --exit-status
```
Expected: el workflow arranca y termina bien. Comprobar si la interfaz muestra el botón «Run workflow» (pedirle al usuario que mire en la pestaña Actions); si no, queda documentado que se usa `gh workflow run`.

- [ ] **Step 2: Verificar las notas incrementales**

```bash
gh release view "$(gh release list -R akilex02/simple-player --limit 1 --json tagName --jq '.[0].tagName')" -R akilex02/simple-player --json body --jq .body | sed -n 1,20p
```
Expected: «Sin cambios desde el release anterior.» (misma commit) y el enlace «Comparar con el release anterior».

- [ ] **Step 3: Verificar la poda con un límite bajo**

```bash
gh workflow run release.yml -R akilex02/simple-player --ref nueva-version -f keep_releases=2
sleep 15; RUN=$(gh run list -R akilex02/simple-player --workflow release.yml --limit 1 --json databaseId --jq '.[0].databaseId')
gh run watch -R akilex02/simple-player "$RUN" --exit-status
gh release list -R akilex02/simple-player --limit 10
```
Expected: quedan exactamente 2 releases `…-dev.N` (los más nuevos) y `SimplePlayer v0.1.0` (tag `simpleplayer`) intacto y como `Latest`. Después volver al valor normal con una ejecución manual por defecto (opcional) o dejar que los siguientes pushes lo normalicen.

- [ ] **Step 4: Borrar el workflow temporal y commitear**

```bash
git rm .github/workflows/ci-check.yml
git commit -m "CI: quita la comprobación temporal de Actions

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```
(PUSH con permiso del usuario; este push solo toca `.github/workflows/ci-check.yml`, que no está en `paths-ignore` del workflow de release, así que dispara un release: avisar al usuario o incluir el cambio junto con el próximo cambio de código.)

- [ ] **Step 5: Notas finales para el usuario**

Dejar escrito en el mensaje final: (a) el `PKGBUILD` sigue apuntando al release antiguo (tag `simpleplayer`, hash viejo): decidir si pasa a usar el AppImage ligero de un release `dev` o espera a una versión estable; (b) cambiar la rama por defecto del repositorio a `nueva-version` haría visible el botón «Run workflow» y haría que `releases/latest` apunte a un release estable futuro; es decisión del propietario.
