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
| V | ~~Prefijos semánticos~~ | ~~`re-`=retry, `des-`=free, `pre-`=comptime~~ | ⛔ Retirado (2026-08-03) — riesgo de colisión con `retornar`/`prestar`/`desde`; `des-` cubierto por R6 (drop automático) |

## Day-0 (no negociable)

- **C ABI por defecto**: layout C, calling conv SystemV/C, mangling off, salida `.o`
- **Span en cada nodo AST**: `Span { inicio, fin, archivo }` — sin span no hay LSP
- **Errores en español con códigos**: `[T001] archivo.fc:7:12: mensaje` — S/T/O/C/M/I/W
- **Documentar al agente**: las adiciones grandes de funcionalidad y los cambios de
  workflow importantes SE DOCUMENTAN en `C:\Users\Lord Gatito\.config\opencode\agents\falcato.md`
  (y en la skill `falcato-language` si cambia sintaxis/builtins). Un builtin nuevo, un
  subcomando nuevo del CLI, una primitiva nueva del runtime → actualizar el agente en la
  misma tanda, no "para después".
- **🚨 SEGURIDAD CRÍTICA — el usuario NUNCA queda expuesto**: cualquier pieza que toque
  la red (DHT, TCP, MCP, HTTP), el sistema (procesos, archivos, terminal) o entrada
  externa (parsing de datos no confiables) se somete a **revisión de seguridad minuciosa
  ANTES de mergear**. Prohibido dejar "exploits estúpidos": sin validar longitud de
  buffers, sin verificar firmas antes de confiar, sin inyección de comandos, sin DoS por
  memoria/CPU. Cada builtin nuevo con superficie externa lleva su análisis de vectores
  de ataque en la descripción del commit. Si un PR toca red/sistema sin nota de
  seguridad, NO se mergea.

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

## Estado del proyecto (v0.3.0)

Pipeline end-to-end operativo. Turing-completo con:
- **Core:** variables, ops, condicionales, bucles, arrays, structs, enums, generics (const+type)
- **Ownership:** `el`/`la`/`un`/`los`/`las`, mover/copiar/prestar, referencias `&T`/`&mut T`, field-level borrowing, lifetimes léxicos, regiones, self-referential `&yo`, efectos `puro`/`muta`/`lee`, branch-aware liveness, borrow checker gradual (N0→N1→N2)
- **Async:** threads reales, TCP (Winsock2), canales mpsc, thread pool, cancelación, stackless futures, `con_executor`, `seleccionar { }`
- **Built-ins:** Texto, Vector, Diccionario, Conjunto, Resultado<T,E>, bitwise, I/O polimórfico, interpolación, file I/O, matemáticas, sizeof
- **Plataforma:** runtime library (Capa A), PlatformRuntime trait (Capa B), BuiltinRegistry (Capa C) — Windows+Linux+macOS
- **LSP:** 6 features, integrado OpenCode, signature help, code actions, context-aware completion
- **Documentación:** GUIA.md + 15 capítulos, REFERENCIA.md, ERRORES.md, skill falcato-language, VS Code Extension (Falcato Dorado)
- **Instalación:** cargo-dist (MSI+shell+powershell), `falcato setup --all`, install.ps1 legacy
- **47/47 tests pasan. 66/73 ejemplos compilan** (7 restantes son errores intencionales de demostración).

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

> ~~R5 — Proyecto ejemplo 500+ líneas~~ **SALTADO (2026-08-03).** Sustituido por R7: el primer gran
> proyecto de Falcato es el CLI **Cid** (agente estilo OpenCode escrito en Falcato, `D:\Cid`).
> Las primitivas nativas que exige el CLI son el dogfooding real — más valiosas que un word counter.

### R6 — Drop automático
- [ ] Análisis de CFG para insertar `free` al final de scope (Texto, Vector, Diccionario, TCP)
- [ ] **Requisito para:** Fase 2 de Cid (loop agente con JSON trees por doquier)

### R7 — Primitivas nativas para el CLI (Cid)
El CLI `cid` necesita tocar el sistema. Estas son las piezas **NATIVAS** (Capa A/B/C) —
todo lo demás (JSON, HTTP, SSE, MCP) son librerías `.fc` y NO tocan el compiler.

