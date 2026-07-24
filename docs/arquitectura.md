# Arquitectura del Compilador Falcato

---

## Pipeline completo de compilación

```
.fc → Lexer (logos) → Parser (chumsky) → AST (con Span)
  → Análisis semántico → FAL IR → Optimizaciones FAL IR
  → LLVM IR (inkwell) → .o → Link → binario
```

---

## 1. Lexer (logos)

Genera tokens type-safe a partir del código fuente. Cada token conserva
su Span original para errores y LSP.

### Tokens del lenguaje

```rust
enum Token {
    // Keywords
    Funcion, Retornar, Si, Entonces, Sino, Mientras, Para, Deja,
    Estructural, Enumeracion, Convencion, Realizacion, Donde,
    Usar, Modulo, Inseguro, Ser, Estar, Fallar, Producir,
    Mueve, Presta, Como, Tipo,

    // Artículos (ownership)
    ArticuloEl, ArticuloLa, ArticuloUn, ArticuloLos, ArticuloLas,

    // Literales
    Entero(i64), Flotante(f64), Palabra(String), Caracter(char),
    Booleano(bool),

    // Identificadores y símbolos
    Identificador(String), Punto, Coma, DosPuntos, PuntoYComa,
    LlaveAbre, LlaveCierra, ParenAbre, ParenCierra,
    Flecha, Igual, Mas, Menos, Asterisco, Barra, Porcentaje,
    Ampersand, BarraVertical, Exclamacion, MenorQue, MayorQue,

    // Especiales
    Error(String), Span(Span),
}
```

---

## 2. Parser (chumsky)

Parser combinator con errores recuperables. Crítico para LSP: si el usuario
escribe código incompleto, el parser no se detiene — produce errores parciales
y sigue parseando.

### Gramática (BNF simplificada)

```
programa   = { declaracion }
declaracion = funcion_decl | estructural_decl | enumeracion_decl
            | modulo_decl | usar_decl

funcion_decl = "función" identificador "(" [param_list] ")" [":" tipo]
               bloque
param_list   = param { "," param }
param        = articulo identificador ":" tipo
bloque       = "{" { sentencia } "}"

tipo         = "Entero" numero | "Flotante" numero | "Natural" numero
             | "Booleano" | "Caracter" | "Palabra" | "Vacio"
             | "Lista" "<" tipo ">" | "Opción" "<" tipo ">"
             | "*" tipo | "&" tipo
             | identificador [ "<" { tipo } ">" ]

articulo     = "el" | "la" | "un" | "los" | "las"
```

---

## 3. AST (con Span)

Cada nodo del AST lleva su ubicación en el código fuente. Esto permite
errores precisos y LSP sin búsquedas adicionales.

```rust
// D: src/span.rs
pub struct Span {
    pub inicio: Posicion,
    pub fin: Posicion,
    pub archivo: Arc<str>,
}

pub struct Posicion {
    pub linea: u32,
    pub columna: u32,
    pub offset: u32,
}

// D: src/ast.rs (simplificado)
pub enum Declaracion {
    Funcion(Span, FuncionDecl),
    Estructural(Span, StructDecl),
    Enumeracion(Span, EnumDecl),
    Modulo(Span, ModuloDecl),
    Usar(Span, Vec<String>),
}

pub struct FuncionDecl {
    pub nombre: String,
    pub tiempo_verbal: TiempoVerbal,  // Presente, Futuro, Subjuntivo, etc.
    pub params: Vec<Parametro>,
    pub retorno: Option<Tipo>,
    pub cuerpo: Bloque,
}

pub struct Parametro {
    pub articulo: Articulo,   // El, La, Un, Los, Las
    pub nombre: String,
    pub tipo: Tipo,
    pub allocator_sufijo: Option<SufijoAllocator>,  // _frame, _temp, _persistente
}

pub enum Articulo {
    El,   // owned, mutable
    La,   // borrowed, inmutable
    Un,   // optional
    Los,  // colección owned
    Las,  // colección borrowed
}

pub enum TiempoVerbal {
    Presente,    // síncrono
    Futuro,      // async
    Subjuntivo,  // fallible
    Imperfecto,  // generador
    Imperativo,  // unsafe
    Nosotros,    // paralelo/colectivo (GPU compute)
}

pub enum SufijoAllocator {
    Normal,       // heap/stack default
    Frame,        // frame allocator
    Temp,         // arena temporal
    Persistente,  // memoria persistente
}
```

---

## 4. Análisis Semántico

### 4.1 Sistema de Tipos (`semantic/tipos.rs`)

Inferencia y verificación de tipos. Implementa el mapeo:
- Artículos → ownership
- `ser` → compile-time constant
- `estar` → runtime mutable

### 4.2 Ownership (`semantic/ownership.rs`)

Verifica las reglas de ownership según el artículo:
- `el`: único propietario, puede mutar, se mueve al asignar
- `la`: prestado, no puede mutar, no se mueve
- `un`: opcional, puede ser nulo
- `los`/`las`: reglas de colección

