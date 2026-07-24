# CLI del Compilador

El compilador obedece a cinco mandatos. No son los Diez, pero
bastan para gobernar la máquina.

## Comandos

### `falcato build <archivo>`

Toma un archivo `.fc` y pare un `.exe`. Así, en seco:

```bash
falcato build ejemplo.fc
# → ejemplo.exe

falcato build ejemplo.fc -o salida.exe
# → salida.exe
```

Banderas que se le pueden colgar:

| Flag | Descripción |
|------|-------------|
| `-o, --output <ruta>` | Ruta del binario de salida |
| `--target <triple>` | Target triple (por defecto: el nativo) |
| `--release` | Optimizaciones de release |
| `--emit-ir` | Mostrar LLVM IR (reservado, no hace nada aún) |

### `falcato run <archivo>`

Compila y ejecuta en un solo envite:

```bash
falcato run ejemplo.fc
falcato run ejemplo.fc -- arg1 arg2  # con argumentos para el programa
```

### `falcato check <archivo>`

Analiza sin generar binario. Útil para ver si el código está en
orden sin tener que compilar del todo:

```bash
falcato check ejemplo.fc
```

### `falcato lsp`

Inicia el servidor LSP que habla por stdio:

```bash
falcato lsp
```

### `falcato version`

Muestra la versión del compilador, por si a alguien le importa:

```bash
falcato version
# → Falcato 0.1.0
```

## Scripts de build

| Script | Propósito |
|--------|-----------|
| `build.bat` | Build con cargo, a la antigua |
| `build.ps1` | Build con PowerShell (detecta Visual Studio solo) |
| `build_release.bat` | Build para soltar a producción |