- [ ] **R7.1 — Spawn procesos + pipes** (CreateProcess/CreatePipe/WaitForSingleObject)
  - [ ] Runtime C: `proceso_crear(comando) -> Handle`, `proceso_esperar(p) -> código`, `proceso_leer_salida(p) -> Texto` (stdout+stderr capturados)
  - [ ] Registry: remapeo de los 3 nombres
  - [ ] Codegen: builtins `proceso_*` con Span
  - [ ] **Criterio:** ejecutar `falcato check` desde un programa Falcato y capturar su salida
  - [ ] *Bloqueante:* sin esto no hay `cid run/build/test`, ni git, ni MCPs stdio
- [ ] **R7.2 — Terminal raw mode + ANSI**
  - [ ] PlatformRuntime: `terminal_modo_raw(activo)`, `terminal_leer_tecla() -> Entero32`
  - [ ] Windows: `SetConsoleMode(ENABLE_VIRTUAL_TERMINAL_PROCESSING)`, `ReadConsoleInput`
  - [ ] Codegen: builtins `terminal_*` con Span
  - [ ] **Criterio:** leer teclas sin Enter y detectar flechas; colores ANSI activados
  - [ ] *Bloqueante:* sin esto no hay TUI de Cid (Pilar IV)
- [ ] **R7.3 — Entrada estándar (stdin)**
  - [ ] Registry: `entrada_leer() -> Texto` (ReadFile sobre STD_INPUT_HANDLE)
  - [ ] **Criterio:** `echo hola | cid` lee el texto
  - [ ] *Bloqueante:* los MCPs y el LSP client hablan por stdio (JSON-RPC)
- [ ] **R7.4 — Date/time formato** (timestamp ya existe; falta formatear)
  - [ ] Librería `.fc` sobre el builtin `timestamp` (strftime manual o FFI)
  - [ ] **Criterio:** sesiones y logs con fecha legible
  - [ ] *No bloqueante:* el timestamp crudo ya sirve para ordenar

### R8 — Sistema de paquetes distribuido (P2P, sin servidores)
Ecosistema estilo crates.io pero **sin registry central**: contenido por hash, índice en la
**DHT de BitTorrent** (BEP44 mutable items firmados), transporte por torrent con seeding.
Casi todo vive en el compiler Rust (como cargo); solo la distribución entra en Capa A.
**Espíritu:** esfuerzo comunitario — la confianza, las denuncias y el contenido los crean los
pares (comunismo absoluto); semilla de confianza cero, nadie por defecto. Patrón validado por
IPFS (Benet 2014): Kademlia + BitTorrent + Git, tamper-resistance por construcción.

- [ ] **R8.1 — Formato y CLI**
  - [ ] `falcato.toml` (nombre, versión, deps, permisos) + `falcato.lock` (árbol resuelto + hashes)
  - [ ] Comandos: `falcato paquete add/publicar/buscar/actualizar`
  - [ ] Resolver semver + integración de imports desde paquetes en `resolver.rs`
  - [ ] **Anti-confusión (ConfuGuard-lite, arXiv 2025):** al resolver, alertar si dos paquetes tienen nombres similares (`texto_util` vs `texto-util`), o si una dependencia transitiva es reciente con 0 avales → confirmación manual
- [ ] **R8.2 — Capa A: cliente torrent + DHT**
  - [ ] `falcato_torrent_descargar(hash, dir)` / `falcato_torrent_publicar(dir) -> hash`
  - [ ] DHT: publicar/consultar `paquete:<nombre>` → versión + hash (BEP44)
  - [ ] Seeding configurable tras descarga
  - [ ] **Anti-eclipse (Inria 2011):** consultas replicadas a múltiples peers; el valor firmado hace que una respuesta falsa falle verificación → el peor daño posible es DoS, no compromiso. La DHT es **directorio, nunca fuente de confianza**
