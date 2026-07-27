# Falcato — AGENTS.md

## Filosofía del proyecto

Poder. Eficiencia. Iberofonía. Interlingua.

Falcato NO es una traducción de Rust al español. Es un lenguaje de bajo nivel
*construido desde cero* sobre **Cranelift** — apuesta estratégica, no temporal.

El sistema de tipos y semántica están diseñados aprovechando las dimensiones
gramaticales del español que el inglés no tiene: género, tiempos verbales,
ser/estar, subjuntivo, prefijos productivos, voz pasiva/activa, compuestos
aglutinantes.

### Visión estratégica

Falcato + Cranelift + WASM = **toolchain nativa para código generado por IA**.

- **Falcato**: lenguaje interlingua entre humanos (español), LLMs (lenguaje natural
  estructurado, baja ambigüedad), y máquinas (WASM/Cranelift)
- **Cranelift**: compilación ultra-rápida (JIT + AOT), ideal para ciclos LLM →
  código → compilar → ejecutar → depurar
- **WASM**: sandbox nativo para ejecución segura de código no confiable generado
  por IA
- **Bytecode Alliance** (Mozilla, Fastly, Intel, Arm, Google, Microsoft, Shopify):
  alineación natural — necesitan ejecución rápida y segura de código arbitrario

**Velocidad de compilación > velocidad de ejecución optimizada.**
**Seguridad del sandbox > control total del hardware.**
**Lenguaje cercano al humano > notación matemática.**

Cranelift no es "lo que tocó" — es el backend oficial y estratégico.
Contribuimos activamente a su ecosistema y roadmap. No planeamos migrar a LLVM.
Si Cranelift necesita features para lenguajes nativos AOT, las implementamos.

## Reglas de diseño

1. **Cada palabra reservada debe aportar semántica, no solo sintaxis.**
   No es `if` → `si`. Es usar el modo subjuntivo para codificar incertidumbre.
   No es `fn` → `función`. Es usar el tiempo verbal para codificar modo de ejecución.

2. **Cero abstracciones gratuitas.** Si una feature no se puede implementar
   con costo cero en runtime, no pertenece al núcleo del lenguaje.

3. **Explotar, no imitar.** Las features del español (género, ser/estar, etc.)
   deben traducirse en *garantías de compilación*, no en azúcar sintáctico.

4. **Iberofonía no es nacionalismo.** Es explorar si un idioma distinto al inglés
   puede aportar algo nuevo a la ingeniería de lenguajes de programación.

## Los 5 Pilares

| # | Pilar | Esencia | Estado |
|---|-------|---------|--------|
| I | **Género = Ownership** | `el`=owned mutable, `la`=borrowed immutable, `un`=option | ✅ Implementado |
| II | **Ser/Estar = Const/Mut** | `es`=identidad permanente, `está`=estado temporal | ✅ Implementado (base) |
| III | **Tiempos = Modos ejecución** | Presente=sync, Futuro=async, Subjuntivo=fallible | ✅ Subjuntivo + Futuro (18A MVP) |
| IV | **C ABI por defecto** | Layout C, calling C, mangling off | ✅ Implementado |
| V | **Prefijos semánticos** | `re-`=retry, `des-`=free, `pre-`=comptime | 📝 Documentados; `des-` parcial vía FFI manual |

### Estrategia de backend

**Cranelift es el backend oficial y estratégico.** No es temporal ni migraremos a LLVM.
Si Cranelift necesita features (mejor cold branch hinting, debug info AOT, etc.),
las implementamos nosotros y contribuimos upstream. Bytecode Alliance comparte
nuestra visión: código generado por IA necesita compilación rápida y ejecución segura.

### Innovación Fase 12: Concordancia de Posesión

Extensión del Pilar I con **tipos afines** como base teórica:
- `el` = affine (usar 0 o 1 veces, owned)
- `la` = no-lineal (usar N veces, borrowed)
- `los`/`las` = shared ownership (reference-counted)
- Lifetimes léxicos: `&dato Texto` en vez de `&'a T`
- Borrow checker **gradual** (Nivel 0 permisivo → Nivel 2 estricto)
- Diseño completo: `docs/diseno_ownership.md`

## Innovación Implementada: Concordancia Lingüística

Aprovechamos que en español los adjetivos **concuerdan** en género y número con
el sustantivo. En Falcato, los valores deben "concordar" en tipo, ownership y estado.

**Errores intuitivos para hispanohablantes:**
```
[T001] test.fc:4:8: Disconcordancia de tipo: 'a' es 'Entero32' pero se declaró como 'Booleano'
       │ sugerencia: Cambia el tipo a 'Entero32' o el valor

[O001] test.fc:5:5: 'constante' no es mutable: se declaró con 'la' (inmutable)
       │ sugerencia: Usa 'el constante' para hacerlo mutable
```

## Day-0: Decisiones arquitectónicas vinculantes

### C ABI por defecto (no negociable)
- Layout de structs = C layout (`repr(C)` es el default)
- Calling convention = SystemV/C
- Name mangling = desactivado (símbolos literales)
- Salida `.o` compatible con `gcc`/`clang`/`link.exe`

### Span en cada nodo del AST (no negociable)
- `Span { inicio: Posicion, fin: Posicion, archivo: Arc<str> }`
- `Posicion { linea: u32, columna: u32, offset: u32 }`
- Sin Span no hay errores con ubicación ni LSP futuro

### Errores en español con códigos (no negociable)
- Formato: `[T001] archivo.fc:7:12: mensaje`
- Categorías: S (sintaxis), T (tipo), O (ownership), C (FFI), M (módulos), I (interno), W (warning)
- Sugerencia opcional en cada error

## Stack técnico

- **CLI:** `clap` 4.5 (Rust)
- **Lexer:** `logos` 0.14 — errores léxicos reportados con span real
- **Parser:** descendente manual modular (Pratt parser), recovery de errores, spans reales
- **AST:** Propio con Span obligatorio, métodos `span()` en nodos
- **Semántica:** "Concordancia Lingüística" — tipos + ownership + bounds
- **Codegen:** `cranelift-codegen` 0.112 (puro Rust), spans reales, IDs únicos para strings
- **LSP:** `tower-lsp` 0.20 — diagnósticos, autocompletado, hover, go-to-definition, find-references
- **Testing:** 40 tests unitarios pasando
- **Target:** x86_64 Windows (msvc)
- **Build:** `build.ps1` (auto-detecta Visual Studio)

### Patrones Cranelift aprendidos (críticos para codegen)

1. **Loop header sealing**: NUNCA sellar el loop header antes del back-edge. Sellar DESPUÉS del `jump` de regreso al header.
2. **Cadena if/else con 1 predecesor**: Cada bloque de la cadena tiene exactly 1 predecesor → sellar inmediato es seguro.
3. **`compilar_sentencia` crea sub-bloques**: El bloque padre debe estar sellado ANTES de llamar `compilar_sentencia`, porque internamente crea bloques (condicionales, loops) que resuelven SSA contra el padre.
4. **SSA dominance**: Valores definidos en un bloque NO pueden usarse en bloques no-dominados. Crear constantes separadas por bloque.
5. **`JumpTableData` en 0.112**: Requiere `BlockCall::new(block, &[args], &mut ValueListPool)` — API compleja. Preferir cadena if/else para dispatch simple.
6. **`iconst` segundo argumento**: SIEMPRE `i64`. Usar `0xFFFFFFFF_u32 as i64` para INFINITE.
7. **`create_sized_stack_slot`**: No `create_stack_slot` (no existe en 0.112).
8. **Tokens parser**: `ParenAbre`/`ParenCierra` (no `ParentesisAbre`/`ParentesisCierra`).
9. **`FunctionBuilderContext`**: Importar desde `cranelift_frontend`, no `cranelift_codegen::ir`.
10. **`define_function`**: Toma 2 args: `(func_id, &mut ctx)`.
11. **Doble sellado = panic**: `assertion failed: !self.is_sealed(block)` significa que se selló un bloque dos veces. Verificar flujo de sellado.
12. **Funciones internas vs externas**: `Linkage::Local` para funciones con cuerpo (define_function). `Linkage::Import` solo para FFI sin cuerpo.

### Codegen Helpers (Fase 15G) — Capa de abstracción

Para eliminar los patrones manuales propensos a errores, se creó `src/codegen_helpers.rs`:

