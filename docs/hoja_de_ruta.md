# Falcato — Plan Central

> *"El mayor poder a la mayor rapidez y siendo baratos."*

---

## Manifiesto Técnico

Falcato es un lenguaje de programación de sistemas construido desde cero sobre LLVM.
No es una traducción de Rust al español. Es un diseño original donde las dimensiones
semánticas del español (género, ser/estar, tiempos verbales, prefijos) se convierten
en *garantías de compilación*: ownership, const/mut, modos de ejecución, y semántica
de operaciones.

**Nuestras convicciones:**

1. **El español puede ser un mejor sistema de tipos que el inglés para 500M de hablantes.**
   El género gramatical, ser/estar, y los tiempos verbales son conocimiento preexistente
   que ningún otro lenguaje explota.

2. **C ABI por defecto es la decisión más pragmática.** El 100% del ecosistema de
   sistemas habla C. Cualquier barrera FFI es una barrera de adopción.

3. **Ultra-rápido no es solo velocidad de CPU.** Es control de memoria, de layout,
   de SIMD, de allocators, y de GPU. Falcato ataca todos estos frentes desde el
   diseño del lenguaje, no desde bibliotecas externas.

4. **Lo que no se usa, no se paga.** Cero abstracciones gratuitas. Cada feature
   del lenguaje debe poder implementarse con costo cero en runtime.

---

## Registro de Decisiones (con justificación)

### D-001: C ABI por defecto

**Decisión:** Toda la salida del compilador usa ABI de C. Layout de structs = C layout.
Calling convention = C. Name mangling = desactivado.

**Por qué:** El análisis externo identificó "Interoperabilidad C/FFI" como desafío crítico
que decide éxito o fracaso. Rust requiere `#[repr(C)]` y `extern "C"` como opt-in, lo que
crea fricción. Falcato nace con esto como default para que cualquier `.o` sea directamente
usable desde C, y cualquier biblioteca C sea llamable sin wrappers.

**Evidencia:** "Todo ecosistema de sistemas habla C. Si Falcato no puede consumir headers C
fácilmente, es una isla inútil." — Análisis estratégico, 2026.

**Alternativas descartadas:**
- Opt-in como Rust: añade fricción FFI sin beneficio para el caso base
- ABI propia: incompatible con el ecosistema existente

### D-002: Span en cada nodo del AST

**Decisión:** Cada nodo del AST lleva `Span { inicio, fin, archivo }`.

**Por qué:** Sin Span no hay errores con ubicación, no hay LSP, no hay tooling.
Es un requisito no negociable que debe estar desde el día 1 porque añadirlo después
requiere reescribir el AST entero.

**Evidencia:** La experiencia de Rust con `Span` en rust-analyzer demuestra que
hacerlo después es dramáticamente más costoso.

### D-003: Stack técnico: logos + chumsky + inkwell

**Decisión:**
- Lexer: `logos` (generación type-safe de tokens)
- Parser: `chumsky` (parser combinators con errores recuperables)
- Backend: `inkwell` (bindings Rust para LLVM)
- CLI: `clap` (derive-based argument parsing)
- LSP: `tower-lsp` (servidor LSP en Rust)
- Testing: `insta` (snapshot testing de IR)

**Por qué:**
- `logos` es la biblioteca de lexer más madura en Rust, genera token streams
  type-safe sin necesidad de escribir el lexer a mano
- `chumsky` permite errores recuperables en el parser, crítico para LSP
  (necesitamos parsear código incompleto sin cascada de errores)
- `inkwell` es el binding más completo a LLVM en Rust, mismo stack que Rust/Clang
- `tower-lsp` es el estándar industrial para servidores LSP en Rust

**Alternativas descartadas:**
- `lalrpop`: más rápido pero errores no recuperables, inviable para LSP
- `pest`: PEG parser, pero chumsky tiene mejor soporte de errores
- `LLVM bindings manuales`: inkwell es más seguro y probado

### D-004: Errores en español con códigos

**Decisión:** Todos los mensajes de error del compilador en español, con formato
`[F001] archivo.fc:7:12: mensaje`.

