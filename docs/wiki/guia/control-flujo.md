# Control de Flujo

No basta con declarar variables y quedarse mirándolas, cual
hidalgo que contempla sus libros sin leerlos. Es menester
que el programa tome decisiones, que salte, que gire, que
se repita. De eso trata este capítulo.

## Condicionales (`si` / `sino`)

Allá va el ejemplo más llano:

```falcato
función principal() -> Entero32 {
    el x: Entero32 = 10;
    si x > 5 {
        retornar 100;
    } sino {
        retornar 0;
    }
}
```

La condición ha de ser de tipo `Booleano`. Si no lo es, el
compilador alza la voz:

```falcato
si x { ... }  // [T011] Error: requiere Booleano, encontrado Entero32
```

No hay término medio ni contemplaciones: o es verdadero, o es falso.

## Ser/Estar en condicionales

Falcato, que bebe del español, distingue entre identidad y estado:

```falcato
si x es 5 {       // "es" = identidad (comparación estructural)
    // ...
}

si x está 10 {    // "está" = estado (por ahora, mismo efecto que "es")
    // ...
}
```

En ediciones venideras, el «está» adquirirá plena semántica de estado
temporal —mas por ahora, entrambos hacen lo mismo: comparar.

## Subjuntivo (`fuese`)

Y éste es otro rasgo singular: el modo subjuntivo, que marca un
camino como improbable —el «cold path» de los compiladores, dicho
sea en romance:

```falcato
si x fuese > 1000 {    // Subjuntivo: branch improbable
    // Cold path — el compilador sabe que esto no ocurrirá a menudo
}
```

El compilador, al ver `fuese`, genera una pista para que el
procesador no se moleste en predecir este camino. Es como decirle
«no apuestes por esto, que rara vez sucede».

## Pattern Matching con enums

Cuando los enums aparecen, el `si` con `es` hace las veces de
`match`:

```falcato
enumeración Estado { Activo, Inactivo }

función principal() -> Entero32 {
    el estado: Estado = Estado.Activo;
    si estado es Estado.Activo {
        retornar 1;
    }
    retornar 0;
}
```

## Bucle `mientras`

Mientras la condición sea verdadera, el cuerpo se repite. No hay
vuelta de hoja:

```falcato
función principal() -> Entero32 {
    el i: Entero32 = 0;
    mientras i < 10 {
        i = i + 1;
    }
    retornar i;  // → 10
}
```

## Bucle `para`

Para recorrer arrays, Falcato ofrece el bucle `para`, que itera
sobre cada elemento:

```falcato
función principal() -> Entero32 {
    los nums: [Entero32; 3] = [10, 20, 30];
    el suma: Entero32 = 0;
    para num en nums {
        suma = suma + num;
    }
    retornar suma;  // → 60
}
```

## Asignación

A las variables declaradas con `el` (que son de uno, como se ha dicho)
se les puede cambiar el valor:

```falcato
el x: Entero32 = 10;
x = 20;  // va y se pone
```

Y a los elementos de un array también:

```falcato
el nums: [Entero32; 3] = [1, 2, 3];
nums[1] = 99;  // → [1, 99, 3]
```
