# ⚔️ Falcato — Lenguaje de Sistemas Iberohablante

**Archivo de presentación unificado — Todo lo que necesitas saber.**

---

## ¿Qué es Falcato?

Un **lenguaje de programación de bajo nivel** construido **desde cero sobre LLVM**,
cuya sintaxis y sistema de tipos explotan las **dimensiones semánticas del español**
que el inglés no tiene (género gramatical, tiempos verbales, ser/estar, subjuntivo,
prefijos productivos, voz pasiva, compuestos aglutinantes).

> **No es Rust con keywords en español. Es un lenguaje completamente nuevo
> donde el español ES el sistema de tipos.**

```
D:\Falcato\          → Proyecto raíz
├── AGENTS.md         → Reglas para IA
├── PRESENTACION.md   ← ESTE ARCHIVO
├── README.md         → Intro breve
├── .gitignore
├── docs\             → Documentación detallada
│   ├── filosofia.md
│   ├── gramatica.md
│   ├── semantica.md
│   ├── hoja_de_ruta.md
│   └── arquitectura.md
├── src\              → Futuro compilador (Rust)
│   ├── main.rs
│   ├── span.rs
│   ├── error.rs
│   ├── lexer.rs
│   ├── parser.rs
│   ├── ast.rs
│   ├── semantic/...
│   ├── ir.rs
│   ├── codegen.rs
│   └── lsp.rs
├── tests\
└── ejemplos\
    └── hola_mundo.fc
```

---

## Filosofía: Los 5 Pilares

### I. Género = Ownership
`el` es owned y mutable. `la` es borrowed e inmutable. `un` es opcional.
Sin borrow checker que aprender — ya lo sabes si hablas español.

### II. Ser/Estar = Const/Mut
`ser` para valores eternos (compile-time). `estar` para lo transitorio (runtime).
Distinción que solo el español tiene y Falcato explota.

### III. Tiempos = Modos de ejecución
Presente es síncrono. Futuro es async. Subjuntivo es fallible.
No estudias `async fn` — conjugas el verbo.

### IV. C ABI por defecto
Layout C, calling C, mangling desactivado. Tus `.o` se linkean
con gcc/clang/link.exe sin wrappers. FFI desde el prototipo.

### V. Prefijos semánticos
`re-` es reintentar. `des-` es liberar. `pre-` es calcular en
compile-time. El compilador entiende los prefijos, no solo el
programador.

> **"El mayor poder a la mayor rapidez y siendo baratos."**

---

## Innovación Semántica: 13 Ventajas del Español

### 1. Género gramatical → Ownership y préstamo

| Artículo | Semántica | Equivalente Rust |
|----------|-----------|-----------------|
| `el` | Propietario único, mutable | `let mut` |
| `la` | Préstamo/Referencia, inmutable | `let` / `&` |
| `un` | Opcional / tal vez exista | `Option<T>` |
| `los` | Colección propietaria | `Vec<T>` |
| `las` | Colección prestada | `&[T]` |

```falcat
el contador: Entero32 = 0;          // owned, mutable
la referencia: &Entero32 = &contador;  // borrowed
un archivo: Opción<Archivo>;        // optional
los items: Lista<Entero32>;         // owned collection
```

### 2. Tiempos verbales → Modos de ejecución

| Tiempo | Ejecución | Equivalente |
|--------|-----------|-------------|
| Presente (`ejecuta`) | Síncrono, bloqueante | `fn call()` |
| Futuro (`ejecutará`) | Asíncrono, `Future` | `async fn call()` |
| Pretérito (`ejecutó`) | Ya completado | `.join()`, resultado |
| Imperfecto (`ejecutaba`) | Iterativo, generador | `yield`, generator |
| Condicional (`ejecutaría`) | Fallback, default | `unwrap_or` |
| Subjuntivo (`ejecute`) | Fallible | `try_`, `Result` |
| Gerundio (`ejecutando`) | Stream, evento | stream, callback |
| Imperativo (`¡ejecuta!`) | Inseguro | `unsafe` |

```falcat
función procesar(dato: Entero32): Entero32    → sync
función descargar(url: Palabra): Futuro<Archivo>  → async
función dividir(a, b): Result<Entero32>       → fallible (subjuntivo)
función fibonacci(): Generador<Entero32>       → iterator (imperfecto)
```

### 3. Ser vs. Estar → Permanencia vs. Transitoriedad

```falcat
ser TAMAÑO: Entero32 = 4096;       // compile-time constant
estar buffer: [Byte; 4096];        // mutable runtime variable
```

### 4. Subjuntivo → Incertidumbre tipada

```falcat
aunque falle, seguir                // failsafe path
quizás retorne valor                → Option/Result
mientras no termine, esperar        // loop hasta condición
```

### 5. Prefijos productivos (20+) → Semántica de operaciones

```falcat
re-intentar(operación fallida)      // retry
des-cargar(archivo)                 // unload / free
pre-calcular(expresión)             // compile-time compute
entre-hilos(enviar mensaje)         // inter-thread
co-procesar(hilo_1, hilo_2)        // concurrent
sobre-escribir(archivo, dato)      // overwrite
contra-bloqueo(recurso)            // deadlock prevention
mal-formato(entrada)               // expected error
```

### 6. Compuestos aglutinantes → APIs auto-documentadas

```falcat
corta-fuegos               // firewall
limpia-memoria             // GC / memory cleaner
cuenta-referencias         // reference counter
busca-ordenar-filtra       // pipeline funcional
porta-datos                // data carrier struct
salva-estado               // state saver
abre-archivo               // file opener
```