| Helper | Propósito | Elimina |
|--------|-----------|---------|
| `BlockBuilder` | Builder de bloques seguro con anti-double-seal | Panic por `assertion failed: !self.is_sealed(block)` |
| `VariableManager` | Gestión de variables SSA con API segura | `HashMap<String, (StackSlot, Tipo, Articulo)>` crudo |
| `CFunctionCache` | Cache de funciones C externas | `asegurar_funcion_c()` repetido |
| `MemoryHelper` | Helpers de memoria (store/load/const) | `builder.ins().store(MemFlags::new(), val, ptr, offset)` manual |
| `tipo_a_cranelift()` | Conversión Tipo Falcato → Type Cranelift | Función duplicada en codegen.rs |
| `tamano_tipo()` | Tamaño en bytes de un Tipo | Función duplicada en codegen.rs |

**Uso en codegen.rs:**
```rust
use crate::codegen_helpers::{BlockBuilder, VariableManager, CFunctionCache, MemoryHelper};

// Antes (peligroso):
let entry_block = builder.create_block();
builder.append_block_param(entry_block, types::I64);
builder.switch_to_block(entry_block);
builder.seal_block(entry_block);  // ← panic si se hace 2 veces

// Después (seguro):
let mut bb = BlockBuilder::new(&mut builder);
let entry = bb.crear_bloque_con_parametros(&[types::I64]);
bb.sellar_si_necesario(entry);  // ← idempotente, seguro
```

**Patrón de cadenas if/else:**
```rust
// Antes (20+ líneas manual):
let bloque_check1 = builder.create_block();
builder.ins().brif(es_cero, bloques[0], &[], bloque_check1, &[]);
builder.switch_to_block(bloques_check1);
builder.seal_block(bloques_check1);
// ... repeat N veces

// Después (declarativo):
let mut bb = BlockBuilder::new(&mut builder);
let conds = vec![(cond1, block1), (cond2, block2)];
bb.cadena_si_sino(&conds, default_block);
```

**Regla de uso:** Siempre usar `BlockBuilder` en vez de `builder.create_block()` + `builder.seal_block()` directo.

**⚠️ PENDIENTE (Fase 15G):** Migrar codegen.rs para usar los helpers:
- Migrar `compilar_funcion()` para usar `BlockBuilder`
- Migrar variables de closures para usar `VariableManager`
- Reemplazar `llamar_malloc/free` con `MemoryHelper`
- Reemplazar funciones duplicadas `tipo_a_cranelift`/`tamano_tipo`

### Platform Runtime Layer — Abstracción multiplataforma (3 capas)

Para que Falcato funcione en Linux y macOS sin reescribir código, se diseñó una
arquitectura de **3 capas** con分工 clara según la complejidad de cada operación:

```
┌───────────────────────────────────────────────────────────┐
│                    codegen.rs                              │
│  (genera Cranelift IR que llama a funciones C-ABI)         │
│                                                             │
│  let ch = platform_call_ret("falcato_channel_new", &[cap]) │  ← Capa A
│  runtime.mutex_lock(&mut ctx, builder, ptr)                 │  ← Capa B
│  asegurar_funcion_c("sleep")                                │  ← Capa C
└──────────┬──────────────────────┬──────────────────────────┘
           │                      │
     ┌─────┴──────┐        ┌──────┴────────┐
     │  Capa A    │        │  Capa B       │
     │ C Runtime  │        │PlatformRuntime│   Capa C: BuiltinRegistry
     │ (Rust .rs) │        │  trait (Rust) │   (remapeo nombre→función)
     │ complex    │        │  primitives   │
     │ multi-step │        │  1 instruc.   │
     └────────────┘        └───────────────┘
```

| Capa | Para qué | Ejemplo | Implementación |
|------|----------|---------|---------------|
| **A: C Runtime** | Operaciones multi-paso con struct layout que difiere por plataforma | `falcato_channel_new()`, `falcato_executor_new()`, `falcato_thread_run()` | `lib/falcato_runtime/` — crate Rust `#![no_std]`, staticlib, raw `extern` declarations. Cero dependencias. |
| **B: PlatformRuntime trait** | Primitivas de sync simples (misma firma, distinta impl) | `mutex_lock/unlock`, `sem_wait/post`, `timestamp()`, `net_init/close` | `src/platform/traits.rs` + `windows.rs` + `linux.rs` + `macos.rs` |
| **C: BuiltinRegistry** | Remapeo de nombre de función C (misma firma, distinto nombre) | `"sleep"`→`Sleep`/`usleep`, `"malloc"`→`malloc` | `src/platform/registry.rs` — tabla plana nombre→función |

**Reglas de uso en codegen.rs:**
```rust
// Capa C — builtins simples (solo remapeo de nombre)
ctx.call_void("sleep", builder, &[ms_val]);

// Capa B — primitivas (trait method, misma firma cross-platform)
let runtime = platform::current_runtime();
let ts = runtime.timestamp(&mut ctx, builder);
runtime.mutex_lock(&mut ctx, builder, canal_ptr);

// Capa A — operaciones complejas (runtime library en Rust staticlib)
let canal = self.platform_call_ret("falcato_channel_new", &[cap, elem_size])?;
```

**Regla de oro:** La codegen NUNCA hace `#[cfg(target_os)]`. La selección de plataforma
es siempre por trait dispatch o nombre de función C remapeado.

**Agregar una nueva plataforma:**
- Capa A: 1 archivo `lib/falcato_runtime/src/miplat.rs` + 1 línea en build.rs
- Capa B: 1 archivo `src/platform/miplat.rs` + 1 línea en mod.rs
- Capa C: solo añadir entradas a la tabla en registry.rs

| Plataforma | Soporte | Estado |
|------------|---------|--------|
| Windows (x86-64) | ✅ Completo | Cranelift `WindowsFastcall` + MSVC link |
| Linux (x86-64, aarch64) | ⚠️ Implementado, no testeado | Cranelift `SystemV` + gcc link |
| macOS (x86-64, aarch64) | ⚠️ Implementado, no testeado | Cranelift `SystemV`/`AppleAarch64` + clang link |

## Pipeline Funcional End-to-End

```
archivo.fc → Lexer → Parser → Análisis Semántico → Codegen (Cranelift) → .o → Linker → .exe
```

## Estructura del proyecto Rust

```
src/
├── main.rs              # CLI (clap) — build, run, check, version, lsp
├── span.rs              # Span + Posición
├── error.rs             # Errores en español con códigos alfanuméricos
├── lexer.rs             # Lexer (logos) — tokens, keywords, artículos, operadores, arrays, ser/estar, subjuntivo, alias fn
├── parser/              # Parser modular manual
│   ├── mod.rs           # ParserCursor (lookahead, spans, recovery, scope de type params), ParserFalcato, tests
│   ├── errores.rs       # ErrorSintaxis con códigos [S###], sugerencias
│   ├── tipos.rs         # parse_articulo(), parse_tipo() incluyendo [T; N], genéricos, bounds
│   ├── expresiones.rs   # Pratt parser + arrays + structs + enums + postfix
│   ├── sentencias.rs    # Variables, asignación, condicionales (ser/estar/subjuntivo), bucles (mientras/para)
│   └── declaraciones.rs # Funciones genéricas, structs, enums genéricos, parámetros con bounds
├── ast.rs               # AST con Span — Expr, Sentencia, Declaración, Tipos, ModoVerbal, ParametroGenerico
├── semantic.rs          # Concordancia Lingüística + Ownership + Arrays + Structs + Enums + Llamadas + Bounds
├── codegen/             # Codegen Cranelift — dividido en submódulos
│   ├── mod.rs           # Codegen struct + helpers (programa, declarar, cache, malloc, etc.)
│   ├── funciones.rs     # compilar_funcion, compilar_funcion_normal, async futures (init/poll/wrapper)
│   ├── sentencias.rs    # compilar_sentencia, bucles, condicionales, con_executor, lanzar hilos
│   ├── expresiones.rs   # compilar_expresion, compilar_llamada, compilar_literal, dispatch builtins
│   ├── builtins.rs      # Todas las funciones built-in (I/O, TCP, canales, texto, vector, diccionario, etc.)
│   ├── generics.rs      # Monomorfización, especialización, sustitución de tipos genéricos
│   └── tipos.rs         # tipo_a_cranelift, tamano_tipo, inferir_tipo, operaciones binarias/unarias
├── codegen_helpers.rs   # Helpers de codegen: BlockBuilder, VariableManager, CFunctionCache, MemoryHelper
├── platform/            # Abstracción multiplataforma
│   ├── mod.rs           # Punto de entrada: current_runtime(), current_registry(), linker_for()
│   ├── registry.rs      # BuiltinRegistry — tabla nombre abstracto → función C por plataforma
│   ├── traits.rs        # PlatformRuntime trait + CodegenCtx
│   ├── linker.rs        # PlatformLinker — configuración del linker por plataforma
│   ├── windows.rs       # Implementación Win32 (CreateMutexW, WaitForSingleObject, etc.)
│   ├── linux.rs         # Implementación POSIX (pthread_mutex, sem_t, clock_gettime)
│   └── macos.rs         # Implementación macOS (AppleAarch64, mach_absolute_time)
├── futuros.rs           # Análisis de futures/async para state machine
├── resolver.rs          # Resolución de módulos e imports
├── backend.rs           # Trait de backend
└── lsp.rs               # Servidor LSP — diagnósticos, autocompletado, hover, go-to-definition
```

