"""
Única capa que ejecuta subprocess contra el binario falcato.
Nada más en Forge debe llamar subprocess directamente hacia falcato.

IMPORTANTE: los flags exactos de `falcato build` / `run` / `check` no están
confirmados todavía contra REFERENCIA.md del repo. Este wrapper asume la
forma más simple posible (`falcato <subcomando> <archivo>`) y expone
`extra_args` para poder pasar flags reales sin tener que tocar la firma
de las funciones cuando se confirmen.
"""
import subprocess
import time
from dataclasses import dataclass, field


@dataclass
class ResultadoCompilacion:
    ok: bool
    returncode: int
    stdout: str
    stderr: str
    elapsed: float
    comando: list[str] = field(default_factory=list)


class FalcatoCompiler:
    def __init__(self, binario: str = "falcato"):
        self.binario = binario

    def _ejecutar(self, subcomando: str, archivo: str | None = None,
                   extra_args: list[str] | None = None) -> ResultadoCompilacion:
        cmd = [self.binario, subcomando]
        if archivo:
            cmd.append(archivo)
        if extra_args:
            cmd.extend(extra_args)

        start = time.time()
        try:
            result = subprocess.run(cmd, capture_output=True, text=True)
        except FileNotFoundError:
            elapsed = time.time() - start
            return ResultadoCompilacion(
                ok=False, returncode=-1, stdout="",
                stderr=f"No se encontró el binario '{self.binario}'. "
                       f"Verifica FALCATO_BIN o el PATH.",
                elapsed=elapsed, comando=cmd,
            )
        elapsed = time.time() - start

        return ResultadoCompilacion(
            ok=result.returncode == 0,
            returncode=result.returncode,
            stdout=result.stdout,
            stderr=result.stderr,
            elapsed=elapsed,
            comando=cmd,
        )

    def build(self, archivo: str, extra_args: list[str] | None = None) -> ResultadoCompilacion:
        """falcato build <archivo>"""
        return self._ejecutar("build", archivo, extra_args)

    def run(self, archivo: str, extra_args: list[str] | None = None) -> ResultadoCompilacion:
        """falcato run <archivo>"""
        return self._ejecutar("run", archivo, extra_args)

    def check(self, archivo: str, extra_args: list[str] | None = None) -> ResultadoCompilacion:
        """falcato check <archivo> — solo valida, no genera binario"""
        return self._ejecutar("check", archivo, extra_args)

    def version(self) -> ResultadoCompilacion:
        """falcato version"""
        return self._ejecutar("version")
