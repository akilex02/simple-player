#!/usr/bin/env bash
# Entrada: tags de releases, uno por línea, en cualquier orden.
# Salida: los tags de releases «dev» que sobran (después de los primeros KEEP).
# «Más nuevo» = N más alto (es el run_number, siempre creciente); la fecha no sirve porque dos
# releases del mismo commit la comparten. Solo se consideran tags con la forma vX.Y.Z-dev.N; cualquier otro nunca se imprime.
set -euo pipefail

KEEP="${KEEP-}"
if ! [[ "$KEEP" =~ ^[0-9]+$ ]]; then
  echo "KEEP debe ser un número entero (recibido: «${KEEP}»)" >&2
  exit 2
fi

{ grep -E '^v.+-dev\.[0-9]+$' || true; } \
  | awk '{ n = $0; sub(/.*-dev\./, "", n); print n, $0 }' \
  | sort -rn \
  | awk -v keep="$KEEP" 'NR > keep { print $2 }'