## Historial de Fases (resumen)

### ✅ Completadas
- **Fase 1-3:** Core del lenguaje (variables, operaciones, condicionales, bucles, ownership básico) + pulido (spans, recovery de errores, LSP).
- **Fase 3.5:** Arrays (`[T; N]`, literales, `todos`, acceso, asignación a elementos).
- **Fase 4:** Structs con layout C (`estructural Punto { ... }`).
- **Fase 5:** Verificación de llamadas, Ser/Estar (`es`/`está`), Subjuntivo (`fuese`).
- **Fase 6:** Bucle `para` sobre arrays.
- **Fase 7:** Enums tag+union con constructores y pattern matching.
- **Fase 8A:** Const generics (`función longitud<N: Entero32>(...)`).
- **Fase 8B:** Base AST/parser para enums genéricos (`enumeración alguno<T> { ... }`).
- **Fase 8C:** Type generics con bounds (`fn máximo<T que Comparable>(...)`), alias `función`/`funcion`/`fn`, monomorfización por tipo concreto.
- **Cold block optimization para Subjuntivo**: reordenamiento de bloques Cranelift.
- **Ser/Estar real**: `ModoVerbal::Estativo`, bare `está` como truthiness check.
- **Find references en LSP**: handler `references` con traversal completo del AST.
- **Fase 9 / 9.5:** Módulos e imports (`módulo`, `usar`, visibilidad `el`/`la`, multi-archivo, glob imports, type-checking cross-file).
- **Backend trait + Resolver multi-archivo**: abstracción de codegen y resolución de dependencias entre archivos.

### 📋 Pendientes (ver Roadmap más abajo)
- Migración de codegen helpers (Fase 15G)
- Cross-platform support (Fase P2-P4)
- Proyecto ejemplo 500+ líneas (Fase R5)
- Drop automático (Fase R6)

## Features Implementadas

### ✅ Core del lenguaje
- Variables con tipos explícitos (`el x: Entero32 = 10`)
- Operaciones aritméticas con precedencia (`+`, `-`, `*`, `/`, `%`)
- Operaciones de comparación (`==`, `!=`, `<`, `>`, `<=`, `>=`)
- Operadores lógicos (`&&`, `||`, `!`)
- Asignación (`x = expr`) a identificadores y elementos de array
- Retorno (`retornar valor`)

### ✅ Control de flujo
- Condicionales `si` / `sino` con modo indicativo, ser/estar y subjuntivo
- Bucles `mientras`
- Bucles `para` sobre arrays (`para num en nums { ... }`)

### ✅ Ownership (Pilar I)
- `el` = mutable (owned)
- `la` = inmutable (borrowed)
- Verificación en tiempo de compilación
- Errores con sugerencias de artículos
- **`mover x`** — transferencia explícita de ownership
- **`copiar x`** — clone explícito
- **Use-after-move detection** (Nivel 1: `verificado`)
- **Borrow checker gradual**: Nivel 0 (permisivo) → Nivel 1 (`verificado`) → Nivel 2 (`estricto`)
- Error `[O001]` con sugerencia concreta para use-after-move
- **Referencias**: `&T` (inmutable), `&mut T` (mutable), `*ref` (dereferencia)
- **Borrowing rules** (Nivel 2): 1 mutable XOR N inmutables, errores `[O002]`/`[O003]`/`[O004]`
- **Field-level borrowing**: `&mut punto.x` vs `&mut punto.y` no conflictúan (resuelve false positive de Rust)
- **Lifetimes léxicos**: `&dato Texto` en vez de `&'a T` — el lifetime ES el nombre de la variable
- **Regiones**: `región nombre { ... }` — arena allocation determinístico, variables se liberan juntas
- **Self-referential structs**: `&yo T` en campos de struct (resuelve limitación fundamental de Rust)
- **Anotaciones de efecto**: `puro`, `muta(campo)`, `lee(campo)` — el compiler razona entre funciones
- **Branch-aware liveness**: borrows mueren por rama del CFG (resuelve limitación de Rust)
- **Artículos extendidos**: `los` = shared ownership (reference-counted), `las` = shared borrowed (solo lectura)
- **Feedback educativo**: errores con múltiples opciones de fix (opción A, B, C)

### ✅ Semántica
- Verificación de tipos ("disconcordancia")
- Detección de variables no declaradas
- Verificación de retornos
- Verificación de condiciones Booleanas
- Verificación de firmas de funciones (cantidad y tipos de argumentos)
- Bounds declarativos (`T que Comparable` / `T que Ordenable`)

### ✅ Spans reales
- Span en cada nodo AST: expresiones, sentencias, declaraciones, bloques
- Spans combinados: expresiones binarias/unarias cubren todo el operando
- Spans de funciones, parámetros, structs, enums

### ✅ Pulido de calidad
- **Lexer**: errores léxicos reportados con span real
- **Parser**: recovery de errores con sincronización hasta siguiente declaración
- **Semantic**: constantes nombradas para códigos de error
- **Semantic**: mensajes de error en español con metáfora de "concordancia gramatical"
- **Codegen**: spans reales en errores, IDs únicos para strings

### ✅ LSP (Language Server Protocol)
- **Diagnósticos en tiempo real**: lexer + parser + semántica al escribir
- **Spans reales**: errores subrayados con ubicación exacta
- **Autocompletado**: keywords, artículos (el/la/un), tipos primitivos
- **Hover information**: tipo y artículo de variables al pasar el cursor
- **Go to definition**: saltar a la declaración de variables y funciones
- **Índice semántico**: construido desde el AST para navegación rápida
- **Find references**: navegación a todas las referencias de un símbolo
- **Comunicación stdio**: compatible con VS Code, Vim, Emacs

### ✅ Arrays
- Tipo `[T; N]`: `los nums: [Entero32; 5]`
- Literal array: `[1, 2, 3]`
- Inicialización `todos expr`: `todos 0`
- Acceso por índice: `nums[0]`, `nums[i]`
- Asignación a elementos: `nums[2] = 30`
- Stack allocation, aritmética de punteros I64

### ✅ Structs
- Declaración: `estructural Punto { x: Entero32, y: Entero32 }`
- Inicialización: `el p: Punto = Punto { x: 10, y: 20 }`
- Acceso a campos: `p.x`, `p.y`
- Layout C con alineación automática
- Verificación semántica completa

### ✅ Enums
- Declaración: `enumeración Estado { Activo, Inactivo }`
- Variantes con datos: `Exito(valor: Entero32)`
- Constructor: `Estado.Activo`, `Resultado.Exito(42)`
- Pattern matching: `si estado es Estado.Activo { ... }`
- Layout tag+union en codegen (I32 tag + datos)
- Verificación semántica: variantes existen, tipos concuerdan
- Base AST/parser para enums genéricos (`enumeración alguno<T> { ... }`)

### ✅ Generics
- **Const generics**: `función longitud<N: Entero32>(los nums: [Entero32; N]) -> Entero32`
- **Type generics**: `fn máximo<T que Comparable>(el a: T, el b: T) -> T`
- **Bounds declarativos**: `que Comparable`, `que Ordenable`
- Monomorfización en punto de llamada (const + type)
- Inferencia de type params desde tipos de argumentos

### ✅ Ser/Estar + Subjuntivo
- `si x es 5` — identidad permanente (==)
- `si x está 10` — estado temporal (==; semántica de estado futura)
- `si x está { ... }` — bare está: truthiness en enteros/booleanos/punteros
- `si x fuese es 100` — subjuntivo, cold path semántico
- `ModoVerbal::Indicativo | Estativo | Subjuntivo` en AST
- Cold block: subjuntivo reordena ramas (else hot path en línea, then cold path afuera)

