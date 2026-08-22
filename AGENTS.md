# Falcato — AGENTS.md

## Filosofía
Lenguaje de bajo nivel *construido desde cero* sobre **Cranelift** (apuesta estratégica, no temporal). NO es traducción de Rust. Es **gramatipado** (la gramática española es el sistema de tipos) y **morfosemántico** (la morfología porta significado de máquina): género, tiempos verbales, ser/estar, subjuntivo.

**Visión:** Falcato + Cranelift + WASM = toolchain nativa para código generado por IA. **Velocidad de compilación > velocidad de ejecución optimizada.**

## Los 5 Pilares
| # | Pilar | Esencia | Estado |
|---|-------|---------|--------|
| I | Género = Ownership | `el`=owned mut, `la`=borrowed inmut, `un`=option | ✅ |
| II | Ser/Estar = Const/Mut | `es`=permanente, `está`=temporal | ✅ |
| III | Tiempos = Modos | Presente=sync, Futuro=async, Subjuntivo=fallible | ✅ |
| IV | C ABI por defecto | Layout C, calling C, mangling off | ✅ |
| V | ~~Prefijos semánticos~~ | ~~`re-`=retry~~ | ⛔ Retirado 2026-08-03 |

## Day-0 (no negociable)
- **🚨 TODO EN ESPAÑOL**: lenguaje, errores, CLI, docs. Excepciones: términos técnicos sin traducción (Cranelift, CLIF, JSON, LSP, WASM).
- **C ABI por defecto**: layout C, SystemV, mangling off, salida `.o`
- **Span en cada nodo AST** — sin span no hay LSP
- **Errores en español con códigos** `[T001] archivo.fc:7:12: mensaje` — S/T/O/C/M/I/W
- **Documentar al agente**: cambios grandes → `falcato.md` + skill `falcato-language` en la misma tanda
- **🚨 SEGURIDAD CRÍTICA**: red/sistema/entrada externa → revisión minuciosa antes de mergear
- **NINGUNA ETIQUETA CAMBIA SEMÁNTICA** — etiqueta solo decide CÓMO se produce el binario
- **`--destino` es la ÚNICA etiqueta de plataforma** — el `.fc` nunca sabe dónde corre
- **Código portable o no compila**: builtin sin impl para target = error
- **Impls juntas**: Windows + POSIX en la misma tanda
- **VERSIONADO**: `MAYOR.menor.parche` — Bump en `Cargo.toml` + tag `vMAYOR.menor.parche`
- **RELEASES EN ESPAÑOL** y **NOVEDADES POR EFECTO** (➕/🔧/🔁, no por fase)

## Stack técnico
- **CLI:** `clap` 4.5 | **Lexer:** `logos` 0.14 | **Parser:** descendente manual + Pratt | **AST:** propio con span
- **Semántica:** Concordancia Lingüística | **Codegen:** `cranelift-codegen` 0.112 | **LSP:** `tower-lsp` 0.20
- **Target:** x86_64 Windows (msvc) | **Build:** `build.ps1` | **Estilo:** Rust inglés en compiler, español snake_case en lenguaje

### Patrones Cranelift (críticos)
1. Loop header: NUNCA sellar antes del back-edge. Sellar DESPUÉS del `jump`.
2. Cadena if/else con 1 predecesor: sellar inmediato es seguro.
3. `compilar_sentencia` crea sub-bloques: padre sellado ANTES de llamarla.
4. SSA dominance: valores de bloque A no se usan en bloque no-dominado.
5. `iconst` 2º arg: siempre `i64`.
6. `create_sized_stack_slot` (no `create_stack_slot` en 0.112).
7. `FunctionBuilderContext`: desde `cranelift_frontend`.
8. `define_function(func_id, &mut ctx)` — 2 args.
9. Doble sellado = panic.
10. `Linkage::Local` para funciones con cuerpo, `Import` solo para FFI.

