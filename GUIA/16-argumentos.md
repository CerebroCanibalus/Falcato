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

## Fase 3: la librería `args_avanzados`

Cuando el patrón de `principal(el args: Struct)` se queda corto (subcomandos,
repetición de etiquetas, posicionales), existe la librería
`librerias/args_avanzados.fc` — pura `.fc`, sin tocar el compilador.

### Uso

```falcato
usar args_avanzados::*;

función principal() -> Entero32 {
    los argv: Vector<Texto> = argumentos();

    // Subcomando: `app desplegar --nombre sebas`
    el sub: Texto = args_subcomando(argv);        // "desplegar"
    imprimir_linea("subcomando: {sub}");
    texto_liberar(sub);

    // Etiqueta con valor (default si falta)
    el nombre: Texto = args_obtener(argv, "--nombre");  // "sebas"
    si texto_longitud(nombre) == 0 {
        nombre = texto_desde("invitado");               // default manual
    }
    imprimir_linea("hola, {nombre}");
    texto_liberar(nombre);

    // Repetición: `--tag a --tag b --tag c`
    los tags: Vector<Texto> = args_todos(argv, "--tag");  // [a, b, c]
    vector_liberar<Texto>(tags);

    // Posicionales después del subcomando
    los pos: Vector<Texto> = args_posicionales(argv);     // [item1, item2]
    vector_liberar<Texto>(pos);

    vector_liberar<Texto>(argv);
    retornar 0;
}
```

### Compilar

```bash
falcato compila app.fc librerias/args_avanzados.fc --salida app.exe
```

### API

| Función | Devuelve | Descripción |
|---------|----------|-------------|
| `args_tiene(argv, "--x")` | `Booleano` | ¿Existe la etiqueta? |
| `args_obtener(argv, "--x")` | `Texto` | Valor de la primera aparición (`""` si falta) |
| `args_todos(argv, "--x")` | `Vector<Texto>` | Todos los valores (repetición) |
| `args_subcomando(argv)` | `Texto` | Primer token sin `--` |
| `args_posicionales(argv)` | `Vector<Texto>` | Tokens sin `--` tras la primera etiqueta |
| `args_cuenta(argv)` | `Entero32` | Nº de argumentos crudos |

**Contrato de memoria:** todas las funciones devuelven **copias independientes**
(`malloc` propias). El caller libera con `texto_liberar`/`vector_liberar`. Nunca se
comparten descriptores con `argv` — así se evita el double-free.

## Próximos pasos

- Error tipado `[T001]` con sugerencia cuando el valor no convierte al tipo.
- Soporte de `los`/`las` para argumentos posicionales (varargs) en `principal`.`
