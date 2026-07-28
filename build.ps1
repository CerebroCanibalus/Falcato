# Script de build para Falcato en Windows
# Configura automáticamente el entorno de Visual Studio y ejecuta comandos cargo

param(
    [Parameter()]
    [ValidateSet("build", "test", "run", "check", "clean", "ejemplo")]
    [string]$Comando = "build",
    
    [Parameter()]
    [string]$Archivo = "",
    
    [Parameter()]
    [switch]$Release
)

# Colores para output
$Rojo = "Red"
$Verde = "Green"
$Amarillo = "Yellow"
$Cyan = "Cyan"

function Escribir-Color {
    param([string]$Texto, [string]$Color = "White")
    Write-Host $Texto -ForegroundColor $Color
}

# ============================================
# 1. Buscar Visual Studio Build Tools
# ============================================
Escribir-Color "[Falcato Build] Buscando Visual Studio Build Tools..." $Cyan

$PosiblesPaths = @(
    "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC",
    "C:\Program Files\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC",
    "C:\Program Files (x86)\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC",
    "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC"
)

$VSEncontrado = $null
foreach ($Path in $PosiblesPaths) {
    if (Test-Path $Path) {
        $Versiones = Get-ChildItem $Path -Directory | Sort-Object Name -Descending
        if ($Versiones) {
            $VSEncontrado = Join-Path $Versiones[0].FullName "lib\x64"
            break
        }
    }
}

if (-not $VSEncontrado) {
    Escribir-Color "[ERROR] No se encontró Visual Studio. Instala BuildTools o Community." $Rojo
    exit 1
}

Escribir-Color "[OK] Librerías encontradas en: $VSEncontrado" $Verde

# ============================================
# 2. Configurar entorno
# ============================================
$env:LIB = $VSEncontrado

# ============================================
# 3. Ejecutar comando
# ============================================
Escribir-Color "[Falcato Build] Ejecutando: $Comando $(if($Release){'(release)'})" $Cyan

switch ($Comando) {
    "build" {
        # 1. Compilar runtime library primero (necesaria para linkear binarios)
        Escribir-Color "[Falcato Build] Compilando falcato_runtime..." $Cyan
        Push-Location lib\falcato_runtime
        $ArgsRuntime = @("build", "--release")
        & cargo @ArgsRuntime
        $RuntimeOk = $LASTEXITCODE -eq 0
        Pop-Location
        
        if (-not $RuntimeOk) {
            Escribir-Color "[ERROR] Falló la compilación de falcato_runtime" $Rojo
            exit 1
        }
        Escribir-Color "[OK] falcato_runtime compilada" $Verde
        
        # 2. Compilar el compilador
        $Args = @("build")
        if ($Release) { $Args += "--release" }
        & cargo @Args
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
        
        Escribir-Color "[OK] Build completado" $Verde
        
        # Mostrar tamaño del binario
        $Binario = if ($Release) { "target\release\falcato.exe" } else { "target\debug\falcato.exe" }
        if (Test-Path $Binario) {
            $Tam = (Get-Item $Binario).Length / 1KB
            Escribir-Color "[INFO] Binario: $Binario ($([math]::Round($Tam, 2)) KB)" $Cyan
        }
    }
    
    "test" {
        & cargo test
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
        Escribir-Color "[OK] Tests completados" $Verde
    }
    
    "run" {
        if (-not $Archivo) {
            Escribir-Color "[ERROR] Especifica un archivo con -Archivo" $Rojo
            exit 1
        }
        
        # Build primero
        & cargo build --release
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
        
        # Ejecutar
        $Binario = "target\release\falcato.exe"
        & $Binario run $Archivo
    }
    
    "check" {
        if (-not $Archivo) {
            Escribir-Color "[ERROR] Especifica un archivo con -Archivo" $Rojo
            exit 1
        }
        
        # Build primero
        & cargo build --release
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
        
        # Ejecutar check
        $Binario = "target\release\falcato.exe"
        & $Binario check $Archivo
    }
    
    "clean" {
        & cargo clean
        Escribir-Color "[OK] Limpieza completada" $Verde
    }
    
    "ejemplo" {
        # Build + ejecutar ejemplos
        $Ejemplos = Get-ChildItem "ejemplos\*.fc" | Select-Object -ExpandProperty BaseName
        
        & cargo build --release
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
        
        foreach ($Ejemplo in $Ejemplos) {
            Escribir-Color "`n[Ejecutando] $Ejemplo.fc..." $Amarillo
            & "target\release\falcato.exe" build "ejemplos\$Ejemplo.fc"
            if ($LASTEXITCODE -eq 0) {
                $Exe = ".\$Ejemplo.exe"
                if (Test-Path $Exe) {
                    & $Exe
                    $Codigo = $LASTEXITCODE
                    Escribir-Color "[Resultado] Exit code: $Codigo" $(if($Codigo -eq 0){$Verde}else{$Amarillo})
                }
            }
        }
    }
}

Escribir-Color "`n[OK] Todo listo, General." $Verde