- [ ] **R8.3 — Seguridad (7 capas, cero mantenimiento)**
  - [ ] **Capa 1 — Integridad:** hash blake3 obligatorio (lo da BitTorrent gratis)
  - [ ] **Capa 2 — Solo fuente:** paquetes = código `.fc`, NUNCA binarios; **sin build scripts** (mata el 80% del malware tipo npm/cargo)
  - [ ] **Capa 3 — Autenticidad:** firma ed25519; **obligatoria en producción, opcional en modo "auditar"** (dev puede instalar sin firma para probar)
  - [ ] **Capa 4 — Tipos como permisos (INNOVACIÓN, capability-based):** permisos declarados `red/archivos/procesos/terminal` en el manifiesto; el compiler verifica por los efectos `puro/muta/lee` que el código no exceda lo declarado → falla la compilación. Enforcement en compile-time, sin sandbox. Más fuerte que Miller ("Capability Myths Demolished"): si el código no tiene la capacidad en su tipo, no compila — no hay bypass
  - [ ] **Capa 5 — WoT distribuida:** **semilla de confianza cero** (nadie por defecto); TOFU al primer contacto; avales entre editores; la confianza fluye entre pares
  - [ ] **Capa 6 — Blocklist comunal:** denuncias firmadas en DHT (`denuncia:<hash>` → razón); todos consultan antes de instalar
  - [ ] **Capa 7 — Transparency log en DHT:** publicaciones BEP44 firmadas e inmutables; historial público auditable; un editor comprometido no borra su rastro
  - [ ] **Capa 8 — Builds reproducibles (v2):** mismo fuente → mismo hash; toda la red es auditor — cualquiera compila y compara
  - [ ] *Permisos = buckets sencillos e intuitivos (red, archivos, procesos, terminal)*
- [ ] **Criterio:** `falcato paquete add <lib>` descarga de la red P2P, verifica hash + firma, valida permisos, compila contra ella
- [ ] *Depende de:* principalmente compiler Rust; Capa A para torrent/DHT

### 🔴 REVISIÓN DE SEGURIDAD — PENDIENTES CRÍTICOS (auditoría 2026-08-03)
> Regla Day-0: NUNCA mergear red/sistema sin nota de seguridad. Vectores detectados
> en el DHT actual (R8.2, `lib/falcato_runtime/src/dht.rs`) que DEBEN cerrarse antes
> de exponer el runtime a redes no confiables:

- [ ] **R8S.1 — SET sin verificación de firma**: `procesar_mensaje` tipo 2 acepta
  cualquier datagrama y lo inserta en el mapa local con `clave_publica=[0;32]` y
  `firma=[0;64]` (falsas). **Un atacante envenena el caché local con items falsos.**
  Fix: verificar firma ed25519 contra `clave_publica` ANTES de insertar; descartar
  si no verifica. (La Capa 3 de R8.3 es la mitigación definitiva — hacerla ya.)
- [ ] **R8S.2 — DoS por memoria**: SET sin límite de tamaño ni de cantidad — un peer
  puede mandar millones de items gigantes y llenar la RAM. Fix: límite de tamaño por
  item (ej. 1 MB), límite de items totales, evict LRU.
- [ ] **R8S.3 — DoS por CPU**: el hilo de escucha no tiene rate-limit; datagramas
  continuos queman CPU. Fix: budget por peer (token bucket), dormir adaptativo.
- [ ] **R8S.4 — Buffer slicing frágil**: `&data[1..1]` (clave de 0 bytes) y slicing
  por offsets fijos pueden dar pánico con mensajes malformados (necesita validación
  de longitudes antes de cada slice). Fix: parseo con checks, nunca indexar sin
  verificar.
- [ ] **R8S.5 — `dht_consultar` devuelve puntero sin longitud**: el caller hace
  strlen — con valores binarios que contienen `\0` el resultado se trunca. Fix:
  devolver (ptr, len) o codificar en el payload.
- [ ] **R8S.6 — `proceso_crear` usa `cmd.exe /C "{}"` sin sanitizar**: si el comando
  proviene de input del usuario (Cid), hay inyección de comandos. Fix: validar el
  comando antes de pasar a la shell, o API de spawn directo (CreateProcessW sin
  shell) con args separados.
- [ ] **R8S.7 — Sin autenticación de peers**: bootstrap acepta cualquier "peer".
  Los peers solo deben ser fuente de *direcciones*, NUNCA de datos confiables
  (ya es así por diseño — mantener: la DHT es directorio, la firma es la verdad).

> **Criterio de cierre:** antes de cualquier release que exponga el runtime a la red
> real (DHT pública, MCP, HTTP), TODOS los R8S.* deben estar verificados. El commit
> que los cierre debe listar el vector, la causa raíz y el fix en la descripción.

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

## Estado de distribución (v0.3.0 — Alpha)

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

## Auditoría (criterios de calificación)

Auditoría completa en cada release mayor (v0.5.0, v0.6.0…). Semáforo:
🟢 cumple · 🟡 parcial · 🔴 no cumple. Umbrales basados en McCabe, SonarQube,
Sebesta, HumanEval.

