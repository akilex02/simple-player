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