### 4.3 Tiempos Verbales como Niveles (`semantic/tiempos.rs`)

Implementa la teoría de "Practical Type Inference with Levels" (PLDI 2025).

Los tiempos verbales son **niveles** en la inferencia de tipos:

```
Nivel 0: Presente  → síncrono, puede llamar a cualquier nivel >= 0
Nivel 1: Futuro    → async, puede llamar a niveles 0 (con await) y 1
Nivel 2: Subjuntivo → fallible, puede fallar
Nivel 3: Imperfecto → generador, produce valores
∞:       Imperativo → unsafe, fuera del sistema de niveles
```

```rust
pub enum NivelEjecucion {
    Sincrono = 0,
    Async = 1,
    Fallible = 2,
    Generador = 3,
    Inseguro = 99,
}
```

### 4.4 Layout Inference (`semantic/layout.rs`) ★

Implementa el polimorfismo de layout (Idea Novel N-01).
Analiza el access pattern y decide SoA vs AoS automáticamente.

```rust
pub enum Layout {
    AoS,  // Array of Structures (default, natural)
    SoA,  // Structure of Arrays (SIMD-óptimo)
    Auto, // El compilador elige según access pattern
}

// Transformación: AoS → SoA
fn lower_aos_to_soa(tipo: &Estructural) -> StructSoA {
    // Para cada campo, crear un array separado
    // e.g., struct Punto { x, y } → struct { x: [], y: [] }
}
```

### 4.5 Allocator System (`semantic/memoria.rs`) ★

Mapea sufijos de artículo a regiones de memoria:

| Sufijo | Allocator | Cuándo se libera |
|--------|-----------|------------------|
| (default) | Heap allocator | Al salir del scope |
| `_frame` | Frame allocator | Al final del frame actual |
| `_temp` | Arena allocator | Al salir del bloque |
| `_persistente` | Mmap/static | Nunca (vida del programa) |

### 4.6 SIMD Inference (`semantic/simd.rs`) ★

Infere el ancho SIMD según:
- Artículo: `el` = denso = SIMD width completo; `la` = puede ser sparse = gather
- Tiempo verbal: futuro plural = SIMD batch
- `#[simd_width(N)]` annotation explícita

---

## 5. FAL IR (Intermedio Propio)

IR optimizable independientemente de LLVM. Permite aplicar transformaciones
específicas de Falcato que LLVM no entendería.

### Formato

```rust
pub enum Instruccion {
    // Operaciones estándar
    EnteroLiteral(i64),
    FlotanteLiteral(f64),
    PalabraLiteral(Box<str>),
    Identificador(String),
    Llamada(String, Vec<Instruccion>),
    Retornar(Option<Box<Instruccion>>),
    Bloque(Vec<Instruccion>),

    // Operaciones de ownership
    Mover(Box<Instruccion>),
    Prestar(Box<Instruccion>),
    Drop(Box<Instruccion>),

    // Operaciones de tiempo verbal (niveles)
    EjecutarAsync(Box<Instruccion>),       // futuro → runtime async
    EjecutarFallible(Box<Instruccion>),    // subjuntivo → try
    Producir(Box<Instruccion>),            // imperfecto → yield

    // Operaciones de prefijo (Pilar V)
    Reintentar(Box<Instruccion>, u32),     // re- → retry loop
    DesCargar(Box<Instruccion>),           // des- → free/drop

    // Operaciones de layout ★
    SoARead(String, String, usize),        // read field from SoA struct
    SoAWrite(String, String, usize, Box<Instruccion>),

    // Operaciones de memoria ★
    AsignarFrame(Box<Instruccion>),        // frame allocator
    AsignarTemp(Box<Instruccion>),         // arena allocator
}
```

### Optimizaciones FAL IR

| Optimización | Qué hace | Por qué |
|-------------|----------|---------|
| Inlining de prefijos | Expande `re-intentar` a loop | Sin esto, los prefijos serían azúcar |
| SoA lowering | Transforma accesos AoS a SoA | Rendimiento SIMD |
| Eliminación de préstamos | Elimina `&` cuando el préstamo no se usa | Menos instrucciones |
| Plega ser | Evalúa expresiones `ser` en compile-time | Cero costo runtime |
| Inferencia de niveles | Determina el nivel mínimo necesario | Optimiza async/fallible a sync si es posible |

---

## 6. Codegen (LLVM via inkwell)

### Estrategia

1. **C ABI por defecto:** Toda función se compila con calling convention C.
   Struct layout sigue las reglas de C (mismo padding/alignment).

2. **Name mangling desactivado:** Los símbolos se exportan con su nombre literal.

3. **Auto-vectorización:** LLVM ya optimiza para SIMD en targets x86_64/ARM.
   Las anotaciones de layout (SoA) maximizan esto.

4. **Targets:**
   - Windows x86_64 (msvc): `x86_64-pc-windows-msvc`
   - Linux x86_64 (gnu): `x86_64-unknown-linux-gnu`
   - WASM: `wasm32-unknown-unknown`
   - ARM64: `aarch64-unknown-linux-gnu`
   - (Futuro) SPIR-V para GPU: vía LLVM SPIR-V backend

