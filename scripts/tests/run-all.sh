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
