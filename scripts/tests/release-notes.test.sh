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
