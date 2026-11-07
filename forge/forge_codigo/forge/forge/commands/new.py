"""
forge new <nombre> — genera el scaffold de un proyecto Falcato nuevo.
"""
from pathlib import Path

from .. import logger

ENTRY_BOILERPLATE = """// {nombre} — proyecto Falcato generado por Forge

fn main() {{
    // TODO: tu código aquí
}}
"""

TOML_TEMPLATE = """[proyecto]
entry = "main.fc"
watch_dir = "."
output_dir = "build"
falcato_bin = "falcato"
"""


def ejecutar(nombre: str) -> bool:
    raiz = Path(nombre)

    if raiz.exists():
        logger.error(f"la carpeta '{nombre}' ya existe")
        return False

    raiz.mkdir(parents=True)
    (raiz / "build").mkdir()

    (raiz / "main.fc").write_text(ENTRY_BOILERPLATE.format(nombre=nombre))
    (raiz / "forge.toml").write_text(TOML_TEMPLATE)

    logger.ok(f"proyecto '{nombre}' creado en {raiz.resolve()}")
    return True
