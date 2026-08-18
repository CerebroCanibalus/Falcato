# Falcato — AGENTS.md

## Filosofía
Lenguaje de bajo nivel *construido desde cero* sobre **Cranelift** (apuesta estratégica, no temporal). NO es traducción de Rust. Es un lenguaje **gramatipado** (la gramática española es el sistema de tipos) y **morfosemántico** (la morfología del idioma porta el significado de máquina): el sistema de tipos explota dimensiones gramaticales del español — género, tiempos verbales, ser/estar, subjuntivo.
**Visión:** Falcato + Cranelift + WASM = toolchain nativa para código generado por IA. Bytecode Alliance comparte la visión. **Velocidad de compilación > velocidad de ejecución optimizada.**

## Los 5 Pilares
| # | Pilar | Esencia | Estado |
|---|-------|---------|--------|
| I | Género = Ownership | `el`=owned mut, `la`=borrowed inmut, `un`=option | ✅ |
| II | Ser/Estar = Const/Mut | `es`=permanente, `está`=temporal | ✅ |
| III | Tiempos = Modos | Presente=sync, Futuro=async, Subjuntivo=fallible | ✅ |
| IV | C ABI por defecto | Layout C, calling C, mangling off | ✅ |
| V | ~~Prefijos semánticos~~ | ~~`re-`=retry, `des-`=free, `pre-`=comptime~~ | ⛔ Retirado 2026-08-03 — colisión con `retornar`/`prestar`/`desde`; `des-` cubierto por R6 |

## Day-0 (no negociable)
- **🚨 TODO EN ESPAÑOL — REGLA ABSOLUTA**: lenguaje, errores, etiquetas CLI, subcomandos, docs, mensajes del compiler. Excepciones: (a) términos técnicos sin traducción cómoda (Cranelift, CLIF, JSON, LSP, WASM, ed25519, TCP), (b) nombres C/builtins identidad de API (printf, malloc). Se permite hispanizar si la traducción es incómoda (`check`→`verificar`). Nombres EN del CLI = SOLO aliases ocultos para scripts/CI, nunca interfaz visible.
- **C ABI por defecto**: layout C, calling conv SystemV/C, mangling off, salida `.o`
- **Span en cada nodo AST**: `Span { inicio, fin, archivo }` — sin span no hay LSP
- **Errores en español con códigos**: `[T001] archivo.fc:7:12: mensaje` — S/T/O/C/M/I/W
- **Documentar al agente**: adiciones grandes y cambios de workflow SE DOCUMENTAN en `C:\Users\Lord Gatito\.config\opencode\agents\falcato.md` (y en skill `falcato-language` si cambia sintaxis/builtins) en la misma tanda, no "para después".
- **🚨 SEGURIDAD CRÍTICA**: toda pieza que toque red (DHT, TCP, MCP, HTTP), sistema (procesos, archivos, terminal) o entrada externa → **revisión de seguridad minuciosa ANTES de mergear**. Prohibido exploits estúpidos (buffers sin validar, firmas sin verificar, inyección, DoS). Cada builtin con superficie externa lleva análisis de vectores de ataque en el commit. PR que toque red/sistema sin nota de seguridad → NO se mergea.
- **NINGUNA ETIQUETA CAMBIA SEMÁNTICA**: si una opción decide qué compila o qué significa (ownership, const/mut, permisos, niveles), NO es etiqueta — es lenguaje puro o directiva por módulo. Una etiqueta solo decide CÓMO se produce el binario (optimización, output, verbosidad) o CUÁNTO se te dice (diagnósticos).
- **`--destino` es la ÚNICA etiqueta de plataforma**: el código `.fc` nunca sabe en qué plataforma corre; las diferencias las absorbe el runtime (Capa B/C). Prohibidas `--windows`/`--linux`/`--compat-*`/`--cfg(os)`.
- **Código portable o no compila**: builtin sin impl para el target = **error de compilación** (con builtin+plataforma en el mensaje), nunca warning ni crash.
- **Impls juntas**: pieza nueva que toque sistema lleva impl Windows + POSIX en la misma tanda (POSIX se verifica en WSL/CI), más nota de seguridad.
- **VERSIONADO**: `MAYOR.menor.parche` — MAYOR: sintaxis nueva/rompe compat/rediseño visible. menor: features/etiquetas/workflow, no rompe (aliases cubren). parche: bugs/docs/mejoras internas. Si el cambio se ve desde fuera del compiler → al menos **menor**. Bump SIEMPRE en `Cargo.toml` (`env!("CARGO_PKG_VERSION")` única fuente de verdad) + tag `vMAYOR.menor.parche` + AGENTS.md sincronizado.
- **🚨 RELEASES EN ESPAÑOL**: título y cuerpo del GitHub Release en español (cargo-dist genera el body desde `CHANGELOG.md` vía `package.changelog`). `release.ps1` valida mojibake/EOL/versión antes de pushear el tag. Regenerar con `dist generate` si cambia config; el job 'plan' falla si `wix/main.wxs` está desactualizado (descripción de Cargo.toml debe coincidir).
- **🚨 NOVEDADES DEL RELEASE POR EFECTO, NO POR FASE**: clasificar por efecto para el USUARIO: **➕ ADICIÓN** (algo nuevo), **🔧 ARREGLO** (bug corregido), **🔁 REDISEÑO** (algo que existía y cambió de forma). Prohibido "Fase 2 — X", "R7.5", "hito del roadmap" en el release body — jerga interna. CHANGELOG.md y release body usan esta clasificación.

## Stack técnico
- **CLI:** `clap` 4.5 | **Lexer:** `logos` 0.14 | **Parser:** descendente manual + Pratt | **AST:** propio con span
- **Semántica:** "Concordancia Lingüística" (tipos + ownership + bounds)
- **Codegen:** `cranelift-codegen` 0.112 (puro Rust) | **LSP:** `tower-lsp` 0.20 — 6 features
- **Target:** x86_64 Windows (msvc) | **Build:** `build.ps1` | **Estilo:** Rust inglés en compiler, español snake_case en lenguaje

### Patrones Cranelift (críticos)
1. Loop header: NUNCA sellar antes del back-edge. Sellar DESPUÉS del `jump`.
2. Cadena if/else con 1 predecesor: sellar inmediato es seguro.
3. `compilar_sentencia` crea sub-bloques: padre sellado ANTES de llamarla.
4. SSA dominance: valores definidos en bloque A no se usan en bloque no-dominado.
5. `iconst` 2º arg: siempre `i64` (`0xFFFFFFFF_u32 as i64` para INFINITE).
6. `create_sized_stack_slot` (no `create_stack_slot` en 0.112).
7. `FunctionBuilderContext`: desde `cranelift_frontend`, no `cranelift_codegen::ir`.
8. `define_function(func_id, &mut ctx)` — 2 args.
9. Doble sellado = panic. Siempre verificar flujo.
10. `Linkage::Local` para funciones con cuerpo, `Linkage::Import` solo para FFI.

### Codegen Helpers (`src/codegen_helpers.rs`)
`BlockBuilder` (anti-double-seal) · `VariableManager` (SSA segura) · `CFunctionCache` (cache C externas) · `MemoryHelper` (store/load/const) · `tipo_a_cranelift()`/`tamano_tipo()`

### Platform Runtime Layer (3 capas)
```
Capa A — C Runtime (lib/falcato_runtime/): ops multi-paso (canales, executor, threads) → Rust staticlib linkeada
Capa B — PlatformRuntime trait (src/platform/): primitivas sync (mutex, sem, timestamp) → impl windows.rs/linux.rs/macos.rs
Capa C — BuiltinRegistry (src/platform/registry.rs): remapeo nombre→función C (sleep→Sleep/usleep, malloc→malloc…)
```
**Regla de oro:** codegen NUNCA hace `#[cfg(target_os)]`. Siempre trait dispatch o registry.

## Pipeline
```
.fc → Lexer → Parser → Analisis Semantico → Codegen (Cranelift) → .o → Linker → .exe
```

