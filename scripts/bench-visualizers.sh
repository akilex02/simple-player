#!/usr/bin/env bash
# Mide CPU y GPU de cada visualizador de la pantalla completa y falla si alguno rebasa los límites.
# Variables: SECS (ventana de medición, 10), CPU_MAX (% de un núcleo, 20), MS_MAX (ms de CPU por frame, 1.5),
# GPU_MAX (% de la GPU, 30), BIN. El CPU % depende de los fps de la pantalla; los ms por frame no.
set -euo pipefail
cd "$(dirname "$0")/.."

SECS="${SECS:-10}"
CPU_MAX="${CPU_MAX:-20}"
MS_MAX="${MS_MAX:-1.5}"
GPU_MAX="${GPU_MAX:-30}"
BIN="${BIN:-target/release/simple-player}"
MODES=("Barras" "Anillo" "Partículas" "Constelación" "Osciloscopio" "Franja")

# Siempre compila (incremental): medir un binario viejo daría números de otro código.
cargo build --release --quiet

over() { awk -v v="$1" -v max="$2" 'BEGIN { exit !(v > max) }'; }

fail=0
printf '%-14s %8s %8s %8s %10s\n' "Modo" "CPU %" "ms/frame" "GPU %" "Vértices"
for mode in "${MODES[@]}"; do
  line=$("$BIN" --allow-multiple --no-hotkeys --fullscreen --viz "$mode" --play --bench "$SECS" 2>/dev/null | grep '^\[bench\]' || true)
  cpu=$(grep -oP 'CPU \K[0-9.]+' <<<"$line" || echo "n/d")
  ms=$(grep -oP '\| \K[0-9.]+(?= ms de CPU)' <<<"$line" || echo "n/d")
  gpu=$(grep -oP 'GPU \K[0-9.]+' <<<"$line" || echo "n/d")
  verts=$(grep -oP '\| \K[0-9]+(?= vértices)' <<<"$line" || echo "n/d")
  printf '%-14s %8s %8s %8s %10s\n' "$mode" "$cpu" "$ms" "$gpu" "$verts"
  # Sin línea [bench] (el binario falló o no imprimió) no se midió nada: eso es un fallo, no un pase.
  if [[ -z "$line" || "$cpu" == "n/d" || "$ms" == "n/d" ]]; then echo "  ✗ sin medición para $mode"; fail=1; continue; fi
  if [[ "$cpu" != "n/d" ]] && over "$cpu" "$CPU_MAX"; then echo "  ✗ CPU $cpu % supera $CPU_MAX %"; fail=1; fi
  if [[ "$ms" != "n/d" ]] && over "$ms" "$MS_MAX"; then echo "  ✗ $ms ms/frame supera $MS_MAX ms"; fail=1; fi
  if [[ "$gpu" != "n/d" ]] && over "$gpu" "$GPU_MAX"; then echo "  ✗ GPU $gpu % supera $GPU_MAX %"; fail=1; fi
done
exit "$fail"