### Pipeline inkwell

```rust
// D: src/codegen.rs (esquema)
fn compilar(ir: &ProgramaFAL, target: &TargetTriple) -> Result<ModuloLLVM> {
    let contexto = Context::create();
    let modulo = contexto.create_module("main");
    modulo.set_target_triple(&target.to_string());
    modulo.set_data_layout(&target.data_layout());

    for funcion in &ir.funciones {
        compilar_funcion(&contexto, &modulo, funcion)?;
    }

    // C ABI por defecto
    // Name mangling desactivado
    // Salida .o listo para linker
    Ok(modulo)
}
```

---

## 7. LSP (tower-lsp)

Comparte el lexer, parser y AST con el compilador. La gramática es la misma,
solo que el parser tolera errores (chumsky tiene soporte nativo para esto).

### Capacidades por fase

| Fase | Capacidad LSP |
|------|---------------|
| Fase 1 | — (solo compilador) |
| Fase 2 | Diagnósticos, semantic tokens, completions de keywords |
| Fase 3 | Hover con tipos, go-to-definition, document symbols |
| Fase 4 | Code actions, rename, find references |
| Fase 5 | Refactors, quick fixes, completions contextuales |

### Arquitectura LSP

```rust
#[LspService]
struct ServidorFalcato {
    documentos: HashMap<Url, DocumentoState>,
}

struct DocumentoState {
    texto: String,
    ast: Option<AST>,
    errores: Vec<Error>,
}

// Actualización incremental:
// 1. Usuario escribe → notificación textDocument/didChange
// 2. Parseamos solo las partes afectadas (parser incremental)
// 3. Enviamos diagnósticos actualizados
// 4. Servimos hover/completions desde el AST cacheado
```

---

## 8. Las 5 Ideas Novel en la Arquitectura

### Cómo se integran en el pipeline

| Idea Novel | Componente | Fase |
|-----------|------------|------|
| N-01: Layout polymorphism | `semantic/layout.rs` + FAL IR lowering | Fase 3 |
| N-02: Género como hint SIMD | `semantic/simd.rs` → LLVM metadata | Fase 4 |
| N-03: Tiempo como ancho SIMD | `semantic/tiempos.rs` + `semantic/simd.rs` | Fase 4 |
| N-04: Allocator por artículo | `semantic/memoria.rs` + FAL IR | Fase 3 |
| N-05: Auto-diferenciación | `diferenciable.rs` → FAL IR → codegen | Fase 5 |

### Prioridad de implementación

Las ideas novel N-01 y N-04 (layout y allocators) tienen prioridad sobre
N-02, N-03 y N-05 porque:
1. Afectan directamente al rendimiento de memoria (el cuello de botella real)
2. Son más fáciles de implementar (transformaciones en FAL IR)
3. Tienen precedentes académicos que las respaldan

---

## 9. Sistema de errores

### Códigos de error

```
S001-S099: Errores de sintaxis
T001-T099: Errores de tipo
O001-O099: Errores de ownership
C001-C099: Errores de FFI / C interop
M001-M099: Errores de módulos
I001-I099: Errores internos del compilador
W001-W099: Warnings
```

### Formato

```
[Código] archivo.fc:línea:columna: mensaje
         │ nota / sugerencia (opcional)

Ejemplos:
[T042] entrada.fc:7:12: no se encontró el tipo 'Entero64'
       │ nota: ¿quisiste decir 'Entero32'?

[O013] entrada.fc:15:5: no se puede mover 'buffer' porque ya se prestó
       │ sugerencia: usa una copia explícita con `.clonar()`
```

---

## 10. CLI del compilador

```bash
# Subcomandos
falcato build [archivo]     # compila a binario
falcato run   [archivo]     # compila y ejecuta
falcato check [archivo]     # solo análisis
falcato lsp                  # modo servidor LSP

# Flags
-o <ruta>          # salida del binario
--target <triple>  # target (x86_64-pc-windows-msvc, etc.)
--release          # optimizaciones -O2
--emit-ir          # muestra LLVM IR
--emit-fal-ir      # muestra FAL IR (debug)
-v                 # verbose
```

---

## 11. Testing

### Estrategia

| Tipo | Herramienta | Qué prueba |
|------|-------------|------------|
| Snapshot unit | `insta` | Salida FAL IR y LLVM IR de fragmentos de código |
| Integration | `assert_cmd` | CLI: `falcato build`, `falcato run` |
| End-to-end | Scripts | Compilar .fc, ejecutar, verificar stdout |
| LSP | `tower-lsp` test utils | Diagnósticos, completions, hover |

### Directorio de tests

```
tests/
├── snapshots/       # Output IR esperado (insta)
├── integration/     # Pruebas de CLI
│   └── hola_mundo/
│       ├── entrada.fc
│       └── salida_esperada.txt
├── lsp/             # Pruebas del servidor LSP
└── e2e/             # End-to-end (compila y ejecuta)
    └── build_run.bat
```
