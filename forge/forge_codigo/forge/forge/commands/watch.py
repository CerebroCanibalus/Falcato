"""
forge watch — conecta watcher/observer.py + debounce.py con commands/build.py
para recompilar automáticamente ante cambios en archivos .fc
"""
import time

from .. import logger
from ..core.config import cargar_config
from ..watcher.observer import ForgeWatcher
from . import build as build_cmd


def ejecutar(archivo: str | None = None, config_path: str = "forge.toml") -> None:
    config = cargar_config(config_path)

    def on_change(path: str) -> None:
        logger.watch_change(path)
        build_cmd.ejecutar(archivo, config_path)

    logger.info(f"vigilando {config.watch_dir} — CTRL+C para salir", tag="watch")

    # primer build al iniciar
    build_cmd.ejecutar(archivo, config_path)

    watcher = ForgeWatcher(config.watch_dir, on_change)
    watcher.iniciar()

    try:
        while True:
            time.sleep(1)
    except KeyboardInterrupt:
        logger.info("deteniendo watch mode", tag="watch")
    finally:
        watcher.detener()
