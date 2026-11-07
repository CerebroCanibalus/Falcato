"""
Lee y valida forge.toml del proyecto del usuario.
Aísla al resto de Forge de tener que parsear TOML directamente.
"""
import sys
from dataclasses import dataclass
from pathlib import Path

if sys.version_info >= (3, 11):
    import tomllib
else:
    import tomli as tomllib  # pip install tomli en Python < 3.11


DEFAULTS = {
    "entry": "main.fc",
    "watch_dir": ".",
    "output_dir": "build",
    "falcato_bin": "falcato",
}


@dataclass
class ForgeConfig:
    entry: str
    watch_dir: str
    output_dir: str
    falcato_bin: str
    raw: dict


def cargar_config(ruta: str = "forge.toml") -> ForgeConfig:
    path = Path(ruta)

    if not path.exists():
        # Sin forge.toml, usa defaults — no truena el proyecto
        return ForgeConfig(**DEFAULTS, raw={})

    with open(path, "rb") as f:
        data = tomllib.load(f)

    proyecto = data.get("proyecto", {})

    return ForgeConfig(
        entry=proyecto.get("entry", DEFAULTS["entry"]),
        watch_dir=proyecto.get("watch_dir", DEFAULTS["watch_dir"]),
        output_dir=proyecto.get("output_dir", DEFAULTS["output_dir"]),
        falcato_bin=proyecto.get("falcato_bin", DEFAULTS["falcato_bin"]),
        raw=data,
    )
