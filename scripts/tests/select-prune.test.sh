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

# 7. Sin ningún tag dev no falla (la tubería no debe romperse por el grep).
out=$(tags simpleplayer v0.1.0 | KEEP=3 bash "$SCRIPT"); code=$?
eq "$code" "0" "sin tags dev: sale bien"
eq "$out" "" "sin tags dev: no imprime nada"

# 8. El orden de entrada no importa: manda el número N del tag (run_number), no la fecha.
#    (Dos releases del mismo commit comparten fecha de creación y salían en cualquier orden.)
out=$(tags v0.1.0-dev.6 v0.1.0-dev.8 v0.1.0-dev.7 v0.1.0-dev.5 | KEEP=2 bash "$SCRIPT" | tr '\n' ' ')
eq "$out" "v0.1.0-dev.6 v0.1.0-dev.5 " "conserva los N más altos aunque lleguen desordenados"
out=$(tags v0.1.0-dev.9 v0.1.0-dev.10 v0.1.0-dev.2 | KEEP=2 bash "$SCRIPT" | tr '\n' ' ')
eq "$out" "v0.1.0-dev.2 " "compara N como número (10 > 9)"

echo
[ "$fails" -eq 0 ] && echo "TODO BIEN" || { echo "$fails prueba(s) fallaron"; exit 1; }
