"""
Wrapper sobre watchdog. Configura qué carpeta vigilar, qué extensiones
filtrar (.fc) y qué callback disparar. No sabe nada de cómo compilar,
solo notifica.
"""
from typing import Callable

from watchdog.events import FileSystemEventHandler
from watchdog.observers import Observer

from .debounce import Debouncer


class ForgeFileHandler(FileSystemEventHandler):
    def __init__(self, on_change: Callable[[str], None], extension: str = ".fc",
                 debounce_seconds: float = 0.5):
        self.on_change = on_change
        self.extension = extension
        self.debouncer = Debouncer(debounce_seconds)

    def on_modified(self, event):
        if event.is_directory:
            return
        if not event.src_path.endswith(self.extension):
            return
        if self.debouncer.deberia_ejecutar():
            self.on_change(event.src_path)


class ForgeWatcher:
    def __init__(self, watch_dir: str, on_change: Callable[[str], None],
                 extension: str = ".fc", debounce_seconds: float = 0.5):
        self.watch_dir = watch_dir
        self.handler = ForgeFileHandler(on_change, extension, debounce_seconds)
        self.observer = Observer()

    def iniciar(self) -> None:
        self.observer.schedule(self.handler, self.watch_dir, recursive=True)
        self.observer.start()

    def detener(self) -> None:
        self.observer.stop()
        self.observer.join()