**Por qué:** La identidad del proyecto es iberohablante. Los errores en español
no son traducción — son el idioma nativo del lenguaje. Esto incluye hispanohablantes
que tienen barreras con el inglés técnico.

**Formato:** `[CódigoCategoría][Número] archivo.fc:línea:columna: mensaje`

### D-005: 5 pilares (no más, no menos)

**Decisión:** El lenguaje se define por exactamente 5 pilares. No 3, no 13.

**Por qué:** El análisis advirtió explícitamente: "Un lenguaje con 3 innovaciones
profundas vence a uno con 13 innovaciones superficiales." Elegimos 5 porque son
los que tienen impacto real y mensurable:
1. Género = Ownership (la innovación flagship)
2. Ser/Estar = Const/Mut (única del español)
3. Tiempos verbales = Modos de ejecución (la más compleja pero más potente)
4. C ABI por defecto (la más pragmática)
5. Prefijos semánticos (la más expresiva)

**Descartado para day-0:**
- Diminutivos/aumentativos: azúcar, no semántica profunda
- Voz pasiva: elegante pero implementable después
- Dialectos regionales: complejidad sin beneficio claro
- Conectores ricos (`a_menos_que`, `con_tal_que`): simpler sugar que se añade después

### D-006: EML descartado para runtime ultra-rápido

**Decisión:** EML no se implementa como primitiva matemática de runtime.

**Por qué:** Tras investigar el paper de Odrzywołek (arXiv 2603.21852, marzo 2026)
y probar su implementación en Rust y otros lenguajes, se determinó:
- Cada operación aritmética requiere 20-140 llamadas a `exp()` y `ln()`
- `x + y` = profundidad 27 = 27 exp+ln por suma
- `sin x` = profundidad 100+ = inviable
- Error numérico se acumula: profundidad 4 ya da error `10^-14`
- Depende de números complejos para funciones trigonométricas

**Lo que SÍ haremos con EML:** Usar EML en compile-time para symbolic regression
(recuperar fórmulas cerradas de datos numéricos) como utilidad opcional de
la biblioteca estándar en Fase 5+.

### D-007: 5 ideas novel para ultra-rápido

**Decisión:** En lugar de EML, proponemos 5 innovaciones originales para
rendimiento extremo en gráficos/rendering:

1. **Polimorfismo de layout** (SoA/AoS automático vía anotaciones)
2. **Género como hint SIMD** (gather vs dense según artículo)
3. **Tiempo verbal como ancho SIMD / modo compute**
4. **Artículo como selector de allocator** (`el_frame`, `el_temp`)
5. **Auto-diferenciación** para rendering inverso (como Slang, pero integrado)

**Por qué son novel:** Ningún lenguaje existente implementa estas ideas.
Jai (Jonathan Blow) se acerca con compile-time reflection para cambiar layouts,
pero no como feature del sistema de tipos. Slang (NVIDIA) tiene auto-diff pero
solo para shaders, no como feature del lenguaje base.

---

## Los 5 Pilares (con justificación técnica)

### Pilar I: Género = Ownership

**Tesis:** Los artículos definidos e indefinidos del español codifican naturalmente
el régimen de ownership y préstamo de memoria.

| Artículo | Semántica | Intuición hispanohablante |
|----------|-----------|--------------------------|
| `el` | owned, mutable | "el coche **de mi propiedad**, lo modifico" |
| `la` | borrowed, inmutable | "la referencia que **observo**, no la poseo" |
| `un` | optional | "un archivo **quizás exista**, quizás no" |
| `los` | colección owned | "los discos **que me pertenecen**" |
| `las` | colección prestada | "las entradas **que me prestaron**" |

**Por qué es óptimo sobre Rust:**
- Rust requiere aprender 3 conceptos nuevos (ownership, borrowing, lifetimes)
- Falcato explota 1 concepto preexistente (género gramatical)
- La curva de aprendizaje se aplana dramáticamente para hispanohablantes

**Por qué es óptimo sobre C:**
- C no tiene ningún sistema de ownership
- Errores de memoria (use-after-free, double-free) son la fuente #1 de CVEs

**Por qué es óptimo sobre Zig:**
- Zig deja la gestión de memoria completamente al programador
- No hay garantías de compilación para evitar errores de ownership

