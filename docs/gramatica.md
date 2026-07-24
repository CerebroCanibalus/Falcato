# Gramática de Falcato (borrador)

> ⚠️ Este documento es especulativo. Todo puede cambiar.

## 1. Sistema de artículos y género

El género gramatical (masculino/femenino) codifica **ownership y préstamo**
de memoria, una dimensión que no existe en los lenguajes de sistemas actuales.

| Artículo | Semántica | Equivalente Rust |
|----------|-----------|-----------------|
| `el` | Propietario único, mutable por defecto | `let mut` |
| `la` | Préstamo/Referencia, inmutable | `let` / `&` |
| `un` | Opcional / tal vez exista | `Option<T>` |
| `los` | Colección propietaria | `Vec<T>` |
| `las` | Colección prestada | `&[T]` |

### Ejemplos

```falcat
el contador: Entero32 = 0;       // owned, mutable
la referencia: &Entero32 = &contador;  // borrowed, inmutable
un archivo: Opción<Archivo> = ...;     // optional, maybe
los items: Lista<Entero32> = ...;      // owned collection
las entradas: &[Entrada] = ...;        // borrowed slice
```

### Plural como indicador de colecciones

```falcat
el proceso: Proceso;             // single item, owned
los procesos: Lista<Proceso>;    // collection, owned
las procesos: &[Proceso];        // collection, borrowed (fem. in gender)
```

## 2. Tiempos verbales como modo de ejecución

| Tiempo | Función | Ejecución |
|--------|---------|-----------|
| **Presente** | `ejecuta` | Síncrona, bloqueante |
| **Futuro** | `ejecutará` | Asíncrona, `async`, `Future` |
| **Pretérito** | `ejecutó` | Ya completada, `.join()`, resultado |
| **Imperfecto** | `ejecutaba` | Iterativa, generadora, `yield` |
| **Condicional** | `ejecutaría` | Fallback, `unwrap_or`, default |
| **Subjuntivo** | `ejecute` | Fallible, `try_`, retorna Result |
| **Gerundio** | `ejecutando` | Stream, callback, evento |
| **Imperativo** | `ejecuta` (con !) | `unsafe`, acceso directo |

### Ejemplos

```falcat
// Presente — síncrono
función procesar(dato: Entero32): Entero32 {
    retornar dato * 2;
}

// Futuro — asíncrono
función descargar(urll: Palabra): Futuro<Archivo> {
    // compila a async
}

// Subjuntivo — fallible
función dividir(a: Entero32, b: Entero32): Result<Entero32, Error> {
    si b sea 0 {
        fallar("división por cero");
    }
    retornar a / b;
}

// Imperfecto — iterador/generador
función fibonacci(hasta: Entero32): Generador<Entero32> {
    sea a = 0, b = 1;
    mientras a < hasta {
        producir a;  // yield
        (a, b) = (b, a + b);
    }
}
```

## 3. Ser vs. Estar — Permanencia vs. Transitoriedad

| Verbo | Significado en Falcato |
|-------|----------------------|
| `ser` | Constante en tiempo de compilación, inmutable global |
| `estar` | Variable mutable, puede cambiar en runtime |

```falcat
ser TAMAÑO_BUFFER: Entero32 = 4096;       // compile-time constant
estar buffer: [Byte; TAMAÑO_BUFFER];       // mutable stack array

// Ser en funciones = siempre inline / constexpr
ser función cuadrado(x: Entero32): Entero32 { x * x }  // comptime
```

## 4. Prefijos semánticos productivos

| Prefijo | Semántica en Falcato |
|---------|----------------------|
| `re-` | Reintentar, rehacer (`retry`, `redo`) |
| `pre-` | Pre-cálculo, pre-carga (`precompute`) |
| `pos-` | Post-procesamiento |
| `des-` | Destruir, deshacer (`drop`, `free`, `un-`) |
| `co-` | Cooperativo, concurrente |
| `sobre-` | Sobrecargar, sobrescribir |
| `sub-` | Sub-proceso, sub-tarea |
| `entre-` | Inter-operación, entre hilos |
| `auto-` | Auto-gestionado |
| `contra-` | Contra-bloqueo, prevención |
| `tras-` | Tras-pasar, transferir |
| `mal-` | Mal-formato, error esperado |

```falcat
re-intentar(operación fallida)     // retry loop
des-cargar(archivo)                // unload / free
pre-calcular(expresión)            // compute at compile time
entre-hilos(enviar mensaje)        // inter-thread operation
co-procesar(hilo_1, hilo_2)        // concurrent processing
```

## 5. Voz activa vs. pasiva

```falcat
// Activa — el sujeto actúa
función transformar(la dato: &Dato): DatoTransformado { ... }

// Pasiva — el dato fluye
dato sea transformado por proceso;   // pipeline: dato → procesado
```

## 6. Compuestos aglutinantes

Los nombres compuestos en español fusionan palabras de forma natural:

```falcat
// API auto-documentada
corta-fuegos                  // firewall
limpia-memoria                // GC / memory cleaner
cuenta-referencias            // reference counter
busca-ordenar-filtra          // pipeline funcional
porta-datos                   // data carrier struct
salva-estado                  // state saver
abre-archivo                  // file opener
```

## 7. Conectores de control flow

```falcat
mientras condición { ... }
hasta_que condición { ... }
tan_pronto_como evento { ... }
a_menos_que condición { ... }     // guard clause
con_tal_que condición { ... }     // precondition
ya_que invariante { ... }         // assertion
en_cuanto señal { ... }           // async trigger
cada_vez_que evento { ... }       // event listener
```

## Keywords provisionales

| Falcato | Concepto |
|---------|----------|
| `función` | Function declaration |
| `retornar` | Return value |
| `si` / `entonces` / `sino` | If / else |
| `machea` | Pattern match (like Rust `match`) |
| `para` | For loop |
| `mientras` | While loop |
| `deja` | Let binding |
| `modulo` | Module |
| `convención` | Trait / interface |
| `realización` | Impl / implementation |
| `estructural` | Struct |
| `enumeración` | Enum |
| `tipo` | Type alias |
| `donde` | Where clause / generics |
| `como` | As cast |
| `usar` | Use / import |
| `fallar` | Return error |
| `producir` | Yield |
| `inseguro` | Unsafe block |
| `presta` | Borrow |
| `mueve` | Move ownership |
