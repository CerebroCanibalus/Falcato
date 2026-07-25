# 06 — Bucles: mientras, para

← [05: Decisiones](05-decisiones.md) | [Indice](../GUIA.md) | [Siguiente: Funciones →](07-funciones.md)

---

Los bucles repiten código. Hay dos formas: `mientras` (no sabes cuántas veces) y `para` (sabes cuántas veces).

## mientras — "sigue mientras se cumpla esto"

Útil cuando **no sabes cuántas iteraciones** vas a necesitar.

```falcato
// Buscar el primer número divisible por 7
el i: Entero32 = 1;
mientras i % 7 != 0 {
    i = i + 1;
}
decir("El primer multiplo de 7 es {i}");
// → "El primer multiplo de 7 es 7"
```

```falcato
// Leer datos hasta que llegue un centinela
el entrada: Entero32 = leer_sensor();
mientras entrada != -1 {          // -1 = "no hay más datos"
    procesar(entrada);
    entrada = leer_sensor();
}
```

### while true

```falcato
mientras verdadero {
    el dato = recibir_dato();
    si dato es 0 { interrumpir; }  // sale del bucle
    procesar(dato);
}
```

### Romper el bucle

```falcato
// interrumpir — sale del bucle
mientras verdadero {
    decir("Trabajando...");
    si ya_termine { interrumpir; }
}

// continuar — salta a la siguiente iteración
para i en 0..10 {
    si i % 2 es 0 { continuar; }  // pares: saltar
    decir("{i} es impar");
}
// → 1, 3, 5, 7, 9
```

## para — "para cada elemento"

Cuando **sabes exactamente** qué recorrer. Es más corto y seguro que `mientras` porque no puedes olvidar incrementar.

```falcato
// Sobre rangos
para i en 0..5 {            // 0, 1, 2, 3, 4
    decir("Vuelta {i}");
}

para i en 0..=5 {           // 0, 1, 2, 3, 4, 5
    decir("Incluye el 5");
}

// Sobre arreglos
los dias: [Palabra; 5] = ["Lu", "Ma", "Mi", "Ju", "Vi"];
para d en dias {
    decir("Hoy es {d}");
}

// Sobre vectores
el datos: Vector<Entero32> = vector_nuevo();
datos.agregar(10);
datos.agregar(20);
datos.agregar(30);

para val en datos {
    decir("Valor: {val}");
}
datos.liberar();
```

## Bucles anidados — ejemplos reales

```falcato
// Tabla de multiplicar
para i en 1..=10 {
    para j en 1..=10 {
        imprimir("{i * j}\t");  // sin salto de línea
    }
    imprimir_linea("");  // salto al cambiar de fila
}

// Buscar un valor en una matriz 2D
los matriz: [[Entero32; 3]; 3] = [
    [1, 2, 3],
    [4, 5, 6],
    [7, 8, 9]
];

el buscado: Entero32 = 5;
para fila en 0..3 {
    para columna en 0..3 {
        si matriz[fila][columna] es buscado {
            decir("Encontrado en [{fila}][{columna}]");
            interrumpir;  // sale del bucle interno
        }
    }
}
```

## Rangos

```falcato
0..5        // 0, 1, 2, 3, 4      (exclusivo: el 5 no entra)
0..=5       // 0, 1, 2, 3, 4, 5  (inclusivo: el 5 sí entra)

// Se pueden invertir con paso manual
para i en 0..5 {
    el invertido = 4 - i;  // 4, 3, 2, 1, 0
}
```

## Errores típicos

```falcato
// Error: bucle infinito (falta incrementar)
el x: Entero32 = 0;
mientras x < 10 {
    decir("x vale {x}");
    // olvidaste: x = x + 1;
}

// Error: confundir rango exclusivo con inclusivo
para i en 0..5 {
    // i llega hasta 4, no hasta 5
}

// Correcto
para i en 0..=5 {
    // i llega hasta 5
}
```

## ¿mientras o para?

| Situación | Usa |
|-----------|-----|
| Sabes cuántas veces | `para` |
| Recorres arreglo/vector | `para` |
| No sabes cuándo termina | `mientras` |
| Depende de una condición externa | `mientras` |
| Bucle infinito con salida interna | `mientras verdadero` |

---

← [05: Decisiones](05-decisiones.md) | [Indice](../GUIA.md) | [Siguiente: Funciones →](07-funciones.md)