## Estructura del proyecto
```
src/
├── main.rs              # CLI (clap) — build, run, check, version, lsp, test, setup
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

## Estado del proyecto (v0.6.1)
Pipeline end-to-end operativo. Turing-completo con:
- **Core:** variables, ops, condicionales, bucles, arrays, structs, enums, generics (const+type)
- **Ownership:** `el`/`la`/`un`/`los`/`las`, mover/copiar/prestar, `&T`/`&mut T`, field-level borrowing, lifetimes léxicos, regiones, self-ref `&yo`, efectos `puro`/`muta`/`lee`, branch-aware liveness, borrow checker gradual (N0→N1→N2)
- **Async:** threads reales, TCP (Winsock2), canales mpsc, thread pool, cancelación, stackless futures, `con_executor`, `seleccionar { }`
- **Built-ins:** Texto, Vector, Diccionario, Conjunto, Resultado<T,E>, bitwise, I/O polimórfico, interpolación, file I/O, matemáticas, sizeof
- **Plataforma:** Capas A/B/C — Windows+Linux+macOS
- **LSP:** 6 features, integrado OpenCode, signature help, code actions, context-aware completion
- **Docs:** GUIA.md + 15 capítulos, REFERENCIA.md, ERRORES.md, skill falcato-language, VS Code Extension (Falcato Dorado)
- **Instalación:** cargo-dist (MSI+shell+powershell), `falcato setup --all`, install.ps1 legacy
- **54/54 tests pasan. 68/75 ejemplos compilan** (7 restantes son errores intencionales de demostración).

## Tipos naturales (commit 1ba66b7 — 2026-08-03)
Nombres en español que mapean a tamaños por defecto + adjetivos de tamaño:

| Categoría | Nombre natural | Equivalente explícito |
|-----------|----------------|----------------------|
| **Entero (con signo)** | `Entero` | `Entero32` |
| | `EnteroLargo` | `Entero64` |
| | `EnteroCorto` | `Entero16` |
| | `EnteroMínimo` | `Entero8` |
| **Natural (sin signo)** | `Natural` | `Natural32` |
| | `NaturalLargo` | `Natural64` |
| | `NaturalCorto` | `Natural16` |
| | `NaturalMínimo` | `Natural8` |
| **Real (flotante)** | `Real` | `Flotante64` |
| | `Real32` / `RealCorto` | `Flotante32` |
| | `Real64` | `Flotante64` |
| **Lógico** | `Lógico` | `Booleano` |

**Apodos de tipos:** `apodo Nombre = Tipo;` — renombrado transparente (ex-alias, anglicismo eliminado).
```falcato
apodo ID = Entero64;
apodo Precio = Real;
apodo Edad = Natural;
el id: ID = 1234567890123;
el precio: Precio = 19.99;
```

Compatibilidad total: `Entero32`, `Flotante64`, `Booleano` siguen funcionando.

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
**Subcomandos en presente simple** (`compila`, `corre`, `verifica`, `prueba`, `instala`). Nombres EN (build, run, check…) e infinitivos anteriores (compilar, ejecutar…) = aliases ocultos para scripts/CI — nunca interfaz visible.

## Etiquetas del toolchain (2026-08-08)
**Regla:** una etiqueta (`--nombre`) solo decide CÓMO se produce el binario (optimización, output, verbosidad) o CUÁNTO se te dice (diagnósticos). Lo que decide qué compila o qué significa NO es etiqueta — es lenguaje puro, directiva de módulo o `falcato.toml`. Etiquetas en español; nombres EN = aliases ocultos.

### Aprobadas
| Etiqueta (ES) | Alias (EN) | Para qué | Prioridad |
|------|----------|-----------|-----------|
| `--lanzar` (+ `--nivel-opt N`) | `release` / `opt-level` | Optimización global (inlining, layout); la optimización *semántica* vive en efectos `puro`/`muta`/`lee` | 🔴 R8 |
| `--emitir-clif` | `emit-clif` | Salida de Cranelift CLIF — debuggear el codegen propio | ✅ 2026-08-07 |
| `--json` | — | Diagnósticos como JSON estructurado (agentes LLM, CI, IDEs — mismo contrato que el LSP) | ✅ 2026-08-07 (verifica/compila) |
| `--incremental` | — | Cache de verificación por hash de fuente — iteración LLM write→check→fix <100ms | ✅ 2026-08-07 (verifica) |
| `--entrada` | `stdin` | `echo "código" \| falcato verifica -` — agentes sin archivos temporales | ✅ 2026-08-07 (verifica) |
| `--destino <triple>` | `target` | Cross-compile (`x86_64-unknown-linux-gnu`, …). **Única etiqueta de plataforma** | 🟡 R8 |
| `--enlazador <path>` / `--raiz-sistema <dir>` | `linker` / `sysroot` | Cross-linking (lld, gcc, link.exe) | 🟡 R8 |
| `--crt-estatico` / `--crt-dinamico` | `crt-static` / `crt-dynamic` | CRT estático (default) vs DLLs del sistema (binario menor). R8 distribuye estático | 🟡 R8 |
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

## Roadmap — Pendiente real
> ~~R5 — Proyecto ejemplo 500+ líneas~~ **SALTADO (2026-08-03).** Sustituido por R7: el primer gran proyecto era el CLI **Cid** (agente estilo OpenCode en Falcato, `D:\Cid`). **REORGANIZADO 2026-08-11:** el orden real es **R7.7 (aritmética consciente) → R9 (DAW, proyecto pequeño pero potente) → Cid**. El DAW es el campo de pruebas ideal para la aritmética: flotantes DSP, rangos MIDI, unidades. Las primitivas nativas que exige cada proyecto son el dogfooding real.

### R6 — Drop automático
- [x] **Análisis de scope para insertar `free` al final de scope** (Texto, Vector, Diccionario, Conjunto) **✅ 2026-08-11**
  - [x] `heap_vivas` (Vec con snapshots `marcar_scope`/`liberar_scope`) en Codegen — libera al final de: función, ramas de condicional, body de bucles (crítico: sin esto, leak acumulado en loops), regiones
  - [x] **Conservador: si hay duda → leak, nunca double-free.** No libera variables: (a) movidas a función con parámetro `el`/`los` (consulta `declaraciones`/`funciones_genericas` por artículo), (b) liberadas manualmente (`.liberar()` → `ends_with("_liberar")`), (c) retornadas (`retornar x` → el caller es dueño)
  - [x] Builtins NO mueven (imprimir/concatenar/agregar prestan o copian) — solo funciones de usuario con parámetro `el` mueven
  - [x] Parámetros `el`/`los` de tipo heap se registran como vivas (el callee es dueño)
  - [x] **Criterio verificado:** bucle 200k Textos sin OOM (antes: leak acumulado); movida/liberada/retornada sin double-free; suite 13/13 + 54/54 + 68/75 sin regresiones. Test: `unitest_drop.fc`
  - [ ] *Limitaciones v2:* closures con captura heap, `compilar_funcion_futuro` (async), TCP, asignación que sobrescribe heap (leak del valor viejo), `vector_agregar<Texto>` copia shallow (semántica existente)
  - [ ] **Requisito para:** Fase 2 de Cid (loop agente con JSON trees por doquier)

### R7 — Primitivas nativas para el CLI (Cid)
Piezas **NATIVAS** (Capa A/B/C); todo lo demás (JSON, HTTP, SSE, MCP) son librerías `.fc` y NO tocan el compiler.

- [x] **R7.1 — Spawn procesos + pipes** (CreateProcess/CreatePipe/WaitForSingleObject) **✅ 2026-08-10**
  - [x] Runtime C: `proceso_crear(comando) -> Handle`, `proceso_esperar(p) -> código`, `proceso_leer_salida(p) -> Texto` (stdout+stderr capturados) + `proceso_cerrar(h)`
  - [x] Registry: remapeo de los 4 nombres | Codegen: builtins `proceso_*` con Span
  - [x] **Criterio verificado:** `proceso_crear("falcato.exe verifica x.fc")` + `proceso_esperar` → EXIT[0], `proceso_leer_salida` captura el stdout real
  - [ ] *Bloqueante:* sin esto no hay `cid run/build/test`, ni git, ni MCPs stdio
- [x] **R7.2 — Terminal raw mode + ANSI** **✅ 2026-08-10**
  - [x] PlatformRuntime: `terminal_modo_raw(activo)`, `terminal_leer_tecla() -> Entero32`
  - [x] Windows: `SetConsoleMode(ENABLE_VIRTUAL_TERMINAL_PROCESSING)`, `ReadConsoleInput`
  - [x] **Criterio verificado:** `terminal_modo_raw(1)` → 1 (activación OK); lectura de teclas sin Enter requiere consola interactiva (no verificable en pipe)
  - [ ] *Bloqueante:* sin esto no hay TUI de Cid (Pilar IV)
- [x] **R7.3 — Entrada estándar (stdin)** **✅ 2026-08-10**
  - [x] Registry: `entrada_leer() -> Texto` (ReadFile sobre STD_INPUT_HANDLE)
  - [x] **Criterio verificado:** `echo hola | prog` → `entrada_leer()` devuelve "hola" · *Bloqueante:* MCPs y LSP client hablan por stdio (JSON-RPC)
- [x] **R7.4 — Date/time formato** (`fecha_unix`/`fecha_ms` ya existen y devuelven epoch real — verificado 2026-08-10; falta formatear a texto legible) **✅ 2026-08-11**
  - [x] Librería `librerias/fecha.fc` — algoritmo civil de Hinnant, aritmética pura sin FFI. API: `fecha_ahora() -> Texto` ("YYYY-MM-DD HH:MI:SS" UTC), `fecha_archivo() -> Texto` ("YYYYMMDD_HHMI"), `fecha_formatear(epoch, formato) -> Texto` (tokens YYYY MM DD HH MI SS + separadores), `fecha_anio/mes/dia/hora/minuto/segundo(epoch) -> Entero32`, `fecha_mes_nombre(m) -> Palabra`, `fecha_mes_actual() -> Palabra`. **Criterio verificado:** epoch 0 → "1970-01-01 00:00:00", 951782400 → "2000-02-29 00:00:00" (bisiesto), fecha actual correcta. *No bloqueante*
  - [ ] **Nota codegen:** structs como retorno entre archivos NO resuelven (`No se puede compilar tipo Nombre 'X' sin resolver` en `src/codegen/tipos.rs:133`) — por eso los componentes se exponen como funciones individuales (`_componente(epoch, cual)`). Bug pendiente de fix en codegen.
- [ ] **R7.5 — Argumentos de línea de comandos (argv tipado, INNOVACIÓN)**
  - [x] **Fase 1:** builtin `argumentos() -> Vector<Texto>` — crudo estilo C. `falcato corre` YA pasa args. Mapeo Capa B/C: `GetCommandLineW`/`CommandLineToArgvW` (Windows), `argv` (POSIX) — sin `--cfg(os)`. **✅ 2026-08-08** (Windows verificado; POSIX falta probar en WSL). Linker: `shell32.lib`.
  - [x] **Fase 2 (la innovación):** `función principal(el args: Struct) -> Entero32` — compiler genera parseo `--campo valor` + validación + `--ayuda` automático en español. **Artículos codifican el esquema**: `el`=requerido, `un`=opcional, `la`=inmutable/validado, `los`=varargs posicionales. No es sintaxis nueva — struct + artículos existentes. **✅ 2026-08-08.** Impl: `src/args_tipados.rs` (preprocesa AST, elimina param ABI, sintetiza prólogo Falcato que recorre argv con `argumentos()`/`vector_obtener`/`texto_comparar`, convierte con builtins `texto_a_entero/natural/flotante/booleano` + `como_entero32`, valida requeridos, construye struct `args`). Verificado: `.\saludo_app.exe --nombre sebas --cuenta 3` → "hola, sebas". Tipos: Texto, Entero32/64, Natural32/64, Flotante64, Booleano. Interpolación soporta `{args.nombre}`.
  - [x] **Fase 3:** librería `librerias/args_avanzados.fc` — subcomandos (`args_subcomando`), defaults, repetición (`args_todos`), posicionales (`args_posicionales`), consultas (`args_tiene`, `args_obtener`, `args_cuenta`). **✅ 2026-08-08.** Contrato de memoria: TODAS devuelven COPIAS independientes (el caller libera; nunca comparte descriptores con argv → evita double-free). Uso: `usar args_avanzados::*` + `falcato compila app.fc args_avanzados.fc`. Sin tocar el compiler.
  - [x] **Criterio:** `.\saludo_app.exe --nombre sebas` → "hola, sebas" ✅
  - [ ] *Bloqueante:* Cid necesita args en Fase 2 · *NO es etiqueta* — es lenguaje puro post-compilado (regla: lo post-compilado se hace en lenguaje). Documentar en GUIA.md ("Argumentos de línea de comandos").

### R7.6 — Unitest de codebase (previa al Cid)
Suite de pruebas de calidad del lenguaje — **requisito antes de empezar Cid** (el primer proyecto grande necesita red de seguridad). Un módulo de pruebas que ejercita el compiler contra casos que los ejemplos no cubren.

**Método (adaptado a Falcato, validado contra Rust ui-tests / Zig build modes / Go spec):**
1. **Descubrir** qué hace hoy el compiler en cada edge case (pasada rápida)
2. **DECIDIR la spec** por caso (lo que el lenguaje PROMETE) y clasificar:
   - 🟢 **Confirmado** — correcto por diseño → se documenta en REFERENCIA.md como spec
   - 🔴 **Bug** — se arregla el compiler
   - 🟡 **Por especificar** — requiere investigación y documentación a fondo antes de decidir; cada caso se abre como tarea y termina en una decisión escrita, nunca en "así es la vida"
3. **Testear la spec** (ver abajo) · 4. **Documentar** la spec en REFERENCIA.md

**Estructura (escalable: añadir test = añadir archivo, sin tocar el orquestador):**
```
pruebas/unitest/
├── unitest_ownership.fc        # + ejecutan (prueba "x" { afirmar })
├── unitest_numeros.fc          # + ejecutan
├── unitest_texto.fc            # + ejecutan
├── unitest_vector.fc           # + ejecutan
├── unitest_compilan/           # DEBE compilar pero no se ejecuta (verifica exit 0)
│   └── mover_en_n0.fc          # use-after-move en N0 → compila (spec: N0 permisivo)
├── unitest_negativos/          # DEBE fallar con código exacto (verifica --json)
│   ├── mover_en_n2.fc          # // ESPERADO: [O001] — función estricto
│   ├── mutar_inmutable.fc      # // ESPERADO: [O001]
│   ├── mutar_la_n0.fc          # // ESPERADO: [O001] — la es inmutable en TODOS los niveles
│   ├── borrow_mut_inmut.fc     # // ESPERADO: [O002] — borrow conflict
│   └── aritmetica_mixta.fc     # // ESPERADO: [T005] — sin promoción automática
├── ESPECIFICACIONES.md         # registro de decisiones por caso (🟢/🔴/🟡) ✅ 2026-08-11
└── correr_unitest.ps1          # orquestador: scan + prueba + verifica + semáforo ✅ 2026-08-11
```

**Los 3 tipos de tests (clave del diseño):**
- **Ejecutan** → `falcato prueba archivo.fc` (patrón `prueba "nombre" { afirmar }`)
- **Compilan (N0)** → `falcato verifica` exit 0 — la filosofía N0 es "todo compila, sugiere"; un test negativo en N2 es POSITIVO en N0
- **Fallan** → `falcato verifica --json` exit 1 + comparar `codigo` con `// ESPERADO: [XNNN]` del header (nuestro trybuild: el contrato JSON ya existe, sin snapshots)

