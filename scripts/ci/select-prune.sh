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
