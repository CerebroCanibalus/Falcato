@echo off
chcp 65001 > nul
echo ==========================================
echo Falcato - Compilador
echo ==========================================
echo.

REM Configurar entorno de Visual Studio para el linker
set VSCMD_START_DIR=%CD%
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat" -arch=x64 > nul

REM Verificar que estamos en el directorio correcto
if not exist "Cargo.toml" (
    echo [ERROR] No se encontró Cargo.toml
    echo [INFO] Ejecuta este script desde la raíz del proyecto
    exit /b 1
)

echo [1/4] Compilando compilador Falcato...
cargo build --release

if errorlevel 1 (
    echo [ERROR] Falló la compilación del compilador
    exit /b 1
)

echo [2/4] Ejecutando tests...
cargo test

if errorlevel 1 (
    echo [ERROR] Fallaron los tests
    exit /b 1
)

echo [3/4] Compilando ejemplo hola_mundo.fc...
target\release\falcato.exe compila ejemplos\hola_mundo.fc --salida hola_mundo.exe

if errorlevel 1 (
    echo [ERROR] Falló la compilación del ejemplo
    exit /b 1
)

echo [4/4] Ejecutando hola_mundo.exe...
hola_mundo.exe

if errorlevel 1 (
    echo [ERROR] Falló la ejecución del ejemplo
    exit /b 1
)

echo.
echo ==========================================
echo ¡Compilación exitosa!
echo ==========================================