### Pilar II: Ser/Estar = Constancia/Transitoriedad

**Tesis:** La distinción única del español entre ser (esencia) y estar (estado)
se mapea directamente a compile-time (ser) vs runtime (estar).

```falcat
ser TAMAÑO: Entero32 = 4096;       // "es" - esencia, no cambia → comptime
estar buffer: [Byte; 4096];        // "está" - estado, puede cambiar → runtime mut
```

**Por qué es óptimo:**
- Ningún otro lenguaje tiene esta distinción
- `const` en C es un calificador, no una semántica de permanencia
- `let`/`let mut` en Rust son runtime ambos; `const` es aparte
- En español la diferencia es intuitiva: "soy alto" (ser=siempre) vs "estoy cansado" (estar=ahora)

### Pilar III: Tiempos verbales = Modos de ejecución

**Tesis:** La conjugación del verbo determina el modo de ejecución sin necesidad
de keywords como `async`, `Result`, `Iterator`, `unsafe`.

| Tiempo | Ejecución | Sin esto necesitarías... |
|--------|-----------|--------------------------|
| Presente `procesa` | Síncrono | `fn proceso()` |
| Futuro `procesará` | Async | `async fn proceso()` |
| Subjuntivo `procese` | Fallible | `fn proceso() -> Result<T, E>` |
| Imperfecto `procesaba` | Generador | `fn proceso() -> impl Iterator` |
| Imperativo `¡procesa!` | Inseguro | `unsafe fn proceso()` |

**Base teórica:** El paper "Practical Type Inference with Levels" (PLDI 2025,
Distinguished Paper) formaliza el uso de *niveles* en inferencia de tipos.
Aplicamos esto directamente: los tiempos verbales son niveles en el sistema de tipos.
- Nivel 0: Presente (síncrono)
- Nivel 1: Futuro (async)
- Nivel 2: Subjuntivo (fallible)
- Nivel 3: Imperfecto (generador)
- `unsafe`: Imperativo (sin nivel, fuera del sistema)

**Por qué es óptimo:** Reduce 4 conceptos ortogonales de Rust a 1 concepto
preexistente (conjugación verbal). El hablante nativo ya sabe cuándo usar
subjuntivo (incertidumbre) vs presente (certeza).

### Pilar IV: C ABI por defecto

**Tesis:** La ABI de C es el estándar universal de interoperabilidad entre lenguajes.
Falcato no lo trata como opt-in, sino como default.

**Decisiones concretas:**
- Layout de structs: mismo padding y alignment que C (repr(C) por defecto)
- Calling convention: `C`
- Name mangling: desactivado (símbolos literales)
- Tipos primitivos: mapeo 1:1 con C (Entero32=int32_t, Flotante64=double, etc.)
- Salida: `.o` compatible con gcc/clang/link.exe

**Por qué es óptimo sobre Rust:**
- Rust requiere `#[repr(C)]` en cada struct y `extern "C"` en cada función
- En Falcato, simplemente declaras e importas — funciona

**Por qué es óptimo sobre Zig:**
- Zig también es C-compatible, pero requiere `extern struct` para layout explícito
- Falcato es más simple porque C es el default, no una opción

### Pilar V: Prefijos semánticos como primitivas

**Tesis:** Los prefijos productivos del español expresan semántica de operaciones
que el compilador puede entender y optimizar, no solo el programador.

| Prefijo | Semántica | Lo que el compilador hace |
|---------|-----------|--------------------------|
| `re-` | Reintentar | Expande a: `para i en 0..N { intentar; si éxito, romper; }` |
| `des-` | Destruir/Liberar | Inserta `drop`/`free` al salir del scope |
| `pre-` | Pre-calcular | Evalúa en compile-time si los args son conocidos |
| `co-` | Cooperativo | Fork-join, paralelización |
| `sobre-` | Sobrescribir | Semántica atómica de reemplazo |
| `entre-` | Entre-hilos | Operación thread-safe |

**Por qué es óptimo:**
- En Rust/C++, `retry` es una función de biblioteca que el compilador no entiende
- En Falcato, `re-intentar` es una primitiva que el compilador expande y optimiza
- APIs auto-documentadas: `des-cargar(archivo)` no necesita comentario

