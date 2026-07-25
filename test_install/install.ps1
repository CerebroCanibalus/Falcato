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

# Configuracion
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
function Write-OK { param($msg) Write-Host "  [OK] $msg" -ForegroundColor $Green }
function Write-Warn { param($msg) Write-Host "  [!] $msg" -ForegroundColor $Yellow }
function Write-Err { param($msg) Write-Host "  [ERR] $msg" -ForegroundColor $Red }
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
    Write-Info "Ya esta en PATH: $PathToAdd"
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
            Write-OK "Extension VS Code desinstalada: $ext"
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
    Write-Host "`n[OK] Desinstalacion completa. Reinicia la terminal." -ForegroundColor $Green
    exit 0
}

# ===== VERIFICACIONES INICIALES =====
Write-Header "INSTALADOR FALCATO v0.2.0"
Write-Info "Directorio de instalacion: $InstallDir"

if (-not (Test-Path $FalcaoExe)) {
    Write-Err "No se encuentra falcato.exe en $FalcaoExe"
    Write-Err "Ejecuta este script desde la carpeta extraida del ZIP (donde esta la carpeta 'bin')"
    exit 1
}

# ===== MENU INTERACTIVO =====
if (-not $Quiet) {
    Write-Header "COMPONENTES A INSTALAR"
    $installPath = -not $NoPath
    $installVSCode = -not $NoVSCode
    $installOpenCode = -not $NoOpenCode
    $installClaude = -not $NoClaude
    $installCursor = -not $NoCursor

    Write-Host "`nSelecciona que instalar (Enter = si por defecto):" -ForegroundColor $Cyan
    
    $installPath = Confirm-Action "falcato.exe en PATH (REQUERIDO)" -Default $true
    if (-not $installPath) { Write-Err "PATH es obligatorio para usar falcato"; exit 1 }
    
    $installVSCode = Confirm-Action "Extension VS Code (syntax + LSP + tema)" -Default $true
    $installOpenCode = Confirm-Action "Agent + Skill para OpenCode" -Default $true
    $installClaude = Confirm-Action "Agent + Skill para Claude Code" -Default $true
    $installCursor = Confirm-Action "Cursor (usa extension VS Code)" -Default $true
}

# ===== CREAR DIRECTORIOS =====
Write-Header "CREANDO ESTRUCTURA"
New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
New-Item -ItemType Directory -Force -Path $ExamplesDir | Out-Null
Write-OK "Directorios creados en $InstallDir"

# ===== COPIAR BINARIO =====
Write-Header "INSTALANDO FALCATO.EXE"
Copy-Item -Path $FalcaoExe -Destination $BinDir -Force
Write-OK "falcato.exe -> $BinDir"

# Copiar ejemplos
$examplesSrc = Join-Path $ScriptDir "ejemplos"
if (Test-Path $examplesSrc) {
    Copy-Item -Path (Join-Path $examplesSrc "*.fc") -Destination $ExamplesDir -Force
    Write-OK "Ejemplos copiados a $ExamplesDir"
}

# ===== PATH (OBLIGATORIO) =====
if ($installPath) {
    Write-Header "CONFIGURANDO PATH"
    Add-ToUserPath $BinDir
}

# ===== VS CODE EXTENSION =====
if ($installVSCode) {
    Write-Header "INSTALANDO EXTENSION VS CODE"
    $vsix = Get-Item $VSIXPath -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($vsix -and (Get-Command code -ErrorAction SilentlyContinue)) {
        try {
            code --install-extension $vsix.FullName --force
            Write-OK "Extension instalada: $($vsix.Name)"
            Write-Info "Tema: Ctrl+K Ctrl+T -> 'Falcato Dorado'"
        } catch {
            Write-Warn "No se pudo instalar automaticamente: $($_.Exception.Message)"
            Write-Info "Instala manualmente: code --install-extension $($vsix.FullName) --force"
        }
    } elseif (-not (Get-Command code -ErrorAction SilentlyContinue)) {
        Write-Warn "VS Code no encontrado en PATH (comando 'code')"
        Write-Info "Instala VS Code y ejecuta: code --install-extension <ruta.vsix>"
    } else {
        Write-Warn "No se encontro .vsix en $VSIXPath"
    }
}

