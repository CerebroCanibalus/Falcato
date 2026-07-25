<# 
.SYNOPSIS
    Instalador interactivo de Falcato - Lenguaje de sistemas iberohablante
.DESCRIPTION
    Instala falcato.exe en PATH y opcionalmente configura:
    - VS Code Extension (syntax + LSP + tema Falcato Dorado)
    - OpenCode Agent + Skill
    - Claude Code Agent + Skill
    - Cursor (usa VS Code extension)
.NOTES
    Requiere: Windows 10/11, PowerShell 5.1+
    Ejecutar desde la carpeta extraída del ZIP de release.
#>

param(
    [switch]$NoPath,           # No agregar al PATH
    [switch]$NoVSCode,         # Saltar VS Code extension
    [switch]$NoOpenCode,       # Saltar OpenCode
    [switch]$NoClaude,         # Saltar Claude Code
    [switch]$NoCursor,         # Saltar Cursor
    [switch]$Quiet,            # Sin prompts, usa defaults (instala todo)
    [switch]$Uninstall         # Desinstalar
)

# Configuración
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$FalcaoExe = Join-Path $ScriptDir "bin\falcato.exe"
$VSIXPath = Join-Path $ScriptDir "bin\falcato-language-*.vsix"
$SkillsDir = Join-Path $ScriptDir "skills\falcato-language"
$AgentPath = Join-Path $ScriptDir "agents\falcato.md"

$InstallDir = "$env:USERPROFILE\.falcato"
$BinDir = Join-Path $InstallDir "bin"
$ExamplesDir = Join-Path $InstallDir "ejemplos"

# Colores
$Green = [ConsoleColor]::Green
$Yellow = [ConsoleColor]::Yellow
$Red = [ConsoleColor]::Red
$Cyan = [ConsoleColor]::Cyan
$Gray = [ConsoleColor]::DarkGray

function Write-Header { param($msg) Write-Host "`n=== $msg ===" -ForegroundColor $Cyan }
function Write-OK { param($msg) Write-Host "  [✓] $msg" -ForegroundColor $Green }
function Write-Warn { param($msg) Write-Host "  [!] $msg" -ForegroundColor $Yellow }
function Write-Err { param($msg) Write-Host "  [✗] $msg" -ForegroundColor $Red }
function Write-Info { param($msg) Write-Host "  $msg" -ForegroundColor $Gray }

function Confirm-Action {
    param([string]$Message, [bool]$Default = $true)
    if ($Quiet) { return $Default }
    $suffix = if ($Default) { "[S/n]" } else { "[s/N]" }
    $choice = Read-Host "$Message $suffix"
    if ([string]::IsNullOrWhiteSpace($choice)) { return $Default }
    return $choice -match '^[sSyY]'
}