### ✅ Módulos e imports (Fase 9 / 9.5)
- Declaración de módulos (`módulo nombre { ... }`)
- Importación explícita (`usar modulo::simbolo`)
- Importación glob (`usar modulo::*`)
- Visibilidad con artículos (`el función` = pública, `la función` = privada)
- Default: top-level de archivo = público; dentro de módulo = privado
- Namespaces y resolución de nombres con spans
- Compilación multi-archivo desde CLI (`falcato build a.fc b.fc`)
- Codegen unificado: todos los módulos en un solo ObjectModule
- Type-checking cross-file real con mapa de símbolos públicos compartido

### ✅ Traits / rasgos (Fase 13)
- Declaración de rasgos: `rasgo Nombre { función ...; ... }`
- Implementación: `implementar Rasgo para Tipo { función ... { ... } ... }`
- Verificación semántica: rasgo existe, métodos requeridos están presentes
- Error `[T060]` si rasgo no existe
- Error `[T061]` si falta método requerido
- Base para bounds reales (`que Comparable` como trait lookup)

### ✅ Método syntax generalizada + Operadores compuestos (Fase 15F)
- `MetodoBitwise` → `Metodo` generalizado: `t.agregar()`, `t.tam()`, `t.liberar()`, `v.agregar()`, etc.
- Tabla de dispatch por tipo: Texto, Vector, Enteros (bitwise)
- `+` para Texto: `a + b` → `texto_concatenar(a, b)`
- `[]` para Texto: `t[0]` → `texto_obtener_byte(t, 0)`, `t[0..5]` → `texto_subtexto(t, 0, 5)`
- `[]` para Vector: `v[0]` → `vector_obtener(v, 0)`
- Aliases: `devolver`/`retornar`, `emparejar`/`coincidir`, `decir`/`imprimir_linea`, `texto_tam`/`vector_tam`
- Bypass polimórfico para `decir` en semántica
- Removido `t.desde()` de tabla de métodos (sin sentido semántico)

### ✅ Bitwise + I/O + Interpolación (Fase 14A)
- Operadores bitwise type-safe: `& | ^ << >> ~ >>>`
- Precedencia C estándar: `|| < && < | < ^ < & < == < < < << < + < *`
- `>>>` shift lógico derecho (zero-fill siempre, incluso para signed)
- Type-safe: solo enteros, error `[T001]` con sugerencia si se usa en float/bool
- Built-ins I/O: `imprimir(msg)`, `imprimir_linea(msg)` — sin FFI manual
- String interpolation: `imprimir_linea("x = {x}, y = {y}")` — type-aware
- `tamaño_de::<T>()` — sizeof comptime (Entero8=1, Entero32=4, [T;N]=N*sizeof(T))
- Sintaxis genérica directa: `nombre::<Tipo>(args)` (sin ruta de módulo)
- Linker: `legacy_stdio_definitions.lib` para printf en Windows SDK moderno

### ✅ Tooling
- CLI con 5 comandos (build, run, check, lsp, version)
- Script `build.ps1` automático
- 40 tests unitarios pasando
- Folder `ejemplos/` limpio (solo `.fc`)

## Comandos CLI

```bash
falcato build <archivo.fc>    # Compila a binario nativo .exe
falcato run <archivo.fc>      # Compila y ejecuta
falcato check <archivo.fc>    # Solo análisis (lexer + parser + semántica)
falcato lsp                   # Inicia servidor LSP (stdio)
falcato version               # Muestra versión
```

## Convenios

- Código fuente: `.fc`
- Documentación en español (salvo código/comentarios técnicos internacionales)
- Nombres de funciones/tipos: español, snake_case
- Constantes: NOMBRE_MAYUSCULA
- Código Rust del compilador: inglés
- Versión: SemVer hasta 1.0

## Estado actual

**Fase 18A-18D COMPLETADA (Async: threads + TCP + canales + thread pool + cancelación).**
Pipeline end-to-end operativo. Turing-completo con variables, operaciones aritméticas,
condicionales, bucles, ownership básico, **arrays**, **structs**, **enums**,
**verificación de tipos en llamadas**, **Ser/Estar** (`es`/`está`),
**Subjuntivo** (`fuese`), **`para`**, **const generics**, **type generics con bounds**,
**módulos e imports** (`módulo`, `usar`, visibilidad `el`/`la`, multi-archivo),
**strings heap (`Texto`)**, **vectores dinámicos (`Vector<T>`)**, **FFI a C runtime**,
**manejo de errores (`Resultado<T,E>`, operador `?`, pattern matching con `como`)**,
**ownership con `mover`/`copiar`**, **use-after-move detection (Nivel 1: `verificado`)**,
**borrow checker gradual (Nivel 0/1/2)**,
**referencias (`&T`, `&mut T`, dereferencia `*ref`)**,
**borrowing rules (1 mut XOR N inmut, Nivel 2: `estricto`)**,
**field-level borrowing (`&mut punto.x` vs `&mut punto.y` no conflictúan)**,
**lifetimes léxicos (`&nombre T` en vez de `&'a T`)**,
**regiones (`región nombre { ... }` — arena allocation determinístico)**,
**self-referential structs (`&yo T` en campos de struct)**,
**anotaciones de efecto (`puro`, `muta(campo)`, `lee(campo)`)**,
**branch-aware liveness (borrows mueren por rama del CFG)**,
**artículos extendidos (`los` = shared ownership, `las` = shared borrowed)**,
**feedback educativo (errores con múltiples opciones de fix)**,
**traits/rasgos (`rasgo`, `implementar`, verificación de impls)**,
**bitwise operators type-safe (`& | ^ << >> ~ >>>`)**,
**built-ins I/O (`imprimir`, `imprimir_linea` — sin FFI manual)**,
**string interpolation (`imprimir_linea("x = {x}")`)**,
**`tamaño_de::<T>()` (sizeof comptime)**
y servidor LSP completo.
Backend Cranelift generando binarios nativos x86_64.

y **método syntax generalizada** (`.agregar()`, `.tam()`, `.liberar()` en Texto/Vector),
**Diccionario<K,V> + Conjunto<T>** (hash map/set con resize automático y hash probe),
**operadores compuestos** (`a + b` concatena Texto, `t[0]`/`t[0..5]` indexa Texto, `v[0]` indexa Vector),
**aliases** (`devolver`/`emparejar`/`decir`/`texto_tam`/`vector_tam`),
**i18n completa de "array" → "arreglo"** en docs y errores de compilador,
y servidor LSP completo con **6 features para agentes**:
  - Autocompletado completo (todos los keywords sin "próximamente", 60+ items)
  - Signature help (parámetros al tipear `(`)
  - Code actions (quick fixes desde diagnósticos [T001], [O001])
  - Document symbols (outline: funciones, structs, enums, traits)
  - Context-aware completion (variables en scope, campos de struct tras `.`)
  - Hover mejorado (tablas de propiedades, structs/enums/traits)
  - ✅ Integrado con OpenCode vía `opencode.jsonc` (global) + verificado end-to-end
Backend Cranelift generando binarios nativos x86_64.

**Documentación completa de usuario:** GUIA.md (hub) + 15 capítulos en GUIA/ (03-15),
INSTALL.md, REFERENCIA.md (built-ins), ERRORES.md (códigos de error),
más skill `falcato-language` para LLMs en OpenCode.

**🖼️ GUI-1: Ventana nativa Win32 operativa.** MessageBox via FFI directo,
ventana completa con RegisterClassExA + CreateWindowExA + message loop via
[trampolín C](lib/trampolin_win32.c) precompilado. `direccion_de`/`dir_de`
para obtener punteros a funciones. `texto_a_puntero` y `como_entero64` built-ins.
Auto-link de `lib/trampolin_win32.obj` desde `src/main.rs`.
Diseño completo en [`docs/diseno_gui.md`](docs/diseno_gui.md).

**40/40 tests pasan. 50+ ejemplos funcionando. Auditoría completa: 0 crashes.**

### Estado de distribución (v0.3.0 — Alpha)

| Aspecto | Estado |
|---------|--------|
| Release build (6.4 MB, LTO) | ✅ `cargo build --release` produce `falcato.exe` |
| Runtime library (static link) | ✅ `falcato_runtime.lib` — canales, executor, threads |
| CRT estático | ✅ CI con `+crt-static`, release .exe sin dependencias DLL |
| Dependencias externas | ⚠️ Developer local: requiere VCRUNTIME140.dll (o `+crt-static`) |
| GitHub repo | ✅ `CerebroCanibalus/falcato` (privado) |
| GitHub Actions CI | ✅ build + test + end-to-end en push |
| GitHub Actions Release | ✅ tag v* → ZIP automático con runtime.lib |
| VS Code Extension | ✅ VSIX instalable, Falcato Dorado theme |
| LSP para agentes (OpenCode) | ✅ 6 features, integrado globalmente, verificado |
| Platform Layer (Linux/macOS) | ✅ Diseñado, implementado, **no testeado** |
| Falso positivo Defender | ⚠️ Riesgo alto sin firma digital |
| Instalador script | ✅ `install.ps1` interactivo |

