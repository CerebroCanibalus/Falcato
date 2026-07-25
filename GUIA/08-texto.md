# 08 — Texto y Palabra

← [07: Funciones](07-funciones.md) | [Indice](../GUIA.md) | [Siguiente: Colecciones →](09-colecciones.md)

---

Falcato tiene **dos tipos** para texto. No es capricho: cada uno sirve para una cosa.

## Palabra — texto fijo (rápido)

```falcato
la saludo: Palabra = "Hola";
```

No se puede modificar. No hay que liberarla. Es instantánea.

## Texto — texto que crece

```falcato
el t: Texto = texto_desde("Hola");
t.agregar(", mundo");
decir(t);            // "Hola, mundo"
el tam = t.tam();   // 11
t.liberar();         // ¡importante!
```

> **Regla de oro:** cada `texto_desde(...)` es un plato prestado. Lo usas, lo lavas (`.liberar()`), lo devuelves.

## Métodos de Texto

| Código | Efecto |
|--------|--------|
| `t.agregar("hola")` | Añade texto al final |
| `t.tam()` | Cuantos bytes tiene |
| `t.liberar()` | Libera la memoria |
| `t[0]` | Byte en la posicion 0 |
| `a + b` | Concatena dos Textos |
| `t[0..5]` | Extrae del byte 0 al 4 |

## Interpolación

```falcato
el nombre: Palabra = "Ana";
el edad: Entero32 = 30;
decir("{nombre} tiene {edad} años");
// → "Ana tiene 30 años"
```

## Concatenación con +

```falcato
el a: Texto = texto_desde("Hola ");
el b: Texto = texto_desde("mundo");
el c: Texto = a + b;   // "Hola mundo" (nuevo)
a.liberar();
b.liberar();
decir(c);
c.liberar();
```

---

← [07: Funciones](07-funciones.md) | [Indice](../GUIA.md) | [Siguiente: Colecciones →](09-colecciones.md)