### 7. Voz activa vs. pasiva → Dirección del flujo

```falcat
// Activa: el sujeto actúa
función transformar(la dato: &Dato): Dato;

// Pasiva: pipeline de datos
dato sea transformado por proceso;
```

### 8. Conectores ricos → Control flow expresivo

```falcat
mientras condición { ... }
hasta_que condición { ... }
tan_pronto_como evento { ... }
a_menos_que condición { ... }      // guard clause
con_tal_que condición { ... }       // precondition
ya_que invariante { ... }           // assertion
en_cuanto señal { ... }             // async trigger
cada_vez_que evento { ... }         // event listener
```

### 9. Reflexivos → Self semantics

```falcat
se_ejecuta()          // self.run() — mutates self
lo_filtra(dato)      // self.filter(dato) — consumes input
se_lo_pasa(dato)     // complex borrow pattern
```

### 10. Concordancia → Tipo seguridad verbal

```falcat
yo proceso            // single-threaded context
tú procesas           // caller/callee distinction
él procesa            // third-party actor
nosotros procesamos   // parallel/collective
ellos procesan        // unknown actors
```

### 11. Diminutivos/aumentativos → Scope hints

```falcat
bucle-cito            // loop pequeño, inlineable
proceso-ote           // proceso pesado, thread pool
archivo-ito           // archivo temporal/cache
```

### 12. Orden de palabras flexible → DSLs naturales

```falcat
// Todos válidos, semántica idéntica:
procesa hilo dato
hilo procesa dato
dato sea procesado por hilo
```

### 13. Negación múltiple → Modos de falla

```falcat
ningún hilo accede      // thread-safe assertion
nunca falla             // infallible operation
tampoco retorna         // deadlock detection
sin_bloqueo             // lock-free
```

---

## Sistema de Tipos (borrador)

### Primitivos

| Falcato | Descripción |
|---------|-------------|
| `Entero{8,16,32,64}` | Enteros con signo |
| `Natural{8,16,32,64}` | Enteros sin signo |
| `Flotante{32,64}` | IEEE 754 |
| `Booleano` | `cierto` / `falso` |
| `Caracter` | Unicode scalar (32-bit) |
| `Palabra` | Cadena UTF-8 |
| `Vacio` | Tipo unidad (void) |
| `Nulo` | Never / bottom |

### Compuestos

```falcat
estructural Punto { el x: Flotante64, el y: Flotante64 }

enumeración Resultado<T, E> { Bien(T), Arf(E) }

convención Comparable<T> {
    función comparar(&la self, &la otro: T): Entero32;
}

estructural Caja<T> donde T: Comparable<T> { el valor: T }
```

---

## Memoria y Concurrencia

| Concepto | Falcato |
|----------|---------|
| Stack | Variables por defecto |
| Heap único | `el caja = Caja::nuevo(valor)` |
| Heap contado | `los nodos = Contado::nuevo(...)` |
| Hilos | `hilo::nuevo(mueve dato, \|\| { ... })` |
| Canales | `canal::nuevo<T>()` → `emisor.enviar()`, `receptor.recibir()` |
| Async | Verbos en futuro: `función leer(): Futuro<[Byte]>` |
| FFI C | ABI de C por defecto en toda la salida |

---

## Hoja de Ruta

```
Fase 0: Diseño conceptual              ← AHORA
Fase 1: Prototipo mínimo + FFI C day-0
Fase 2: Sistema de tipos + LSP inicio
Fase 3: Memoria y ownership (+ LSP completo)
Fase 4: Concurrencia
Fase 5: Maduración (macros, auto-hospedaje, forja)
```

> **Day-0:** C ABI por defecto, errores en español con códigos,
> Span en cada nodo AST, CLI `falcato build/run`, linkeo con libc.

---

## Stack técnico

| Componente | Elección |
|------------|----------|
| CLI | `clap` (Rust) |
| Lexer | `logos` (Rust) |
| Parser | `chumsky` (Rust) |
| AST/IR propio | Structs Rust con Span |
| Backend | LLVM IR via `inkwell` |
| LSP | `tower-lsp` |
| Testing | `insta` (snapshots) |
| Target inicial | x86_64 Windows / Linux |

---

## Ejemplo Day-0 (`ejemplos/hola_mundo.fc`)

```falcat
// Sin std. Sin runtime. FFI directo a libc.
inseguro función puts(mensaje: *const Caracter): Entero32;

función principal(): Entero32 {
    puts("¡Falcato forja poder!");
    retornar 0;
}
```

---

## ¿Qué lo diferencia de otros lenguajes "en español"?

| Proyecto | Enfoque | Bajo nivel? | Innovación semántica? |
|----------|---------|-------------|----------------------|
| **Latino** | Interpretado, sintaxis español | ❌ | ❌ (traducción) |
| **EsJS** | JavaScript con keywords español | ❌ | ❌ (traducción) |
| **Rustico** | Proc-macro Rust español | 🟡 (sobre Rust) | ❌ (traducción) |
| **Qriollo** | Funcional, rioplatense, compila a C | 🟡 | 🟡 (joda, no formal) |
| **Falcato** | **Compilador propio sobre LLVM** | ✅ | ✅ **Explota género, tiempos, ser/estar, prefijos...** |

Falcato es **el único** que no traduce keywords, sino que **diseña el sistema
de tipos y ejecución desde las propiedades del español**.

---

> *Falcato no es un experimento de traducción. Es un experimento de ingeniería
> de lenguajes donde 500+ años de evolución del español se convierten en
> garantías de compilación.*