**Estado 2026-08-11: suite 12/12 verde** (6 ejecutan + 1 compila + 5 negativos). 54/54 tests del compiler sin regresiones. 68/75 ejemplos compilan (7 intencionales). **Los 4 🟡 de ESPECIFICACIONES.md están CERRADOS con decisión escrita** — R7.6 completo.

**Descubrimientos de la pasada (ver ESPECIFICACIONES.md completo):**
- 🟢 **Overflow = wrap módulo 2ⁿ** (estilo Go): MAX+1 → MIN, Natural MAX+1 → 0. Spec confirmada
- 🟢 **Cast trunca** módulo 2ⁿ: `como_entero32(2^32+1)` → 1
- 🟢 **Aritmética mixta NO compila** (T005): AGENTS.md decía "operando mayor manda" — INCORRECTO; fecha.fc usa `como_entero32()` explícito
- 🟢 **Mutar `la` = O001 en TODOS los niveles** — el artículo es semántica estática (Pilar I), no regla N0/N2
- 🟢 **Borrow mut+inmut = O002** (código propio, no O001)
- 🟢 **`(*ref) == 42` requiere paréntesis** — `*ref == 42` se parsea como `*(ref == 42)` → T030
- 🟢 **`función estricto principal()` es S004** — formato correcto: `función principal() -> T estricto`
- 🟢 **Turbofish obligatorio** en builtins de colecciones: `vector_nuevo<Entero32>()`
- ✅ **`verifica --json` exit code FIXED 2026-08-11**: exit 1 con `ok:false` (marcador `JSON_YA_IMPRESO` en main.rs)
- ✅ **Diccionario FIXED 2026-08-11**: 4 causas raíz — (a) `body_block` sin sellar en `compilar_buscar_en_diccionario`; (b) SSA dominance: `stride_i64`/`val_offset_val` en found_block usados en not_found_block → movidos al bloque dominante; (c) `diccionario_nuevo` cap=0 → `hash % 0` crash → cap inicial 16 + resize realloc 2×; (d) **internado de strings** (`strings_internados`): literales iguales = mismo global → comparación de Palabra por puntero funciona
- ✅ **`vector_obtener` bounds check FIXED 2026-08-11**: fuera de rango → 0 definido (antes UB inestable, corridas alternas 1/0)
- ✅ **Aritmética flotante COMPLETA FIXED 2026-08-11**: `compilar_operacion_binaria` usaba sdiv/srem/icmp (enteros) con F64 → Verifier error en TODA operación flotante. Fix: dispatch por tipo — fadd/fsub/fmul/fdiv/fcmp para F32/F64; módulo emulado `a - floor(a/b)*b` (Cranelift 0.112 sin frem nativo). IEEE 754 verificado: 1.0/0.0=inf, 0.0/0.0=nan. Test: `unitest_flotantes.fc`
- 🟢 **División por cero entera** = UB documentado estilo C (crash 0xC0000095) — decisión 2026-08-11; crash controlado N2 = tarea futura
- 🟢 **Doble free** = UB documentado estilo C (crash 0xC0000005) — decisión 2026-08-11; detección N2 = tarea futura (R6 mitiga)
- 🟢 **Escritura vía `*ref_mut = x`** = limitación documentada (S003) — decisión 2026-08-11; feature futura N2
- 🟢 **`falcato prueba -` (stdin)** no soportado — solo archivos

**Cobertura por categorías:**
- [x] **Mutabilidad y ownership:** `el`/`la`/`un`/`los`/`las`, mover/copiar/prestar, `&T`/`&mut T`, field-level borrowing, use-after-move (N0 compila, N1/N2 detectan), branch-aware liveness — niveles como dial por función
- [x] **Edge cases numéricos:** overflow de Entero32/64 (wrap confirmado), división por cero (🟢 UB documentado), literales límite (Entero32::MAX/MIN), conversiones `como_entero32` truncando, aritmética mixta (T005 — sin promoción), **aritmética flotante completa** (✅ fix dispatch por tipo + IEEE 754)
- [x] **Edge cases de texto/colecciones:** `texto_nuevo()` vacío (fix "(null)"), `vector_agregar` con realloc (fix cap=1), `vector_obtener` fuera de rango (✅ 0 definido), `Diccionario` con clave inexistente (✅ fix completo), doble free (🟢 UB documentado), strings con escapes `\n\t\\`
- [ ] **Calidad de código:** complejidad ciclomática ≤10, módulos ≤1500 LOC, panics/unwrap <2/1000 LOC, spans en todos los errores (Day-0), `#[cfg(target_os)]` solo en platform/
- [ ] **Formato:** suite verde en `falcato prueba` + orquestador `correr_unitest.ps1` en CI
- [ ] **Criterio:** suite completa verde, sin regresiones en 54/54 tests existentes ✅, ESPECIFICACIONES.md sin casos 🟡 abiertos (todos resueltos o con tarea) — ✅ 4/4 cerrados con decisión escrita
- [ ] *Bloqueante:* Cid Fase 2 (loop agente con JSON trees) necesita confianza en mutabilidad y memoria

### R7.8 — Primitivas nativas para Cid (2026-08-18)
> **Objetivo:** 20 primitivas nativas que desbloquean Cid completo (MCP, HTTP, TUI, sesiones).
> **Decisión de diseño:** modularización por dominio para evitar archivos >500 LOC.

#### Estrategia de modularización
```
lib/falcato_runtime/src/
├── texto_dinamico.rs      # Strings dinámicos (P0-A: 4 funciones)
├── conversion_numerica.rs # Número↔texto (P0-B: 3 funciones)
├── archivo_avanzado.rs    # Archivos + entorno (P1: 8 funciones)
├── tls.rs                 # TLS/HTTPS schannel (P2: ~500 LOC)
├── proceso.rs             # Ya existe — añadir fixes (P0-C: 3 fixes)
└── tcp_cliente.rs         # Ya existe — añadir timeout (P0-C: 1 fix)
```
**Regla:** cada módulo ≤500 LOC. Si crece, dividir por subdominio.

#### FASE 1: P0-C — Fixes críticos (bloquea MCP)
> **Prioridad MÁXIMA** — sin estos fixes, MCP servers cuelgan.

| # | Función | Bug actual | Fix | Complejidad |
|---|---------|-----------|-----|-------------|
| 1 | `proceso_listo_para_leer` | Ignora timeout (`_ms`), PeekNamedPipe una vez | Loop con `WaitForSingleObject` + `PeekNamedPipe` cada 10ms | 🟢 Baja |
| 2 | `proceso_leer_salida_chunk` | ReadFile bloqueante sin timeout | `WaitForSingleObject(pipe, timeout)` antes de `ReadFile` | 🟢 Baja |
| 3 | `tcp_datos_disponibles` | ioctlsocket una vez, no espera | `select()` con timeout (POSIX) / `WSAPoll` (Windows) | 🟡 Media |

