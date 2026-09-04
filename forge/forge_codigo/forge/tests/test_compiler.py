"""
Tests de core/compiler.py mockeando subprocess para no depender
del binario falcato real.
"""
from unittest.mock import MagicMock, patch

from forge.core.compiler import FalcatoCompiler


@patch("forge.core.compiler.subprocess.run")
def test_build_ok(mock_run):
    mock_run.return_value = MagicMock(returncode=0, stdout="compilado ok", stderr="")

    compiler = FalcatoCompiler()
    resultado = compiler.build("main.fc")

    assert resultado.ok is True
    assert resultado.returncode == 0
    mock_run.assert_called_once()


@patch("forge.core.compiler.subprocess.run")
def test_build_error(mock_run):
    mock_run.return_value = MagicMock(returncode=1, stdout="", stderr="[T001] error de tipos")

    compiler = FalcatoCompiler()
    resultado = compiler.build("main.fc")

    assert resultado.ok is False
    assert "T001" in resultado.stderr


def test_binario_no_encontrado():
    compiler = FalcatoCompiler(binario="falcato_que_no_existe")
    resultado = compiler.build("main.fc")

    assert resultado.ok is False
    assert resultado.returncode == -1