---

## Las 5 Ideas Novel para Ultra-Rápido

### N-01: Polimorfismo de Layout (SoA/AoS automático)

**Problema:** En gráficos/rendering, Structure of Arrays (SoA) es hasta 10x más rápido
que Array of Structures (AoS) para SIMD, pero el programador escribe más naturalmente
en AoS. Cambiar entre ambos requiere refactor masivo.

**Solución:** El programador escribe en AoS (natural), y una anotación `#[layout]`
le dice al compilador qué layout usar internamente.

```falcat
estructural Triangulo {
    el v0: Vector3, el v1: Vector3, el v2: Vector3,
}

#[layout("soa")]
los triangulos: Lista<Triangulo>;
// Compilador reordena internamente a:
//   struct { Float32 v0_x[], v0_y[], v0_z[], v1_x[], ... }

#[layout("auto")]  // profiling-guided: el compilador elige según access pattern
los vertices: Lista<Vertice>;
```

**Estado del arte:** Investigación activa (DL GPU data layout, PLDI 2025).
Ningún lenguaje de propósito general lo tiene como feature de primera clase.

### N-02: Género como hint SIMD (gather vs dense)

**Problema:** El compilador no sabe si un acceso a memoria será denso (SIMD-friendly)
o disperso (gather necesario). Hoy se resuelve con perfiles manuales o intrínsecas.

**Solución:** El artículo informa al compilador sobre el patrón de acceso.

```falcat
el posiciones: [Vector3; N];    // owned = denso = SIMD lineal directo
la indices: &[Entero32];        // borrowed = pueda ser sparse = preparar gather
```

### N-03: Tiempo verbal como ancho SIMD / modo compute

**Problema:** SIMD, multithreading CPU, y GPU compute son 3 conceptos separados
con APIs diferentes (intrínsecas, std::thread, CUDA/Vulkan).

**Solución:** Un único mecanismo (la conjugación) controla la granularidad:

```falcat
función procesar_pixel(x: Entero32, y: Entero32): Color;           // escalar

función procesarán_pixeles(xs: Vec8<Entero32>, ys: Vec8<Entero32>): Vec8<Color>;
// → SIMD AVX2/NEON width=8

nosotros lanzamos_rayos(escena: &Escena, imagen: &mut Imagen);
// → CPU multithread o Vulkan compute o CUDA según target
```

### N-04: Artículo como selector de allocator

**Problema:** En motores de juegos, el 90% de las asignaciones son temporales
(frame allocator) y no deberían pasar por malloc. Hoy se resuelve con
bibliotecas externas y convenciones manuales.

**Solución:** El sufijo del artículo selecciona la región de memoria:

```falcat
el normal: Vector3;                    // heap/stack default
el_frame posicion: Vector3;            // frame allocator (se limpia cada frame)
el_temp buffer: [Byte; 256];          // arena temporal (se limpia al salir)
el_persistente cache: Mapa<K, V>;     // memoria persistente
```

### N-05: Auto-diferenciación para rendering inverso

**Problema:** El rendering diferenciable (neural graphics, inverse rendering) requiere
implementar forward + backward pass manualmente o usar Slang (NVIDIA).

**Solución:** Falcato incorpora auto-diferenciación como feature del lenguaje:

```falcat
diferenciable función renderizar(escena: &Escena, params: &mut Parametros): Imagen {
    // Compilador genera forward + backward pass automáticamente
    // Integrable con PyTorch vía FFI
}
```

---

## Arquitectura del Compilador

### Diagrama de flujo