function Add-ToUserPath {
    param([string]$PathToAdd)
    $currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($currentPath -notlike "*$PathToAdd*") {
        $newPath = "$currentPath;$PathToAdd"
        [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
        Write-OK "Agregado al PATH de usuario: $PathToAdd"
        Write-Warn "Reinicia la terminal o ejecuta: refreshenv"
        return $true
    }
    Write-Info "Ya está en PATH: $PathToAdd"
    return $false
}

function Remove-FromUserPath {
    param([string]$PathToRemove)
    $currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($currentPath -like "*$PathToRemove*") {
        $newPath = ($currentPath -split ';' | Where-Object { $_ -ne $PathToRemove }) -join ';'
        [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
        Write-OK "Removido del PATH: $PathToRemove"
        return $true
    }
    return $false
}

# ===== UNINSTALL =====
if ($Uninstall) {
    Write-Header "DESINSTALANDO FALCATO"
    Remove-FromUserPath $BinDir
    if (Test-Path $InstallDir) {
        Remove-Item -Recurse -Force $InstallDir -ErrorAction SilentlyContinue
        Write-OK "Eliminado: $InstallDir"
    }
    # VS Code extension
    if (Get-Command code -ErrorAction SilentlyContinue) {
        $ext = code --list-extensions | Where-Object { $_ -like "falcato*" }
        if ($ext) {
            code --uninstall-extension $ext --force
            Write-OK "Extensión VS Code desinstalada: $ext"
        }
    }
    # OpenCode
    $ocAgent = "$env:APPDATA\opencode\agents\falcato.md"
    $ocSkill = "$env:APPDATA\opencode\skills\falcato-language"
    if (Test-Path $ocAgent) { Remove-Item $ocAgent -Force; Write-OK "OpenCode agent removido" }
    if (Test-Path $ocSkill) { Remove-Item $ocSkill -Recurse -Force; Write-OK "OpenCode skill removida" }
    # Claude Code
    $ccAgent = "$env:USERPROFILE\.claude\agents\falcato.md"
    $ccSkill = "$env:USERPROFILE\.claude\skills\falcato-language"
    if (Test-Path $ccAgent) { Remove-Item $ccAgent -Force; Write-OK "Claude Code agent removido" }
    if (Test-Path $ccSkill) { Remove-Item $ccSkill -Recurse -Force; Write-OK "Claude Code skill removida" }
    Write-Host "`n✅ Desinstalación completa. Reinicia la terminal." -ForegroundColor $Green
    exit 0
}

# ===== VERIFICACIONES INICIALES =====
Write-Header "INSTALADOR FALCATO v0.2.0"
Write-Info "Directorio de instalación: $InstallDir"

if (-not (Test-Path $FalcaoExe)) {
    Write-Err "No se encuentra falcato.exe en $FalcaoExe"
    Write-Err "Ejecuta este script desde la carpeta extraída del ZIP (donde está la carpeta 'bin')"
    exit 1
}

# ===== CREAR DIRECTORIOS =====
Write-Header "CREANDO ESTRUCTURA"
New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
New-Item -ItemType Directory -Force -Path $ExamplesDir | Out-Null
Write-OK "Directorios creados en $InstallDir"

# ===== COPIAR BINARIO =====
Write-Header "INSTALANDO FALCATO.EXE"
Copy-Item -Path $FalcaoExe -Destination $BinDir -Force
Write-OK "falcato.exe → $BinDir"

# Copiar ejemplos
$examplesSrc = Join-Path $ScriptDir "ejemplos"
if (Test-Path $examplesSrc) {
    Copy-Item -Path (Join-Path $examplesSrc "*.fc") -Destination $ExamplesDir -Force
    Write-OK "Ejemplos copiados a $ExamplesDir"
}

# ===== PATH (OBLIGATORIO) =====
if (-not $NoPath) {
    Write-Header "CONFIGURANDO PATH"
    if ($Quiet -or (Confirm-Action "¿Agregar $BinDir al PATH de usuario?")) {
        Add-ToUserPath $BinDir
    } else {
        Write-Warn "Saltado. Deberás agregar manualmente: $BinDir"
    }
}

# ===== VS CODE EXTENSION =====
if (-not $NoVSCode) {
    Write-Header "EXTENSIÓN VS CODE"
    $vsix = Get-Item $VSIXPath -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($vsix -and (Get-Command code -ErrorAction SilentlyContinue)) {
        if ($Quiet -or (Confirm-Action "¿Instalar extensión VS Code (syntax + LSP + tema Falcato Dorado)?")) {
            try {
                code --install-extension $vsix.FullName --force
                Write-OK "Extensión instalada: $($vsix.Name)"
                Write-Info "Tema: Ctrl+K Ctrl+T → 'Falcato Dorado'"
            } catch {
                Write-Warn "Falló instalación automática. Instala manualmente:"
                Write-Info "  code --install-extension $($vsix.FullName) --force"
            }
        }
    } elseif (-not $vsix) {
        Write-Warn "No se encontró .vsix en bin/"
    } else {
        Write-Warn "'code' no está en PATH. Instala VS Code o usa: code --install-extension <ruta.vsix>"
    }
}

# ===== OPENCODE =====
if (-not $NoOpenCode) {
    Write-Header "OPENCODE AGENT + SKILL"
    $ocBase = "$env:APPDATA\opencode"
    $ocAgentDir = Join-Path $ocBase "agents"
    $ocSkillDir = Join-Path $ocBase "skills"
    $ocAgentDest = Join-Path $ocAgentDir "falcato.md"
    $ocSkillDest = Join-Path $ocSkillDir "falcato-language"

    if (Test-Path $ocBase) {
        if ($Quiet -or (Confirm-Action "¿Instalar agent y skill en OpenCode ($ocBase)?")) {
            New-Item -ItemType Directory -Force -Path $ocAgentDir | Out-Null
            New-Item -ItemType Directory -Force -Path $ocSkillDir | Out-Null
            Copy-Item $AgentPath $ocAgentDest -Force
            Copy-Item $SkillsDir $ocSkillDest -Recurse -Force
            Write-OK "Agent → $ocAgentDest"
            Write-OK "Skill → $ocSkillDest"
        }
    } else {
        Write-Info "OpenCode no detectado en $ocBase (se crea al primer uso)"
        if ($Quiet -or (Confirm-Action "¿Crear directorios e instalar anyway?")) {
            New-Item -ItemType Directory -Force -Path $ocAgentDir | Out-Null
            New-Item -ItemType Directory -Force -Path $ocSkillDir | Out-Null
            Copy-Item $AgentPath $ocAgentDest -Force
            Copy-Item $SkillsDir $ocSkillDest -Recurse -Force
            Write-OK "Agent + Skill instalados (OpenCode los detectará al iniciar)"
        }
    }
}

# ===== CLAUDE CODE =====
if (-not $NoClaude) {
    Write-Header "CLAUDE CODE AGENT + SKILL"
    $ccBase = "$env:USERPROFILE\.claude"
    $ccAgentDir = Join-Path $ccBase "agents"
    $ccSkillDir = Join-Path $ccBase "skills"
    $ccAgentDest = Join-Path $ccAgentDir "falcato.md"
    $ccSkillDest = Join-Path $ccSkillDir "falcato-language"

    if (Test-Path $ccBase) {
        if ($Quiet -or (Confirm-Action "¿Instalar agent y skill en Claude Code ($ccBase)?")) {
            New-Item -ItemType Directory -Force -Path $ccAgentDir | Out-Null
            New-Item -ItemType Directory -Force -Path $ccSkillDir | Out-Null
            Copy-Item $AgentPath $ccAgentDest -Force
            Copy-Item $SkillsDir $ccSkillDest -Recurse -Force
            Write-OK "Agent → $ccAgentDest"
            Write-OK "Skill → $ccSkillDest"
        }
    } else {
        Write-Info "Claude Code no detectado en $ccBase"
        if ($Quiet -or (Confirm-Action "¿Crear directorios e instalar anyway?")) {
            New-Item -ItemType Directory -Force -Path $ccAgentDir | Out-Null
            New-Item -ItemType Directory -Force -Path $ccSkillDir | Out-Null
            Copy-Item $AgentPath $ccAgentDest -Force
            Copy-Item $SkillsDir $ccSkillDest -Recurse -Force
            Write-OK "Agent + Skill instalados (Claude Code los detectará al iniciar)"
        }
    }
}

# ===== CURSOR =====
if (-not $NoCursor) {
    Write-Header "CURSOR"
    Write-Info "Cursor usa la misma extensión que VS Code."
    Write-Info "Si instalaste la extensión VS Code, ya funciona en Cursor."
    Write-Info "Si no: abre Cursor → Extensiones → 'Install from VSIX' → selecciona $vsix"
}

# ===== VERIFICACIÓN FINAL =====
Write-Header "VERIFICACIÓN"
$testExe = Join-Path $BinDir "falcato.exe"
if (Test-Path $testExe) {
    $ver = & $testExe version 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-OK "falcato.exe funciona: $ver"
    } else {
        Write-Warn "falcato.exe existe pero falló: $ver"
    }
} else {
    Write-Err "falcato.exe no encontrado en $BinDir"
}

Write-Host "`n=== INSTALACIÓN COMPLETA ===" -ForegroundColor $Green
Write-Host "Próximos pasos:" -ForegroundColor $Cyan
Write-Host "  1. Abre una terminal NUEVA (para que PATH se actualice)" -ForegroundColor $Gray
Write-Host "  2. Ejecuta: falcato version" -ForegroundColor $Gray
Write-Host "  3. Prueba: falcato run ejemplos\hola_mundo.fc" -ForegroundColor $Gray
Write-Host "`nPara desinstalar: .\install.ps1 -Uninstall" -ForegroundColor $Gray