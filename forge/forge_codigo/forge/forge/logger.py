"""
Formato de salida consistente para toda la herramienta Forge.
Centraliza los prints para no repetir formato distinto en cada módulo.
"""
import sys
import time


def _timestamp() -> str:
    return time.strftime("%H:%M:%S")


def info(msg: str, tag: str = "forge") -> None:
    print(f"[{tag}] {msg}")


def ok(msg: str, elapsed: float | None = None, tag: str = "forge") -> None:
    if elapsed is not None:
        print(f"[{tag}] OK ({elapsed:.3f}s) — {msg}")
    else:
        print(f"[{tag}] OK — {msg}")


def error(msg: str, tag: str = "forge") -> None:
    print(f"[{tag}] ERROR: {msg}", file=sys.stderr)


def warn(msg: str, tag: str = "forge") -> None:
    print(f"[{tag}] WARN: {msg}", file=sys.stderr)


def watch_change(path: str, tag: str = "watch") -> None:
    print(f"[{tag}] Cambio detectado en {path}, recompilando...")