### A. Calidad de código
| Parámetro | Umbral | Estándar |
|-----------|--------|----------|
| Complejidad ciclomática | ≤10 por función (🟡 11–20) | McCabe/NIST, SonarQube |
| Tamaño de módulo | ≤1500 LOC (🟡 1500–2500) | Cohesión: 1 archivo = 1 responsabilidad |
| Panics / unwrap / expect | <2 por 1000 LOC | Prevención de crashes en toolchain |
| `#[allow]` silenciosos | 0 | Clean Code (Sonar) |
| `unsafe` | Solo en `src/platform/` | Regla de oro platform layer |
| Cobertura de tests (core) | ≥60% parser+semantic | Quality gate SonarQube |
| Build release incremental | <60s | DX de toolchain |

### B. Arquitectura
| Regla | Verificación |
|-------|--------------|
| Layering unidireccional: CLI→resolver→parser→semantic→codegen→platform | Script: parsea `use` de cada capa |
| Todo nodo AST lleva `Span` (Day-0) | Grep en ast.rs |
| `#[cfg(target_os)]` prohibido fuera de platform/ | Grep |
| Capas A/B/C sin acoplamiento circular | cargo machete + review |

### C. El lenguaje (criterios Sebesta)
| Parámetro | Umbral |
|-----------|--------|
| 5 pilares implementados | I–IV ✅, V ⛔ retirado (2026-08-03) |
| Legibilidad: errores con span+sugerencia | 100% (Day-0) |
| Escribibilidad: ejemplo 500+ líneas (R5) | 1 proyecto real compilable |
| Fiabilidad: `falcato test` | 47/47 ✅ |
| Expresividad: linked list, bitfields, self-ref sin pelear | Checklist "superar a Rust" |
| Paridad doc/código (GUIA.md↔features) | 100% |
| Ejemplos | 66/73 compilan (7 intencionales) |

### D. Iteración LLM (benchmarking HumanEval-style)
| Métrica | Umbral |
|---------|--------|
| Compile pass rate @1 | ≥90% en corpus fijo de prompts |
| N0→N2 | <3 ciclos (verificado por agente OpenCode) |
| Fix rate de errores del compilador | ≥80% primer intento |

### E. Entrega
| Parámetro | Umbral |
|-----------|--------|
| LSP | 6 features operativas |
| Instalación | MSI + installers en máquina limpia |
| CI | GitHub Actions verde (build+test+e2e) |
| Distribución | winget + Scoop (roadmap) |

## 🚨 REPORTE DE AUDITORÍA (2026-08-03) — CRÍTICO, PRIORIDAD INMEDIATA

**Estado real del codebase: 4.5/10 🔴 · Lenguaje en sí: 7/10 🟡.**
La documentación NO reflejaba la realidad. Progreso 2026-08-03: 5 de 5 puntos del plan resueltos (Verifier errors, warnings, arquitectura, docs, Pilar V retirado). Pendiente: deuda técnica (semantic.rs 3230 LOC, panics 4.2/1000).

### ✅ Bloqueante #1 — 17 regresiones de codegen ("Verifier errors") — RESUELTO (2026-08-03)
- **Síntoma original:** 17/73 ejemplos NO compilaban, incluido el más básico `imprimir_simple.fc` (9 líneas).
- **Causa raíz (2 bugs):**
  1. **`printf` variádica mal declarada** (`src/platform/registry.rs:227`): el registry la registraba con firma `&[I64]` (1 param) y `remap()` sobrescribía la firma pedida por el caller (2 params) → `call fn1(v4, v3): got 2, expected 1` → Verifier error. Fix: flag `variadic` en `BuiltinEntry` + `insert_variadic()` — el registry solo remapea el nombre, la firma exacta la decide el caller.
  2. **Mojibake en string de match** (`src/codegen/expresiones.rs:1049,1076`): `"tamaño_de"` tenía la ñ corrupta (`E2 94 9C E2 96 92` en vez de `C3 B1`) → el match exacto fallaba → "Función 'tamaño_de' no encontrada". Fix: reemplazo de bytes a ñ UTF-8 correcta.