**Criterio:** MCP server responde sin colgar en diálogo JSON-RPC.

#### FASE 2: P0-A — Strings dinámicos (bloquea JSON/HTTP)
> **Crítico** — sin esto, JSON/HTTP/diff son imposibles.

| # | Función | Firma | Implementación | Complejidad |
|---|---------|-------|----------------|-------------|
| 4 | `texto_agregar_texto` | `(Texto, Texto) -> Vacío` | Realloc si `len + frag.len >= cap`, memcpy | 🟢 Baja |
| 5 | `texto_poner_byte` | `(Texto, Entero32, Entero32) -> Vacío` | Bounds check + `ptr[i] = b` | 🟢 Baja |
| 6 | `texto_puntero` | `(Texto) -> Entero64` | Retornar `descriptor.ptr` | 🟢 Trivial |
| 7 | `texto_desde_bytes` | `(Entero64, Entero32) -> Texto` | malloc(n+1), memcpy, construir descriptor | 🟢 Baja |

**Criterio:** `texto_agregar_texto` construye JSON dinámicamente sin leaks.

#### FASE 3: P0-B — Conversión número↔texto
> Necesita strings dinámicos (Fase 2).

| # | Función | Firma | Implementación | Complejidad |
|---|---------|-------|----------------|-------------|
| 8 | `entero_a_texto` | `(Entero64) -> Texto` | `snprintf("%lld")` + `texto_desde_bytes` | 🟢 Baja |
| 9 | `flotante_a_texto` | `(Flotante64) -> Texto` | `snprintf("%.17g")` + `texto_desde_bytes` | 🟢 Baja |
| 10 | `booleano_a_texto` | `(Booleano) -> Texto` | `b ? "verdadero" : "falso"` + `texto_desde_bytes` | 🟢 Trivial |

**Criterio:** `entero_a_texto(123)` → `"123"`, `flotante_a_texto(3.14)` → `"3.14"`.

#### FASE 4: P1 — Archivos + entorno (bloquea MCP fs)
> Desbloquea MCP fs, sesiones, API keys.

| # | Función | Firma | Windows API | POSIX | Complejidad |
|---|---------|-------|-------------|-------|-------------|
| 11 | `archivo_agregar` | `(Palabra, Texto) -> Vacío` | `CreateFile(FILE_APPEND_DATA)` | `open(O_APPEND)` + `write` | 🟢 Baja |
| 12 | `archivo_borrar` | `(Palabra) -> Vacío` | `DeleteFile` | `unlink` | 🟢 Trivial |
| 13 | `archivo_renombrar` | `(Palabra, Palabra) -> Vacío` | `MoveFile` | `rename` | 🟢 Trivial |
| 14 | `archivo_listar` | `(Palabra) -> Vector<Texto>` | `FindFirstFile/FindNextFile` | `opendir/readdir` | 🟡 Media |
| 15 | `archivo_escribir_bytes` | `(Palabra, Entero64, Entero32) -> Vacío` | `CreateFile` + `WriteFile` | `open` + `write` | 🟢 Baja |
| 16 | `entorno_obtener` | `(Palabra) -> Texto` | `GetEnvironmentVariable` | `getenv` | 🟢 Trivial |
| 17 | `directorio_actual` | `() -> Texto` | `GetCurrentDirectory` | `getcwd` | 🟢 Trivial |
| 18 | `aleatorio` | `() -> Entero64` | `rand()` (ya en runtime) | `rand()` | 🟢 Trivial |

**Criterio:** `archivo_listar(".")` lista archivos, `entorno_obtener("PATH")` devuelve PATH.

#### FASE 5: P2 — TUI + HTTPS
> **Decisión TLS: Opción A — Schannel nativo en runtime**

| # | Función | Firma | Implementación | Complejidad |
|---|---------|-------|----------------|-------------|
| 19 | `terminal_dimensiones` | `() -> (Entero32, Entero32)` | `GetConsoleScreenBufferInfo` (Win) / `ioctl(TIOCGWINSZ)` (POSIX) | 🟢 Baja |
| 20 | **TLS/HTTPS** | — | **Schannel nativo** (ver abajo) | 🔴 Alta |

##### Decisión TLS/HTTPS — Schannel nativo (Opción A)
**Justificación:**
- ✅ Más seguro (certs del sistema, sin dependencias externas)
- ✅ Control total (handshake, ALPN, encrypt/decrypt)
- ✅ Consistente con filosofía Falcato (todo nativo, sin FFI complejo)
- ❌ ~500-800 LOC en `tls.rs` (pero modularizable)

**Estructura modular propuesta:**
```
lib/falcato_runtime/src/tls.rs          # API pública (50 LOC)
lib/falcato_runtime/src/tls/
├── mod.rs                              # Re-exports
├── schannel_windows.rs                 # Impl Windows (300 LOC)
├── schannel_posix.rs                   # Impl Linux/macOS (300 LOC)
└── handshake.rs                        # Lógica común (150 LOC)
```

**API propuesta:**
```falcato
// Conexión HTTPS
el conn: Entero64 = tls_conectar("api.openai.com", 443);
// Enviar request
tls_escribir(conn, "GET / HTTP/1.1\r\nHost: api.openai.com\r\n\r\n");
// Leer respuesta
el respuesta: Texto = tls_leer(conn);
tls_cerrar(conn);
```

**Criterio:** `tls_conectar("api.openai.com", 443)` conecta exitosamente, handshake TLS 1.2/1.3.

#### Orden de ejecución
```
FASE 1 (P0-C): 1-2 horas → MCP servers estables
FASE 2 (P0-A): 2-3 horas → JSON/HTTP posibles
FASE 3 (P0-B): 1-2 horas → Serialización completa
FASE 4 (P1):   3-4 horas → MCP fs, sesiones, API keys
FASE 5 (P2):   4-6 horas → TUI + HTTPS
TOTAL: 11-17 horas → Cid completo
```

#### Notas de seguridad
- **TLS (Fase 5):** revisión de seguridad crítica antes de mergear (Day-0). Validar certs, evitar MITM, verificar ALPN.
- **Archivos (Fase 4):** validar rutas (evitar traversal `../`), permisos correctos.
- **Entorno (Fase 4):** `entorno_obtener` no debe exponer secrets en logs.

#### Estado
- [x] **FASE 1 (P0-C):** Fixes críticos — ✅ 2026-08-18
  - [x] `proceso_listo_para_leer`: loop con WaitForSingleObject + PeekNamedPipe cada 10ms (Windows), select con timeout (POSIX)
  - [x] `proceso_leer_salida_chunk`: WaitForSingleObject(100ms) antes de ReadFile (Windows), select(100ms) antes de read (POSIX)
  - [x] `tcp_datos_disponibles`: select() con timeout 100ms (Windows y POSIX)
  - [x] **Criterio verificado:** ejemplo `timeout_fix.fc` demuestra que los timeouts funcionan correctamente
- [ ] **FASE 2 (P0-A):** Strings dinámicos — 🔴 PENDIENTE
- [ ] **FASE 3 (P0-B):** Conversión número↔texto — 🔴 PENDIENTE
- [ ] **FASE 4 (P1):** Archivos + entorno — 🔴 PENDIENTE
- [ ] **FASE 5 (P2):** TUI + HTTPS — 🔴 PENDIENTE

**Criterio general:** Cid puede usar MCP, HTTP, TUI, sesiones sin workarounds.

### R7.7 — Aritmética consciente (REDISEÑO — plan aprobado 2026-08-11)
> La aritmética es de las cosas más importantes: el rediseño NO es parchear bugs, es darle semántica gramatical.
> **Investigación completa en el diseño (sesión 2026-08-11):** evidencia histórica + estado del arte + propuesta. Resumen abajo.

**El problema (el mayor de la aritmética en todos los lenguajes):** la desconexión entre la matemática que el programador cree escribir y la aritmética de máquina que ejecuta. Tres manifestaciones con desastres reales:
- **Overflow silencioso** — Ariane 5 ($370M, 1996, conversión F64→entero que desbordó), Heartbleed
- **Precisión flotante** — Patriot missile (28 muertos, 1991, redondeo en el reloj), 0.1+0.2≠0.3, dinero
- **Unidades** — Mars Climate Orbiter ($125M, 1999, libras vs newtons)

**Estado del arte (nadie lo soluciona completo):**
- Rust: panic en debug, wrap en release → **el mismo código se comporta distinto según el build** (fallo de diseño)
- Go: wrap silencioso siempre → consistente pero sin opción checked
- Zig: UB + builtins `@addWithOverflow` → correcto pero feo (fuera de banda)
- Swift: trapping (excepción) + `&+` → excepciones en hot path
- Ada: rangos (`1..100` con checks) — casi perfecto, sintaxis de ingeniería
- F#: unidades de medida — dimensional analysis, solo en F#

**La innovación de Falcato — la gramática española ES la semántica aritmética.** El español YA tiene las distinciones; no se inventa sintaxis, se da significado aritmético a la gramática existente:

| Gramática | Significado aritmético | Ejemplo |
|-----------|----------------------|---------|
| **Indicativo** (modo por defecto) | Aserto → **wrap módulo 2ⁿ** (rápido, como hoy) | `el x = a + b` |
| **Subjuntivo** (`fuese`) | Hipótesis → **checked** (si desborda → Resultado.Error) | `el x = a + b fuese` |
| **`un`** (artículo incierto, Pilar I) | Opcional → **checked con None** (si desborda, x = nada) | `un x = a + b` |
| **`entre` / `hasta`** (preposiciones) | **Rangos en tipos** (como Ada, en español natural) | `el edad: Entero8 entre 0 y 120` |
| **`por` / `entre`** (preposiciones) | **Unidades** ("tres POR cuatro" = ×; "doce ENTRE cuatro" = ÷) | `el v: Metro por Segundo` |
| **Niveles N0/N1/N2** | Verificación: N0 wrap + sugiere; N1 sugiere `fuese`/`un`; N2 exige intención; literales verificados en compile-time | `2147483647 + 1` → error en N2 |

