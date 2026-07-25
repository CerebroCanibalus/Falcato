# clean.ps1 — Limpia basura compilada del proyecto
# Elimina .exe, .o, .pdb sueltos que el .gitignore ya ignora

$dirs = @(
    "D:\Falcato",
    "D:\Falcato\ejemplos",
    "D:\Falcato\stdlib",
    "D:\Falcato\multi_modulo"
)

$total = 0
foreach ($dir in $dirs) {
    if (-not (Test-Path $dir)) { continue }
    $files = Get-ChildItem $dir -Include *.exe, *.o, *.pdb, *.rlib, *.d -Recurse -Force -ErrorAction SilentlyContinue
    foreach ($f in $files) {
        Remove-Item $f.FullName -Force -ErrorAction SilentlyContinue
        $total++
    }
}

Write-Host "🧹 Eliminados $total archivos de build."