## Roadmap hacia un lenguaje productivo

### Fase 9 — Módulos e imports
- Declaración de módulos (`módulo nombre { ... }`)
- Importación (`usar modulo::simbolo`, `usar modulo::*`)
- Visibilidad con artículos (`el función` = pública, `la función` = privada)
- Namespaces y resolución de nombres con spans
- Compilación multi-archivo desde CLI

### Fase 10 — Strings, vectores y allocación dinámica ✅ COMPLETADA
- Tipo `Texto` growable heap-allocado (`{ ptr, len, cap }`)
- Built-ins de Texto: `texto_nuevo()`, `texto_desde(Palabra)`, `texto_agregar(Texto, Palabra)`, `texto_longitud(Texto)`, `texto_liberar(Texto)`
- Tipo `Vector<T>` genérico heap-allocado (`{ ptr, len, cap }`)
- Built-ins de Vector: `vector_nuevo<T>()`, `vector_agregar<T>(Vector<T>, T)`, `vector_obtener<T>(Vector<T>, Entero32)`, `vector_longitud<T>(Vector<T>)`, `vector_liberar<T>(Vector<T>)`
- Literales de string con secuencias de escape: `\\n`, `\\t`, `\\r`, `\\0`, `\\\\`, `\\"`, `\\xNN`
- FFI a C runtime: `malloc`, `free`, `realloc`, `memcpy`, `strlen`
- Linker actualizado: `ucrt.lib`, `vcruntime.lib`, `kernel32.lib` + LIBPATHs de Windows SDK
- Convención de llamada Windows x64 (`WindowsFastcall`) para funciones externas e internas en Windows
- LSP: autocompletado incluye `Texto`
- Ejemplos: `texto_simple.fc`, `vector_simple.fc`, `hola_mundo.fc` (`puts`)

### Fase 11 — Manejo de errores ✅ COMPLETADA
- Tipo `Resultado<T, E>` (enum genérico built-in)
- Operador de propagación `?` (extrae valor de `Exito`, propaga `Error`)
- Pattern matching con `es Enum.Variante como variable`
- Constructores: `Resultado.Exito(valor)`, `Resultado.Error(codigo)`
- Verificación semántica: `?` requiere `Resultado<T,E>`, binding verifica tipo
- Codegen: `?` extrae campo de datos; `es` compara tag
- Ejemplos: `resultado_simple.fc`, `operador_interrogacion.fc`

### Fase 12 — "Concordancia de Posesión" (Ownership + Borrow Checking) ✅ COMPLETADA
> Diseño completo: `docs/diseno_ownership.md`
> Tesis: superar a Rust en ergonomía sin sacrificar seguridad, aprovechando
> gramática española + tipos afines + análisis gradual.

**Innovaciones clave (7):**
1. **Lifetimes léxicos**: `&dato Texto` en vez de `&'a T` — el lifetime ES el nombre de la variable
2. **Borrow checker gradual**: Nivel 0 (default, permisivo) → Nivel 1 (`verificado`) → Nivel 2 (`estricto`)
3. **Field-level borrowing**: `&mut punto.x` y `&mut punto.y` no conflictúan (resuelve false positive de Rust)
4. **Branch-aware analysis**: borrows mueren por rama del CFG (resuelve limitación de Rust)
5. **Self-referential structs**: `&yo` + regiones de memoria (resuelve limitación fundamental de Rust)
6. **Anotaciones de efecto**: `puro`, `muta(campo)`, `lee(campo)` — el compiler razona entre funciones
7. **Feedback educativo**: errores con múltiples opciones de fix, no solo "no compila"

**Sintaxis de ownership (artículos extendidos):**
```falcato
el x: T = ...;           // owned, mutable (affine: usar 0 o 1 veces)
la x: T = ...;           // borrowed, inmutable (no-lineal: usar N veces)
los x: T = ...;          // shared ownership (reference-counted, como Arc)
las x: &T = ...;         // shared borrowed
un x: T = ...;           // optional (puede ser nulo)
mover x a f;             // transferencia explícita de ownership
copiar x;                // clone explícito
prestar &x a f;          // borrow explícito
```

**Sub-fases:**
- **12A**: ✅ Moves + `mover`/`copiar` + use-after-move detection (Nivel 1: `verificado`)
- **12B**: ✅ Referencias (`&T`, `&mut T`, `*ref`) + Borrowing rules + Field-level
- **12C**: ✅ Lifetimes léxicos (`&nombre T`) + Elisión + Compatibilidad
- **12D**: ✅ Regiones (`región { ... }`) + Self-referential (`&yo`) + Arena allocation
- **12E**: ✅ Efectos (`puro`, `muta(campo)`, `lee(campo)`) + Branch-aware liveness
- **12F**: ✅ Artículos extendidos (`los`/`las`) + Feedback educativo

**Limitaciones conocidas:**
- Drop automático no implementado (requiere análisis de CFG, se hará en 12B)
- `prestar` parseado pero no implementado en codegen
- Nivel 2 (`estricto`) implementado para borrows + lifetimes

**Para kernels:** regiones = arena allocation determinístico, stack-only por defecto,
`inseguro` para hardware, `pre-` para comptime, sin GC.

**Para IA:** Nivel 0 siempre compila (LLM genera → compiler sugiere → LLM refina),
errores con código+span+fix concreto (parseables), WASM sandbox para ejecución segura.

### Fase 13 — Traits / interfaces lingüísticas ✅ COMPLETADA
- Declaración de rasgos: `rasgo Nombre { función ...; ... }`
- Implementación: `implementar Rasgo para Tipo { función ... { ... } ... }`
- Verificación semántica: rasgo existe, métodos requeridos están presentes
- Error `[T060]` si rasgo no existe
- Error `[T061]` si falta método requerido
- Base para bounds reales (`que Comparable` como trait lookup)
- Keywords: `rasgo`, `implementar`, `para`
- AST: `RasgoDecl`, `FirmaMetodo`, `ImplDecl`

### Fase 14 — Sprint de Usabilidad
> Objetivo: que un programador pueda abrir Falcato y hacer algo real sin FFI manual.
> Incluye bitwise operators (crítico para sistemas/kernels).

- **14A**: ✅ Bitwise operators type-safe (`& | ^ << >> ~ >>>`) + Built-ins I/O (`imprimir`, `imprimir_linea`) + interpolación `{x}` + `tamaño_de::<T>()`
- **14B**: ✅ Rangos (`0..10`, `0..=10`) + `para` sobre rangos
- **14C**: ✅ Closures (`|x| expr`) + captura de variables
- **14D**: ✅ Match exhaustivo (`coincidir x { ... }`)
- **14E**: ✅ Testing en el lenguaje (`prueba { afirmar(...) }`)

**Bitwise — diseño concreto:**
```falcato
// Operadores tradicionales (type-safe: mismo ancho obligatorio, sin widening implícito)
a & b      // AND    |  a | b   // OR     |  a ^ b   // XOR
a << n     // shift left  |  a >> n  // shift right (arith signed, logical unsigned)
~a         // NOT   |  a >>> n // shift lógico derecho (zero-fill siempre)

// Built-ins semánticos (zero-cost, 1-2 instrucciones cada uno):
x.poner_bit(3)       // x |= (1 << 3)
x.quitar_bit(3)      // x &= ~(1 << 3)
x.alternar_bit(3)    // x ^= (1 << 3)
x.extraer_bits(4, 8) // (x >> 4) & 0xFF
x.ceros_izquierda()  // clz
x.unos()             // popcount
```

**Innovación clave (Fase 15B): Campos de bits como tipos:**
```falcato
estructural RegistroUART {
    bits {
        habilitado: Natural1,   // bit 0
        modo_tx: Natural2,      // bits 1-2
        baud_div: Natural12,    // bits 3-14
    }
}
// reg.baud_div = 868; → compiler genera shifts+masks. Verifica rango en compile-time.
// LLM genera "reg.baud_div = 868" en vez de calcular máscaras (donde alucina).
```

### Fase 15 — Biblioteca estándar
> Innovación: toda función de stdlib declara efectos (`puro`/`muta`/`lee`).
> Ningún otro lenguaje de sistemas hace esto. Permite al compiler razonar
> sobre aliasing, paralelización, y eliminación de código muerto de forma sound.