**Coherencia total con los pilares:** una sola regla gramatical atraviesa el lenguaje — `el` = definido/owned (wrap definitivo), `un` = incierto/option (checked), indicativo = aserto, subjuntivo = hipótesis, `por`/`entre`/`hasta` = operaciones/rangos que el español ya usa al hablar de matemáticas. El programador hispanohablante entiende `a + b fuese` sin leer docs ("si la suma se desbordara...").

**Fases (implementación):**
- [x] **F1 — Subjuntivo aritmético** (`a + b fuese`): parsear modificador de operación binaria, semántica checked → Resultado. La semilla de todo **✅ 2026-08-11**
  - [x] Sintaxis: `el x = a + b fuese` → `Resultado<T, Entero32>` (Exito(valor) | Error(1 = desbordamiento))
  - [x] Desambiguación lingüística: token siguiente al `fuese` — `;` `)` `}` `,` EOF → checked; `{` `>` `<` → subjuntivo de condición (`si x fuese > 10` intacto)
  - [x] Overflow checks: suma signed `((a^r)&(b^r))<0`, resta signed `((a^b)&(a^r))<0`, mul signed `smulhi!=sext(r)`, unsigned suma `r<a`, resta `r>a`, mul `umulhi!=0`
  - [x] Packed I64 (layout enum ≤8 bytes): tag low 32, data high 32, `select(overflow, error, exito)`
  - [x] Entero8/16/32 + Natural8/16/32; Entero64/Natural64 → **pendiente F1.1** (Resultado de 12 bytes = layout enum grande)
  - [x] **Criterio verificado:** 9/9 unitest_fuese (suma/resta/mul overflow+ok, unsigned carry, wrap intacto, Gauss 5050, 13! desborda) + demo cuentas cabronas 10/10 (cuadrática, π, gravitación 3.542e22 N, umbral 46340²/46341²) + suite 14/14 GREEN + 54/54 tests + 68/75 ejemplos sin regresiones
  - [ ] *Pendientes del diseño (para F1.1+):* Entero64/Natural64, propagación con `?` ya funciona, `fuese` sobre expresiones no-binarias (error claro T106 ya), documentación GUIA.md/REFERENCIA.md
- [ ] **F2 — Artículo incierto** (`un x = a + b`): checked con None; requiere tipos numéricos opcionales **✅ 2026-08-11**
  - [x] `Tipo::Option(Box<Tipo>)` en AST + parser (`Option<T>`) + registro enum `Option` (Algo(valor) | Nada) en semantic y layout packed en codegen
  - [x] Semantic: artículo `un` + aritmética entera → `Option<T>` (T109 si no es entero ≤32 bits); `EsVariante` acepta Option; `?` extrae de Option
  - [x] Codegen: declaración `un x = a+b` sin tipo explícito fuerza Option (inferir_tipo del codegen devuelve Entero32 — bug: slot de 4 bytes guardaba valor crudo); overflow → Nada(tag 1), ok → Algo(tag 0)
  - [x] **Criterio verificado:** 7/7 unitest_un (Algo sin desborde, Nada en suma/resta/mul/unsigned, tipo explícito `Option<Entero32>`, wrap de `el` intacto) + suite 17/17 GREEN + 54/54 tests + 68/75 ejemplos
  - [ ] *Pendientes F2:* binding `como v` en `si expr es Enum.Variante` (hoy se ignora — usar `?` para extraer), Entero64/Natural64
- [ ] **F3 — Rangos** (`entre ... y ...`): verificación compile-time (literales) + runtime checks + integración con niveles
- [ ] **F4 — Unidades** (`por`, `entre`): dimensiones en tipos, verificación dimensional en semantic
- [ ] **F5 — Decimal nativo**: dinero exacto (0.1 + 0.2 = 0.3 de verdad)
- [ ] **Criterio:** `a + b fuese` compila a checked sin doble branch innecesario; `un x = a + b` desborda → None; N2 rechaza overflow de literales; suite unitest verde; documentado en GUIA.md/REFERENCIA.md
- [ ] **Nota Day-0:** al implementar sintaxis nueva → documentar en `falcato.md` del agente + skill `falcato-language` en la misma tanda

### R9 — Capacidades del lenguaje para DAW + Cid — PLAN 2026-08-13
> Capacidades nativas del lenguaje que DESBLOQUEAN tanto el DAW como Cid. Orden reorganizado 2026-08-17: matemática fundamental primero, luego JSON, luego Cid.

**Bloque 1 — Matemática y trigonometría (FUNDAMENTAL para el lenguaje):**
- [x] **R9.0.1 — Structs entre archivos** ✅ 2026-08-13
- [x] **R9.0.2 — Diccionario con tipos compuestos** ✅ 2026-08-13
- [x] **R9.0.3 — Conversión numérica completa** ✅ 2026-08-13
- [x] **R9.0.4 — Trigonometría y exp/log nativas** ✅ 2026-08-13
  - [x] **F1 — Builtins libm (preciso)**: 17 funciones trigonométricas
  - [x] **F2 — Builtins _rapido/_2pi**: funcional (usan libm)
  - [x] **F3 — Builtins _aprox**: placeholder
  - [x] **F4 — Tipos de precisión**: `Real_preciso`, `Real_rapido`, `Real_aprox` via `apodo` + builtins `_preciso` (`seno_preciso`, `coseno_preciso`, `tangente_preciso`, `exp_preciso`, `log_preciso`) **✅ 2026-08-17**
  - [x] **F5 — Efecto `vectorizable`**: keyword + parser + AST (metadata; codegen SIMD pendiente) **✅ 2026-08-17**
  - [x] **F6 — Fase nativa**: `estructural Fase` + `fase_avanzar` en stdlib (`librerias/math.fc`) **✅ 2026-08-17**
  - [x] **F7 — Polinomios minimax REALES**: seno_2pi (Horner grado 5) + coseno_2pi (Horner grado 6) en math.rs **✅ 2026-08-17**

**Bloque 2 — JSON parsing (DESBLOQUEA Cid):**
- [ ] **R9.1.0 — Librería JSON mínima** (`librerias/json.fc`): parser recursive descent para MCP
  - [ ] Tipos: `JsonValor` (enum: Objeto/Arreglo/Texto/Numero/Logico/Nulo)
  - [ ] Parser: `json_parsear(Texto) -> JsonValor` (~500 LOC)
  - [ ] Extractores: `json_texto()`, `json_numero()`, `json_objeto()`, `json_arreglo()`
  - [ ] Serializer: `json_serializar(JsonValor) -> Texto`
  - [ ] Tests contra fixtures MCP reales
  - [ ] *Criterio:* parsea requests MCP JSON-RPC correctamente

**Bloque 3 — Cid (PRIMER PROYECTO GRANDE):**
- [ ] **R9.2.0 — Cid core**: agente de código estilo OpenCode en Falcato
  - [ ] Loop agente: leer input → procesar → generar código → ejecutar
  - [ ] Integración LSP (ya existe)
  - [ ] Manejo de contexto (archivos, patches, memoria)
  - [ ] MCP stdio (ya tiene procesos: `proceso_crear/esperar/leer_salida`)
  - [ ] *Bloqueante para:* Cid es el primer dogfooding real del lenguaje

**Bloque 4 — DAW (DESPUÉS de Cid):**
- [ ] **R9.0.5 — Llamar punteros de función**: `direccion_de(fn)` + llamar por puntero (host VST)
- [ ] **R9.0.6 — Endianness helpers**: `Natural16/24/32/64` little-endian (WAV RIFF)
- [ ] **R9.0.7 — Carga dinámica de DLL**: `cargar_biblioteca`/`obtener_simbolo` (LoadLibrary/GetProcAddress)
- [ ] **R9.0.8 — Linker: libs GUI/MIDI**: `user32.lib`, `gdi32.lib`, `winmm.lib`, `comdlg32.lib`
- [ ] **R9.0.9 — GUI nativa mínima**: ventana + canvas + input (diseño en `docs/diseno_gui.md`)
- [ ] **R9.3.0 — Motor de audio offline**: WAV reader/writer, buffers, mezcla multi-pista
- [ ] **R9.3.1 — Secuenciador TUI**: pattern grid, piano roll, pistas
- [ ] **R9.3.2 — Mezclador + efectos**: volumen/pan, delay, reverb, EQ

**Criterio general:** matemática completa (F4-F7) → JSON parsing → Cid loop agente → DAW

### R8 — Sistema de paquetes distribuido (P2P, sin servidores)
Ecosistema estilo crates.io pero **sin registry central**: contenido por hash, índice en la **DHT de BitTorrent** (BEP44 mutable items firmados), transporte por torrent con seeding. Casi todo vive en el compiler Rust (como cargo); solo la distribución entra en Capa A. **Espíritu:** esfuerzo comunitario — confianza, denuncias y contenido los crean los pares (comunismo absoluto); semilla de confianza cero. Patrón validado por IPFS (Benet 2014): Kademlia + BitTorrent + Git, tamper-resistance por construcción.

- [ ] **R8.1 — Formato y CLI**
  - [ ] `falcato.toml` (nombre, versión, deps, permisos) + `falcato.lock` (árbol resuelto + hashes)
  - [ ] Comandos: `falcato paquete add/publicar/buscar/actualizar` · Resolver semver + imports en `resolver.rs`
  - [ ] **Anti-confusión (ConfuGuard-lite, arXiv 2025):** alertar si dos paquetes tienen nombres similares (`texto_util` vs `texto-util`), o si una dep transitiva es reciente con 0 avales → confirmación manual
- [ ] **R8.2 — Capa A: cliente torrent + DHT**
  - [ ] `falcato_torrent_descargar(hash, dir)` / `falcato_torrent_publicar(dir) -> hash`
  - [ ] DHT: publicar/consultar `paquete:<nombre>` → versión + hash (BEP44) · Seeding configurable
  - [ ] **Anti-eclipse (Inria 2011):** consultas replicadas a múltiples peers; el valor firmado hace que una respuesta falsa falle verificación → el peor daño es DoS, no compromiso. La DHT es **directorio, nunca fuente de confianza**
