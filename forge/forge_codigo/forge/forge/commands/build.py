"""
forge build — compilación puntual usando core/compiler.py y core/project.py
"""
from .. import logger
from ..core.compiler import FalcatoCompiler
from ..core.config import cargar_config
from ..core.project import ProyectoFalcato


def ejecutar(archivo: str | None = None, config_path: str = "forge.toml") -> bool:
    config = cargar_config(config_path)
    proyecto = ProyectoFalcato(config)

    target = archivo or str(proyecto.entry_path)

    compiler = FalcatoCompiler(binario=config.falcato_bin)
    resultado = compiler.build(target)

    if resultado.ok:
        logger.ok(f"compilado {target}", elapsed=resultado.elapsed)
    else:
        logger.error(f"fallo compilando {target}\n{resultado.stderr}")

    return resultado.ok
