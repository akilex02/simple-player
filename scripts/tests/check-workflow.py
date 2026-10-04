#!/usr/bin/env python3
"""Prueba de política de .github/workflows/release.yml (sin red): permisos, disparadores,
acciones permitidas y el orden de publicación."""
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
check(
    jobs["package"]["needs"] == "build" and set(jobs["publish"]["needs"]) == {"build", "package"},
    "build -> package -> publish",
)
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
check(0 <= i_create < i_publish < i_prune, "orden: crear borrador -> publicar -> podar")
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