- **15A**: ✅ Bitwise built-ins (`poner_bit`, `quitar_bit`, `alternar_bit`, `extraer_bits`, `ceros_izquierda`, `unos`)
  - Sintaxis de método en enteros: `x.poner_bit(3)`, `x.unos()`, `x.extraer_bits(4, 8)`
  - AST: `Expresion::MetodoBitwise(receptor, nombre, args, span)`
  - Parser: postfix `.metodo(args)` detectado en `parse_postfix`
  - Semantic: verifica receptor entero + args según método
  - Codegen: desugarea a ops bitwise existentes (ishl, bor, band, bnot, bxor, ushr, clz, popcnt)
  - `tamaño_de`, `alinear_de`, punteros raw (`*T`, `*mut T`) pendiente
- **15B**: ✅ Campos de bits (`bits { }` en structs)
  - Sintaxis: `estructural RegistroUART { bits { habilitado: Natural8, baud_div: Natural16 } }`
  - AST: `CampoBits { nombre, ancho_bits, offset_bits, span }` en `EstructuralDecl.campos_bits`
  - Parser: detecta `bits { }` dentro de struct, extrae ancho de NaturalN/EnteroN
  - Semantic: `InfoStruct.campos_bits`, valida campos en init y acceso
  - Codegen: struct respaldado por un entero (u8/u16/u32/u64 según total bits)
    - Read: `(val >> offset) & ((1 << ancho) - 1)`
    - Write: `reg = (reg & ~(mask << offset)) | ((val & mask) << offset)`
    - `Lugar::Campo(expr, nombre)` para asignación a campos
  - volatile read/write para MMIO pendiente
- **15C**: ✅ String ops + `imprimir` polimórfico
  - `texto_concatenar(a: Texto, b: Texto) -> Texto` — nuevo Texto con a+b (no modifica originales)
  - `texto_subtexto(t: Texto, inicio: Entero32, fin: Entero32) -> Texto` — extrae bytes [inicio, fin)
  - `texto_comparar(a: Texto, b: Texto) -> Entero32` — comparación byte a byte (0=iguales, <0 a<b, >0 a>b)
  - `texto_obtener_byte(t: Texto, indice: Entero32) -> Entero8` — byte en posición dada
  - `imprimir`/`imprimir_linea` ahora polimórficos: aceptan Texto, Entero32, Entero8, Booleano, Palabra
  - Dispatch por tipo inferido: Texto→extrae ptr+puts, Entero→printf %d, Booleano→"verdadero"/"falso"
  - Semántica: bypass de verificación de tipos para imprimir/imprimir_linea (polimórficos)
  - Pattern Cranelift aprendido: loop header NUNCA sellar antes del back-edge; Variable::from_u32 (no ::new)
- **15D**: ✅ File I/O con C runtime (fopen/fread/fwrite/fclose)
  - `archivo_leer(ruta: Palabra) -> Texto` — lee archivo completo, Texto vacío si no existe
  - `archivo_escribir(ruta: Palabra, contenido: Texto) -> Entero32` — escribe (0=ok, -1=error)
  - `archivo_existe(ruta: Palabra) -> Booleano` — verifica existencia
  - Codegen: bloques condicionales con merge (Variable SSA para descriptor resultado)
  - Bug fix crítico: `crear_string_literal` no agregaba null terminator → crash silencioso en puts/printf
  - `imprimir` polimórfico: Booleano imprime "verdadero"/"falso" con bloques condicionales
- **15E**: ✅ Matemáticas (`abs`, `max`, `min`, `raiz`, `potencia`) + Literales flotantes + `imprimir` Flotante64
  - `abs(x: Entero32) -> Entero32` — select con icmp SignedLessThan + ineg
  - `max(a, b: Entero32) -> Entero32` — select con icmp SignedGreaterThan
  - `min(a, b: Entero32) -> Entero32` — select con icmp SignedLessThan
  - `raiz(x: Flotante64) -> Flotante64` — C `sqrt()` (ucrt.lib)
  - `potencia(base, exp: Flotante64) -> Flotante64` — C `pow()` (ucrt.lib)
  - `Literal::Flotante` en codegen: `builder.ins().f64const(n)`
  - `imprimir` Flotante64: bitcast F64→I64 para Windows x64 variadic ABI (printf %f)
  - Colecciones (`Diccionario`, `Conjunto`) pendiente — requiere hash + heap + generics reales

**Stdlib con efectos — ejemplo real:**
```falcato
puro función longitud(la t: Texto) -> Entero64;           // sin side effects
muta(ptr, len, cap) función agregar(el t: Texto, la s: Palabra);  // muta exactamente estos campos
lee(fd) función escribir(el fd: Entero32, la datos: &[Entero8]) -> Resultado<Entero32, Entero32>;
```

### Fase 16 — Documentación
- Libro oficial (estilo Rust Book): hola mundo → ownership → generics → async
- Referencia del lenguaje completa
- Guía de migración desde C/Rust/Python
- Documentación de errores: cada código con explicación y fix

### Fase 17 — Package manager
- Manifiesto `Falcato.toml`
- Dependencias locales y remotas (git / registry)
- Comandos: `nuevo`, `agregar`, `construir`, `probar`, `publicar`

### Fase 18 — Async / futuro
> Diseño completo: `docs/diseno_async.md`
> Ventaja Falcato: `&yo` resuelve Pin, `región` da arena para tareas, `puro`/`muta` razona seguridad.

- **18A**: ✅ MVP async con threads reales del OS
  - `fut función` = async fn, `esperar expr` = await, `lanzar expr` = spawn
  - `bloquear(expr)` = bridge sync→async (error [T084] dentro de fut función)
  - [T080]: `esperar` solo dentro de `fut función`
  - `lanzar` → CreateThread real con wrapper `__hilo_N` + buffer heap para args
  - `dormir(ms)` → Sleep de kernel32 (MVP, blocking)
  - `esperar`/`bloquear` → pass-through en MVP (sin scheduling real)
  - Concurrencia verificada: dos threads intercalando output
- **18B**: ✅ TCP I/O (Winsock2 directo, blocking por thread)
  - `tcp_vincular(puerto)` → WSAStartup + socket + bind + listen
  - `tcp_aceptar(listener)` → accept (blocking)
  - `tcp_leer(sock, buf, tam)` → recv
  - `tcp_escribir(sock, buf, tam)` → send
  - `tcp_cerrar(sock)` → closesocket
  - Array-to-pointer decay en semantic para buffers
  - `ws2_32.lib` en linker
  - Echo server verificado end-to-end (multi-hilo con `lanzar`)
  - IOCP real pendiente para 18D (multiplexing sin thread-per-conn)
- **18C**: ✅ Canales mpsc + select pattern
  - `canal_nuevo(cap)` → CreateMutexW + CreateSemaphoreW + ring buffer heap
  - `canal_enviar(canal, valor)` → lock, write, unlock, ReleaseSemaphore
  - `canal_recibir(canal)` → WaitForSingleObject(sem), lock, read, unlock
  - `canal_intentar(canal)` → non-blocking try_recv (timeout=0), sentinel i32::MIN
  - `canal_cerrar(canal)` → CloseHandle × 2 + free
  - Productor-consumidor verificado (lockstep perfecto)
  - Select pattern verificado (polling 2 canales con `canal_intentar`)
  - `seleccionar { }` como sintaxis sugar ✅ (parser + semantic + codegen + LSP)
  - `con_executor(hilos: N)` ✅ — thread pool real (CreateThread workers + ring buffer + mutex/semaphore)
  - `lanzar` dentro de `con_executor` encola al pool (no CreateThread directo)
  - `cancelar()` ✅ — cancelación estructurada (cancelled flag + drain skip + graceful in-flight)
  - Cancelación con `región` integrada: tasks en queue se cancelan, in-flight terminan graceful
- **18D**: ✅ Thread pool + cancelación estructurada
  - `con_executor(hilos: N)` ✅ — thread pool real (CreateThread workers + ring buffer + mutex/semaphore)
  - `lanzar` dentro de `con_executor` encola al pool (no CreateThread directo)
  - `cancelar()` ✅ — cancelación estructurada (cancelled flag + drain skip + graceful in-flight)
  - Cancelación con `región` integrada: tasks en queue se cancelan, in-flight terminan graceful
