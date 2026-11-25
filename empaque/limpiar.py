#!/usr/bin/env python3
"""Borra las compilaciones viejas y deja las actuales. Lo usa el comando /aceptado.

Rust guarda cada versión de cada programa de prueba (nombre-<16 letras>) y nunca borra las anteriores: en una
sesión de trabajo se juntan gigas. Aquí, de cada nombre se deja el más nuevo; lo que se borre y haga falta, cargo lo
vuelve a armar solo. También quita las compilaciones de depuración (el proyecto usa --release) y los temporales que
dejó la app hace más de una hora.

    python3 empaque/limpiar.py            # limpia
    python3 empaque/limpiar.py --probar   # solo dice qué borraría
"""
import os
import re
import shutil
import sys
import tempfile
import time

RAIZ = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PROBAR = "--probar" in sys.argv
COPIA = re.compile(r"^(.+)-[0-9a-f]{16}$")


def peso(ruta):
    if os.path.isfile(ruta):
        return os.path.getsize(ruta)
    total = 0
    for base, _, archivos in os.walk(ruta):
        for a in archivos:
            try:
                total += os.path.getsize(os.path.join(base, a))
            except OSError:
                pass
    return total


def quitar(ruta):
    p = peso(ruta)
    if not PROBAR:
        if os.path.isdir(ruta):
            shutil.rmtree(ruta, ignore_errors=True)
        else:
            try:
                os.remove(ruta)
            except OSError:
                return 0
    return p


def copias_viejas(carpeta):
    """De cada programa (ejecutable sin extensión, nombre-<hash>), todos menos el más nuevo."""
    if not os.path.isdir(carpeta):
        return []
    grupos = {}
    for a in os.listdir(carpeta):
        r = os.path.join(carpeta, a)
        m = COPIA.match(a)
        if m and os.path.isfile(r) and os.access(r, os.X_OK):
            grupos.setdefault(m.group(1), []).append(r)
    viejas = []
    for v in grupos.values():
        v.sort(key=os.path.getmtime, reverse=True)
        viejas += v[1:]
    return viejas


def main():
    total, cuantos = 0, 0
    for c in ["target/release/deps", "target/release/examples", "daw/target/release/deps", "daw/target/release/examples"]:
        for r in copias_viejas(os.path.join(RAIZ, c)):
            total += quitar(r)
            cuantos += 1
    for c in ["target/debug", "daw/target/debug"]:
        r = os.path.join(RAIZ, c)
        if os.path.isdir(r):
            total += quitar(r)
            cuantos += 1
    # Temporales de la app (exportar, pruebas) de hace más de una hora: los de ahora pueden estar en uso.
    tmp = tempfile.gettempdir()
    hace = time.time() - 3600
    for a in os.listdir(tmp):
        r = os.path.join(tmp, a)
        if a.startswith(("beethoven-craft", "bc-", ".toma-uno")):
            try:
                if os.path.getmtime(r) < hace:
                    total += quitar(r)
                    cuantos += 1
            except OSError:
                pass
    verbo = "Borraría" if PROBAR else "Borré"
    print(f"{verbo} {cuantos} cosas viejas: {total / 1e9:.1f} GB.")
    libre = shutil.disk_usage(RAIZ).free
    print(f"Libre en el disco: {libre / 1e9:.1f} GB.")


main()