```
.fc
  │
  ▼
┌─────────────┐
│   Lexer     │ logos — tokenización type-safe
│             │ cada token conserva Span
└─────────────┘
  │ stream de tokens
  ▼
┌─────────────┐
│   Parser    │ chumsky — parser combinators
│             │ errores recuperables (crítico para LSP)
└─────────────┘
  │ AST con Span en cada nodo
  ▼
┌──────────────────────────────┐
│   Análisis Semántico         │
│                              │
│  ┌──────────┐ ┌───────────┐ │
│  │ tipos.rs │ │tiempos.rs │ │ — Levels-based type inference
│  └──────────┘ └───────────┘ │
│  ┌──────────┐ ┌───────────┐ │
│  │ownership │ │ layout.rs │ │ — SoA/AoS lowering ★
│  └──────────┘ └───────────┘ │
│  ┌──────────┐ ┌───────────┐ │
│  │memoria.rs│ │  simd.rs  │ │ — SIMD width inference ★
│  └──────────┘ └───────────┘ │
└──────────────────────────────┘
  │ FAL IR (intermedio propio)
  ▼
┌──────────────────────────────┐
│   Optimizaciones FAL IR      │
│  • Inlining de prefijos      │
│  • SoA lowering              │
│  • Comptime plegado (ser)    │
│  • Eliminación de préstamos  │
└──────────────────────────────┘
  │
  ▼
┌──────────────────────┐
│    Codegen LLVM      │ inkwell
│  • C ABI por defecto │
│  • Auto-vectorización│
│  • Targets: x86_64,  │
│    ARM, WASM         │
└──────────────────────┘
  │
  ▼
┌──────────────────────┐
│   Linker + Salida    │
│  • .o compatible C   │
│  • link.exe / ld     │
│  • Binario nativo    │
└──────────────────────┘
```

### Por qué esta arquitectura es óptima

1. **Separación clara de concerns:** Cada fase del pipeline es independiente,
   reemplazable, y testeable por separado.

2. **LSP comparte gramática con el compilador:** Al usar logos+chumsky con Span,
   el LSP puede parsear código incompleto (el parser de chumsky es recuperable)
   y obtener los mismos AST nodes que el compilador.

3. **FAL IR como capa de innovación:** Las optimizaciones específicas de Falcato
   (SoA lowering, inlining de prefijos, niveles de tiempo verbal) se aplican en
   IR propio antes de bajar a LLVM, sin depender de que LLVM las entienda.

4. **LLVM como backend probado:** No reinventamos la rueda del codegen. LLVM da
   optimizaciones de clase mundial, targets para todas las arquitecturas, y
   un ecosistema maduro de tooling.

5. **C ABI en la salida:** Al generar código C-compatible desde el primer día,
   cualquier `.o` de Falcato se integra en proyectos C/C++/Zig/Rust sin fricción.

### Estructura del proyecto Rust

```
src/
├── main.rs           # CLI con clap — falcato build/run/check/lsp
├── span.rs           # Span, Posicion — localización en código fuente
├── error.rs          # Errores en español con códigos
├── lexer.rs          # Tokenizer con logos
├── parser.rs         # Parser con chumsky (errores recuperables)
├── ast.rs            # Nodos del AST con Span
├── semantic/         # Análisis semántico
│   ├── mod.rs
│   ├── tipos.rs      # Sistema de tipos
│   ├── ownership.rs  # Artículos → ownership (el/la/un/los/las)
│   ├── tiempos.rs    # Tiempos verbales → niveles de ejecución
│   ├── layout.rs     # ★ NUEVO: inferencia SoA/AoS
│   ├── memoria.rs    # ★ NUEVO: regiones de memoria y allocators
│   └── simd.rs       # ★ NUEVO: inferencia de ancho SIMD
├── ir.rs             # FAL IR — representación intermedia propia
├── codegen.rs        # Generación LLVM IR con inkwell
├── codegen_gpu.rs    # ★ NUEVO: target GPU (Fase 5+)
├── diferenciable.rs  # ★ NUEVO: auto-diferenciación (Fase 5+)
└── lsp.rs            # Servidor LSP con tower-lsp
```

---

## Hoja de Ruta

### Fase 0: Diseño conceptual ✅ COMPLETADA

- [x] 5 pilares definidos y justificados
- [x] Stack técnico decidido
- [x] Decisiones day-0 vinculantes (C ABI, Span, errores, CLI)
- [x] 5 ideas novel para ultra-rápido documentadas
- [x] Investigación EML completada (descartado para runtime)
- [ ] Pendiente: elegir licencia

### Fase 1: Prototipo de compilador mínimo (Day-0)

**Objetivo:** Compilar `hola_mundo.fc` que llame a `puts` de libc → binario nativo.

