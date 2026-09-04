"""
Detecta la estructura de un proyecto Falcato existente:
dónde están los .fc, cuál es el archivo de entrada, etc.
Usa config.py para no hardcodear rutas.
"""
from pathlib import Path

from .config import ForgeConfig


class ProyectoFalcato:
    def __init__(self, config: ForgeConfig, raiz: str = "."):
        self.config = config
        self.raiz = Path(raiz)

    @property
    def entry_path(self) -> Path:
        return self.raiz / self.config.entry

    @property
    def output_path(self) -> Path:
        return self.raiz / self.config.output_dir

    def archivos_fc(self) -> list[Path]:
        """Lista todos los archivos .fc dentro del watch_dir configurado."""
        watch_dir = self.raiz / self.config.watch_dir
        return sorted(watch_dir.rglob("*.fc"))

    def validar(self) -> list[str]:
        """Devuelve lista de problemas encontrados (vacía si todo bien)."""
        problemas = []
        if not self.entry_path.exists():
            problemas.append(f"No se encontró el archivo de entrada: {self.entry_path}")
        if not self.archivos_fc():
            problemas.append(f"No hay archivos .fc en {self.raiz / self.config.watch_dir}")
        return problemas
