# Falcato — AGENTS.md

## Filosofía del proyecto

Falcato NO es una traducción de Rust al español. Es un lenguaje de bajo nivel
*construido desde cero* sobre **Cranelift** — apuesta estratégica, no temporal.
El sistema de tipos explota dimensiones gramaticales del español que el inglés
no tiene: género, tiempos verbales, ser/estar, subjuntivo.

### Visión estratégica

Falcato + Cranelift + WASM = **toolchain nativa para código generado por IA**.
Cranelift no es "lo que tocó" — es el backend oficial y estratégico. Bytecode Alliance
(Mozilla, Fastly, Intel, Arm, Google, Microsoft, Shopify) comparte nuestra visión.

**Velocidad de compilación > velocidad de ejecución optimizada.**

## Los 5 Pilares

| # | Pilar | Esencia | Estado |
|---|-------|---------|--------|
| I | **Género = Ownership** | `el`=owned mutable, `la`=borrowed inmutable, `un`=option | ✅ |
| II | **Ser/Estar = Const/Mut** | `es`=identidad permanente, `está`=estado temporal | ✅ |
| III | **Tiempos = Modos ejecución** | Presente=sync, Futuro=async, Subjuntivo=fallible | ✅ |
| IV | **C ABI por defecto** | Layout C, calling C, mangling off | ✅ |
| V | **Prefijos semánticos** | `re-`=retry, `des-`=free, `pre-`=comptime | 📝 Parcial |

## Day-0 (no negociable)

- **C ABI por defecto**: layout C, calling conv SystemV/C, mangling off, salida `.o`
- **Span en cada nodo AST**: `Span { inicio, fin, archivo }` — sin span no hay LSP
- **Errores en español con códigos**: `[T001] archivo.fc:7:12: mensaje` — S/T/O/C/M/I/W

## Stack técnico

- **CLI:** `clap` 4.5 | **Lexer:** `logos` 0.14 | **Parser:** descendente manual + Pratt | **AST:** propio con span
- **Semántica:** "Concordancia Lingüística" (tipos + ownership + bounds)
- **Codegen:** `cranelift-codegen` 0.112 (puro Rust)
- **LSP:** `tower-lsp` 0.20 — 6 features (diagnósticos, completado, hover, go-to-def, find-refs, signature help)
- **Target:** x86_64 Windows (msvc) | **Build:** `build.ps1`
- **Estilo:** Rust inglés en compiler, español snake_case en el lenguaje

### Patrones Cranelift (críticos)

1. **Loop header**: NUNCA sellar antes del back-edge. Sellar DESPUÉS del `jump`.
2. **Cadena if/else con 1 predecesor**: sellar inmediato es seguro.
3. **`compilar_sentencia` crea sub-bloques**: padre sellado ANTES de llamarla.
4. **SSA dominance**: valores definidos en bloque A no se usan en bloque no-dominado.
5. **`iconst` 2º arg**: siempre `i64` (`0xFFFFFFFF_u32 as i64` para INFINITE).
6. **`create_sized_stack_slot`** (no `create_stack_slot` en 0.112).
7. **`FunctionBuilderContext`**: desde `cranelift_frontend`, no `cranelift_codegen::ir`.
8. **`define_function(func_id, &mut ctx)`** — 2 args.
9. **Doble sellado** = panic. Siempre verificar flujo.
10. **Linkage::Local** para funciones con cuerpo, **Linkage::Import** solo para FFI.

### Codegen Helpers (`src/codegen_helpers.rs`)

| Helper | Propósito |
|--------|-----------|
| `BlockBuilder` | Builder de bloques seguro con anti-double-seal |
| `VariableManager` | Gestión SSA con API segura |
| `CFunctionCache` | Cache de funciones C externas |
| `MemoryHelper` | store/load/const helpers |
| `tipo_a_cranelift()` / `tamano_tipo()` | Conversión Tipo→Cranelift |

### Platform Runtime Layer (3 capas)

```
Capa A — C Runtime (lib/falcato_runtime/): operaciones multi-paso (canales, executor, threads)
  → Rust staticlib, linkeada al binario final
Capa B — PlatformRuntime trait (src/platform/): primitivas sync (mutex, sem, timestamp)
  → impl por plataforma: windows.rs, linux.rs, macos.rs
Capa C — BuiltinRegistry (src/platform/registry.rs): remapeo nombre→función C
  → sleep→Sleep/usleep, malloc→malloc, puts→puts, etc.
```

**Regla de oro:** codegen NUNCA hace `#[cfg(target_os)]`. Siempre trait dispatch o registry.

## Pipeline

```
.fc → Lexer → Parser → Analisis Semantico → Codegen (Cranelift) → .o → Linker → .exe
```

## Estructura del proyecto

```
src/
├── main.rs              # CLI (clap) — build, run, check, version, lsp, **test, setup**
├── span.rs / error.rs   # Span + Errores con códigos
├── lexer.rs             # Lexer (logos)
├── parser/              # Parser modular (mod, errores, tipos, expresiones, sentencias, declaraciones)
├── ast.rs               # AST con Span
├── semantic.rs          # Concordancia Lingüística
├── codegen/             # Cranelift (mod, funciones, sentencias, expresiones, builtins, generics, tipos)
├── codegen_helpers.rs   # BlockBuilder, VariableManager, CFunctionCache, MemoryHelper
├── platform/            # multiplataforma (mod, registry, traits, linker, windows, linux, macos)
├── futuros.rs           # Análisis async/state machine
├── resolver.rs          # Módulos e imports
├── backend.rs           # Backend trait
└── lsp.rs               # Servidor LSP

lib/falcato_runtime/     # Canal, Executor, Thread — staticlink
wix/main.wxs             # Plantilla MSI (cargo-dist)
dist-workspace.toml      # Config cargo-dist
```

