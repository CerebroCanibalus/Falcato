# Genéricos

Los genéricos —o parámetros de tipo, que todo es uno— permiten
escribir código que funciona con cualquier tipo que cumpla ciertas
condiciones. Es como escribir un molde que luego se rellena con
el tipo concreto, y Falcato lo hace con monomorfización, que es
término complicado para decir «genera código distinto para cada
tipo que uses».

## Type Parameters

He aquí una función que halla el máximo entre dos valores, sea
cual sea su tipo —con tal de que sea comparable, que es lo justo:

```falcato
función máximo<T que Comparable>(el a: T, el b: T) -> T {
    si a > b {
        retornar a;
    } sino {
        retornar b;
    }
}
```

### Uso

El compilador infiere el tipo concreto, que no es lerdo:

```falcato
función principal() -> Entero32 {
    el max = máximo(10, 20);  // T = Entero32, inferido
    retornar max;
}
```

## Bounds declarativos

Los bounds —restricciones, en buen romance— se declaran con la
partícula `que`, que es muy española:

```falcato
función ejemplo<T que Comparable>(el a: T, el b: T) -> T { ... }
función ejemplo2<T que Ordenable>(el a: T) -> T { ... }
```

Sintaxis: `T que NombreBound`.

### Bounds disponibles

Por ahora, los bounds están grabados en piedra —hardcoded, que
dicen— y son éstos:

| Bound | Operaciones permitidas |
|-------|------------------------|
| `Comparable` | `==`, `!=`, `<`, `>`, `<=`, `>=` |
| `Ordenable` | `<`, `>`, `<=`, `>=` |
| `Numérico` | `+`, `-`, `*`, `/`, `%`, negación |

## Const Generics

Además de tipos, los genéricos pueden ser constantes —números
concretos, que la máquina entiende mejor que las abstracciones:

```falcato
función longitud<N: Entero32>(los nums: [Entero32; N]) -> Entero32 {
    // N es un parámetro constante de tipo Entero32
    retornar 0;
}
```

### Uso

```falcato
los arr: [Entero32; 5] = todos 0;
longitud(arr);  // compila con N=5
```

## Inferencia de type params

No es menester especificar el tipo cada vez. El compilador lo
deduce de los argumentos:

```falcato
máximo(10, 20)     // T = Entero32 (desde literales)
máximo(3.14, 2.0) // T = Flotante64
```

## Monomorfización

Cada vez que se usa una función genérica con un tipo concreto,
el compilador genera código separado para esa combinación. Como
en C++ o Rust: cada tipo tiene su propia función, con sus propias
instrucciones. Es generoso en código, pero eficiente en ejecución.
