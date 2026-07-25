---
name: falcato-language
description: Falcato (.fc) systems language in Spanish. Use when generating, reviewing, debugging, or compiling Falcato code. Covers grammar, types, ownership (el/la), FFI, memory management, hardware bitfields, async, and error patterns.
---

# Falcato Language

Compiled systems language in Spanish. Backend: Cranelift. Target: x86_64 Windows. C ABI default.

## Workflow

```
falcato check archivo.fc   # analyze only (fast)
falcato build archivo.fc   # compile to .exe
falcato run archivo.fc     # compile + run
```

Built-in linked libraries: `kernel32.lib`, `ws2_32.lib`, `ucrt.lib`, `vcruntime.lib`

## Grammar Quick Reference

### Declarations
```falcato
fn / función nombre<Gen>(params) -> Tipo { cuerpo }   // function
inseguro fn nombre(params) -> Tipo;                     // FFI (no body)
fut fn nombre(params) -> Tipo { cuerpo }                // async function
estructural Nombre { campo: Tipo, ... }                 // struct
estructural Reg { bits { campo: NaturalN, ... } }      // bitfield struct
enumeración Nombre { Var, Var(dato: Tipo) }            // enum
rasgo Nombre { fn metodo(...) -> Tipo; ... }
implementar Rasgo para Tipo { fn metodo(...) { ... } }
módulo nombre { ... }
usar modulo::simbolo;  usar modulo::*;
prueba "nombre" { afirmar(expr); }
```

### Variables (Ownership — 5 artículos)

| Artículo | Dueño | Mutable? | Compartido? | Cuándo usar |
|----------|-------|----------|-------------|-------------|
| `el` | Único dueño | Sí | No | Contadores, buffers, acumuladores |
| `la` | Prestado | No | Sí (lectura) | Parámetros de función, config |
| `un` | Quizás nadie | Depende | — | Búsquedas, parseos, opcionales |
| `los` | Todos (ref-count) | Sí | Sí | Cachés, estado global entre hilos |
| `las` | Todos (préstamo) | No | Sí (lectura) | Logs centralizados, config global |

```falcato
el x: Tipo = valor;     // owned, mutable — "mío, lo cambio"
la x: Tipo = valor;     // borrowed, inmutable — "prestado, solo leo"
un x: Tipo;             // optional — "quizás existe, quizás no"
los x: Tipo = valor;    // shared owned — "de todos, todos cambian"
las x: &Tipo = ref;     // shared borrowed — "todos leen, nadie escribe"
mover x a fn;           // transfer ownership
copiar x;               // explicit clone
```

**Regla práctica:** Usa `la` siempre que solo leas. Si el compilador pide mutabilidad, cambia a `el`. `un` para cosas que pueden no existir (como buscar un usuario por ID). `los`/`las` solo cuando múltiples hilos acceden al mismo dato.

### Types
| Type | Size | Notes |
|------|------|-------|
| Entero8/16/32/64 | 1/2/4/8 bytes | signed ints |
| Natural8/16/32/64 | 1/2/4/8 bytes | unsigned ints |
| Flotante32/64 | 4/8 bytes | floats |
| Booleano | 1 byte | verdadero/falso |
| Palabra | 8 bytes | &str literal (fijo, inmutable) |
| Texto | 24 bytes | heap string (liberar!) |
| Vacio | 0 bytes | unit type |
| [T; N] | N * size(T) | stack array |
| Vector<T> | 24 bytes | heap vector (liberar!) |
| Resultado<T,E> | 4 + size(T/E) | Exito(valor) / Error(codigo) |
| &T, &mut T | 8 bytes | references |
| &nombre T | 8 bytes | lexical lifetime reference |

### Control Flow
```falcato
si cond { } sino { }
si x es 5 { }           // ser = identidad (==)
si x está 5 { }         // estar = estado temporal (==)
si x está { }           // bare = truthiness (int/bool/ptr, no floats)
si x fuese es 5 { }     // subjuntivo = cold path optimization
mientras cond { }
para var en iterable { }  // over arrays, vectors, ranges
coincidir expr { pat => expr, _ => expr }  // match
seleccionar { canal como v => {}, _ => {} }  // chan select
```

### Operators (C precedence)
```
||  &&  |  ^  &  == !=  < >  <<  + -  * / %
~a          // bitwise NOT
a >>> b     // logical shift right (zero-fill)
```

