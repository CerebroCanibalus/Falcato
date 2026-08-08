# 16 — Argumentos de línea de comandos (etiquetas tipadas)

← [14: Ownership](14-ownership.md) | [Indice](../GUIA.md)

---

Cuando ejecutas un programa desde la terminal, puedes pasarle argumentos:

```bash
.\saludo_app.exe --nombre sebas --cuenta 3
```

Este capítulo explica **la forma natural de leerlos en Falcato**: declaras un struct,
los **artículos de sus campos codifican el esquema**, y el compilador genera todo el
parseo, la validación de tipos y la ayuda automática. Sin librerías, sin FFI manual,
sin strings parseados a mano.

## La forma cruda: `argumentos()`

Antes de la forma elegante, existe el builtin `argumentos() -> Vector<Texto>`:

```falcato
función principal() -> Entero32 {
    los args: Vector<Texto> = argumentos();
    el n: Entero32 = vector_longitud<Texto>(args);
    imprimir_linea("recibí {n} argumentos:");

    el i: Entero32 = 0;
    mientras i < n {
        el arg: Texto = vector_obtener<Texto>(args, i);
        imprimir_linea("  [{i}] {arg}");
        texto_liberar(arg);
        i = i + 1;
    }

    vector_liberar<Texto>(args);
    retornar 0;
}
```

- `args[0]` es el nombre del ejecutable.
- `args[1..]` son los argumentos reales.
- Los `Texto` del vector ya traen null terminator (el runtime los copia a heap propio).

Es útil para herramientas que quieren el control total, pero para CLIs normales existe
una forma mucho mejor.

## La innovación: etiquetas tipadas

Falcato detecta un patrón especial en `principal`: si recibe **un parámetro de tipo
struct**, el compilador genera automáticamente el parseo de `--etiqueta valor`.

### Paso 1: declara un struct de argumentos

Los **artículos de los campos** dicen si cada argumento es obligatorio u opcional:

| Artículo | Significado |
|----------|-------------|
| `el campo` | **Requerido** — error si falta `--campo` |
| `un campo` | **Opcional** — valor por defecto si falta |
| `la campo` | **Inmutable/validado** — se valida tipo al asignar |
| `los campo` | Varargs posicionales (en desarrollo) |

```falcato
estructural Saludo {
    el nombre: Texto,       // --nombre es obligatorio
    un cuenta: Entero32,    // --cuenta es opcional (0 si falta)
    la saludo: Texto,       // --saludo obligatorio e inmutable
}
```

### Paso 2: usa el struct en `principal`

```falcato
función principal(el args: Saludo) -> Entero32 {
    imprimir_linea("hola, {args.nombre}");
    si args.cuenta > 0 {
        imprimir_linea("  cuenta: {args.cuenta}");
    }
    imprimir_linea("  saludo: {args.saludo}");
    retornar 0;
}
```

### Paso 3: compila y ejecuta

```bash
falcato compila saludo_app.fc --salida saludo_app.exe
.\saludo_app.exe --nombre sebas --cuenta 3 --saludo buenos
```

```
hola, sebas
  cuenta: 3
  saludo: buenos
```

**Eso es todo el código.** El compilador hizo el resto:

1. Leyó `argumentos()`.
2. Para cada `--etiqueta`, encontró el campo correspondiente y convirtió el valor
   al tipo del campo (`texto_a_entero`, `texto_a_flotante`, etc.).
3. Validó que los campos `el`/`la` estén presentes.
4. Generó `--ayuda` y `-h` automáticos.

## Comportamiento automático

### Faltó un argumento requerido

```bash
.\saludo_app.exe
```

```
Falta el argumento requerido: --nombre
```

Termina con código de salida 1. La validación se genera automáticamente.

### Ayuda automática

```bash
.\saludo_app.exe --ayuda
```

```
Uso: principal [--nombre <valor>] [--cuenta <valor>] [--saludo <valor>]
```

Termina con código 0. `-h` hace lo mismo.

### Opcional faltante

```bash
.\saludo_app.exe --nombre ana --saludo hola
```

```
hola, ana
  saludo: hola
```

`args.cuenta` quedó en 0 (su valor por defecto).

### Valor con tipo incorrecto

```bash
.\saludo_app.exe --nombre x --cuenta abc --saludo h
```

```
hola, x
  saludo: h
```

`abc` no es un número → la conversión devuelve 0. En la Fase 2 actual la conversión
es *silenciosa* (devuelve el valor por defecto del tipo); el error tipado con
sugerencia `[T001]` está previsto como siguiente mejora.

## Tipos soportados

| Tipo | Conversión automática |
|------|----------------------|
| `Texto` | Copia directa del argumento |
| `Entero32` | `como_entero32(texto_a_entero(valor))` |
| `Entero64` | `texto_a_entero(valor)` |
| `Natural32` | `como_entero32(texto_a_natural(valor))` |
| `Natural64` | `texto_a_natural(valor)` |
| `Flotante64` | `texto_a_flotante(valor)` |
| `Booleano` | `texto_a_booleano(valor)` (`true`, `1`, `sí`) |

## Cómo funciona por dentro

No es sintaxis nueva. Es un **preprocesamiento del AST**: `src/args_tipados.rs`
transforma `principal(el args: Saludo)` en una `principal()` sin parámetro ABI cuyo
cuerpo empieza con un prólogo generado que:

1. Llama `argumentos()`.
2. Recorre los argumentos con un bucle.
3. Compara cada uno con las etiquetas (`texto_comparar`).
4. Convierte el valor según el tipo del campo.
5. Valida los requeridos y construye el struct.

El código del usuario queda intacto después del prólogo, viendo `args` con tipos
correctos.

## Próximos pasos

- **Fase 3:** `args_avanzados` — subcomandos, valores por defecto explícitos y
  repetición de etiquetas (como librería `.fc`, sin tocar el compilador).
- Error tipado `[T001]` con sugerencia cuando el valor no convierte al tipo.
- Soporte de `los`/`las` para argumentos posicionales (varargs).