### Platform Runtime Layer (3 capas)
```
Capa A — C Runtime (lib/falcato_runtime/): ops multi-paso → staticlib
Capa B — PlatformRuntime trait (src/platform/): primitivas sync → windows.rs/linux.rs/macos.rs
Capa C — BuiltinRegistry (src/platform/registry.rs): remapeo nombre→función C
```
**Regla de oro:** codegen NUNCA hace `#[cfg(target_os)]`.

## Pipeline
```
.fc → Lexer → Parser → Analisis Semantico → Codegen (Cranelift) → .o → Linker → .exe
```

## Estructura del proyecto
```
src/
├── main.rs, span.rs, error.rs, lexer.rs, parser/, ast.rs
├── semantic/ (mod, tipos, ownership, funciones, sentencias)
├── codegen/ (mod, funciones, sentencias, expresiones, generics, tipos) + builtins/
├── platform/ (mod, registry, traits, linker, windows, linux, macos)
├── futuros.rs, resolver.rs, backend.rs
└── lsp/ (mod, indice, completar, diagnostico, hover, referencias)
lib/falcato_runtime/   # Canal, Executor, Thread — staticlink
```

## Estado del proyecto (v0.7.5)
Pipeline end-to-end operativo. Turing-completo:
- **Core:** variables, ops, condicionales, bucles, arrays, structs, enums, generics, apodos
- **Ownership:** `el`/`la`/`un`/`los`/`las`, `&T`/`&mut T`, field-level borrowing, lifetimes, `puro`/`muta`/`lee`
- **Async:** threads, TCP, canales mpsc, thread pool, `seleccionar { }`
- **Built-ins:** Texto, Vector, Diccionario, Conjunto, Resultado, Option, bitwise, I/O, file I/O, math, trig, sizeof
- **Aritmética consciente:** `a + b fuese` (checked), `un x = a + b` (Option), `romper`/`continuar`
- **Two-pass:** forward refs + shadowing T031-T035
- **v0.7.5 (2026-08-21):** coerción polimórfica `42`→`Entero64`/`Natural`, azúcar `42 largo`/`natural largo`/`corto`, profiling `reloj_mono_ns`+`perfil_*`, cross-file `verifica a.fc b.fc`, fixes F-008/F-009/F-013/F-014/F-016/F-017
- **54/54 tests + 19 unitest. 76/83 ejemplos compilan** (7 intencionales)

## Tipos naturales
| Categoría | Nombre | Equiv. |
|-----------|--------|--------|
| Entero | `Entero` | `Entero32` |
|  | `EnteroLargo` | `Entero64` |
|  | `EnteroCorto` | `Entero16` |
|  | `EnteroMínimo` | `Entero8` |
| Natural | `Natural` | `Natural32` |
| Real | `Real` | `Flotante64` |
| Lógico | `Lógico` | `Booleano` |
```falcato
apodo ID = Entero64;
el id: ID = 1234567890123;
```

## Comandos CLI
```bash
falcato compila <file.fc> --salida out.exe
falcato corre <file.fc>
falcato verifica <file.fc>        # multi-archivo: verifica a.fc b.fc
falcato prueba <file.fc>
falcato lsp
falcato instala --todo
```

## Etiquetas del toolchain
**Regla:** etiqueta solo decide CÓMO se produce el binario, no qué significa.

| Etiqueta | Alias | Para qué | Estado |
|----------|-------|----------|--------|
| `--emitir-clif` | `emit-clif` | CLIF debug | ✅ |
| `--json` | — | Diagnósticos JSON | ✅ |
| `--incremental` | — | Cache verificación <100ms | ✅ |
| `--entrada` | `stdin` | `echo "code" \| verifica -` | ✅ |
| `--destino <triple>` | `target` | Cross-compile — **única etiqueta plataforma** | 🟡 R8 |
| `--lanzar` | `release` | Optimización global | 🔴 R8 |
| `-o, --salida` / `--detallado` / `-g` | `output`/`verbose` | Output/verbosidad | ✅ |

**Prohibidas:** N0/N1/N2, `--windows`, `--cfg(os)`, `--compat-*` (todo es lenguaje o `falcato.toml`)

## Roadmap — Pendiente real