**Infraestructura base:**
- [ ] Proyecto Rust con logos + chumsky + inkwell + clap
- [ ] CLI: `falcato build`, `falcato run`, `falcato check`
- [ ] Span/SourceLocation desde el lexer
- [ ] Sistema de errores en español con códigos
- [ ] Salida `.o` compatible con gcc/clang/link.exe
- [ ] Scripts .bat para build/test/run

**Lenguaje mínimo:**
- [ ] Lexer: tokens en español (función, retornar, Entero32, etc.)
- [ ] Parser: funciones, tipos primitivos, llamadas, strings
- [ ] AST inicial con Span
- [ ] Generación LLVM IR con C ABI por defecto
- [ ] Declaraciones FFI: `inseguro función`
- [ ] Linkeo con libc

**Métrica de éxito:**
```bash
falcato build ejemplos/hola_mundo.fc -o hola.exe
./hola.exe
# → "¡Falcato forja poder!"
```

### Fase 2: Sistema de tipos + inicio LSP

- [ ] Tipos primitivos completos
- [ ] Estructuras (`estructural`) con layout C
- [ ] Enumeraciones (`enumeración`)
- [ ] Genéricos básicos
- [ ] Artículos como marcadores de ownership (`el`/`la`/`un`)
- [ ] Módulos (`usar`, resolución de rutas)
- [ ] Servidor LSP con tower-lsp
- [ ] Syntaxis highlighting básico
- [ ] Errores en vivo (diagnósticos)

### Fase 3: Memoria, ownership + LSP completo

- [ ] Semántica de movimiento (`mueve`) y préstamo (`presta`/`&`)
- [ ] Tiempos de vida (`'la`, `'el`)
- [ ] Heap: `Caja<T>` (Box), `Contado<T>` (Rc/Arc)
- [ ] Layout inference: SoA/AoS (N-01)
- [ ] Allocator system: regiones de memoria (N-04)
- [ ] LSP: hover, go-to-definition, autocompletado

### Fase 4: Concurrencia + SIMD + GPU inicio

- [ ] Hilos (`hilo::nuevo`), canales (`canal::nuevo`)
- [ ] Async/tiempo futuro (tiempos verbales como niveles)
- [ ] EnviarEntreHilos / CompartirEntreHilos
- [ ] SIMD hints por género (N-02)
- [ ] Tiempo verbal como ancho SIMD (N-03)
- [ ] Dynamic Region Ownership (concurrencia segura sin borrow checker completo)

### Fase 5: Maduración + GPU + auto-diferenciación

- [ ] Target GPU: compilar a SPIR-V / CUDA vía LLVM
- [ ] `nosotros` como workgroup: kernel de compute shader (N-03)
- [ ] Auto-diferenciación (N-05)
- [ ] Generación de bindings C desde headers
- [ ] Macros / metaprogramación
- [ ] Forja (package manager)
- [ ] Playground WASM online
- [ ] Auto-hospedaje (compilador escrito en Falcato)

---

## Comparativa: Falcato vs otros lenguajes

| Dimensión | Falcato | Rust | Zig | C | Go |
|-----------|---------|------|-----|---|----|
| Ownership | Género gramatical | Borrow checker | Manual | Ninguno | GC |
| Const/Mut | Ser/Estar (comptime/runtime) | let/let mut/const | const/var | const | var |
| Async/Await | Tiempo futuro | async fn | No nativo | No | goroutines |
| Fallible | Subjuntivo | Result<T,E> | Error union | errno | error |
| Generadores | Imperfecto | impl Iterator | No | No | No |
| Unsafe | Imperativo | unsafe | No (safety) | Todo es unsafe | No |
| C FFI | Default (C ABI) | extern "C" + repr(C) | extern struct | Nativo | CGo |
| SIMD | Hints por artículo | std::simd (inestable) | @Vector | intrínsecas | No |
| Layout | SoA/AoS por anotación | repr(C)/repr(Rust) | extern struct | Manual | No |
| Allocators | Por artículo | Global allocator | Parámetro | Manual | GC |

---

> *"Forjamos Falcato porque el poder sin control no es poder. Es caos."*
