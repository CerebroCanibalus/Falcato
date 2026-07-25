# 14 — Ownership en profundidad

← [13: Async](13-async.md) | [Indice](../GUIA.md) | [Siguiente: Glosario →](15-glosario.md)

---

En el [capítulo 3](03-variables.md) viste los 5 artículos. Este capítulo explica **cómo se comporta la propiedad** cuando los datos se mueven entre funciones, referencias, prestamos y liberaciones.

## Mover (transferir propiedad)

Cuando pasas un `el` a una función, **transfieres la propiedad**. El original ya no es válido.

```falcato
fn consumir(/*el*/ dato: Texto) {
    imprimir_linea(dato);
    dato.liberar();  // el que recibe libera
}

fn main() {
    el mensaje: Texto = texto_desde("Hola");
    consumir(mensaje);
    // Aquí 'mensaje' YA NO EXISTE — se movió a consumir
    // imprimir_linea(mensaje);  // Error: [O001] use-after-move
}
```

**¿Por qué?** Para evitar que dos partes liberen la misma memoria. Si `main` y `consumir` liberaran `mensaje`, sería un **double-free** — crash o vulnerabilidad.

**La alternativa explícita con `mover`:**

```falcato
// mover es opcional (es el comportamiento por defecto con el)
mover mensaje a consumir;
// Pero es útil para ser explícito en código complejo
```

## Copiar (duplicar el valor)

Con `copiar` le dices al compilador: "Crea una copia independiente de este dato."

```falcato
fn main() {
    el original: Texto = texto_desde("Hola");
    el clon: Texto = copiar original;
    
    // Ahora existen DOS textos independientes
    original.agregar(" Mundo");
    clon.agregar("!");

    imprimir_linea(original);  // "Hola Mundo"
    imprimir_linea(clon);      // "Hola!"
    
    original.liberar();
    clon.liberar();  // cada uno se libera por separado
}
```

**¿Cuándo usar `copiar`?**
- Cuando necesitas el mismo valor en dos lugares y cada uno va a modificarlo
- Cuando quieres preservar el original después de pasarlo a una función
- Para tipos pequeños como `Entero32`, `Booleano`, la copia es automática (no necesitas `copiar`)

## Referencias (`&T`, `&mut T`)

Una referencia es un **préstamo temporal**. El dueño original conserva la propiedad.

```falcato
fn leer(la datos: &Texto) {
    imprimir_linea("Longitud: ");
    imprimir_linea(datos.tam());
    // datos.liberar();  // Error: es prestado, no dueño
}

fn main() {
    el mensaje: Texto = texto_desde("Hola");
    leer(&mensaje);       // prestas &mensaje a leer
    // 'mensaje' sigue siendo válido aquí
    mensaje.agregar("!"); // lo puedes seguir usando
    mensaje.liberar();    // tú eres el dueño, tú liberas
}
```

### Referencia mutable (`&mut`)

A veces necesitas que una función **modifique** tu dato sin tomar propiedad. Ahí usas `&mut`:

```falcato
fn decorar(mensaje: &mut Texto) {
    mensaje.agregar(" [PROCESADO]");
}

fn main() {
    el saludo: Texto = texto_desde("Hola");
    decorar(&mut saludo);
    imprimir_linea(saludo);  // "Hola [PROCESADO]"
    saludo.liberar();
}
```

### Las reglas de los préstamos (borrow checker)

Falcato tiene **tres niveles** de rigor:

| Nivel | Significado | Para quién |
|-------|-------------|------------|
| 0 (default) | Permisivo — préstamos sin verificación estricta | Principiantes, prototipos, LLMs |
| 1 (`verificado`) | Detecta use-after-move | Equilibrio entre libertad y seguridad |
| 2 (`estricto`) | Borrow checker completo: 1 mutable XOR N inmutables | Sistemas críticos |

**En nivel 2 (`estricto`)**:

```falcato
estricto;  // activa modo estricto

fn main() {
    el datos: Texto = texto_desde("Hola");
    
    la ref1: &Texto = &datos;   // préstamo inmutable → OK
    la ref2: &Texto = &datos;   // otro préstamo inmutable → OK
    
    // el ref3: &mut Texto = &mut datos;
    // Error [O002]: ya hay préstamos inmutables activos
    
    imprimir_linea(ref1);  // se usan aquí
    imprimir_linea(ref2);  // y aquí
    
    el ref4: &mut Texto = &mut datos;  // ahora sí — los inmutables ya murieron
    ref4.agregar("!");                  // modificación
}
```

### Field-level borrowing

Prestar **campos distintos** del mismo struct al mismo tiempo funciona sin conflictos:

```falcato
estructural Persona {
    nombre: Texto,
    apellido: Texto,
}

fn main() {
    el p: Persona = Persona {
        nombre: texto_desde("Ana"),
        apellido: texto_desde("López"),
    };
    
    la nom: &mut Texto = &mut p.nombre;    // prestas solo nombre
    la ape: &mut Texto = &mut p.apellido;  // prestas solo apellido
    // Sin conflicto — son campos diferentes
    
    nom.agregar(" María");
    ape.agregar(" García");
}
```

**Esto resuelve un problema famoso de Rust** — en Rust, `&mut p.x` y `&mut p.y` no pueden coexistir, en Falcato sí.

## Lifetimes léxicos (`&nombre T`)

En vez de `&'a T` como Rust, Falcato usa el **nombre de la variable** como lifetime:

```falcato
fn mas_larga(la a: &texto1 Texto, la b: &texto2 Texto) -> &texto1 Texto {
    si a.tam() > b.tam() { retornar a; }
    retornar a;  // retorna la que vive al menos tanto como texto1
}
```

El lifetime `texto1` significa "esta referencia vive al menos tanto como la variable `texto1`". Es más intuitivo: el lifetime **es el nombre** de la variable original.

## Regiones — arena allocation

Una `región` es un bloque donde **todas las variables se liberan juntas** al salir:

```falcato
fn procesar_lote() {
    región {
        el a: Texto = texto_desde("A");
        el b: Texto = texto_desde("B");
        
        // usar a y b...
        imprimir_linea(a);
        imprimir_linea(b);
        
    }  // a.liberar() y b.liberar() automáticos al salir de región
    
    // a y b ya no existen
}
```

**¿Por qué regiones?** Para kernels y sistemas donde no quieres depender del garbage collector ni tener `liberar()` por cada variable. La región libera todo al salir, como un arena allocator.

## Self-referential structs con `&yo`

Un struct que contiene una referencia a sí mismo (algo que en Rust es imposible sin trucos):

```falcato
estructural Nodo {
    dato: Entero32,
    siguiente: &yo Nodo,   // referencia al Nodo contenedor
}

fn main() {
    el raiz: Nodo = Nodo {
        dato: 10,
        siguiente: &yo raiz,  // se referencia a sí misma
    };
    imprimir_linea(raiz.siguiente.dato);  // 10
}
```

**Esto no existe en ningún otro lenguaje de sistemas.** Permite linked lists, árboles, y grafos autoreferenciales sin `unsafe`.

## Resumen rápido

| Operación | Sintaxis | Qué hace |
|-----------|----------|----------|
| Mover | `fn(x)` | Transfiere propiedad al llamar |
| Mover explícito | `mover x a fn` | Igual pero más legible |
| Copiar | `copiar x` | Clona el valor |
| Prestar inmutable | `&x` | Préstamo solo lectura |
| Prestar mutable | `&mut x` | Préstamo con permiso de escritura |
| Dereferenciar | `*ref` | Acceder al valor detrás de la referencia |
| Lifetime léxico | `&nombre T` | Referencia vinculada a vida de `nombre` |
| Región | `región { }` | Arena alloc: libera todo junto |
| Self-ref | `&yo T` | Struct que se referencia a sí mismo |

---

← [13: Async](13-async.md) | [Indice](../GUIA.md) | [Siguiente: Glosario →](15-glosario.md)
