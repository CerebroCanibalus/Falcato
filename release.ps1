# release.ps1 — Script de release simple (sin npm, sin node, sin drama)
#
# Uso:
#   .\release.ps1                    # release normal (cargo build --release)
#   .\release.ps1 -SkipBuild         # solo empaqueta (si ya compilaste)
#   .\release.ps1 -Version "0.3.0"   # versión custom (default: detecta del tag)
#
# Produce: falcato-<version>.zip en la raíz del proyecto.
# No requiere npm, node, ni dependencias externas.

param(
    [switch]$SkipBuild,
    [string]$Version = ""
)

$ErrorActionPreference = "Stop"
$ProjectRoot = Split-Path -Parent $PSScriptRoot
if (-not $ProjectRoot) { $ProjectRoot = $PSScriptRoot }
$ReleaseDir = "$ProjectRoot\release"
$DistDir = "$ReleaseDir\dist"

Write-Host "=== Falcato Release Script ===" -ForegroundColor Cyan
Write-Host ""

# 1. Detectar versión
if (-not $Version) {
    # Intentar desde tag de git
    $tag = git -C $ProjectRoot describe --tags --exact-match 2>$null
    if ($tag) {
        $Version = $tag
        Write-Host "[1/5] Versión detectada desde tag: $Version" -ForegroundColor Green
    } else {
        # Usar versión del Cargo.toml
        $cargo = Get-Content "$ProjectRoot\Cargo.toml" | Select-String -Pattern '^version = "(.*)"' | ForEach-Object { $_.Matches.Groups[1].Value }
        $Version = "v$cargo"
        Write-Host "[1/5] Versión desde Cargo.toml: $Version" -ForegroundColor Yellow
    }
} else {
    if (-not $Version.StartsWith("v")) { $Version = "v$Version" }
    Write-Host "[1/5] Versión manual: $Version" -ForegroundColor Green
}

# 2. Build
if (-not $SkipBuild) {
    Write-Host "[2/5] Compilando falcato.exe (release)..." -ForegroundColor Green
    Push-Location $ProjectRoot
    try {
        $result = cargo build --release 2>&1
        if ($LASTEXITCODE -ne 0) {
            Write-Host "Error de compilación:" -ForegroundColor Red
            Write-Host $result
            exit 1
        }
    } finally {
        Pop-Location
    }
    Write-Host "      OK: target\release\falcato.exe" -ForegroundColor Green
} else {
    Write-Host "[2/5] Build saltado (-SkipBuild)" -ForegroundColor Yellow
}

# 3. Preparar directorio de distribución
Write-Host "[3/5] Preparando directorio de distribución..." -ForegroundColor Green

# Limpiar directorios previos
Remove-Item -Path $DistDir -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path "$DistDir\bin" | Out-Null
New-Item -ItemType Directory -Force -Path "$DistDir\ejemplos" | Out-Null
New-Item -ItemType Directory -Force -Path "$DistDir\skills" | Out-Null
New-Item -ItemType Directory -Force -Path "$DistDir\agents" | Out-Null

# Copiar binario
if (Test-Path "$ProjectRoot\target\release\falcato.exe") {
    Copy-Item "$ProjectRoot\target\release\falcato.exe" "$DistDir\bin\"
} else {
    Write-Host "ERROR: No se encuentra target\release\falcato.exe" -ForegroundColor Red
    exit 1
}

# Copiar ejemplos
Copy-Item "$ProjectRoot\ejemplos\*.fc" "$DistDir\ejemplos\"

# Copiar docs esenciales
foreach ($doc in @("README.md", "LICENSE", "INSTALL.md", "GUIA.md", "REFERENCIA.md", "ERRORES.md", "CHANGELOG.md")) {
    $path = "$ProjectRoot\$doc"
    if (Test-Path $path) { Copy-Item $path "$DistDir\" }
}

# Copiar skills y agents
if (Test-Path "$ProjectRoot\skills") {
    Copy-Item "$ProjectRoot\skills\*" "$DistDir\skills\" -Recurse
}
if (Test-Path "$ProjectRoot\agents") {
    Copy-Item "$ProjectRoot\agents\*" "$DistDir\agents\" -Recurse
}

# Copiar instalador
if (Test-Path "$ProjectRoot\install.ps1") {
    Copy-Item "$ProjectRoot\install.ps1" "$DistDir\"
}

# Copiar bundle DLLs (para releases locales sin CRT static)
if (Test-Path "$ProjectRoot\bundle_dlls.ps1") {
    Copy-Item "$ProjectRoot\bundle_dlls.ps1" "$DistDir\"
}

Write-Host "      Archivos copiados a $DistDir" -ForegroundColor Green

# 4. Empaquetar ZIP
Write-Host "[4/5] Empaquetando ZIP..." -ForegroundColor Green
$zipName = "falcato-$Version.zip"
$zipPath = "$ReleaseDir\$zipName"
Remove-Item -Path $zipPath -Force -ErrorAction SilentlyContinue
Compress-Archive -Path "$DistDir\*" -DestinationPath $zipPath
Write-Host "      ZIP creado: $zipPath" -ForegroundColor Green

# 5. Limpiar
Write-Host "[5/5] Limpiando temporales..." -ForegroundColor Green
Remove-Item -Path $DistDir -Recurse -Force -ErrorAction SilentlyContinue

Write-Host ""
Write-Host "=== Release listo: $zipName ===" -ForegroundColor Cyan
Write-Host "Tamaño: $([math]::Round((Get-Item $zipPath).Length / 1MB, 2)) MB" -ForegroundColor Cyan
Write-Host ""
Write-Host "Para publicar en GitHub:"
Write-Host "  1. Crea un tag:    git tag v0.2.0"
Write-Host "  2. Push el tag:    git push origin v0.2.0"
Write-Host "  3. O sube manual:  Sube $zipName a GitHub Releases"
Write-Host ""
