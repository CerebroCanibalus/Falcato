# release.ps1 — Script de release para Falcato
#
# Uso:
#   .\release.ps1                           # release completo
#   .\release.ps1 -SkipBuild                # solo empaqueta
#   .\release.ps1 -Version "0.3.0"          # versión custom
#   .\release.ps1 -SkipVsix                 # salta build VSIX
#
# Produce: falcato-<version>.zip en release/
# Incluye: binario, ejemplos, docs, skills, install.ps1 y VSIX (si hay npm)

param(
    [switch]$SkipBuild,
    [switch]$SkipVsix,
    [string]$Version = ""
)

$ErrorActionPreference = "Stop"
$ProjectRoot = $PSScriptRoot
if (-not $ProjectRoot) { $ProjectRoot = $PSScriptRoot }
$ReleaseDir = "$ProjectRoot\release"
$DistDir = "$ReleaseDir\dist"

Write-Host "=== Falcato Release Script ===" -ForegroundColor Cyan
Write-Host ""

# 1. Detectar versión
if (-not $Version) {
    $tag = git -C $ProjectRoot describe --tags --exact-match 2>$null
    if ($tag) {
        $Version = $tag
        Write-Host "[1/6] Versión desde tag: $Version" -ForegroundColor Green
    } else {
        $cargo = Get-Content "$ProjectRoot\Cargo.toml" | Select-String -Pattern '^version = "(.*)"' | ForEach-Object { $_.Matches.Groups[1].Value }
        $Version = "v$cargo"
        Write-Host "[1/6] Versión desde Cargo.toml: $Version" -ForegroundColor Yellow
    }
} else {
    if (-not $Version.StartsWith("v")) { $Version = "v$Version" }
    Write-Host "[1/6] Versión manual: $Version" -ForegroundColor Green
}

# 2. Build binario
if (-not $SkipBuild) {
    Write-Host "[2/6] Compilando falcato.exe (release)..." -ForegroundColor Green
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
    Write-Host "[2/6] Build saltado (-SkipBuild)" -ForegroundColor Yellow
}

# 3. Build VSIX (si hay npm y no se saltó)
$vsixIncluido = $false
if (-not $SkipVsix) {
    Write-Host "[3/6] Extensión VS Code..." -ForegroundColor Green
    $vsixDir = "$ProjectRoot\falcato-vscode"
    if (Test-Path "$vsixDir\package.json") {
        $npmPath = (Get-Command "npm" -ErrorAction SilentlyContinue).Source
        if ($npmPath) {
            Write-Host "      npm detectado: $npmPath" -ForegroundColor Green
            Push-Location $vsixDir
            try {
                # Si no hay node_modules, instalarlos
                if (-not (Test-Path "$vsixDir\node_modules")) {
                    Write-Host "      Instalando dependencias npm..." -ForegroundColor Yellow
                    npm ci 2>&1 | Out-Null
                }
                # Construir VSIX
                Write-Host "      Construyendo VSIX..." -ForegroundColor Green
                npx vsce package 2>&1 | Out-Null
                $vsixFile = Get-Item "$vsixDir\*.vsix" | Select-Object -First 1
                if ($vsixFile) {
                    Write-Host "      VSIX: $($vsixFile.Name)" -ForegroundColor Green
                    $vsixIncluido = $true
                } else {
                    Write-Host "      AVISO: No se generó .vsix (posible error de vsce)" -ForegroundColor Yellow
                }
            } catch {
                Write-Host "      AVISO: Error construyendo VSIX: $_" -ForegroundColor Yellow
            } finally {
                Pop-Location
            }
        } else {
            Write-Host "      ⚠ npm no encontrado. VSIX no se incluirá." -ForegroundColor Yellow
            Write-Host "        Para generar el VSIX manualmente:" -ForegroundColor Yellow
            Write-Host "        cd falcato-vscode && npm install && npx vsce package" -ForegroundColor Yellow
        }
    } else {
        Write-Host "      ⚠ Directorio 'falcato-vscode/' no encontrado" -ForegroundColor Yellow
    }
} else {
    Write-Host "[3/6] VSIX saltado (-SkipVsix)" -ForegroundColor Yellow
}

# 4. Preparar directorio de distribución
Write-Host "[4/6] Preparando directorio de distribución..." -ForegroundColor Green

Remove-Item -Path $DistDir -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path "$DistDir\bin" | Out-Null
New-Item -ItemType Directory -Force -Path "$DistDir\ejemplos" | Out-Null
New-Item -ItemType Directory -Force -Path "$DistDir\skills" | Out-Null
New-Item -ItemType Directory -Force -Path "$DistDir\agents" | Out-Null

