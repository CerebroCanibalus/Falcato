"""
Evita triggers duplicados cuando el editor dispara varios eventos de
"modificado" casi simultáneos al guardar.
"""
import time


class Debouncer:
    def __init__(self, wait_seconds: float = 0.5):
        self.wait_seconds = wait_seconds
        self.last_run: float = 0.0

    def deberia_ejecutar(self) -> bool:
        now = time.time()
        if now - self.last_run < self.wait_seconds:
            return False
        self.last_run = now
        return True