## Estado del proyecto (v0.4.0)

Pipeline end-to-end operativo. Turing-completo con:
- **Core:** variables, ops, condicionales, bucles, arrays, structs, enums, generics (const+type)
- **Ownership:** `el`/`la`/`un`/`los`/`las`, mover/copiar/prestar, referencias `&T`/`&mut T`, field-level borrowing, lifetimes léxicos, regiones, self-referential `&yo`, efectos `puro`/`muta`/`lee`, branch-aware liveness, borrow checker gradual (N0→N1→N2)
- **Async:** threads reales, TCP (Winsock2), canales mpsc, thread pool, cancelación, stackless futures, `con_executor`, `seleccionar { }`
- **Built-ins:** Texto, Vector, Diccionario, Conjunto, Resultado<T,E>, bitwise, I/O polimórfico, interpolación, file I/O, matemáticas, sizeof
- **Plataforma:** runtime library (Capa A), PlatformRuntime trait (Capa B), BuiltinRegistry (Capa C) — Windows+Linux+macOS
- **LSP:** 6 features, integrado OpenCode, signature help, code actions, context-aware completion
- **Documentación:** GUIA.md + 15 capítulos, REFERENCIA.md, ERRORES.md, skill falcato-language, VS Code Extension (Falcato Dorado)
- **Instalación:** cargo-dist (MSI+shell+powershell), `falcato setup --all`, install.ps1 legacy
- **40/40 tests pasan. 50+ ejemplos.**

## Comandos CLI

```bash
falcato build <file.fc>      # Compila a binario nativo
falcato run <file.fc>        # Compila y ejecuta
falcato check <file.fc>      # Solo análisis (lexer + parser + semántica)
falcato test <file.fc>       # Ejecuta pruebas del lenguaje
falcato lsp                  # Inicia servidor LSP (stdio)
falcato setup --all          # Instala VS Code extension + agentes
falcato setup --vscode       # Solo VS Code extension
falcato setup --agents       # Solo agentes/skills OpenCode/Claude
falcato setup --uninstall    # Desinstala componentes adicionales
falcato version              # Muestra versión
```

## Roadmap — Pendiente real

### R5 — Proyecto ejemplo 500+ líneas
- [ ] Word counter: lee archivo, tokeniza, cuenta frecuencia, ordena

### R6 — Drop automático
- [ ] Análisis de CFG para insertar `free` al final de scope (Texto, Vector, Diccionario, TCP)

### 15G — Migración de codegen helpers
- [ ] Migrar `compilar_funcion()` a `BlockBuilder`
- [ ] Migrar variables de closures a `VariableManager`
- [ ] Reemplazar `llamar_malloc/free` con `MemoryHelper`

### P5c — Probar Linux (WSL)
- [ ] Abierto a colaboradores — runtime ya tiene stubs POSIX

### Calidad
- [ ] Azure Trusted Signing (~$10/mes) para eliminar falsos positivos
- [ ] Publicar en winget + Scoop
- [ ] Fix interpolación (`{var}` en strings, roto desde antes de migración)

## Estado de distribución (v0.4.0 — Alpha)

| Aspecto | Estado |
|---------|--------|
| Release build (LTO) | ✅ `falcato.exe` |
| Runtime library | ✅ `falcato_runtime.lib` linkeada estáticamente |
| CRT estático | ✅ CI con `+crt-static` |
| GitHub Actions CI | ✅ build + test + end-to-end |
| GitHub Actions Release | ✅ **cargo-dist** genera MSI + tarballs + shell/powershell installers |
| VS Code Extension | ✅ VSIX, tema Falcato Dorado |
| LSP en agentes (OpenCode) | ✅ 6 features, integrado, verificado |
| Platform Layer (Linux/macOS) | ✅ Diseñado + implementado, **no testeado** |
| Falso positivo Defender | ⚠️ Sin firma digital — requiere Azure Trusted Signing |
| `falcato setup` | ✅ VSIX + agentes/skills desde CLI |

## Curva de aprendizaje

```
Nivel 0 (permisivo, como C):    todo compila, compiler SUGIERE
Nivel 1 (verificado):           use-after-move, errores educativos A/B/C
Nivel 2 (estricto):             borrow checker completo, 1 mut XOR N inmut
```
**Para LLMs:** N0 siempre compila → compiler sugiere → LLM refina → N2 en <3 iteraciones.

## Criterio de "listo para usar"

1. Proyecto >500 líneas en varios archivos compilable
2. stdlib suficiente (I/O, strings, colecciones) — ✅ parcial
3. Manejo de errores sin `retornar 1` manual
4. Borrow checker evite fugas sin GC
5. Documentación clara para hispanohablante — ✅
6. Manipular registros hardware sin FFI manual (campos de bits) — ✅

## Criterio de "superar a Rust"

1. Linked list sin pelear con el compiler
2. LLM genera código N0 → N2 en <3 iteraciones
3. Kernel module con menos líneas que Rust
4. Errores de ownership se entienden sin leer docs
5. Self-referential structs sin workarounds
6. LLM genera bit manipulation sin alucinar máscaras (campos de bits) — ✅ parcial
7. Compiler auto-vectoriza loops `puro` sin `unsafe`

---

Para la versión ultra-compacta del agente OpenCode: `C:\Users\Lord Gatito\.config\opencode\agents\falcato.md`
