# 08 — Texto y Palabra

← [07: Funciones](07-funciones.md) | [Indice](../GUIA.md) | [Siguiente: Colecciones →](09-colecciones.md)

---

Falcato tiene **dos tipos** para texto. No es capricho: cada uno sirve para una cosa distinta.

## Palabra — texto fijo (rápido, sin liberar)

```falcato
la saludo: Palabra = "Hola";
```

- **Fijo**: no se puede modificar, no se puede alargar
- **Gratis**: no hay que liberarlo, vive en el binario del programa
- **Rápido**: es solo un puntero a datos estáticos

Usa `Palabra` para: mensajes fijos, constantes, nombres, configuraciones.

## Texto — texto que crece (flexible, hay que liberar)

```falcato
el t: Texto = texto_desde("Hola");
t.agregar(", mundo");
decir(t);           // "Hola, mundo"
el tam = t.tam();   // 11
t.liberar();        // ← ¡siempre!
```

- **Dinámico**: puedes agregar, cortar, concatenar
- **Vivo en heap**: memoria flexible, pagas por ello
- **Hay que liberarlo**: cada `texto_desde`, cada `a + b`, cada `t[0..5]` crea un Texto nuevo

> **Regla de oro:** cada `texto_desde()` o `+` entre Textos es como un plato prestado. Lo usas, lo lavas (`.liberar()`), lo devuelves. Olvidarlo = pérdida de memoria.

## Caracter — una letra

```falcato
el letra: Caracter = 'A';
// Las comillas simples distinguen: 'A' es Caracter, "A" es Palabra
```

Un Caracter es un solo byte (Entero8). Sirve para recorrer texto letra por letra.

## Secuencias de escape

Dentro de las comillas dobles `"..."` puedes poner caracteres especiales:

| Secuencia | Significado |
|-----------|-------------|
| `\n` | Salto de línea (nueva línea) |
| `\t` | Tabulador |
| `\\` | Barra invertida literal |
| `\"` | Comilla doble literal |
| `\0` | Caracter nulo (fin de string C) |
| `\xNN` | Byte en hexadecimal (ej: `\x48` = 'H') |

```falcato
el texto: Palabra = "Línea 1\nLínea 2\tTabulado";
decir(texto);
// → Línea 1
//   Línea 2    Tabulado

el ruta: Palabra = "C:\\Usuarios\\Ana\\docs";  // las barras necesitan \\
el comilla: Palabra = "Ella dijo: \"Hola\"";
el binario: Palabra = "\x48\x6F\x6C\x61";  // "Hola" en hex
```

## Interpolación

```falcato
el nombre: Palabra = "Ana";
el edad: Entero32 = 30;
decir("{nombre} tiene {edad} años");
// → "Ana tiene 30 años"
```

Puedes meter cualquier expresión:

```falcato
decir("Suma: {2 + 3}");
decir("Mayor: {max(10, 20)}");
decir("{nombre}: {edad + 1} el año que viene");
```

## Concatenación con +

```falcato
el a: Texto = texto_desde("Hola ");
el b: Texto = texto_desde("mundo");
el c: Texto = a + b;   // "Hola mundo" — nuevo Texto
a.liberar();
b.liberar();
decir(c);
c.liberar();
```

`a + b` **no modifica** a ni b. Crea un Texto nuevo. Tienes que liberar los tres.

## Procesamiento real de strings

```falcato
fn contar_vocales(la texto: Palabra) -> Entero32 {
    // Convertir Palabra a Texto para poder acceder por índice
    el t: Texto = texto_desde(texto);
    el contador: Entero32 = 0;

    para i en 0..t.tam() {
        el c: Entero8 = t[i];
        // Cada byte es un Caracter (Entero8)
        si c es 65 || c es 69 || c es 73 || c es 79 || c es 85 {  // A,E,I,O,U
            contador = contador + 1;
        } o si c es 97 || c es 101 || c es 105 || c es 111 || c es 117 {  // a,e,i,o,u
            contador = contador + 1;
        }
    }

    t.liberar();
    retornar contador;
}
```

```falcato
fn primera_palabra(la texto: Palabra) -> Texto {
    el t: Texto = texto_desde(texto);

    // Buscar el primer espacio
    para i en 0..t.tam() {
        si t[i] es 32 {  // espacio en ASCII
            el resultado = t[0..i];  // subtexto hasta el espacio
            t.liberar();
            retornar resultado;
        }
    }

    // No encontró espacio → devolver todo
    retornar t;  // pasamos la propiedad
}
```

## Comparar Palabra y Texto

```falcato
// Las Palabras se comparan con es
si nombre es "Ana" { decir("Eres Ana"); }

// Los Textos necesitan texto_comparar()
el t1: Texto = texto_desde("Hola");
el t2: Texto = texto_desde("Hola");
si texto_comparar(t1, t2) es 0 {
    decir("Son iguales");
}
t1.liberar();
t2.liberar();
```

## Métodos útiles de Texto

| Código | Efecto | Devuelve |
|--------|--------|----------|
| `t.agregar("x")` | Añade texto al final | `Vacio` |
| `t.tam()` | Cuántos bytes tiene | `Entero32` |
| `t.liberar()` | Libera la memoria | `Vacio` |
| `t.obtener(i)` | Byte en posición i | `Entero8` |
| `t[0]` | Byte en posición 0 | `Entero8` |
| `a + b` | Concatena dos Textos (nuevo) | `Texto` |
| `t[0..5]` | Extrae bytes 0 a 4 (nuevo) | `Texto` |

## ¿Palabra o Texto?

| Situación | Usa | Por qué |
|-----------|-----|---------|
| Mensaje fijo | `Palabra` | No se modifica, no hay que liberar |
| Constante del programa | `Palabra` | Vive en el binario |
| Entrada del usuario | `Texto` | No sabes cuánto mide |
| Construir un mensaje | `Texto` | Necesitas agregar, concatenar |
| Parámetro de función (solo lectura) | `la` + `Palabra` | Más ligero |
| Devolver string dinámico | `Texto` | Tiene que vivir después del return |

---

← [07: Funciones](07-funciones.md) | [Indice](../GUIA.md) | [Siguiente: Colecciones →](09-colecciones.md)
