# 07 — Funciones

← [06: Bucles](06-bucles.md) | [Indice](../GUIA.md) | [Siguiente: Texto y Palabra →](08-texto.md)

---

## Declaración básica

```falcato
función sumar(el a: Entero32, el b: Entero32) -> Entero32 {
    retornar a + b;
}
```

Partes: `función` + nombre + `(parámetros)` + `-> Tipo` + `{ cuerpo }`

## Llamar

```falcato
función principal() -> Entero32 {
    el resultado = sumar(3, 4);  // 7
    decir("3 + 4 = {resultado}");
    retornar 0;
}
```

## Sin retorno

```falcato
función saludar(la nombre: Palabra) -> Vacio {
    decir("Hola, {nombre}");
}
```

## Formas de escribir "función"

Todas funcionan:
```
función suma(...) { }   // con tilde
funcion suma(...) { }   // sin tilde
fn suma(...) { }        // corta
```

## Genéricos

Una función que funciona con cualquier tipo:

```falcato
función maximo<T que Comparable>(el a: T, el b: T) -> T {
    si a > b { retornar a; } sino { retornar b; }
}

maximo(3, 5);       // T = Entero32
maximo(3.14, 2.71); // T = Flotante64
```

`<T que Comparable>` = "T debe poderse comparar con >, <, etc."

## devolver

Puedes usar `devolver` en vez de `retornar`:

```falcato
función suma(a: Entero32, b: Entero32) -> Entero32 {
    devolver a + b;
}
```

---

← [06: Bucles](06-bucles.md) | [Indice](../GUIA.md) | [Siguiente: Texto y Palabra →](08-texto.md)