- [ ] **R8.3 — Seguridad (7 capas, cero mantenimiento)**
  - [ ] **Capa 1 — Integridad:** hash blake3 obligatorio (lo da BitTorrent gratis)
  - [ ] **Capa 2 — Solo fuente:** paquetes = código `.fc`, NUNCA binarios; **sin build scripts** (mata el 80% del malware tipo npm/cargo)
  - [ ] **Capa 3 — Autenticidad:** firma ed25519; **obligatoria en producción, opcional en modo "auditar"**
  - [ ] **Capa 4 — Tipos como permisos (INNOVACIÓN, capability-based):** permisos `red/archivos/procesos/terminal` en el manifiesto; el compiler verifica por los efectos `puro/muta/lee` que el código no exceda lo declarado → falla la compilación. Enforcement en compile-time, sin sandbox. Más fuerte que Miller: si el código no tiene la capacidad en su tipo, no compila — no hay bypass
  - [ ] **Capa 5 — WoT distribuida:** semilla de confianza cero; TOFU al primer contacto; avales entre editores
  - [ ] **Capa 6 — Blocklist comunal:** denuncias firmadas en DHT (`denuncia:<hash>` → razón); todos consultan antes de instalar
  - [ ] **Capa 7 — Transparency log en DHT:** publicaciones BEP44 firmadas e inmutables; historial público auditable; un editor comprometido no borra su rastro
  - [ ] **Capa 8 — Builds reproducibles (v2):** mismo fuente → mismo hash; toda la red es auditor
- [ ] **Criterio:** `falcato paquete add <lib>` descarga de la red P2P, verifica hash + firma, valida permisos, compila contra ella
- [ ] *Depende de:* principalmente compiler Rust; Capa A para torrent/DHT

### 🔴 REVISIÓN DE SEGURIDAD — PENDIENTES CRÍTICOS (auditoría 2026-08-03)
> Regla Day-0: NUNCA mergear red/sistema sin nota de seguridad. Vectores en el DHT actual (R8.2, `lib/falcato_runtime/src/dht.rs`) que DEBEN cerrarse antes de exponer el runtime a redes no confiables:

- [ ] **R8S.1 — SET sin verificación de firma**: `procesar_mensaje` tipo 2 acepta cualquier datagrama y lo inserta con `clave_publica=[0;32]` y `firma=[0;64]` (falsas). **Un atacante envenena el caché local con items falsos.** Fix: verificar firma ed25519 contra `clave_publica` ANTES de insertar; descartar si no verifica. (Capa 3 de R8.3 es la mitigación definitiva — hacerla ya.)
- [ ] **R8S.2 — DoS por memoria**: SET sin límite de tamaño ni cantidad — un peer puede llenar la RAM. Fix: límite de tamaño por item (ej. 1 MB), límite de items totales, evict LRU.
- [ ] **R8S.3 — DoS por CPU**: el hilo de escucha no tiene rate-limit. Fix: budget por peer (token bucket), dormir adaptativo.
- [ ] **R8S.4 — Buffer slicing frágil**: `&data[1..1]` (clave de 0 bytes) y slicing por offsets fijos pueden dar pánico con mensajes malformados. Fix: parseo con checks, nunca indexar sin verificar.
- [ ] **R8S.5 — `dht_consultar` devuelve puntero sin longitud**: el caller hace strlen — con valores binarios que contienen `\0` el resultado se trunca. Fix: devolver (ptr, len) o codificar en el payload.
- [ ] **R8S.6 — `proceso_crear` usa `cmd.exe /C "{}"` sin sanitizar**: si el comando proviene de input del usuario (Cid), hay inyección de comandos. Fix: validar el comando antes de pasar a la shell, o API de spawn directo (CreateProcessW sin shell) con args separados.
- [ ] **R8S.7 — Sin autenticación de peers**: bootstrap acepta cualquier "peer". Los peers solo deben ser fuente de *direcciones*, NUNCA de datos confiables (ya es así por diseño — mantener: la DHT es directorio, la firma es la verdad).

> **Criterio de cierre:** antes de cualquier release que exponga el runtime a la red real (DHT pública, MCP, HTTP), TODOS los R8S.* deben estar verificados. El commit que los cierre debe listar el vector, la causa raíz y el fix en la descripción.

### 15G — Migración de codegen helpers
- [ ] Migrar `compilar_funcion()` a `BlockBuilder` · Migrar variables de closures a `VariableManager` · Reemplazar `llamar_malloc/free` con `MemoryHelper`

### P5c — Probar Linux (WSL)
- [ ] Abierto a colaboradores — runtime ya tiene stubs POSIX