- **18E**: Optimización (en progreso)
  - Stackless desugaring (state machine) para futuros simples ✅ MVP
    - `futuros.rs`: análisis de await points + liveness (vars que cruzan suspensión)
    - `__init_N(args) -> ptr`: malloc struct + state=0 + store params
    - `__poll_N(ptr) -> i64`: dispatch por state, timer check con GetTickCount64, 0=Pending/1=Ready
    - Wrapper sync: init + poll loop con Sleep(1) (para uso fuera de executor)
    - `esperar dormir(ms)` como punto de suspensión (deadline = GetTickCount64() + ms)
    - Sellado de bloques Cranelift: 1 predecesor → sellar inmediato; loops → sellar después del back-edge
    - Verificado end-to-end: futuro_simple.fc con 2 suspensiones compila y ejecuta correctamente
  - Work-stealing scheduler, arena allocation en región (pendiente)
  - Timer wheel O(1), stack pooling, benchmarks 100K+ tareas (pendiente)
  - Integración con con_executor ✅: `lanzar fut_función` encola futuro (init+poll loop en worker thread)
    - `compilar_hilos_pendientes` detecta `__poll_N`/`__init_N` → genera poll loop en wrapper `__hilo_N`
    - Verificado end-to-end: futuro_executor.fc con 3 tareas en 2 workers, intercalado real

### Fase 19 — Optimización y debug info
- Optimizador de Cranelift activado (`opt_level`)
- Generación de PDB/DWARF
- Cold-block hints para subjuntivo
- Auto-vectorización de loops `puro` sin aliasing (SIMD)
- Profile-guided optimization (futuro)

## Hoja de ruta priorizada (2026)

> El Roadmap original (Fases 9-19) es el plan conceptual. Esta sección
> define **lo que toca ahora**, en orden de prioridad real.

### Fase R1 — Distribución e instalación (RELEASE) ✅ COMPLETADA
| # | Tarea | Archivos | Estado |
|---|-------|----------|--------|
| 1 | Repositorio GitHub creado | `CerebroCanibalus/falcato` | ✅ |
| 2 | `.gitignore` mejorado | 1 | ✅ |
| 3 | `clean.ps1` — limpiar basura compilada | 1 | ✅ |
| 4 | `bundle_dlls.ps1` — bundle VCRUNTIME140 para releases locales | 1 | ✅ |
| 5 | `.github/workflows/ci.yml` — build + test en push | 1 | ✅ |
| 6 | `.github/workflows/release.yml` — build + ZIP en tag | 1 | ✅ |
| 7 | `install.ps1` — instalador interactivo con menú de componentes | 1 | ✅ |
| 8 | `+crt-static` en CI | 1 línea | ⏳ Pendiente |

**Resultado:** Repo listo con CI + instalador interactivo (`install.ps1`). El release build de GitHub Actions produce .exe sin DLLs externas.

### Fase R2 — VS Code Extension ✅ COMPLETADA
| # | Tarea | Archivos | Estado |
|---|-------|----------|--------|
| 1 | `package.json` (extension manifest) | 1 | ✅ |
| 2 | `falcato.tmLanguage.json` (syntax highlighting) | 1 | ✅ |
| 3 | `language-configuration.json` (brackets, comentarios) | 1 | ✅ |
| 4 | `client.js` — LSP client (lanza `falcato lsp`) | 1 | ✅ |
| 5 | `themes/falcato-color-theme.json` (tema "Falcato Dorado") | 1 | ✅ |
| 6 | VSIX empaquetado | `npx vsce package` | ✅ |

**Resultado:** Syntax highlighting + LSP + tema único "Falcato Dorado". Instalar vía VSIX.

