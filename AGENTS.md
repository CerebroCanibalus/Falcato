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

- **🚨 TODO VA EN ESPAÑOL — REGLA ABSOLUTA, NI UNA SOLA COSA EN INGLÉS**: el lenguaje,
  los errores, las etiquetas del CLI (`--nombre`), los subcomandos, la documentación,
  los mensajes del compilador — TODO en español. Nada en inglés, salvo (a) términos
  técnicos sin traducción cómoda (Cranelift, CLIF, JSON, LSP, WASM, ed25519, TCP) y
  (b) nombres de funciones C / builtins que son identidad de API (printf, malloc). Se
  permite **hispanizar** términos en inglés cuando la solución al español es incómoda o
  muy larga (ej: `check` → `verificar`, `build` → `compilar`). Los nombres en inglés
  del CLI existen SOLO como aliases ocultos para compatibilidad con scripts/CI, nunca
  como interfaz visible. Excepción implícita: si un día queremos enseñar Falcato a
  angloparlantes, esa versión será una doc aparte — el lenguaje y el toolchain siguen
  en español.
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
- **NINGUNA ETIQUETA CAMBIA SEMÁNTICA**: si una opción decide qué programas compilan o
  qué significan (ownership, const/mut, permisos, sanitizers, niveles), NO es etiqueta —
  es lenguaje puro o directiva por módulo. Una etiqueta solo decide CÓMO se produce el
  binario (optimización, output, verbosidad) o CUÁNTO se te dice (diagnósticos). El día
  que una etiqueta decide si el código compila, es una promesa que el compilador no
  puede verificar.
- **`--destino` es la ÚNICA etiqueta de plataforma**: el código `.fc` nunca sabe en qué
  plataforma corre. Las diferencias (procesos, terminal, stdin, fecha, rutas, EOL) las
  absorbe el runtime (Capa B/C) con builtins neutros + dispatch interno. Prohibidas
  `--windows`/`--linux`/`--compat-*`/`--cfg(os)` — crearían dos lenguajes.
- **Código portable o no compila**: un builtin sin impl para el target es **error de
  compilación** (con builtin + plataforma en el mensaje), nunca warning ni crash.
- **Impls juntas**: toda pieza nueva que toque sistema (procesos, terminal, red) lleva
  impl Windows + POSIX en la misma tanda (aunque solo se pruebe Windows; POSIX se
  verifica en WSL / CI matrix), más su nota de seguridad.
- **VERSIONADO ACORDE A LA ESCALA DEL CAMBIO**: `MAYOR.menor.parche`
  - **MAYOR** (x.0.0): cambios gigantes — sintaxis nueva, rompe compatibilidad, rediseño
    del lenguaje o del CLI visible.
- **menor** (0.x.0): refactorizaciones, cambios importantes o moderados — features
  nuevas del lenguaje/toolchain, etiquetas nuevas, cambios de workflow. NO rompe
  compatibilidad (aliases cubren lo viejo).
  - **parche** (0.0.x): bugs, quality-of-life, pequeños — fixes, docs, mejoras internas.
  - Regla de oro: si el cambio se ve desde fuera del compilador (CLI, lenguaje, errores),
    al menos sube **menor**. Si rompe algo existente, **MAYOR**. Si es invisible o un
    arreglo, **parche**. El bump SIEMPRE en `Cargo.toml` (`env!("CARGO_PKG_VERSION")`
    es la única fuente de verdad) + tag `vMAYOR.menor.parche` + `AGENTS.md` sincronizado.

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

## Estado del proyecto (v0.5.0)

