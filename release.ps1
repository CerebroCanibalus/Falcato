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

Write-Host "=== Validación pre-release para $Tag ===" -ForegroundColor Cyan

# ── 1. Mojibake en Cargo.toml ────────────────────────────────────────────────
Write-Host "[1/5] Verificando Cargo.toml sin mojibake..."
$cargoBytes = [System.IO.File]::ReadAllBytes("$repo\Cargo.toml")
$cargoText = [System.Text.Encoding]::UTF8.GetString($cargoBytes)
# Patrón de doble-codificación: Ã  Â  â€  â€™  etc. (caracteres UTF-8 corruptos comunes)
if ($cargoText -match "[ÃÂâ€œ™]") {
    Fail "Cargo.toml contiene caracteres doble-codificados (mojibake). Revisa description/license con: git show HEAD:Cargo.toml | findstr description"
}
Write-Host "      OK" -ForegroundColor Green

# ── 2. wix/main.wxs EOL ─────────────────────────────────────────────────────
Write-Host "[2/5] Verificando wix/main.wxs en CRLF puro..."
$wxsText = [System.IO.File]::ReadAllText("$repo\wix\main.wxs", [System.Text.Encoding]::UTF8)
$crlfCount = ([regex]::Matches($wxsText, "`r`n")).Count
$lfTotal = ([regex]::Matches($wxsText, "`n")).Count
if ($crlfCount -ne $lfTotal) {
    Fail "wix/main.wxs tiene líneas mixtas ($crlfCount CRLF vs $lfTotal LF). cargo-dist falla el plan. Normaliza con: (Get-Content wix/main.wxs -Raw) -replace \"`r?`n\", \"`r`n\" | Set-Content wix/main.wxs -NoNewline"
}
Write-Host "      OK ($crlfCount líneas CRLF)" -ForegroundColor Green

# ── 3. Versión de Cargo.toml == tag ─────────────────────────────────────────
Write-Host "[3/5] Verificando versión de Cargo.toml == tag..."
$verMatch = [regex]::Match($cargoText, 'version\s*=\s*"([^"]+)"')
if (-not $verMatch.Success) { Fail "No se pudo leer la versión de Cargo.toml" }
$cargoVer = $verMatch.Groups[1].Value
$tagVer = $Tag.TrimStart('v')
if ($cargoVer -ne $tagVer) {
    Fail "Cargo.toml dice v$cargoVer pero el tag es $Tag. Sincroniza primero."
}
Write-Host "      OK (v$cargoVer)" -ForegroundColor Green

# ── 4. Working tree limpio ──────────────────────────────────────────────────
Write-Host "[4/5] Verificando working tree limpio..."
$status = git status --porcelain
if ($status) {
    Write-Host "      Archivos sin commitear:" -ForegroundColor Yellow
    $status | ForEach-Object { Write-Host "      $_" -ForegroundColor Yellow }
    Fail "Hay cambios sin commitear. Commitéalos antes del release."
}
Write-Host "      OK" -ForegroundColor Green

# ── 5. Build de verificación + tag + push ───────────────────────────────────
Write-Host "[5/5] Build de verificación (debug)..."
cargo build 2>&1 | Out-Null
if ($LASTEXITCODE -ne 0) { Fail "cargo build falló" }
Write-Host "      OK" -ForegroundColor Green

Write-Host ""
Write-Host "=== Creando y pusheando tag $Tag ===" -ForegroundColor Cyan
$exists = git tag -l $Tag
if (-not $exists) {
    git tag -a $Tag -m "Release $Tag"
}
git push origin $Tag
if ($LASTEXITCODE -ne 0) { Fail "git push del tag falló" }

Write-Host ""
Write-Host "✅ Release $Tag pusheado. El workflow de GitHub Actions lo construye." -ForegroundColor Green
Write-Host "   Monitorea: https://github.com/CerebroCanibalus/falcato/actions" -ForegroundColor Green