### Fase R3 — Documentación completa + Skills para IA ✅ COMPLETADA
| # | Tarea | Archivos | Estado |
|---|-------|----------|--------|
| 1 | `INSTALL.md` — cómo instalar Falcato en 3 pasos | 1 | ✅ |
| 2 | `QUICKSTART.md` — tutorial rápido | 1 | ⏳ (cubierto por GUIA/02, pendiente archivo separado) |
| 3 | `REFERENCIA.md` — catálogo completo de built-ins | 1 | ✅ |
| 4 | `ERRORES.md` — cada código [T###] explicado con causa y fix | 1 | ✅ |
| 5 | Skill `falcato-language` (OpenCode) + reference/builtins.md | 2 | ✅ |
| 6 | `AGENTS.md` como referencia de diseño del proyecto | 1 | ✅ |
| 7 | Carpeta `GUIA/` con 15 capítulos + diagramas + ejemplos reales | 17 | ✅ |
| 8 | i18n completa: "array" → "arreglo" en docs y errores del compilador | 7 | ✅ |

**Resultado:** Documentación completa de usuario (GUIA.md hub + 15 capítulos), referencia (REFERENCIA.md + ERRORES.md), skill para IA (falcato-language con reference/builtins.md), e i18n de términos.

### Fase R4 — Colecciones (Diccionario + Conjunto) ✅ COMPLETADA
| # | Tarea | Archivos | Estado |
|---|-------|----------|--------|
| 1 | `Tipo::Diccionario(K,V)` + `Tipo::Conjunto(T)` en AST | `ast.rs` | ✅ |
| 2 | Hash multiplicativo + probe loop en codegen | `codegen.rs` | ✅ (FNV-1a pendiente) |
| 3 | Built-ins: `diccionario_nuevo/insertar/obtener/existe/eliminar/longitud/liberar` | `semantic.rs`, `codegen.rs` | ✅ + método syntax |
| 4 | Built-ins: `conjunto_nuevo/insertar/contiene/eliminar/liberar` | `semantic.rs`, `codegen.rs` | ✅ (wrapper de diccionario) |
| 5 | Resize automático (realloc + memset) | `codegen.rs` | ✅ (duplica al llenar) |
| 6 | Hash probe con wrap-around | `codegen.rs` | ✅ (open addressing) |
| 7 | LSP autocompletado | `lsp.rs` | ⏳ Pendiente |

**Resultado:** Diccionario<K,V> y Conjunto<T> funcionales con resize y hash probe. ~700 líneas de código nuevo.

### Fase R5 — Proyecto ejemplo 500+ líneas
- [ ] Word counter: lee archivo, tokeniza, cuenta frecuencia con Diccionario, ordena con Vector
- [ ] Valida el pipeline completo: módulos, colecciones, archivos, I/O

### Fase R6 — Drop automático
- [ ] Análisis de CFG para insertar `free` al final de scope
- [ ] Elimina fugas en Texto, Vector, Diccionario, Conjunto, TCP sockets

### Fase R7 — Package manager (post-v1)
- [ ] `Falcato.toml`, dependencias git, comandos `nuevo`/`construir`

### Fase 15G — Migración de codegen helpers
- [ ] Migrar `compilar_funcion()` para usar `BlockBuilder`
- [ ] Migrar variables de closures para usar `VariableManager`
- [ ] Reemplazar `llamar_malloc/free` con `MemoryHelper`
- [ ] Reemplazar funciones duplicadas `tipo_a_cranelift`/`tamano_tipo`

### Fase P1 — Platform Runtime Layer ✅ COMPLETADA
- [x] `BuiltinRegistry` — tabla declarativa nombre → función C por plataforma
- [x] `PlatformRuntime` trait — primitivas de sync + builtins complejos
- [x] `PlatformLinker` — configuración del linker por plataforma
- [x] `WindowsRuntime` — CreateMutexW/CreateSemaphoreW/WaitForSingleObject
- [x] `LinuxRuntime` — pthread_mutex/sem_t/clock_gettime
- [x] `MacOsRuntime` — macOS POSIX (sin testear)

### Fase P2 — Migración de codegen.rs para usar platform/ ✅ (Capas A+B+C completadas para canales)

**Estrategia:** Las operaciones multi-paso con layout de struct que difiere por plataforma
(canales, executor, threads) se mueven a la **Capa A: librería runtime Rust**.
Las primitivas de 1 instrucción (mutex_lock/sem_wait/timestamp) van a la **Capa B: PlatformRuntime trait**.

| Capa | Qué migra | Estado |
|------|-----------|--------|
| ✅ C | `sleep`, `malloc`, `free`, `realloc`, `memcpy`, `strlen` (registry) | Hecho |
| ✅ C | `puts`, `printf`, `sqrt`, `pow`, `fopen/fread/fwrite/fclose` (registry) | Hecho |
| ✅ B | `exit_process`, `sleep`, `timestamp`, `net_init`, `mutex_lock/unlock`, `sem_wait/post/trywait` | Hecho |
| ✅ A | `canal_nuevo`, `canal_cerrar`, `canal_enviar`, `canal_recibir` → `falcato_channel_*` | **HECHO (P5a)** |
| ✅ A | `canal_intentar` → `falcato_channel_try_recv` (nuevo en runtime) | **HECHO (P5a)** |
| ✅ A | `falcato_channel_try_recv(ch, data) -> i32` — non-blocking try_recv | **HECHO** |
| ✅ | Linker: `ntdll.lib` + `userenv.lib` para Rust std de runtime | **HECHO** |
| ⏳ A | `con_executor init` → `falcato_executor_new` | Pendiente |
| ⏳ A | `executor_worker loop` → mover loop a C | Pendiente |
| ⏳ A | `cancelar` → `falcato_executor_cancel/close` | Pendiente |
| ⏳ A | `lanzar` thread fallback → `falcato_thread_run` | Pendiente |

**Resultado P5a:** ~200 líneas de Cranelift IR con Win32 directo (CreateMutexW, CreateSemaphoreW, CloseHandle, mutex_lock/unlock, sem_wait/post, ring buffer arithmetic)
→ **~50 líneas de IR** (5 llamadas a runtime). El layout de canal legacy (una semáforo) fue reemplazado por el correcto layout de la runtime (dos semáforos signal+space con backpressure).

### Fase P3 — Linker cross-platform
- [x] `PlatformLinker` struct con cmd/flags/libs/entry/extra_objs
- [x] Linker para Windows (link.exe), Linux (gcc), macOS (clang)
- [ ] Probar compilación nativa en Linux (WSL)
- [ ] Probar compilación nativa en macOS

### Fase P4 — CI cross-platform
- [ ] Workflow GitHub Actions: `ubuntu-latest` + `windows-latest` matrix
- [ ] `cargo test` cross-platform
- [ ] Compilar + ejecutar hello world en Linux

### Fase P5 — Runtime Library (Capa A)
- [x] Crear `lib/falcato_runtime/` — crate Rust, staticlib (`#![no_std]`→`std` para c_void)
- [x] `falcato_channel_new/send/recv/close/try_recv` — channel con 2 semáforos (signal+space), backpressure, ring buffer
- [x] `falcato_executor_new/submit/cancel/close` — thread pool (creado, no migrado en codegen)
- [x] `falcato_thread_run` — thread creation wrapper (CreateThread / pthread_create)
- [x] `build.ps1`: compila runtime antes del compiler
- [x] `linker.rs` / `main.rs`: añade runtime .lib + ntdll.lib + userenv.lib
- [x] **codegen.rs: migración de CANALES completa** (~200 líneas IR → ~50 líneas con 5 calls a runtime)
  - `builtin_canal_nuevo` → `falcato_channel_new(capacity, 4)` (1 línea IR)
  - `builtin_canal_cerrar` → `falcato_channel_close(ch)` (1 línea IR)
  - `builtin_canal_enviar` → `falcato_channel_send(ch, &val)` (malloc+store+call+free)
  - `builtin_canal_recibir` → `falcato_channel_recv(ch, &val)` (malloc+call+load+free)
  - `builtin_canal_intentar` → `falcato_channel_try_recv(ch, &val)` (call + check ret_code + load/free)
- [x] **codegen.rs: migración de EXECUTOR + THREADS completa** (~260 líneas IR → ~25 líneas)
  - `compilar_con_executor` → `falcato_executor_new(hilos, capacidad)` + `falcato_executor_close`
  - `compilar_lanzar_hilo` → `falcato_executor_submit` (pool) / `falcato_thread_run` (standalone)
  - `builtin_cancelar` → `falcato_executor_cancel(exec)`
  - `asegurar_executor_worker` → ELIMINADO (runtime maneja workers internamente)
  - `compilar_seleccionar_cadena` → usa `builtin_canal_intentar`
- [ ] **P5c**: Probar en Linux (WSL) — runtime ya tiene stub POSIX en `platform.rs`
  - `asegurar_executor_worker` → ELIMINADO (runtime maneja workers internamente)
  - `compilar_seleccionar_cadena` → usa `builtin_canal_intentar` (que va a runtime)
  - ~260 líneas de IR con CreateThread/WaitForSingleObject/ReleaseMutex/CreateSemaphoreW → **~25 líneas** (4 llamadas a runtime)

## Checklist para release v0.3.0

- [x] `+crt-static` en CI (GitHub Actions tiene VS Build Tools completo)
- [x] Release build limpio (CI produce .exe sin DLLs externas)
- [x] Script `bundle_dlls.ps1` para releases locales
- [x] GitHub repo creado con README y LICENSE
- [x] GitHub Actions CI (build + test)
- [x] **Tag v0.3.0 creado y release generado**
- [x] VS Code Extension (syntax + LSP + theme Falcato Dorado)
- [x] Documentación básica (INSTALL + REFERENCIA + ERRORES)
- [x] GUIA completa (15 capítulos, ownership, arreglos, funciones, errores)
- [x] Skill `falcato-language` para LLMs (con reference/builtins.md)
- [x] Diccionario + Conjunto implementados
- [ ] Proyecto ejemplo >500 líneas funcionando (R5)
- [ ] Falso positivo reportado a Microsoft Security Center
- [x] Script `install.ps1` probado en máquina limpia
- [x] Codegen helpers (`src/codegen_helpers.rs`) — BlockBuilder, VariableManager, CFunctionCache, MemoryHelper
- [x] Platform Runtime Layer (`src/platform/`) — BuiltinRegistry, PlatformRuntime, PlatformLinker + Windows/Linux/macOS
- [x] **Runtime Library (`lib/falcato_runtime/`)** — canales, executor, threads migrados a Rust staticlib
- [x] **CI actualizado** — compila runtime + test end-to-end de ejemplos
- [x] **Release incluye runtime .lib** — `falcato_runtime.lib` en el ZIP
- [ ] **Soporte Linux (WSL)** — Pendiente, busca colaborador. Ver Fase P5c en AGENTS.md

## Curva de aprendizaje (diseño vinculante)

Falcato es un lenguaje de bajo nivel (kernel-capable como C/Rust) pero con curva
gradual. El programador sube de nivel cuando está listo, no cuando el compiler lo exige.

```
Nivel 0 (default — permisivo, como C):
    Todo compila. El compiler SUGIERE pero no rechaza.
    Un principiante escribe código C-like y funciona.

Nivel 1 (verificado — como Rust gentil):
    Use-after-move detection. Errores educativos con opciones A/B/C.
    El compiler enseña ownership con feedback, no con rechazo.

Nivel 2 (estricto — como Rust completo):
    Borrow checker completo. 1 mut XOR N inmut. Lifetimes verificados.
    Pero con &yo, región, puro/muta como herramientas EXTRA que Rust no tiene.
```

**Para kernels**: Nivel 2 + `inseguro` para hardware + `región` para arena determinístico.
**Para LLMs**: Nivel 0 siempre compila → compiler sugiere → LLM refina → <3 iteraciones a Nivel 2.
**Para humanos**: Python → C → Rust, en ese orden, en UN lenguaje.

## Criterio de "listo para usar"

Falcato será un lenguaje que valga la pena usar cuando:
1. Exista un proyecto de >500 líneas compilable en varios archivos.
2. Tenga stdlib suficiente para I/O, strings y colecciones.
3. Tenga manejo de errores sin caer en `retornar 1` manual.
4. El borrow checker evite fugas de memoria sin GC.
5. Haya documentación clara para un programador hispanohablante nuevo.
6. Un programador de sistemas pueda manipular registros hardware sin FFI manual.

## Criterio de "superar a Rust"

Falcato supera a Rust cuando:
1. Un programador hispanohablante escribe un linked list **sin pelear con el compiler**.
2. Un LLM genera código que compila en Nivel 0 y pasa a Nivel 2 con <3 iteraciones.
3. Un kernel module se escribe en Falcato con **menos líneas** que en Rust equivalente.
4. Los errores de ownership se entienden **sin leer documentación**.
5. Self-referential structs funcionan sin workarounds vergonzosos.
6. Un LLM genera código de bit manipulation **sin alucinar máscaras** (campos de bits como tipos).
7. El compiler auto-vectoriza loops `puro` sin `unsafe` (Rust no puede hacer esto de forma sound).

Hasta entonces, sigue siendo un demostrador técnicamente sólido y una plataforma de experimentación lingüística que explora el nicho de **toolchain nativa para código generado por IA**.

## Agente experto de OpenCode

Este archivo es la referencia de diseño del proyecto. Para la versión ultra-compacta usada por el agente experto de OpenCode, ver:

`C:\Users\Lord Gatito\.config\opencode\agents\falcato.md`
