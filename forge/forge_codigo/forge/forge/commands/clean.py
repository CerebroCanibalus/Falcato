"""
forge clean — borra artefactos de compilación generados por Falcato/Forge.
No toca los .fc fuente.
"""
import shutil
from pathlib import Path

from .. import logger
from ..core.config import cargar_config


def ejecutar(config_path: str = "forge.toml") -> bool:
    config = cargar_config(config_path)
    output = Path(config.output_dir)

    if not output.exists():
        logger.info(f"nada que limpiar, {output} no existe")
        return True

    try:
        shutil.rmtree(output)
        output.mkdir()  # deja la carpeta vacía lista para el siguiente build
        logger.ok(f"limpiado {output}")
        return True
    except OSError as e:
        logger.error(f"no se pudo limpiar {output}: {e}")
        return False