Pipeline end-to-end operativo. Turing-completo con:
- **Core:** variables, ops, condicionales, bucles, arrays, structs, enums, generics (const+type)
- **Ownership:** `el`/`la`/`un`/`los`/`las`, mover/copiar/prestar, referencias `&T`/`&mut T`, field-level borrowing, lifetimes léxicos, regiones, self-referential `&yo`, efectos `puro`/`muta`/`lee`, branch-aware liveness, borrow checker gradual (N0→N1→N2)
- **Async:** threads reales, TCP (Winsock2), canales mpsc, thread pool, cancelación, stackless futures, `con_executor`, `seleccionar { }`
- **Built-ins:** Texto, Vector, Diccionario, Conjunto, Resultado<T,E>, bitwise, I/O polimórfico, interpolación, file I/O, matemáticas, sizeof
- **Plataforma:** runtime library (Capa A), PlatformRuntime trait (Capa B), BuiltinRegistry (Capa C) — Windows+Linux+macOS
- **LSP:** 6 features, integrado OpenCode, signature help, code actions, context-aware completion
- **Documentación:** GUIA.md + 15 capítulos, REFERENCIA.md, ERRORES.md, skill falcato-language, VS Code Extension (Falcato Dorado)
- **Instalación:** cargo-dist (MSI+shell+powershell), `falcato setup --all`, install.ps1 legacy
- **54/54 tests pasan. 68/75 ejemplos compilan** (7 restantes son errores intencionales de demostración).

## Comandos CLI

```bash
falcato compila <file.fc> --salida out.exe   # Compila a binario nativo (alias: build, compilar)
falcato corre <file.fc>                      # Compila y ejecuta (alias: run, ejecutar)
falcato verifica <file.fc>                   # Solo análisis (alias: check, verificar)
falcato prueba <file.fc>                     # Ejecuta pruebas del lenguaje (alias: test, probar)
falcato lsp                                  # Inicia servidor LSP (stdio)
falcato instala --todo                       # Instala VS Code extension + agentes (alias: setup --all)
falcato instala --agentes                    # Solo agentes/skills OpenCode/Claude
falcato instala --desinstalar                # Desinstala componentes adicionales
falcato version                              # Muestra versión
```

**Subcomandos en presente simple** (regla de estilo: cortos y verbales — `compila`,
`corre`, `verifica`, `prueba`, `instala`). Los nombres en inglés (build, run, check,
test, setup…) e infinitivos anteriores (compilar, ejecutar, verificar, probar,
instalar) funcionan como aliases ocultos para compatibilidad con scripts/CI — nunca
como interfaz visible.

## Etiquetas del toolchain (2026-08-08)

**Regla:** una etiqueta (`--nombre`) solo decide CÓMO se produce el binario
(optimización, output, verbosidad) o CUÁNTO se te dice (diagnósticos). Lo que decide
qué compila o qué significa el código NO es etiqueta — es lenguaje puro, directiva de
módulo o `falcato.toml`. **Las etiquetas están en español** (el lenguaje es español);
los nombres en inglés funcionan como aliases ocultos para compatibilidad con scripts/CI.

### Aprobadas (decisión de invocación)

| Etiqueta (ES) | Alias (EN) | Para qué | Prioridad |
|------|----------|-----------|-----------|
| `--lanzar` (+ `--nivel-opt N`) | `release` / `opt-level` | Optimización global (inlining, layout); la optimización *semántica* vive en efectos `puro`/`muta`/`lee` | 🔴 R8 |
| `--emitir-clif` | `emit-clif` | Salida de Cranelift CLIF — debuggear el codegen propio (backend propietario) | ✅ 2026-08-07 |
| `--json` | — | Diagnósticos como JSON estructurado (agentes LLM, CI, IDEs — mismo contrato que el LSP) | ✅ 2026-08-07 (verifica/compila) |
| `--incremental` | — | Cache de verificación por hash de fuente — iteración LLM write→check→fix <100ms | ✅ 2026-08-07 (verifica) |
| `--entrada` | `stdin` | `echo "código" \| falcato verifica -` — agentes sin archivos temporales | ✅ 2026-08-07 (verifica) |
| `--destino <triple>` | `target` | Cross-compile (`x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`, …). **Única etiqueta de plataforma** | 🟡 R8 |
| `--enlazador <path>` / `--raiz-sistema <dir>` | `linker` / `sysroot` | Cross-linking (lld, gcc, link.exe) | 🟡 R8 |
| `--crt-estatico` / `--crt-dinamico` | `crt-static` / `crt-dynamic` | CRT estático (default, funciona en cualquier máquina) vs DLLs del sistema (binario menor). R8 distribuye estático | 🟡 R8 |
| `-o, --salida <ruta>` / `--detallado` / `-g` / `-j N` | `output` / `verbose` / — / — | Output, verbosidad, debug info, paralelismo | 🟢 Baja |
| `--edicion` | `edition` | Versionar sintaxis — diseñar el formato YA aunque no haya cambios aún | 🟢 Baja |
| `--todo` / `--agentes` / `--desinstalar` / `--recursos` | `all` / `agents` / `uninstall` / `resources` | Subcomando `instala` — instalar VS Code extension, agentes, skills | ✅ |

