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
├── codegen.rs           # Codegen Cranelift — funciones, variables, ops, condicionales, bucles, arrays, structs, enums, monomorfización
└── lsp.rs               # Servidor LSP — diagnósticos, autocompletado, hover, go-to-definition
```

## Historial de Fases (resumen)

- **Fase 1-3:** Core del lenguaje (variables, operaciones, condicionales, bucles, ownership básico) + pulido (spans, recovery de errores, LSP).
- **Fase 3.5:** Arrays (`[T; N]`, literales, `todos`, acceso, asignación a elementos).
- **Fase 4:** Structs con layout C (`estructural Punto { ... }`).
- **Fase 5:** Verificación de llamadas, Ser/Estar (`es`/`está`), Subjuntivo (`fuese`).
- **Fase 6:** Bucle `para` sobre arrays.
- **Fase 7:** Enums tag+union con constructores y pattern matching.
- **Fase 8A:** Const generics (`función longitud<N: Entero32>(...)`).
- **Fase 8B:** Base AST/parser para enums genéricos (`enumeración alguno<T> { ... }`).
- **Fase 8C:** Type generics con bounds (`fn máximo<T que Comparable>(...)`), alias `función`/`funcion`/`fn`, monomorfización por tipo concreto.
- **Cold block optimization para Subjuntivo**: reordenamiento de bloques Cranelift (else hot path en línea, then cold path afuera).
- **Ser/Estar real**: `ModoVerbal::Estativo`, bare `está` como truthiness check (enteros/booleanos/punteros, no floats).
- **Find references en LSP**: handler `references` con traversal completo del AST.
- **Fase 9 / 9.5:** Módulos e imports (`módulo`, `usar`, visibilidad `el`/`la`, multi-archivo, glob imports, type-checking cross-file).
- **Backend trait + Resolver multi-archivo**: abstracción de codegen y resolución de dependencias entre archivos.

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

**40/40 tests pasan. 50+ ejemplos funcionando. Auditoría completa: 0 crashes.**

### Estado de distribución (v0.1.0 — pre-release)

| Aspecto | Estado |
|---------|--------|
| Release build (6.4 MB, LTO) | ✅ `cargo build --release` produce `falcato.exe` |
| Dependencias DLL | ⚠️ Requiere `VCRUNTIME140.dll` (local: bundle, CI: static) |
| CRT estático | ⏳ Pendiente — funciona en GitHub Actions (VS completo), no en VS Insiders local |
| GitHub repo | ✅ `CerebroCanibalus/falcato` (privado) |
| GitHub Actions CI | ❌ Pendiente |
| VS Code Extension | ✅ Archivos creados (faltan npm install + test) |
| Falso positivo Defender | ⚠️ Riesgo alto sin firma digital |
| Instalador script | ❌ Pendiente |

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

### Fase R1 — Distribución e instalación (RELEASE)
| # | Tarea | Archivos | Esfuerzo |
|---|-------|----------|----------|
| 1 | `+crt-static` en Cargo.toml (solo CI) | 1 línea | 5 min |
| 2 | `bundle_dlls.ps1` para releases locales | 1 | 10 min |
| 3 | Release build + verificar DLLs | — | 15 min |
| 4 | `clean.ps1` — limpiar basura (exes/ós sueltos) | 1 | 10 min |
| 5 | `.github/workflows/ci.yml` — build + test en push | 1 | 30 min |
| 6 | `.github/workflows/release.yml` — build + ZIP en tag | 1 | 30 min |
| 7 | `scripts/install.ps1` — PATH + VS Code + ejemplos | 1 | 30 min |
| 8 | Probar en máquina limpia (VM sin Rust) | — | 30 min |
| 9 | Subir primer release a GitHub, reportar falso positivo a Microsoft | — | 1h |

**Resultado:** Alguien descarga ZIP de GitHub Releases, ejecuta `install.ps1`, y tiene `falcato` en PATH + VS Code configurado.

### Fase R2 — VS Code Extension
| # | Tarea | Archivos |
|---|-------|----------|
| 1 | `falcato-vscode/package.json` (extension manifest) | 1 |
| 2 | `syntaxes/falcato.tmLanguage.json` (syntax highlighting) | 1 |
| 3 | `language-configuration.json` (brackets, comentarios) | 1 |
| 4 | `client.js` — LSP client (lanza `falcato lsp`) | 1 |
| 5 | `install.ps1` actualizado para copiar extensión | 1 |

**Resultado:** Syntax highlighting + errores en rojo + autocompletado + hover en VS Code.

### Fase R3 — Skill Falcato para LLMs
| # | Tarea | Archivos |
|---|-------|----------|
| 1 | Skill `falcato-language` (OpenCode) — gramática + tipos + ownership + patrones | 1 |
| 2 | `AGENTS.md` genérico para cualquier LLM (check→fix→build workflow) | 1 |
| 3 | Carpeta `falcato-skill/` con ejemplos de referencia | varios |

**Resultado:** Cualquier LLM (Claude, GPT, Gemini) puede escribir Falcato siguiendo la skill.

### Fase R4 — Colecciones (Diccionario + Conjunto)
| # | Tarea | Archivos | Líneas |
|---|-------|----------|--------|
| 1 | `Tipo::Diccionario(K,V)` + `Tipo::Conjunto(T)` en AST | `ast.rs` | 2 |
| 2 | Hash FNV-1a en codegen | `codegen.rs` | 30 |
| 3 | Built-ins: `diccionario_nuevo/insertar/obtener/existe/eliminar/longitud/liberar` | 2 (semantic + codegen) | ~250 |
| 4 | Built-ins: `conjunto_nuevo/insertar/contiene/eliminar/liberar` | 2 (semantic + codegen) | ~120 |
| 5 | LSP autocompletado | `lsp.rs` | 10 |

**Resultado:** Programas reales con hash maps y conjuntos.

### Fase R5 — Documentación
| # | Tarea | Archivos |
|---|-------|----------|
| 1 | `INSTALL.md` — instalar en 3 pasos | 1 |
| 2 | `QUICKSTART.md` — hola mundo → archivo → colección | 1 |
| 3 | `REFERENCIA.md` — todas las built-ins | 1 |
| 4 | `ERRORES.md` — cada código [T###] explicado | 1 |

### Fase R6 — Proyecto ejemplo 500+ líneas
- Word counter: lee archivo, tokeniza, cuenta frecuencia con Diccionario, ordena con Vector
- Valida el pipeline completo: módulos, colecciones, archivos, I/O

### Fase R7 — Drop automático
- Análisis de CFG para insertar `free` al final de scope
- Elimina fugas en Texto, Vector, Diccionario, Conjunto, TCP sockets

### Fase R8 — Package manager (post-v1)
- `Falcato.toml`, dependencias git, comandos `nuevo`/`construir`

## Checklist para release v0.2.0

- [ ] `+crt-static` en CI (GitHub Actions tiene VS Build Tools completo)
- [ ] Release build limpio (CI produce .exe sin DLLs externas)
- [ ] Script `bundle_dlls.ps1` para releases locales (copia VCRUNTIME140.dll)
- [ ] GitHub repo creado con README y LICENSE
- [ ] GitHub Actions CI (build + test)
- [ ] GitHub Actions Release (tag → ZIP)
- [ ] VS Code Extension (syntax + LSP)
- [ ] Skill `falcato-language` para LLMs
- [ ] Diccionario + Conjunto implementados
- [ ] Documentación básica (INSTALL + QUICKSTART)
- [ ] Proyecto ejemplo >500 líneas funcionando
- [ ] Falso positivo reportado a Microsoft Security Center
- [ ] Script `install.ps1` probado en máquina limpia

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
