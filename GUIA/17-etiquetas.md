# 17 — Etiquetas del toolchain (CLI)

← [16: Argumentos](16-argumentos.md) | [Indice](../GUIA.md)

---

El comando `falcato` tiene subcomandos en **presente simple** y etiquetas (`--nombre`)
que controlan **cómo** se produce el binario o **cuánto** se te dice. Nada de esto
cambia la semántica del código: lo que decide qué compila es el lenguaje puro.

Los nombres en inglés (`build`, `run`, `check`, `--output`, `--release`…) existen
solo como **aliases ocultos** para scripts/CI — la interfaz visible es en español.

## Subcomandos

| Subcomando | Alias (EN) | Qué hace |
|------------|-----------|----------|
| `falcato compila` | `build`, `compilar` | Compila `.fc` a binario nativo |
| `falcato corre` | `run`, `ejecutar` | Compila y ejecuta |
| `falcato verifica` | `check`, `verificar` | Solo análisis, sin binario |
| `falcato prueba` | `test`, `probar` | Ejecuta las pruebas `prueba "..." { }` |
| `falcato instala` | `setup`, `instalar` | Instala VS Code extension, agentes, skills |
| `falcato lsp` | — | Servidor LSP (stdio) |
| `falcato version` | — | Muestra la versión |
| `falcato paquete` | — | Sistema de paquetes (R8) |

## `falcato compila`

```bash
falcato compila app.fc --salida app.exe
```

| Etiqueta | Alias (EN) | Qué hace |
|----------|-----------|----------|
| `-o, --salida <ruta>` | `output` | Ruta del binario generado |
| `--destino <triple>` | `target` | Cross-compile (`x86_64-unknown-linux-gnu`, …). Única etiqueta de plataforma |
| `--lanzar` | `release` | Modo lanzamiento: binario optimizado para entrega |
| `--emitir-clif` | `emit-clif` | Emite el CLIF de Cranelift (debuggear el codegen propio) |
| `--json` | — | Diagnósticos como JSON estructurado (agentes LLM, CI) |

**Ejemplos:**
```bash
falcato compila app.fc --salida app.exe
falcato compila app.fc --lanzar -o app_release.exe
falcato compila app.fc --destino x86_64-unknown-linux-gnu
falcato compila app.fc --json          # errores en JSON
```

## `falcato corre`

```bash
falcato corre app.fc -- --nombre sebas
```

Todo lo que va después de `--` se pasa como argumento al programa ejecutado.
Ver [Capítulo 16: Argumentos](16-argumentos.md).

## `falcato verifica`

| Etiqueta | Alias (EN) | Qué hace |
|----------|-----------|----------|
| `--json` | — | Diagnósticos como JSON estructurado |
| `--entrada` | `stdin` | Lee el código desde stdin: `echo "código" \| falcato verifica -` |
| `--incremental` | — | Cache por hash de fuente — iteración LLM write→check→fix <100ms |

```bash
falcato verifica app.fc
falcato verifica app.fc --json
echo "función principal() { retornar 0; }" | falcato verifica - --entrada
```

## `falcato instala`

| Etiqueta | Alias (EN) | Qué hace |
|----------|-----------|----------|
| `--todo` | `all` | Instala todo (VS Code + agentes) |
| `--agentes` | `agents` | Solo agentes/skills para OpenCode/Claude |
| `--vscode` | — | Solo la extensión de VS Code |
| `--desinstalar` | `uninstall` | Desinstala componentes adicionales |
| `--recursos <dir>` | `resources` | Ruta al directorio de recursos (VSIX, skills) |

```bash
falcato instala --todo
falcato instala --agentes
falcato instala --desinstalar
```

## `falcato paquete` (R8)

| Subcomando | Alias (EN) | Qué hace |
|------------|-----------|----------|
| `paquete inicia` | `init`, `iniciar` | Crea proyecto con `falcato.toml` + `falcato.lock` |
| `paquete agrega` | `add`, `agregar` | Añade una dependencia al manifiesto |
| `paquete muestra` | `mostrar` | Muestra el manifiesto y dependencias |

```bash
falcato paquete inicia --nombre mi_app
falcato paquete agrega texto_util --version ^0.1.0
falcato paquete muestra
```

## Filosofía de las etiquetas

Una etiqueta **nunca cambia la semántica** del código. Si una opción decidiera qué
compila o qué significa (ownership, const/mut, permisos, niveles de borrow checker),
no sería una etiqueta — sería lenguaje puro o directiva de módulo (`# nivel 2`).

Lo que sí puede hacer una etiqueta:

1. **Cómo se produce el binario**: `--lanzar` (optimización), `--destino` (plataforma)
2. **Cuánto se te dice**: `--json`, `--emitir-clif`, `--incremental`

`--destino` es la **única** etiqueta de plataforma: tu código `.fc` nunca sabe en qué
plataforma corre; las diferencias las absorbe el runtime. No existen
`--windows`/`--linux`/`--cfg(os)`.

---

← [16: Argumentos](16-argumentos.md) | [Indice](../GUIA.md)