### R7.7 — Aritmética consciente (pendiente)
- [ ] **F3 — Rangos** (`entre ... y ...`): verificación compile-time + runtime checks
- [ ] **F4 — Unidades** (`por`/`entre`): dimensiones en tipos
- [ ] **F5 — Decimal nativo**: `0.1 + 0.2 = 0.3` exacto

### R9 — DAW + Cid
- [ ] **R9.1.0 — Librería JSON mínima** (`librerias/json.fc`) — parser para MCP
- [ ] **R9.2.0 — Cid core**: loop agente + contexto + MCP stdio
- [ ] **R9.3.0 — DAW**: WAV, buffers, mezcla, secuenciador, efectos

### R8 — Sistema de paquetes P2P
- Formato `falcato.toml` + `falcato.lock`, `falcato paquete add/publicar/buscar`
- Cliente torrent + DHT (BEP44), 8 capas de seguridad (hash, solo fuente, ed25519, permisos por tipos, WoT, blocklist, transparency log)
- **Pendientes seguridad R8S.1-7** (ver AGENTS previo): SET sin firma, DoS memoria/CPU, buffer slicing, `dht_consultar` len, `proceso_crear` inyección, auth peers — cerrar antes de exponer a red real

### Calidad y deuda
- [ ] Azure Trusted Signing (falso positivo Defender) + winget/Scoop
- [ ] `cargo fix` warnings (100 en release), reducir panics <2/1000 LOC

### 📋 Mejoras 0.7.6 (reporte 2026-08-21)
> **Bugs confirmados para 0.7.6 (no bloqueantes Cid, pero reporte pendiente):**
- **F-019** `archivo_listar("literal")` — coerción Palabra→Texto con descriptor fantasma en rutas largas. Fix: misma `strlen+malloc+memcpy` que otros builtins + free temporal.
- **F-020** `"{f(x)}"` interpolación con llamada → vacío silencioso. Fix: diagnóstico `[S080] interpolación con llamada no soportada, usa variable temporal` (implementación real en 0.8.0 si se pide).
- **F-021** `a > 0` infiere `Entero32` no `Booleano` → `T011`. Fix: `Binaria` con `> < >= <= == !=` siempre `Booleano` en `codegen/tipos.rs` y `semantic/tipos.rs`.

> **Sugerencias de lenguaje (defer a RFC 0.8.0, no a 0.7.6):**
> Renombres con breaking change → necesitan `MAYOR` + aliases. No tocar en 0.7.6.
> - `apodo` ↔ `alias` (RAE vs pureza), `rasgo` ↔ `protocolo`, `vector` ↔ `lista`, `Booleano`/`Lógico` unificar, `Entero` default 32→64 (rompe ABI), `retornar`→`devolver` canónico, `romper`→`salir`.
> - Mensajes: traducir `Borrow`→`préstamo`, `Verifier`→`[C]` español, `T001 disconcordancia`→`no coincide`.
> - Artículos `el/la/un` opacos → documentar tabla + sugerencias clippy.
> - Azúcares `texto_desde→de`, `como_entero64→.entero64` → esperar 3 proyectos reales.

### Gotchas operativos (no bugs)
- **Exe zombie:** `build.bat` falla pero `.exe` viejo sigue. Verificar siempre `target/release/falcato.exe` timestamp.
- **PowerShell .ps1 sin BOM → ANSI corrompe acentos:** usar `write` con UTF-8 + anclas ASCII, nunca `Set-Content` directo en `.fc`.
- **LSP cross-file:** `verifica a.fc b.fc` sí es cross-file, el LSP solo ve 1 archivo → ansiedad normal, no bug.

## Criterio de "listo para usar"
1. Proyecto >500 líneas multi-archivo compilable
2. stdlib suficiente (I/O, strings, colecciones)
3. Errores sin `retornar 1` manual
4. Borrow checker sin GC
5. Docs claras para hispanohablante
6. Registros hardware sin FFI manual (bitfields)

## Auditoría
Ver `ESTADO.md` para criterios McCabe/SonarQube/Sebesta/HumanEval. Semáforo 🟢/🟡/🔴.