### Calidad
- [ ] Azure Trusted Signing (~$10/mes) para eliminar falsos positivos · Publicar en winget + Scoop · Fix interpolación (`{var}` en strings, roto desde antes de migración)

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
| Platform Layer (Linux/macOS) | 🟡 Diseñado + implementado, **19 fallos en macOS** (Issue #5) |
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
Auditoría completa en cada release mayor (v0.5.0, v0.6.0…). Semáforo: 🟢 cumple · 🟡 parcial · 🔴 no cumple. Umbrales: McCabe, SonarQube, Sebesta, HumanEval.

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
|---------|-------|
| Compile pass rate @1 | ≥90% en corpus fijo de prompts |
| N0→N2 | <3 ciclos (verificado por agente OpenCode) |
| Fix rate de errores del compilador | ≥80% primer intento |

### E. Entrega
| Parámetro | Umbral |
|-----------|-------|
| LSP | 6 features operativas |
| Instalación | MSI + installers en máquina limpia |
| CI | GitHub Actions verde (build+test+e2e) |
| Distribución | winget + Scoop (roadmap) |

## 🚨 REPORTE DE AUDITORÍA (2026-08-03) — CRÍTICO, PRIORIDAD INMEDIATA
**Estado real del codebase: 4.5/10 🔴 · Lenguaje en sí: 7/10 🟡.** La documentación NO reflejaba la realidad. Progreso 2026-08-03: 5 de 5 puntos del plan resueltos. Pendiente: deuda técnica.

### ✅ RESUELTO (2026-08-03)
1. **17 regresiones de codegen ("Verifier errors")** — 2 bugs: (a) `printf` variádica mal declarada (`src/platform/registry.rs:227`): el registry registraba firma `&[I64]` y `remap()` sobrescribía la firma del caller → Verifier error. Fix: flag `variadic` en `BuiltinEntry` + `insert_variadic()` — el registry solo remapea el nombre, la firma la decide el caller. (b) **Mojibake en string de match** (`src/codegen/expresiones.rs:1049,1076`): `"tamaño_de"` con ñ corrupta (`E2 94 9C E2 96 92` en vez de `C3 B1`) → "Función 'tamaño_de' no encontrada". Fix: bytes a ñ UTF-8 correcta. Estado: **68/75 ejemplos compilan** (7 restantes = errores intencionales de demo), **54/54 tests**.
2. **Arquitectura rota** — `cfg(target_os)` fuera de platform/ (`src/codegen/mod.rs:178`), ciclo platform↔codegen, layering invertido (semantic→parser). Fix: `call_conv_default` movido a `PlatformRuntime` → 0 cfg fuera de platform/.
3. **Documentación miente** — AGENTS.md sincronizado (v0.5.0, 54 tests, 68/75 ejemplos); Pilar V retirado (no existía en el lexer).
4. **Etiquetas toolchain** — `--emitir-clif`, `--json`, `--entrada`, `--incremental` implementados y verificados (2026-08-07).
5. **build.bat** — CRLF fix (cmd.exe no parsea LF-only).

### 🔴 PENDIENTE (deuda técnica)
- **Módulos sobre límite:** semantic.rs 3230 LOC (límite 1500), builtins.rs 1731, lsp.rs 1511.
- **Panics/unwrap/expect: 4.2/1000 LOC** (61 unwrap + 12 panic) — umbral <2.
- **Warnings:** 100 en build release (48 duplicados), 187 en clippy (~53 únicos). `cargo fix --bin falcato` elimina 36.
- **Código muerto:** BlockBuilder, VariableManager, MemoryHelper, CodegenBuilder, PlatformLinker, BackendFalcato trait — NUNCA construidos (roadmap 15G no conectado).
- **Cobertura de tests no medible** (sin tarpaulin/llvm-cov).
- **Errores internos de codegen con `sugerencia: None`** — deben pasar por la tubería de errores con span+sugerencia (violación Day-0).

### ✅ Lo que SÍ está bien (no tocar)
54/54 tests (0.01s) · LSP completo: 11 handlers · 0 `unsafe` · CI verde (ci.yml + release.yml) · `falcato test` OK · build.ps1 OK (146.5s, binario 6.8 MB).

---

## Descubrimientos (lecciones — causa raíz + fix)
- **R9.0.2 — el Diccionario distinguía structs por TAMAÑO, no por TIPO (2026-08-13)**: `diccionario_cargar_valor`/`guardar_valor` usaban `match tamano_tipo` — un struct de 8 bytes (2×Entero32, como InfoNota) caía en el caso I64 y guardaba el PUNTERO en vez del struct (y al cargar devolvía el struct empaquetado como si fuera un puntero → `canal: -1396705240` con 1 entrada, y basura con resize). Fix: distinguir por `tipo_es_struct` (no por tamaño) — structs se copian del ptr (loads/stores) y se cargan devolviendo el PUNTERO al bucket. Además: declaración con `diccionario_obtener<K,Struct>` copia del ptr al slot (patrón R9.0.1), e `inferir_tipo` del codegen conoce que `diccionario_obtener<K,V>` retorna V. **Lección:** el tamaño no define la semántica — un struct de 8 bytes NO es un Entero64.
- **R6 — retorno temprano en bucle panica `liberar_scope` (bug pre-existente, 2026-08-13)**: `retornar 1` dentro de un `si` anidado en un bucle llamaba `liberar_scope(0)` que libera TODO el heap (incluidas variables de scopes exteriores como `d: Diccionario`). Los epílogos de bucles/ramas intentaban luego `liberar_scope(marca)` con marcas obsoletas (desde > len) → panic "range start index X out of range". Fix en `liberar_scope`: guard `if desde > heap_vivas.len() { podar scope_marcas; return Ok }` + poda de `scope_marcas` tras truncate. **Efecto colateral:** arregló `ejemplos/argumentos_tipados.fc` (panica pre-existente) → 68/75 ejemplos (paridad con AGENTS.md).
- **R9.0.3 — conversión numérica era un builtin por función, no una familia (2026-08-13)**: `como_entero32`/`como_entero64` eran `ireduce`/`sextend` fijos y la semántica registraba `como_entero32` esperando SOLO `Entero64`. Fix: helper único `builtin_conversion_numerica(destino)` que despacha por tipo fuente→destino — entero→entero (sextend/ireduce), flotante→entero (`fcvt_to_sint` trunca hacia cero como cast C), entero→flotante (`fcvt_from_sint`), flotante→flotante (`fpromote`/`fdemote`). En semántica, flag `es_conversion_numerica` (como `es_polimorfica`) acepta cualquier tipo numérico. API: `como_entero8/16/32/64` + `como_flotante32/64`. **Lección:** una familia de builtins con misma semántica = un helper con `Tipo` destino, no N funciones duplicadas. Verificado: `3.7→3`, `-3.9→-3` (trunca hacia cero), `440→f64 440`, i8/i16/i64. `test_conv.fc`.
- **R9.0.1 — "structs entre archivos" era en realidad "retornar structs NO funcionaba NI en el mismo archivo" (2026-08-13)**: el T020 semántico (`Struct 'X' no declarado`) se arregló colectando structs/enums públicos en `resolver.rs::colectar_simbolos_publicos_decl` (solo colectaba funciones) y añadiendo `buscar_struct`/`buscar_enum` en semantic.rs (local → import específico → glob). PERO el codegen panica igual: `tipo_a_cranelift` no sabía construir firmas con `Tipo::Nombre` (retorno O parámetro de struct). Fix codegen — patrón **sret**: (a) retorno de struct → primer parámetro oculto I64 + retorno void; (b) parámetros struct → I64 por referencia (el callee copia el ptr al slot local en el prólogo); (c) caller aloca slot temporal y lo pasa como sret; (d) `retornar struct` copia al sret ptr (`copiar_mem` emite loads/stores de 8/4/2/1 sin memcpy); (e) `inferir_tipo` del codegen consulta `declaraciones` para el retorno real de funciones de usuario (antes devolvía Entero32 siempre). **Lección:** el T020 ocultaba un bug MÁS profundo de codegen — "no funciona cross-file" a veces es "no funciona en absoluto"; probar el caso single-file primero. Verificado: `test_struct_cross.fc` + `test_struct_chain.fc` (acceso directo `crear_nota(60,0.5).tono`, struct como parámetro, retorno en cadena) en `pruebas/dawn_check/`.
- **Rutas Windows con `\` en strings Falcato se comen la barra**: `"D:\Falcato\target"` → `\F`/`\t`/`\r` se interpretan como escapes → cmd.exe recibe `"D:Falcato"` y falla. Fix: usar forward slashes `"D:/Falcato/target"` o `\\` escapado. **Lección:** rutas en strings `.fc` SIEMPRE con `/` o `\\` (verificado 2026-08-10 en prueba R7.1).
- **build.bat LF-only rompía cmd.exe**: `.bat` con `\n` (LF) en vez de `\r\n` (CRLF) → `if errorlevel` rotos. Fix: CRLF (`-replace "\r?\n", "\r\n"`, sin BOM). **Lección:** los `.bat` SIEMPRE en CRLF.
- **PowerShell `Set-Content -Encoding UTF8` corrompe acentos**: re-codifica `í`/`ó`/`ñ` en `.fc` (BOM + re-encoding) → `[S008] carácter no válido`. Fix: `git checkout -- <archivo>`. **Lección: NUNCA editar `.fc` con Set-Content**; usar tools del agente (UTF-8 correcto).
- **`cargo build` directo falla con LNK1104 (msvcrt.lib)**: cargo usa linker de VS Insiders sin entorno MSVC. Fix: `build.bat` (llama `VsDevCmd.bat -arch=x64`) o preceder con `call "...\VsDevCmd.bat" -arch=x64`.
- **Cranelift no tiene `get_ctx` post-compilación**: `cranelift-module 0.112` no expone método para extraer CLIF de función ya definida. `--emit-clif` imprime `ctx.func.display()` ANTES de cada `define_function` (6 sitios: funciones, futures `__init`/`__poll`/wrapper, closures, hilos).
- **Métricas LLM (R7)**: `check --incremental` cache hit **23 ms** (<100 ms objetivo) — hash por versión+archivo+fuente, cache `.falcato-cache/` solo para OK. `--json`/`--stdin` verificados: JSON sigue contrato LSP.
- **`vector_obtener<Texto>` rompía el verifier (2026-08-08)**: el `sextend` del índice estaba condicionado al tipo del ELEMENTO; con `Texto` no se extendía a I64 → offset no cuadraba → SSA dominance rota. Fix: el índice SIEMPRE se extiende a I64. Desbloquea R7.5 Fase 2 y Cid.
- **Runtime lib tiene target SEPARADO (trampa de recompilación)**: el linker apunta a `lib/falcato_runtime/target/release/falcato_runtime.lib` (crate standalone), NO al target del workspace. `cargo build --release` en la raíz NO recompila la lib que enlaza. Fix: recompilar DESDE `D:\Falcato\lib\falcato_runtime`.
- **Interpolación no soportaba acceso a campo (R7.5 F2)**: `builtin_imprimir_interpolado` buscaba `variables.get(contenido)` por nombre EXACTO; `{args.nombre}` no resolvía. Fix: si el segmento contiene `.`, compilar `Expresion::AccesoCampo` real (1 nivel) e imprimir por tipo. Archivos: `src/codegen/builtins.rs`, `src/codegen/tipos.rs`.
- **`Diccionario<K,V>`/`Conjunto<T>` no resolvían tipos concretos (2026-08-08)**: caían en `Tipo::NombreGenerico` y `sustituir_genericos` no recorría los contenedores → retorno con `Generico("K")` sin sustituir. Fix: parser mapea a tipos reales (como Vector); `sustituir_genericos` recorre los 3 contenedores. **Nota:** `Diccionario` con tipos compuestos (valor `Vector<Texto>`) aún rompe el verifier (`blockN is not sealed`) — bug en `builtin_diccionario_insertar`, pendiente.
- **`texto_nuevo()` imprimía "(null)" (2026-08-08)**: delegaba en `descriptor_nuevo` con `ptr=0` (NULL) → `printf("%s", NULL)`. Fix: `builtin_texto_nuevo` crea buffer real de 1 byte con `'\0'` (`ptr` válido, `len=0`, `cap=1`). `descriptor_nuevo` sigue `ptr=0/cap=0` (colecciones — NO tocar: cap=1 falso rompe `vector_agregar`).
- **`vector_agregar<Texto>` escribía fuera de heap (2026-08-08)**: fix anterior de `descriptor_nuevo` (cap=1 falso) → `vector_nuevo<Texto>` con cap=1 y ptr a buffer de 1 byte → `vector_agregar` NO reallocaba (len=0 < cap=1) y escribía el puntero en el buffer de 1 byte → heap overflow (0xC0000005). Fix: separar responsabilidades — `descriptor_nuevo` vuelve a `ptr=0/cap=0` (colecciones), `builtin_texto_nuevo` crea el buffer vacío real. **Lección:** `descriptor_nuevo` es compartido por Texto Y colecciones; cambiar su layout rompe los builtins de Vector/Diccionario.
- **NO hay `break`/`interrumpir` en bucles (2026-08-11, demo cuentas cabronas)**: tuve que usar banderas `si ... { fallo = verdadero }` para salir de `para`. Esencial para DSP loops (R9 DAW: early-exit en procesamiento de buffers). Fix: token `interrumpir` (break) + `continuar` (continue) en parser/sentencias + codegen — pendiente post-R7.7.
- **NO hay notación científica en literales flotantes (2026-08-11)**: `6.674e-11` no parsea (lexer solo `[0-9]+\.[0-9]+` → `e` se lee como identificador → S003). Fix: ampliar regex flotante a `[0-9]+\.[0-9]+([eE][+-]?[0-9]+)?` en lexer + parse f64 — pendiente post-R7.7. Crítico para física/DSP en el DAW (constantes como 6.674e-11, 1.602e-19).
- **Impresión flotante trunca a 6 decimales (2026-08-11)**: `0.1 + 0.2` imprime `0.300000` (el `%f` default de printf), aunque el valor en memoria es `0.30000000000000004` correcto (IEEE 754 verificado). Fix: builtin imprimir flotante con `%.17g` (round-trip exacto) — pendiente post-R7.7. Nota: la comparación `suma_f es 0.30000000000000004` SÍ funciona — solo la impresión trunca.
- **F1 `fuese` — declaración sin tipo explícito requiere inferir_tipo en CODEGEN (2026-08-11)**: `el r = a + b fuese` sin `: Resultado<...>` explícito → `self.inferir_tipo` (codegen/tipos.rs) caía en default `Entero32` → slot de 4 bytes → `stack_store` I64 packed (8 bytes) → slot overflow (0xC0000005). Fix: caso `Expresion::Checked` en inferir_tipo del codegen → `Resultado<T, Entero32>`. **Lección:** TODA expresión nueva necesita su caso en AMBOS inferir_tipo (semantic + codegen) o las declaraciones sin tipo crashean.
- **F1 `fuese` — el check en parse_expresion_precedencia envuelve el lado DERECHO (2026-08-11)**: `a + b fuese` → `Binaria(a, +, Checked(b))` en vez de `Checked(Binaria(a,+,b))` porque el Pratt parser llama `parse_expresion_precedencia` recursivamente para cada lado. Fix: el check de `fuese` vive en `parse_expresion` (entrada pública, no recursiva), desambiguado por el token siguiente (`;`/`)`/`}`/`,`/EOF → checked; `{`/`>`/`<` → subjuntivo de condición). **Lección:** los modificadores postfix de expresión completa van en la entrada pública del parser, NO en el loop recursivo.
- **`romper`/`continuar` requieren un bloque EPILOGUE por bucle (2026-08-11)**: `continuar` NO puede saltar directo al header — se saltaría el `i++` del `para` (loop infinito) y el liberar_scope R6 (leak). Fix: cada bucle (mientras/para-rango/para-array) crea `epilogue_block` donde vive `liberar_scope + i++ + jump(header)`; `continuar` → jump(epilogue), `romper` → jump(exit). El body normal también pasa por el epilogue (evita doble liberación). Pila `pila_bucles: Vec<(epilogue, exit)>` en Codegen. Tras `romper`/`continuar` se crea un bloque huérfano sellado (código inalcanzable — el verifier de Cranelift exige bloques con terminator).
- **Los 3 pendientes post-R7.7 (break, notación científica, impresión flotante) RESUELTOS 2026-08-11**: `romper`/`continuar` + regex `[0-9]+\.[0-9]+([eE][+-]?[0-9]+)?` + `FlotanteExponente` (`1e3`) + `%.17g` en los 2 caminos de impresión flotante. Suite 16/16 GREEN + 54/54 tests + 68/75 ejemplos. **Lección:** el formato `%.17g` no es solo precisión — da notación científica en la SALIDA (`3.5422368558580461e+22`), perfecto para física/DSP.
- **F2 `un` — el codegen NUNCA ve el Option sin fix (2026-08-11)**: `un x = 40 + 2` sin tipo explícito → `inferir_tipo` del codegen devuelve `Entero32` (Binaria => Entero32 simplificado) → slot de 4 bytes → guarda el valor CRUDO (sin empaquetar Algo) → EsVariante lee tag de memoria inválida → 0xC0000005. Fix: en `DeclaracionVariable` del codegen, si artículo `Un` + valor Binaria aritmética + sin tipo explícito → forzar `Tipo::Option(inferir_tipo(izq))` ANTES del match de slot. **Lección:** el artículo codifica semántica que el codegen DEBE re-aplicar — no puede confiar solo en `inferir_tipo` (que ignora el artículo).
- **F2 `un` — pattern matching requiere `Option.Algo` (no `Algo` a secas) (2026-08-11)**: `si x es Algo` no parsea (el parser de condicionales espera `Enum.Variante`). Usar `si x es Option.Algo` (consistente con `Resultado.Exito`). El binding `como v` en `si` NO está implementado (se ignora `_binding`) — usar `el v: T = x?;` dentro del bloque. **Lección:** el binding de variante solo existe en `coincidir`, no en `si es`.

---

## 🔴 Issue #5 — macOS: Diagnóstico completo (2026-08-16)
> **Reporte:** sebastiankmilo — MacBook Pro M1, macOS Tahoe 26.5.2. Compiló 76 ejemplos, 67 generaron binario, **19 fallan al ejecutar** (10 crash SIGSEGV, 5 colgados, 3 salida basura, 1 incoherente). 48 OK. Referencia: `github.com/CerebroCanibalus/falcato/issues/5`

### Causa de fondo (única)
La capa macOS fue **copiada de Linux sin verificar constantes, tamaños ni llamadas al sistema**. Nadie ejecutó en un Mac real hasta este issue. Los 3 pilares rotos:

| Capa | Archivo | Bug | Efecto |
|------|---------|-----|--------|
| **A (Runtime)** | `lib/falcato_runtime/src/platform.rs:107` | `PTHREAD_MUTEX_SIZE=40` (glibc Linux) → macOS necesita **64** | Heap overflow en todo canal/executor |
| **B (Codegen)** | `src/platform/macos.rs:65` | `CLOCK_MONOTONIC=1` (Linux) → macOS necesita **6** | Timestamp basura → futuros nunca expiran |
| **C (Registry)** | `src/platform/registry.rs:264` | `"sleep"→nanosleep` firma `(I64,I64)` pero se llama con 1 arg I32 | Verifier error / no duerme |

### Los 19 fallos por grupo

#### GROUP A — Texto / Interpolación (4 crashes)
**Ejemplos:** `interpolacion`, `texto_ops`, `bitwise_metodos`, `closures`
- **Mecanismo:** todos imprimen con `printf`/`puts` usando **strings literales globales** (`Linkage::Local`). En Mach-O arm64 PIC, `global_value` genera relocaciones `PAGE21/PAGEOFF12` contra símbolos locales — si ld64 no los resuelve → puntero basura → `puts(0xDEADBEEF)` → SIGSEGV.
- **Causa adicional en `closures.fc`:** la interpolación `{resultado}` justo después del `call_indirect` amplifica el problema de globales.

#### GROUP B — Crashes sueltos (6 SIGSEGV)
**Ejemplos:** `apodo_tipos`, `match_simple`, `tamano_de`, `rangos`, `bitfields`, `calculadora`
- **Mecanismo mixto:** CR-1 (linkeo sin runtime) + CR-3 (globales Mach-O). Todos usan `imprimir` con strings literales que dependen de globales. Hipótesis: si `hola.fc` (1 global) funciona pero estos (2+ globales) crashean, el problema es la resolución de símbolos locales en Mach-O.

#### GROUP C — Async colgado (5 hangs)
**Ejemplos:** `futuro_simple`, `cancelar_simple`, `seleccionar_simple`, `select_simple`, `productor_consumidor`
- **3 bugs encadenados:**
  1. `CLOCK_MONOTONIC=1` → `clock_gettime(1)` → EINVAL → timestamp basura → `now >= never` → poll retorna 0 para siempre
  2. `nanosleep` firma rota → sleep no funciona → loop al 100% CPU
  3. `PTHREAD_MUTEX_SIZE=40` → heap overflow en canales → semáforos corruptos → waiters nunca despiertan
- **Cadena típica:** `esperar(futuro)` → wrapper sync → `while __poll(ptr) == 0 { dormir(1) }` → poll nunca avanza + dormir no funciona → loop infinito.

#### GROUP D — Argumentos tipados (1 basura)
**Ejemplo:** `argumentos_tipados` — imprime punteros basura en vez de "¡Hola, sebas!"
- **Mecanismo:** `argumentos()` en POSIX usa `_NSGetArgc()`/`_NSGetArgv()`. El linker POSIX no genera trampolín `_main`, así que `argc`/`argv` nunca llegan a la función → lee basura de registros.

#### GROUP E — Archivo I/O (1 incoherente)
**Ejemplo:** `archivo_io` — reporta 2422080 bytes escritos
- **Mecanismo:** `archivo_escribir` probablemente llama a `write()` con firma C mal declarada en macOS → el valor retornado es basura del stack o resultado de una POSIX distinta.

### Causas raíz detalladas

| ID | Severidad | Bug | Archivos | Afecta |
|----|-----------|-----|----------|--------|
| CR-1 | 🔴 CRÍTICA | Linkeo POSIX sin runtime/entry point (`link_objetos` branch no-Windows no pasa runtime, `-lm`, ni genera trampolín `_main`) | `main.rs:1123-1138` | Todos los 19 |
| CR-2 | 🔴 CRÍTICA | `CallConv::SystemV` en macOS ARM64 (debería ser `AppleAarch64`) → printf variádica lee registros FP no inicializados | `macos.rs:38-40` | Printf basura en floats |
| CR-3 | 🔴 CRÍTICA | Globales `Linkage::Local` en Mach-O PIC → relocaciones no resueltas → punteros basura | `builtins.rs:790-826`, `expresiones.rs:141-169` | 10+ crashes |
| CR-4 | 🔴 CRÍTICA | `PTHREAD_MUTEX_SIZE=40` (glibc) en macOS (64 bytes) → heap buffer overflow en canales/executor | `platform.rs:107-110` | 5 hangs + crashes |
| CR-5 | 🔴 CRÍTICA | `CLOCK_MONOTONIC=1` (Linux) en macOS (=6) → timestamp basura → poll eterno | `macos.rs:65` | 5 hangs |
| CR-6 | 🟠 ALTA | `"sleep"→nanosleep` firma `(I64,I64)` pero se llama con 1 arg I32 → verifier error o no duerme | `registry.rs:264` | 5 hangs |
| CR-7 | 🟠 ALTA | Printf declarada `(I64,I64)→I32` no-variádica + bitcast doubles a I64 → flotantes por GPR en vez de FP regs | `builtins.rs:117-128` | Flotantes basura (latente) |
| CR-8 | 🟡 MEDIA | Closures `call_indirect` usa firma I32 fija, no tipo real → mismatch ABI para tipos ≠ Entero32 | `expresiones.rs:1177-1183` | Closures con Texto/Entero64 |
| CR-9 | 🟡 MEDIA | `canal_cerrar` destruye semáforos con threads en espera → use-after-free de `sem_t` | `canal.rs:173-181` | Shutdown async |
| CR-10 | 🟡 MEDIA | `falcato_executor_close` hace `pthread_join` infinito sin timeout | `executor.rs:245-248` | Hang en cierre executor |

### Plan de corrección (orden de impacto)

**Fase 1 — Desbloquear binarios (sin esto nada corre):**
1. Fix linker POSIX: pasar runtime + `-lm -lpthread` + generar trampolín `_main` (CR-1)
2. Fix `CLOCK_MONOTONIC` → 6 en macOS (CR-5)
3. Fix `PTHREAD_MUTEX_SIZE` → 64 en macOS (CR-4)

**Fase 2 — Fix stdio (10+ crashes):**
4. Fix globales Mach-O: `Linkage::Preemptible` o strings en sección `__TEXT,__cstring` (CR-3)
5. Fix `CallConv::AppleAarch64` en macOS ARM64 (CR-2)

**Fase 3 — Fix async (5 hangs):**
6. Fix `nanosleep` → `usleep` (como Linux) o declarar bien la firma (CR-6)
7. Fix `canal_cerrar` para despertar waiters antes de destruir (CR-9)

**Fase 4 — Fix menores (3 fallos):**
8. Fix printf variádica (bitcast doubles → F64 real) (CR-7)
9. Fix closures firma (I32 fijo → tipo real) (CR-8)
10. Fix archivo_io (revisar firma POSIX de `write`) (CR-10)

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