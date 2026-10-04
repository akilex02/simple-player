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
    echo
    if [ -n "$compare_url" ]; then
      echo "… y ${more} cambios más ([ver la comparación completa](${compare_url}))."
    else
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
  for f in $(ls "$ASSETS_DIR"/*.AppImage | grep -v -- '-light\.AppImage$' || true) $(ls "$ASSETS_DIR"/*.AppImage | grep -- '-light\.AppImage$' || true); do
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

Al abrirlo por primera vez, la app ofrece registrarse en el menú de aplicaciones (un \`.desktop\` y el ícono en \`~/.local/share\`, sin permisos de administrador): así el escritorio muestra su ícono en la barra de tareas y los controles multimedia. También está en Configuración → Acerca de.

---

Esta es la versión nativa (Rust + egui) de Simple Player. La rama \`master\` conserva la versión anterior como referencia.
EOF
