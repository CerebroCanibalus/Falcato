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
| `--destino <triple>` | `target` | Plataforma destino del binario (cross-compile) |
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

### `--destino <triple>` — la única etiqueta de plataforma

`--destino` decide **para qué plataforma** se produce el binario. El valor es un
*triple de destino* (arquitectura-sistema-abi), el mismo formato que usa Rust:

| Triple | Plataforma |
|--------|-----------|
| `x86_64-pc-windows-msvc` | Windows x64 (el nativo por defecto) |
| `x86_64-unknown-linux-gnu` | Linux x64 (GNU) |
| `aarch64-apple-darwin` | macOS ARM (Apple Silicon) |
| `x86_64-apple-darwin` | macOS Intel |

**Sin `--destino`** el compilador usa la plataforma nativa (la máquina donde corre
`falcato`). Con `--destino` produces un binario para otra plataforma — **cross-compile**.

#### Por qué es la ÚNICA etiqueta de plataforma

Tu código `.fc` **nunca sabe en qué plataforma corre**. No existe `--windows`,
`--linux` ni `--cfg(os)`. Las diferencias entre plataformas (procesos, terminal,
stdin, fecha, rutas, fin de línea) las absorbe el **runtime** (Capa B/C del
compilador): un builtin como `proceso_crear` tiene su implementación Windows y su
implementación POSIX, y el runtime elige la correcta automáticamente según el
destino.

Esto significa que **el mismo código compila para todas las plataformas**:

```falcato
// Este código es idéntico para Windows, Linux y macOS:
el h: Entero64 = proceso_crear("falcato verifica app.fc");
el codigo: Entero32 = proceso_esperar(h);
```

```bash
falcato compila app.fc --destino x86_64-pc-windows-msvc     # para Windows
falcato compila app.fc --destino x86_64-unknown-linux-gnu   # para Linux
falcato compila app.fc --destino aarch64-apple-darwin       # para macOS
```

**Si un builtin no tiene implementación para el destino pedido**, es un **error de
compilación** (con builtin + plataforma en el mensaje) — nunca un warning ni un
crash en tiempo de ejecución. Así el código portable se garantiza en compilación.

### Requisitos para cross-compile

Compilar para otra plataforma requiere un **enlazador** para esa plataforma y, en
algunos casos, la **raíz del sistema** (librerías del destino). Estas etiquetas
están en el roadmap (R8):

| Etiqueta (futura) | Alias (EN) | Para qué |
|-------------------|-----------|----------|
| `--enlazador <path>` | `linker` | Ruta al enlazador (lld, gcc, link.exe) |
| `--raiz-sistema <dir>` | `sysroot` | Raíz del sistema destino (librerías C) |
| `--crt-estatico` / `--crt-dinamico` | `crt-static` / `crt-dynamic` | CRT estático (default) vs DLLs del sistema |

> **Estado actual:** el cross-compile está diseñado en el roadmap (R8). Hoy el
> compilador produce binarios nativos; las etiquetas de enlazador/sysroot se
> activarán con el sistema de paquetes.

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

## Etiquetas futuras (roadmap)

Estas etiquetas están **diseñadas** (en AGENTS.md) pero aún no implementadas.
Cuando se activen, seguirán la misma filosofía: solo deciden **cómo** se produce el
binario o **cuánto** se te dice — nunca la semántica del código.

| Etiqueta | Alias (EN) | Para qué | Estado |
|----------|-----------|----------|--------|
| `--nivel-opt N` | `opt-level` | Nivel de optimización global (con `--lanzar`) | 🔴 R8 |
| `--enlazador <path>` | `linker` | Enlazador para cross-compile (lld, gcc, link.exe) | 🔴 R8 |
| `--raiz-sistema <dir>` | `sysroot` | Raíz del sistema destino (librerías C) | 🔴 R8 |
| `--crt-estatico` / `--crt-dinamico` | `crt-static` / `crt-dynamic` | CRT estático (default) vs DLLs del sistema | 🔴 R8 |
| `--edicion` | `edition` | Versionar la sintaxis del lenguaje | 🟢 Baja |
| `--detallado` | `verbose` | Más información en la salida | 🟢 Baja |
| `-g` | — | Información de depuración | 🟢 Baja |
| `-j N` | — | Paralelismo de compilación | 🟢 Baja |

**Qué NO existirá jamás** (semántica → lenguaje puro):

| Cosa | Hogar correcto |
|------|----------------|
| `--windows` / `--linux` / `--cfg(os)` | Nunca — el runtime absorbe la diferencia |
| `--deny-warnings` | El nivel de módulo (`# nivel 2`) ES el `-Werror` |
| Nivel N0/N1/N2 como etiqueta | Directiva por módulo (`# nivel 2`) |
| Bypass de permisos | Efectos `puro`/`muta`/`lee` + `falcato.toml` |

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