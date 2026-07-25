# 03 — Variables: el, la, un

← [02: Tu primer programa](02-tu-primer-programa.md) | [Indice](INDICE.md) | [Siguiente: Operaciones →](04-operaciones.md)

---

Las variables se declaran con un **artículo**. Como en español de verdad.

## el — tuyo, puedes cambiarlo

```falcato
el edad: Entero32 = 25;
edad = 26;    // se puede
```

## la — prestado, solo lectura

```falcato
la nombre: Palabra = "Ana";
nombre = "Luis";  // error: no se puede cambiar
```

## un — quizas existe, quizas no

```falcato
un apodo: Palabra;  // no sabemos que contiene
```

## El truco para no pensar

Usa `la` siempre. Si el compilador se queja, cambia a `el`. Asi el compilador te guia.

## Tipos basicos

| Tipo | Tamaño | Valores |
|------|--------|---------|
| `Entero32` | 4 bytes | numeros normales |
| `Flotante64` | 8 bytes | decimales precisos |
| `Booleano` | 1 byte | `verdadero` o `falso` |
| `Palabra` | 8 bytes | texto fijo |

## Errores tipicos

```falcato
// Error: tipo incorrecto
el nombre: Entero32 = "Ana";

// Error: variable inmutable
la edad: Entero32 = 25;
edad = 26;

// Correcto
el nombre: Palabra = "Ana";
el edad: Entero32 = 25;
edad = 26;
```

---

← [02: Tu primer programa](02-tu-primer-programa.md) | [Indice](INDICE.md) | [Siguiente: Operaciones →](04-operaciones.md)