### Text-specific operators
```falcato
a + b       // concatenate two Textos
t[0]        // byte at index (Entero8)
t[0..5]     // slice as new Texto
v[0]        // element at index (Vector<T>)
```

### Method syntax (preferred over bare functions)
```falcato
t.agregar("x")   t.tam()   t.liberar()
v.agregar(x)     v.tam()   v.liberar()
x.poner_bit(n)   x.unos()  x.ceros_izquierda()
```

### Literals
```falcato
42           → Entero32
3.14         → Flotante64
"Hola"       → Palabra
'H'          → Caracter
verdadero    → Booleano
[1, 2, 3]    → [Entero32; 3]
todos 0      → array fill
0..5         → 0,1,2,3,4 (exclusive)
0..=5        → 0,1,2,3,4,5 (inclusive)
```

### Interpolation
```falcato
decir("x = {x}, y = {y}");
// Variables dentro de strings con {nombre}
```

## Systems / Kernel Patterns

### Memory: REGLA DE ORO — liberar!
```falcato
el t: Texto = texto_desde("x");
t.liberar();  // SIEMPRE

el v: Vector<Entero32> = vector_nuevo();
v.liberar();  // SIEMPRE
```

Objects needing `.liberar()`: Texto (nuevo/desde), Vector<T> (nuevo), return from archivo_leer(), texto_concatenar(), texto_subtexto().

### Bitfields — hardware registers
```falcato
estructural RegistroUART {
    bits {
        habilitado: Natural8,    // 1 bit
        modo_tx: Natural8,       // 1 bit
        baud_div: Natural16,     // 2 bits
    }
}
// Compiler generates shifts/masks. Access like: reg.habilitado = 1;
```

### FFI to C
```falcato
inseguro función puts(la s: Palabra) -> Entero32;
inseguro función malloc(tam: Natural64) -> Puntero<Vacio>;
inseguro función free(ptr: Puntero<Vacio>) -> Vacio;

// Usage
inseguro {
    el ptr = malloc(100);
    free(ptr);
}
```

### Error handling pattern
```falcato
fn dividir(a: Entero32, b: Entero32) -> Resultado<Entero32, Entero32> {
    si b es 0 { retornar Resultado.Error(-1); }
    retornar Resultado.Exito(a / b);
}

// Propagation with ?
fn procesar() -> Resultado<Entero32, Entero32> {
    el v = dividir(10, 2)?;  // if Error, auto-return
    retornar Resultado.Exito(v * 2);
}
```

### Generics
```falcato
fn maximo<T que Comparable>(el a: T, el b: T) -> T
fn longitud<N: Entero32>(nums: [Entero32; N]) -> Entero32
```

### Testing
```falcato
prueba "suma" {
    afirmar(sumar(2, 2) es 4);
}
```

## Built-in Functions (see reference/builtins.md for full signatures)

### I/O — polymorphic (int, string, float, bool)
```falcato
imprimir(x)          // print without newline
imprimir_linea(x)    // print with newline
decir(x)             // alias
```

### Key math
```falcato
abs(x)  max(a,b)  min(a,b)        // Entero32 only
raiz(x)  potencia(base, exp)      // Flotante64 only
tamaño_de::<T>()                  // sizeof (comptime)
```

### Async
```falcato
dormir(ms)                         // Sleep(ms)
lanzar expr                        // spawn thread
canal_nuevo(cap)                   // create channel
canal_enviar(c, v)                 // send
canal_recibir(c)                   // recv (blocking)
con_executor(N) { lanzar ...; cancelar(); }
```

## Error Codes

| Code | Meaning |
|------|---------|
| S001 | syntax: missing `;` or `}` |
| T001 | type mismatch |
| T005 | operand types don't match |
| O001 | use after move / mutate immutable |
| M001 | private symbol |

## Keywords
```
fn/función/funcione  retornar/devolver
si/sino/es/está/fuese  mientras/para/en
coincidir/emparejar  estructural  enumeración
rasgo/implementar  módulo/usar  inseguro
mover/copiar/prestar  mut/como
verificado/estricto  región/region
puro/muta/lee  yo
fut/esperar/lanzar/bloquear  seleccionar/con_executor
prueba/afirmar  verdadero/falso  todos  tipo
```
