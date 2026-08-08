# ============================================================
# release.ps1 — Validación y release de Falcato (anti-frágil)
# ============================================================
# Uso: powershell -ExecutionPolicy Bypass -File release.ps1 <tag>
#   powershell -ExecutionPolicy Bypass -File release.ps1 v0.6.1
#
# Qué hace:
#   1. Verifica que Cargo.toml no tenga mojibake (causa #1 de fallo del plan de cargo-dist)
#   2. Verifica que wix/main.wxs esté en CRLF y sin líneas mixtas
#   3. Verifica que la versión de Cargo.toml coincida con el tag
#   4. Verifica que no haya archivos sin commitear
#   5. Crea el tag (si no existe) y lo pushea
#
# Si algún check falla, ABORTA con mensaje claro (no rompe el release en CI).

param(
    [Parameter(Mandatory = $true)]
    [string]$Tag
)

$ErrorActionPreference = "Stop"
$repo = "D:\Falcato"
Set-Location $repo

function Fail([string]$msg) {
    Write-Host "[ERROR] $msg" -ForegroundColor Red
    Write-Host "Release ABORTADO. Corrige el problema y reintenta." -ForegroundColor Red
    exit 1
}

Write-Host "=== Validacion pre-release para $Tag ===" -ForegroundColor Cyan

# -- 1. Mojibake en Cargo.toml -----------------------------------------------
# Detecta doble-codificacion UTF-8: bytes 0xC3 0x83 (A-tilde) son el prefijo
# de un caracter ya codificado dos veces. La 'o' correcta es 0xC3 0xB3.
Write-Host "[1/5] Verificando Cargo.toml sin mojibake..."
$cargoBytes = [System.IO.File]::ReadAllBytes("$repo\Cargo.toml")
$mojibake = $false
for ($i = 0; $i -lt $cargoBytes.Length - 1; $i++) {
    if ($cargoBytes[$i] -eq 0xC3 -and $cargoBytes[$i+1] -eq 0x83) { $mojibake = $true }
    if ($cargoBytes[$i] -eq 0xC3 -and $cargoBytes[$i+1] -eq 0x82) { $mojibake = $true }
    if ($cargoBytes[$i] -eq 0xE2 -and $i+1 -lt $cargoBytes.Length -and $cargoBytes[$i+1] -eq 0x80) { $mojibake = $true }
}
if ($mojibake) {
    Fail "Cargo.toml contiene bytes doble-codificados (mojibake). Revisa description/license. Git: git show HEAD:Cargo.toml"
}
Write-Host "      OK" -ForegroundColor Green

# -- 2. wix/main.wxs EOL -----------------------------------------------------
Write-Host "[2/5] Verificando wix/main.wxs en CRLF puro..."
$wxsText = [System.IO.File]::ReadAllText("$repo\wix\main.wxs", [System.Text.Encoding]::UTF8)
$crlfCount = ([regex]::Matches($wxsText, "`r`n")).Count
$lfTotal = ([regex]::Matches($wxsText, "`n")).Count
if ($crlfCount -ne $lfTotal) {
    Fail "wix/main.wxs tiene lineas mixtas ($crlfCount CRLF vs $lfTotal LF). cargo-dist falla el plan. Normaliza el archivo a CRLF."
}
Write-Host "      OK ($crlfCount lineas CRLF)" -ForegroundColor Green

# -- 3. Version de Cargo.toml == tag -----------------------------------------
Write-Host "[3/5] Verificando version de Cargo.toml == tag..."
$cargoText = [System.Text.Encoding]::UTF8.GetString($cargoBytes)
$verMatch = [regex]::Match($cargoText, 'version\s*=\s*"([^"]+)"')
if (-not $verMatch.Success) { Fail "No se pudo leer la version de Cargo.toml" }
$cargoVer = $verMatch.Groups[1].Value
$tagVer = $Tag.TrimStart('v')
if ($cargoVer -ne $tagVer) {
    Fail "Cargo.toml dice v$cargoVer pero el tag es $Tag. Sincroniza primero."
}
Write-Host "      OK (v$cargoVer)" -ForegroundColor Green

# -- 4. Working tree limpio --------------------------------------------------
Write-Host "[4/5] Verificando working tree limpio..."
$status = git status --porcelain
if ($status) {
    Write-Host "      Archivos sin commitear:" -ForegroundColor Yellow
    $status | ForEach-Object { Write-Host "      $_" -ForegroundColor Yellow }
    Fail "Hay cambios sin commitear. Commitealos antes del release."
}
Write-Host "      OK" -ForegroundColor Green

# -- 5. Build de verificacion + tag + push -----------------------------------
Write-Host "[5/5] Build de verificacion (debug)..."
# cargo necesita el entorno MSVC (linker) — mismo patrón que build.bat
$vsDevCmd = "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat"
if (Test-Path $vsDevCmd) {
    cmd /c "call ""$vsDevCmd"" -arch=x64 > nul && cargo build 2>&1" | Out-Null
} else {
    cargo build 2>&1 | Out-Null
}
if ($LASTEXITCODE -ne 0) { Fail "cargo build fallo" }
Write-Host "      OK" -ForegroundColor Green

Write-Host ""
Write-Host "=== Creando y pusheando tag $Tag ===" -ForegroundColor Cyan
$exists = git tag -l $Tag
if (-not $exists) {
    git tag -a $Tag -m "Release $Tag"
}
git push origin $Tag
if ($LASTEXITCODE -ne 0) { Fail "git push del tag fallo" }

Write-Host ""
Write-Host "[OK] Release $Tag pusheado. El workflow de GitHub Actions lo construye." -ForegroundColor Green
Write-Host "     Monitorea: https://github.com/CerebroCanibalus/falcato/actions" -ForegroundColor Green