### Prohibidas (semántica → lenguaje puro)

| Cosa | Hogar correcto |
|------|----------------|
| Nivel N0/N1/N2 | Directiva por módulo (`# nivel 2`), NUNCA etiqueta global |
| Warnings/sugerencias educativas | El compiler sugiere en N0; el nivel de módulo ES el `-Werror`. Sin `--deny-warnings` |
| Permisos (R8.3 Capa 4) | Efectos `puro/muta/lee` + `falcato.toml` — una etiqueta de bypass de seguridad NO existe |
| Ownership, const/mut, ABI | Artículos, `es`/`está`, C ABI por defecto |
| Bounds checks / sanitizers | Efecto declarado en el tipo o directiva de módulo |
| `--windows` / `--linux` / `--cfg(os)` / `--compat-*` | Nunca — el runtime (Capa B/C) absorbe la diferencia |

**Nota:** las etiquetas están en español; los nombres en inglés son aliases ocultos (funcionan, no se muestran en help).

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
- [ ] **R7.5 — Argumentos de línea de comandos (argv tipado, INNOVACIÓN)**
  - [x] **Fase 1 (baseline):** builtin `argumentos() -> Vector<Texto>` — crudo estilo C.
        `falcato corre` YA pasa args al binario (`cmd.args(args)`); falta que el
        lenguaje los lea. Mapeo Capa B/C: `GetCommandLineW`/`CommandLineToArgvW` en
        Windows, `argv` en POSIX — sin `--cfg(os)`. **✅ 2026-08-08** (Windows verificado;
        POSIX implementado en runtime, falta probar en WSL). Linker: añadido `shell32.lib`.
  - [x] **Fase 2 (la innovación):** `función principal(el args: Struct) -> Entero32` — el
        compiler genera parseo `--campo valor` + validación de tipos + `--ayuda`
        automático en español. **Los artículos codifican el esquema**: `el`=requerido,
        `un`=opcional, `la`=inmutable/validado, `los`=varargs posicionales.
        No es sintaxis nueva — struct + artículos que ya existen.
        **✅ 2026-08-08.** Implementación: `src/args_tipados.rs` (preprocesa el AST de
        `principal`, elimina el param ABI y sintetiza un prólogo Falcato que recorre
        argv con `argumentos()`/`vector_obtener`/`texto_comparar`, convierte con los
        nuevos builtins `texto_a_entero/natural/flotante/booleano` + `como_entero32`,
        valida requeridos y construye el struct `args`). Verificado:
        `.\saludo_app.exe --nombre sebas --cuenta 3` → "hola, sebas". Tipos soportados:
        Texto, Entero32/64, Natural32/64, Flotante64, Booleano. Interpolación ahora
        soporta acceso a campo `{args.nombre}`.
  - [x] **Fase 3:** librería `.fc` `librerias/args_avanzados.fc` — subcomandos
        (`args_subcomando`), defaults (opcional → default), repetición
        (`args_todos`), posicionales (`args_posicionales`), consultas
        (`args_tiene`, `args_obtener`, `args_cuenta`). **✅ 2026-08-08.**
        Contrato de memoria: TODAS las funciones devuelven COPIAS independientes
        (el caller libera; nunca comparte descriptores con argv → evita double-free).
        Uso: `usar args_avanzados::*` + `falcato compila app.fc args_avanzados.fc`.
        Sin tocar el compiler.
  - [x] **Criterio:** `.\saludo_app.exe --nombre sebas` → imprime "hola, sebas" ✅
  - [ ] *Bloqueante:* Cid necesita args en Fase 2 (loop agente con opciones)
  - [ ] *NO es etiqueta* — es lenguaje puro post-compilado (regla: lo post-compilado se
        hace en lenguaje). Documentar a fondo en GUIA.md (sección "Argumentos de
        línea de comandos") cuando esté listo.

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

## Estado de distribución (v0.5.0 — Alpha)

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
| Fiabilidad: `falcato test` | 54/54 ✅ |
| Expresividad: linked list, bitfields, self-ref sin pelear | Checklist "superar a Rust" |
| Paridad doc/código (GUIA.md↔features) | 100% |
| Ejemplos | 68/75 compilan (7 intencionales) |

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
- **Estado actual:** **68/75 ejemplos compilan** (los 7 restantes son errores intencionales de demostración: borrow_error, efecto_puro_error, feedback_educativo, field_borrow_error, rasgo_error_existe, rasgo_error_metodo, use_after_move). **54/54 unit tests pasan.** Verificado con release oficial `build.ps1`.
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
- ~~AGENTS.md dice v0.4.0 y "40/40 tests"~~ → sincronizado: **0.5.0**, **54 tests**.
- ~~AGENTS.md dice "50+ ejemplos compilan y corren"~~ → sincronizado: **66/73** (7 intencionales).
- AGENTS.md dice "Build: build.ps1" → **sí existe** (build.ps1, build.bat, build_release.bat).
- AGENTS.md dice "Pilar V (prefijos) 📝 parcial" → **no existe en el lexer** → ⛔ **RETIRADO (2026-08-03)** — riesgo de colisión con `retornar`/`prestar`/`desde`; `des-` cubierto por R6 (drop automático).

### 🟡 Bloqueo #5. Deuda de calidad (no bloqueante pero urgente)
- **Clippy: 187 warnings** en bin (~53 únicos): ptr_arg, collapsible_if, expect_fun_call, manual_is_multiple_of, if_same_then_else.
- **Cobertura de tests no medible** (sin tarpaulin/llvm-cov instalado) — 47 unit tests + suite `falcato test` manual.
- **Código muerto confirmado:** BlockBuilder, VariableManager, MemoryHelper, CodegenBuilder, PlatformLinker, BackendFalcato trait — todos "never constructed" (roadmap 15G no conectado).

### ✅ Lo que SÍ está bien (no tocar)
- **54/54 unit tests pasan** (0.01s).
- **LSP completo:** 11 handlers (initialize, did_open/change/close, completion, signature_help, code_action, document_symbol, hover, references, goto_definition).
- **0 `unsafe`** en todo el codebase.
- **CI verde:** ci.yml + release.yml (build + test + e2e + artifact).
- `falcato test` funciona en pruebas_simple (2 OK).
- Build oficial vía `build.ps1` OK (146.5s release, binario 6.8 MB).

### 📋 Plan de acción (orden sugerido)
1. **Bug #1 (bloqueante):** ✅ RESUELTO — printf variádica + mojibake tamaño_de (2026-08-03).
2. **Quick win:** ✅ `cargo fix --bin falcato` — 100 → 61 warnings (2026-08-03).
3. **Arquitectura:** ✅ `call_conv_default` movido a `PlatformRuntime` — 0 cfg(target_os) fuera de platform/ (2026-08-03).
4. **Docs:** ✅ AGENTS.md sincronizado (0.5.0, 54 tests, 68/75 ejemplos) (2026-08-03, actualizado 2026-08-07).
5. **Pilar V:** ✅ **RETIRADO (2026-08-03)** — riesgo de colisión con `retornar`/`prestar`/`desde`; `des-` cubierto por R6 (drop automático).
6. **Etiquetas toolchain:** ✅ `--emitir-clif`, `--json`, `--entrada`, `--incremental` implementados y verificados (2026-08-07).
7. **build.bat:** ✅ CRLF fix — cmd.exe no parsea `.bat` con finales de línea LF; el archivo tenía LF-only y rompía todos los `if errorlevel`.

---

## Descubrimientos (2026-08-07)

### 🔧 build.bat tenía LF-only — cmd.exe lo rompía
- **Causa raíz:** `build.bat` con finales de línea `\n` (LF) en vez de `\r\n` (CRLF). cmd.exe
  no parsea LF-only de forma fiable → todos los `if errorlevel` se rompían
  (`"rlevel" no se reconoce como un comando interno...`).
- **Fix:** convertir a CRLF (PowerShell: `-replace "\r?\n", "\r\n"`, guardar sin BOM).
- **Archivos:** `build.bat`. **Lección:** los `.bat` SIEMPRE en CRLF; verificar
  `file`/bytes si cmd empieza a dar errores raros.

### 🔧 PowerShell `Set-Content` corrompe archivos UTF-8 con acentos
- **Causa raíz:** `Set-Content -Encoding UTF8` de Windows PowerShell 5.1 re-codifica y
  corrompe `í`/`ó`/`ñ` en `.fc` (BOM + re-encoding) → el lexer escupe `[S008] carácter
  no válido` en cada acento.
- **Fix:** restaurar con `git checkout -- <archivo>`. **Lección:** NUNCA editar `.fc`
  con `Set-Content`; usar las tools de edición del agente (UTF-8 correcto).

### 🔧 `cargo build` directo falla con LNK1104 (msvcrt.lib)
- **Causa raíz:** cargo usa el linker de VS Insiders sin entorno MSVC configurado.
- **Fix:** compilar vía `build.bat` (que llama `VsDevCmd.bat -arch=x64`) o preceder con
  `call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat" -arch=x64`.

### 🚀 API de cranelift: no hay `get_ctx` para extraer CLIF post-compilación
- **Hallazgo:** `cranelift-module 0.112` no expone método público para obtener el CLIF de
  una función ya definida. El `--emit-clif` se implementó imprimiendo `ctx.func.display()`
  ANTES de cada `define_function` (cuando el Context aún tiene el IR). Puntos cubiertos:
  funciones normales, futures (`__init`/`__poll`/wrapper), closures e hilos (6 sitios).

### 📏 Métricas de la iteración LLM (R7 flags)
- `check --incremental` cache hit: **23 ms** (objetivo <100 ms) — hash por
  versión+archivo+fuente, cache `.falcato-cache/` solo para resultados OK.
- `check --json` y `--stdin` verificados con release oficial: el JSON sigue el contrato
  del LSP (código/archivo/línea/columna/mensaje/sugerencia).

### ✅ `vector_obtener<Texto>` rompía el verifier de Cranelift — RESUELTO (2026-08-08)
- **Síntoma:** `el arg: Texto = vector_obtener<Texto>(v, i)` — incluso fuera de bucle,
  en un vector vacío — producía `Verifier errors` al definir `principal`.
- **Causa raíz:** en `builtin_vector_obtener` el `sextend` del índice estaba condicionado
  al tipo del ELEMENTO; con `Texto` el índice no se extendía a I64 → el offset no cuadraba
  con el array de punteros → SSA dominance rota en el merge.
- **Fix:** el índice SIEMPRE se extiende a I64, sin depender del tipo del elemento.
- **Verificado:** bug_vector2/3/4 compilan y corren; `argumentos()` recorre argv con
  `vector_obtener<Texto>` sin verifier errors.
- **Impacto:** desbloquea R7.5 Fase 2 (parseo tipado) y Cid.

### 🐛 La runtime lib tiene target SEPARADO (trampa de recompilación)
- **Causa raíz:** el linker apunta a `lib/falcato_runtime/target/release/falcato_runtime.lib`
  (crate standalone), NO al target del workspace. `cargo build --release` en la raíz
  recompila el compiler pero NO la runtime lib que enlaza.
- **Fix:** para cambios en `lib/falcato_runtime/`, recompilar DESDE ese directorio
  (`cargo build --release` con workdir `D:\Falcato\lib\falcato_runtime`) antes de probar.
- **Lección:** el fix del null terminator de `argumentos()` parecía funcionar, pero era
  la lib vieja; la prueba real (args limpios en `saludo_app`) llegó al recompilar la lib.

### 🔧 Interpolación no soportaba acceso a campo (R7.5 Fase 2)
- **Causa raíz:** `builtin_imprimir_interpolado` buscaba `variables.get(contenido)` por
  nombre EXACTO; `{args.nombre}` no resolvía → imprimía vacío.
- **Fix:** si el segmento contiene `.`, compilar `Expresion::AccesoCampo` real
  (1 nivel: base.campo) e imprimir con `imprimir_valor_interpolado` por tipo.
- **Archivos:** `src/codegen/builtins.rs`, `src/codegen/tipos.rs` (inferir_tipo
  AccesoCampo).

### ✅ `Diccionario<K,V>`/`Conjunto<T>` no resolvían tipos concretos — RESUELTO (2026-08-08)
- **Causa raíz:** el parser mapeaba `Vector<T>` a `Tipo::Vector` pero `Diccionario`/
  `Conjunto` caían en `Tipo::NombreGenerico`; además `sustituir_genericos` no recorría
  `Diccionario`/`Conjunto`/`Resultado` → el retorno de `diccionario_nuevo<T,V>` quedaba
  con `Generico("K")` sin sustituir → disconcordancia con la declaración.
- **Fix:** parser mapea `Diccionario<K,V>` y `Conjunto<T>` a tipos reales (como Vector);
  `sustituir_genericos` recorre los 3 contenedores.
- **Nota:** `Diccionario` con tipos compuestos (ej: valor `Vector<Texto>`) aún rompe el
  verifier de Cranelift (`FunctionBuilder finalized, but block blockN is not sealed`) —
  bug de codegen en `builtin_diccionario_insertar`, pendiente.

### ✅ `texto_nuevo()` imprimía "(null)" — RESUELTO (2026-08-08)
- **Causa raíz:** `builtin_texto_nuevo` delegaba en `descriptor_nuevo` que ponía
  `ptr = 0` (NULL) → `printf("%s", NULL)` imprime "(null)".
- **Fix:** `builtin_texto_nuevo` crea un buffer real de 1 byte con `'\0'`
  (`ptr` válido, `len=0`, `cap=1`). `descriptor_nuevo` sigue con `ptr=0/cap=0`
  (para vectores/diccionarios — NO tocar: un cap=1 falso rompe `vector_agregar`).

### ✅ `vector_agregar<Texto>` escribía fuera de heap — RESUELTO (2026-08-08)
- **Síntoma:** `vector_agregar<Texto>` crasheaba (0xC0000005) al segundo elemento.
- **Causa raíz:** mi fix anterior de `descriptor_nuevo` (cap=1 falso) hacía que
  `vector_nuevo<Texto>` creara el descriptor del vector con `cap=1` y `ptr` a un
  buffer de 1 byte → `vector_agregar` NO reallocaba (len=0 < cap=1) y escribía el
  puntero del elemento en el buffer de 1 byte → heap overflow.
- **Fix:** separar responsabilidades — `descriptor_nuevo` vuelve a `ptr=0/cap=0`
  (correcto para colecciones), `builtin_texto_nuevo` crea el buffer vacío real.
- **Lección:** `descriptor_nuevo` es compartido por Texto Y colecciones; cambiar su
  layout rompe los builtins de Vector/Diccionario.

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