- **Estado actual:** **66/73 ejemplos compilan** (los 7 restantes son errores intencionales de demostración: borrow_error, efecto_puro_error, feedback_educativo, field_borrow_error, rasgo_error_existe, rasgo_error_metodo, use_after_move). **47/47 unit tests pasan.** Verificado con release oficial `build.ps1`.
- **Pendiente:** los errores internos de codegen siguen con `sugerencia: None` — hacer que pasen por la tubería de errores con span+sugerencia (violación Day-0).

### 🔴 Bloquei #2. Arquitectura rota
- **`#[cfg(target_os)]` FUERA de platform/:** `src/codegen/mod.rs:178` usa `cfg(target_os = "windows")` justo debajo de un comentario que dice que NUNCA debe hacerlo.
- **Ciclo platform↔codegen:** `platform→codegen_helpers` + `codegen→platform`.
- **Layering invertido:** `semantic→parser` y `platform→codegen_helpers` rompen el orden declarado CLI→resolver→parser→semantic→codegen→platform.
- **Fix:** mover `call_conv_default` de codegen a `PlatformRuntime` (arregla cfg + ciclo).

### 🔴 Bloqueo #3. Deuda técnica masiva
- **Módulos sobre límite:** semantic.rs 3230 LOC (límite 1500), builtins.rs 1731, lsp.rs 1511.
- **Panics/unwrap/expect: 4.2/1000 LOC** (61 unwrap + 12 panic) — umbral <2.
- **Warnings:** 100 en build release (48 duplicados), 187 en clippy (~53 únicos). `cargo fix --bin falcato` elimina 36 automáticamente.
- **Código muerto:** BlockBuilder, VariableManager, MemoryHelper, CodegenBuilder, PlatformLinker, BackendFalcato trait — NUNCA construidos (roadmap 15G no conectado).

### ✅ Bloqueo4. Documentación miente — RESUELTO (2026-08-03)
- ~~AGENTS.md dice v0.4.0 y "40/40 tests"~~ → sincronizado: **0.3.0**, **47 tests**.
- ~~AGENTS.md dice "50+ ejemplos compilan y corren"~~ → sincronizado: **66/73** (7 intencionales).
- AGENTS.md dice "Build: build.ps1" → **sí existe** (build.ps1, build.bat, build_release.bat).
- AGENTS.md dice "Pilar V (prefijos) 📝 parcial" → **no existe en el lexer** → ⛔ **RETIRADO (2026-08-03)** — riesgo de colisión con `retornar`/`prestar`/`desde`; `des-` cubierto por R6 (drop automático).

### 🟡 Bloqueo #5. Deuda de calidad (no bloqueante pero urgente)
- **Clippy: 187 warnings** en bin (~53 únicos): ptr_arg, collapsible_if, expect_fun_call, manual_is_multiple_of, if_same_then_else.
- **Cobertura de tests no medible** (sin tarpaulin/llvm-cov instalado) — 47 unit tests + suite `falcato test` manual.
- **Código muerto confirmado:** BlockBuilder, VariableManager, MemoryHelper, CodegenBuilder, PlatformLinker, BackendFalcato trait — todos "never constructed" (roadmap 15G no conectado).

### ✅ Lo que SÍ está bien (no tocar)
- **47/47 unit tests pasan** (0.01s).
- **LSP completo:** 11 handlers (initialize, did_open/change/close, completion, signature_help, code_action, document_symbol, hover, references, goto_definition).
- **0 `unsafe`** en todo el codebase.
- **CI verde:** ci.yml + release.yml (build + test + e2e + artifact).
- `falcato test` funciona en pruebas_simple (2 OK).
- Build oficial vía `build.ps1` OK (146.5s release, binario 6.8 MB).

### 📋 Plan de acción (orden sugerido)
1. **Bug #1 (bloqueante):** ✅ RESUELTO — printf variádica + mojibake tamaño_de (2026-08-03).
2. **Quick win:** ✅ `cargo fix --bin falcato` — 100 → 61 warnings (2026-08-03).
3. **Arquitectura:** ✅ `call_conv_default` movido a `PlatformRuntime` — 0 cfg(target_os) fuera de platform/ (2026-08-03).
4. **Docs:** ✅ AGENTS.md sincronizado (0.3.0, 47 tests, 66/73 ejemplos) (2026-08-03).
5. **Pilar V:** ✅ **RETIRADO (2026-08-03)** — riesgo de colisión con `retornar`/`prestar`/`desde`; `des-` cubierto por R6 (drop automático).

---

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