# Binario
if (Test-Path "$ProjectRoot\target\release\falcato.exe") {
    Copy-Item "$ProjectRoot\target\release\falcato.exe" "$DistDir\bin\"
} else {
    Write-Host "ERROR: No se encuentra target\release\falcato.exe" -ForegroundColor Red
    exit 1
}

# VSIX (si se generó)
if ($vsixIncluido) {
    $vsixFile = Get-Item "$ProjectRoot\falcato-vscode\*.vsix" | Select-Object -First 1
    if ($vsixFile) {
        Copy-Item $vsixFile.FullName "$DistDir\bin\"
        Write-Host "      VSIX incluido en bin/" -ForegroundColor Green
    }
}

# Ejemplos
Copy-Item "$ProjectRoot\ejemplos\*.fc" "$DistDir\ejemplos\"

# Docs
foreach ($doc in @("README.md", "LICENSE", "INSTALL.md", "GUIA.md", "REFERENCIA.md", "ERRORES.md", "CHANGELOG.md")) {
    $path = "$ProjectRoot\$doc"
    if (Test-Path $path) { Copy-Item $path "$DistDir\" }
}
# Carpeta GUIA/
if (Test-Path "$ProjectRoot\GUIA") {
    Copy-Item "$ProjectRoot\GUIA" "$DistDir\GUIA" -Recurse
}
# Carpeta docs/
if (Test-Path "$ProjectRoot\docs") {
    New-Item -ItemType Directory -Force -Path "$DistDir\docs" | Out-Null
    Copy-Item "$ProjectRoot\docs\*.md" "$DistDir\docs\"
}

# Skills y agents
if (Test-Path "$ProjectRoot\skills") {
    Copy-Item "$ProjectRoot\skills\*" "$DistDir\skills\" -Recurse
}
if (Test-Path "$ProjectRoot\agents") {
    Copy-Item "$ProjectRoot\agents\*" "$DistDir\agents\" -Recurse
}

# Instaladores
if (Test-Path "$ProjectRoot\install.ps1") {
    Copy-Item "$ProjectRoot\install.ps1" "$DistDir\"
}
if (Test-Path "$ProjectRoot\bundle_dlls.ps1") {
    Copy-Item "$ProjectRoot\bundle_dlls.ps1" "$DistDir\"
}

Write-Host "      Archivos copiados a $DistDir" -ForegroundColor Green

# 5. Empaquetar ZIP
Write-Host "[5/6] Empaquetando ZIP..." -ForegroundColor Green
$zipName = "falcato-$Version.zip"
$zipPath = "$ReleaseDir\$zipName"
Remove-Item -Path $zipPath -Force -ErrorAction SilentlyContinue
Compress-Archive -Path "$DistDir\*" -DestinationPath $zipPath
Write-Host "      ZIP creado: $zipPath" -ForegroundColor Green

# 6. Limpiar
Write-Host "[6/6] Limpiando temporales..." -ForegroundColor Green
Remove-Item -Path $DistDir -Recurse -Force -ErrorAction SilentlyContinue
# Limpiar VSIX temporal del directorio falcato-vscode
if ($vsixIncluido) {
    Remove-Item "$ProjectRoot\falcato-vscode\*.vsix" -Force -ErrorAction SilentlyContinue
}

Write-Host ""
Write-Host "=== Release listo: $zipName ===" -ForegroundColor Cyan
Write-Host "Tamaño: $([math]::Round((Get-Item $zipPath).Length / 1MB, 2)) MB" -ForegroundColor Cyan
Write-Host "Contenido:" -ForegroundColor Cyan
if ($vsixIncluido) {
    Write-Host "  ✅ falcato.exe + VSIX + ejemplos + docs + install.ps1" -ForegroundColor Cyan
} else {
    Write-Host "  ✅ falcato.exe + ejemplos + docs + install.ps1" -ForegroundColor Cyan
    Write-Host "  ⚠ VSIX no incluido (npm no disponible)" -ForegroundColor Yellow
}
Write-Host ""
Write-Host "Para publicar en GitHub:" -ForegroundColor Gray
Write-Host "  1. Crea un tag:    git tag v0.2.0" -ForegroundColor Gray
Write-Host "  2. Push el tag:    git push origin v0.2.0" -ForegroundColor Gray
Write-Host "  3. O sube manual:  Sube $zipName a GitHub Releases" -ForegroundColor Gray
Write-Host ""
