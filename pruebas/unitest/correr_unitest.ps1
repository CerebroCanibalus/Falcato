# correr_unitest.ps1 - Orquestador de unitests de Falcato (R7.6)
# Escalable: anadir test = anadir archivo. Este script NO se toca al anadir tests.
#
# Tipos de tests:
#   1. unitest_*.fc           -> falcato prueba (patron prueba "nombre" { afirmar })
#   2. unitest_compilan/*.fc  -> falcato verifica exit 0 (compila, no se ejecuta)
#   3. unitest_negativos/*.fc -> falcato verifica exit 1 + // ESPERADO: [XNNN]
#
# Uso: powershell -ExecutionPolicy Bypass -File correr_unitest.ps1 [-Falcato <ruta>]

param(
    [string]$Falcato = ""
)

# NOTA: NO usar $ErrorActionPreference = "Stop" — en PS 5.1 convierte el stderr
# de falcato (errores esperados en negativos) en error terminante y mata el script.
$ROOT = $PSScriptRoot
$ok = @()
$fallos = @()

# --- Resolver binario ---
if (-not $Falcato) {
    $candidatos = @(
        "D:\Falcato\target\debug\falcato.exe",
        "D:\Falcato\target\release\falcato.exe",
        (Get-Command falcato -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source)
    ) | Where-Object { $_ -and (Test-Path $_) }
    $FalcatoExe = $candidatos | Select-Object -First 1
} else {
    $FalcatoExe = $Falcato
}
if (-not $FalcatoExe) { Write-Error "No se encontro falcato.exe. Usa -Falcato <ruta>"; exit 2 }
if (-not (Test-Path $FalcatoExe)) { Write-Error "No existe: $FalcatoExe"; exit 2 }

$version = & $FalcatoExe version 2>&1 | Select-Object -First 1

Write-Output "=============================================="
Write-Output "  FALCATO UNITEST - R7.6"
Write-Output "  Binario: $FalcatoExe ($version)"
Write-Output "=============================================="

# --- FASE 1: unitest_*.fc (ejecutan) ---
Write-Output ""
Write-Output "--- FASE 1: unitest_*.fc (prueba + afirmar) ---"
$ejecutan = Get-ChildItem "$ROOT\unitest_*.fc" | Sort-Object Name
foreach ($archivo in $ejecutan) {
    & $FalcatoExe prueba $archivo.FullName 2>&1 | Out-Null
    $exit = $LASTEXITCODE
    if ($exit -eq 0) {
        Write-Output ("  [OK]   {0}" -f $archivo.Name)
        $ok += $archivo.Name
    } else {
        Write-Output ("  [FAIL] {0} (exit {1})" -f $archivo.Name, $exit)
        $fallos += $archivo.Name
    }
}

# --- FASE 2: unitest_compilan/*.fc (compilan, no ejecutan) ---
Write-Output ""
Write-Output "--- FASE 2: unitest_compilan/ (verifica exit 0) ---"
$compilan = Get-ChildItem "$ROOT\unitest_compilan\*.fc" -ErrorAction SilentlyContinue | Sort-Object Name
foreach ($archivo in $compilan) {
    & $FalcatoExe verifica $archivo.FullName 2>&1 | Out-Null
    $exit = $LASTEXITCODE
    if ($exit -eq 0) {
        Write-Output ("  [OK]   {0}" -f $archivo.Name)
        $ok += $archivo.Name
    } else {
        $j = & $FalcatoExe verifica $archivo.FullName --json 2>&1 | Out-String
        $codigo = ""
        if ($j -match '"codigo":"([^"]+)"') { $codigo = $Matches[1] }
        Write-Output ("  [FAIL] {0} (exit {1}, {2})" -f $archivo.Name, $exit, $codigo)
        $fallos += $archivo.Name
    }
}

# --- FASE 3: unitest_negativos/*.fc (deben fallar con codigo exacto) ---
Write-Output ""
Write-Output "--- FASE 3: unitest_negativos/ (verifica exit 1 + ESPERADO) ---"
$negativos = Get-ChildItem "$ROOT\unitest_negativos\*.fc" -ErrorAction SilentlyContinue | Sort-Object Name
foreach ($archivo in $negativos) {
    # Leer // ESPERADO: [XNNN] del header (lineas 1-15)
    $esperado = ""
    foreach ($linea in Get-Content $archivo.FullName -TotalCount 15) {
        if ($linea -match 'ESPERADO: \[([A-Z]+[0-9]+)\]') { $esperado = $Matches[1]; break }
    }
    if (-not $esperado) {
        Write-Output ("  [WARN] {0} sin header // ESPERADO: [XNNN]" -f $archivo.Name)
        $fallos += $archivo.Name
        continue
    }
    & $FalcatoExe verifica $archivo.FullName 2>&1 | Out-Null
    $exit = $LASTEXITCODE
    if ($exit -eq 1) {
        # exit 1 = error correcto. Comparar codigo (--json: usar ok:false, exit bug conocido)
        $j = & $FalcatoExe verifica $archivo.FullName --json 2>&1 | Out-String
        $codigo = ""
        if ($j -match '"codigo":"([^"]+)"') { $codigo = $Matches[1] }
        if ($codigo -eq $esperado) {
            Write-Output ("  [OK]   {0} -> {1} (esperado)" -f $archivo.Name, $codigo)
            $ok += $archivo.Name
        } else {
            Write-Output ("  [FAIL] {0} -> {1}, esperado {2}" -f $archivo.Name, $codigo, $esperado)
            $fallos += $archivo.Name
        }
    } elseif ($exit -eq 0) {
        Write-Output ("  [FAIL] {0} COMPILO cuando debia fallar con {1}" -f $archivo.Name, $esperado)
        $fallos += $archivo.Name
    } else {
        Write-Output ("  [FAIL] {0} exit {1} inesperado" -f $archivo.Name, $exit)
        $fallos += $archivo.Name
    }
}

# --- Resumen ---
Write-Output ""
$total = $ok.Count + $fallos.Count
if ($fallos.Count -gt 0) { $sem = "RED" } else { $sem = "GREEN" }
Write-Output "=============================================="
Write-Output ("  {0} RESUMEN: {1} OK / {2} FALLOS / {3} total" -f $sem, $ok.Count, $fallos.Count, $total)
if ($fallos.Count -gt 0) {
    Write-Output "  Fallos:"
    foreach ($f in $fallos) { Write-Output ("    - {0}" -f $f) }
    exit 1
}
Write-Output "=============================================="
exit 0