# ===== OPENCODE =====
if ($installOpenCode) {
    Write-Header "CONFIGURANDO OPENCODE"
    $ocAgentDir = "$env:APPDATA\opencode\agents"
    $ocSkillDir = "$env:APPDATA\opencode\skills"
    New-Item -ItemType Directory -Force -Path $ocAgentDir | Out-Null
    New-Item -ItemType Directory -Force -Path $ocSkillDir | Out-Null
    
    if (Test-Path $AgentPath) {
        Copy-Item $AgentPath (Join-Path $ocAgentDir "falcato.md") -Force
        Write-OK "Agent copiado a $ocAgentDir"
    } else { Write-Warn "Agent no encontrado en $AgentPath" }
    
    if (Test-Path $SkillsDir) {
        $dest = Join-Path $ocSkillDir "falcato-language"
        if (Test-Path $dest) { Remove-Item $dest -Recurse -Force }
        Copy-Item $SkillsDir $dest -Recurse -Force
        Write-OK "Skill copiada a $ocSkillDir"
    } else { Write-Warn "Skill no encontrada en $SkillsDir" }
}

# ===== CLAUDE CODE =====
if ($installClaude) {
    Write-Header "CONFIGURANDO CLAUDE CODE"
    $ccAgentDir = "$env:USERPROFILE\.claude\agents"
    $ccSkillDir = "$env:USERPROFILE\.claude\skills"
    New-Item -ItemType Directory -Force -Path $ccAgentDir | Out-Null
    New-Item -ItemType Directory -Force -Path $ccSkillDir | Out-Null
    
    if (Test-Path $AgentPath) {
        Copy-Item $AgentPath (Join-Path $ccAgentDir "falcato.md") -Force
        Write-OK "Agent copiado a $ccAgentDir"
    } else { Write-Warn "Agent no encontrado en $AgentPath" }
    
    if (Test-Path $SkillsDir) {
        $dest = Join-Path $ccSkillDir "falcato-language"
        if (Test-Path $dest) { Remove-Item $dest -Recurse -Force }
        Copy-Item $SkillsDir $dest -Recurse -Force
        Write-OK "Skill copiada a $ccSkillDir"
    } else { Write-Warn "Skill no encontrada en $SkillsDir" }
}

# ===== CURSOR =====
if ($installCursor) {
    Write-Header "CURSOR"
    Write-Info "Cursor usa la misma extension que VS Code."
    if ($installVSCode) {
        Write-OK "Extension VS Code instalada -> Cursor la detectara automaticamente"
    } else {
        Write-Warn "Instala la extension VS Code primero (opcion anterior)"
    }
}

# ===== RESUMEN =====
Write-Header "INSTALACION COMPLETA"
Write-Host "`n[OK] falcato.exe instalado en: $BinDir" -ForegroundColor $Green
Write-Host "[OK] Ejemplos en: $ExamplesDir" -ForegroundColor $Green
if ($installPath) { Write-Host "[OK] PATH actualizado (reinicia terminal)" -ForegroundColor $Green }
if ($installVSCode) { Write-Host "[OK] Extension VS Code instalada" -ForegroundColor $Green }
if ($installOpenCode) { Write-Host "[OK] OpenCode agent + skill configurados" -ForegroundColor $Green }
if ($installClaude) { Write-Host "[OK] Claude Code agent + skill configurados" -ForegroundColor $Green }
if ($installCursor) { Write-Host "[OK] Cursor listo (usa extension VS Code)" -ForegroundColor $Green }

Write-Host "`nPROXIMOS PASOS:" -ForegroundColor $Cyan
Write-Host "  1. Abre una terminal NUEVA" -ForegroundColor $Gray
Write-Host "  2. Ejecuta: falcato version" -ForegroundColor $Gray
Write-Host "  3. Prueba: falcato run ejemplos\hola_mundo.fc" -ForegroundColor $Gray
Write-Host "  4. Abre un .fc en VS Code -> LSP activo" -ForegroundColor $Gray
Write-Host "  5. Tema: Ctrl+K Ctrl+T -> 'Falcato Dorado'" -ForegroundColor $Gray

Write-Host "`nPara desinstalar: .\install.ps1 -Uninstall" -ForegroundColor $Gray