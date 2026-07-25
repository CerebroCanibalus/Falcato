# 🚨 Falcato — Códigos de error

> Todos los errores del compilador tienen un código como `[T001]` y una sugerencia
> de cómo arreglarlos. Esta guía explica cada categoría.
>
> **📖 Primero lee:** [GUIA.md](GUIA.md) — tutorial desde cero

---

## 📚 Documentación relacionada

| Guía | Descripción |
|------|-------------|
| [📖 GUIA.md](GUIA.md) | Tutorial completo desde cero |
| [⚙️ INSTALL.md](INSTALL.md) | Instalación |
| [📗 REFERENCIA.md](REFERENCIA.md) | Catálogo de funciones |
| [🚨 ERRORES.md](ERRORES.md) | **← Estás aquí** |

---

## Formato de los errores

```
[T001] archivo.fc:7:12: mensaje de error
       │ sugerencia: cómo arreglarlo
```

| Parte | Significado |
|-------|-------------|
| `[T001]` | Categoría `T` (Tipo), número `001` |
| `archivo.fc:7:12` | Archivo, línea 7, columna 12 |
| `mensaje...` | Qué pasó, en español |
| `sugerencia:` | Cómo arreglarlo (cuando aplica) |

---

## Categorías de error

| Código | Categoría | Qué significa |
|--------|-----------|---------------|
| `[S###]` | Sintaxis | Algo está mal escrito |
| `[T###]` | Tipo | Los tipos no concuerdan |
| `[O###]` | Ownership | Problema de propiedad/borrowing |
| `[C###]` | FFI | Error en llamada a C |
| `[M###]` | Módulos | Error de importación/visibilidad |
| `[I###]` | Interno | Error del compilador (reportar) |
| `[W###]` | Warning | Advertencia (no impide compilar) |

---

## [S###] — Errores de sintaxis

Ocurren cuando el compilador no entiende lo que escribiste.

| Código | Significado | Solución |
|--------|-------------|----------|
| `S001` | Token inesperado | Revisa la línea, falta un símbolo (`;`, `}`, `)`, etc.) |
| `S002` | Fin de archivo inesperado | Olvidaste cerrar un bloque `{ }` |
| `S003` | Identificador esperado | Después de `.` debe ir un nombre |
| `S008` | Carácter no válido | El archivo contiene caracteres extraños (BOM, UTF-16) |
| `S012` | Artículo fuera de contexto | `el`/`la` solo precede a `función` en contexto de módulo |

**Ejemplo:**
```
[S001] hola.fc:3:1: Token inesperado: se esperaba ';', encontrado '}'
       │ sugerencia: Revisa que todas las sentencias terminen con ;
```

---

## [T###] — Errores de tipo

Ocurren cuando mezclas tipos que no deberían mezclarse.

| Código | Significado | Solución |
|--------|-------------|----------|
| `T001` | Disconcordancia de tipo | Cambia el tipo de la variable o el valor |
| `T002` | Disconcordancia de retorno | El tipo retornado no coincide con la firma de la función |
| `T003` | Retorno faltante | La función declara un tipo de retorno pero falta `retornar` |
| `T004` | Variable no declarada | El nombre no existe en el contexto actual. Revisa el artículo y ortografía |
| `T005` | Disconcordancia de operandos | Ambos lados de una operación deben ser del mismo tipo |
| `T006` | Operación aritmética inválida | Solo números pueden sumarse, restarse, etc. |
| `T007` | Comparación inválida | `==`/`<`/`>` entre tipos incomparables |
| `T008` | Operación lógica inválida | `&&`/`||` solo entre booleanos |
| `T009` | Negación aritmética inválida | `-` solo aplicable a números |
| `T010` | Negación lógica inválida | `!` solo aplicable a booleanos |
| `T011` | Condicional no booleano | `si` requiere una condición que sea verdadero/falso |
| `T012` | Bucle no booleano | `mientras` requiere una condición booleana |
| `T013` | Asignación incompatible | El tipo del valor no coincide con el tipo de la variable |
| `T015` | Índice de arreglo no entero | Los índices deben ser `Entero32` o `Entero64` |
| `T016` | Acceso a no-arreglo | El operador `[]` solo funciona en arreglos |
| `T017` | Arreglo heterogéneo | Todos los elementos de un arreglo deben ser del mismo tipo |
| `T018` | Campo de struct no existe | El struct no tiene ese campo |
| `T019` | Falta campo en inicialización | Debes inicializar todos los campos del struct |
| `T020` | Struct no declarado | Usa `estructural` para declarar el struct primero |
| `T021` | Acceso a campo en no-struct | El operador `.` solo funciona en structs |
| `T022` | Cantidad de argumentos incorrecta | La función espera un número específico de argumentos |
| `T024` | Argumentos de variante incorrectos | El constructor del enum espera ciertos argumentos |
| `T025` | Variante sin datos con argumentos | Esa variante del enum no acepta datos |
| `T026` | Variante de enum no existe | La enumeración no tiene esa variante |
| `T027` | Enumeración no declarada | Usa `enumeración` para declarar el enum primero |
| `T028` | Pattern matching tipo incorrecto | El tipo de la expresión no coincide con el enum |
| `T029` | Operador `?` en no-Resultado | `?` solo funciona con `Resultado<T,E>` |
| `T030` | Desreferencia inválida | `*` solo funciona en referencias (`&T`) |
| `T060` | Rasgo no existe | El nombre del rasgo está mal escrito |
| `T061` | Falta método requerido | El rasgo exige implementar ese método |
| `T070` | Match en tipo no-enum | `coincidir` solo funciona con enums |
| `T071` | Patrón de variante no existe | Esa variante no está definida en el enum |
| `T072` | Binding de variante sin datos | No puedes extraer datos de una variante sin datos |
| `T073` | Falta binding en variante con datos | Debes usar `como nombre` para extraer los datos |
| `T074` | Variante duplicada en match | Ya cubriste esa variante |
| `T080` | `esperar` fuera de `fut función` | Usa `esperar` solo dentro de funciones `fut` |
| `T081` | `esperar` requiere `Futuro<T>` | La expresión no es un futuro |
| `T084` | `bloquear` causaría deadlock | No uses `bloquear` dentro de `fut función` |
| `T085` | `direccion_de` requiere función | El nombre debe ser una función visible en el scope actual |
| `T090` | Error en `seleccionar` | El canal no existe o el patrón es inválido |
| `T091` | Error en `seleccionar` | Variante duplicada o tipo incorrecto |
| `T099` | División o módulo por cero | El divisor no puede ser cero literal |

**Ejemplo:**
```
[T001] test.fc:4:8: Disconcordancia de tipo: 'a' es 'Entero32' pero se declaró como 'Booleano'
       │ sugerencia: Cambia el tipo a 'Entero32' o el valor
```

---

## [O###] — Errores de ownership

Ocurren cuando violas las reglas de quién es dueño de un dato.

| Código | Significado | Solución |
|--------|-------------|----------|
| `O001` | Uso después de mover | La variable ya fue movida a otro lado. Opción A: úsala antes de mover. Opción B: haz `copiar x` antes de mover |
| `O002` | Borrow mutable duplicado | Ya tienes un `&mut` activo. Opción A: usa el existente. Opción B: termínalo antes de crear otro |
| `O003` | Borrow mutable + inmutable | No puedes tener `&mut` y `&` al mismo tiempo |
| `O004` | Borrow inmutable + mutable | No puedes crear `&mut` si ya hay `&` activos |
| `O050` | Función `puro` muta parámetro | Una función pura no puede modificar sus parámetros |

**Ejemplo:**
```
[O001] test.fc:5:5: 'constante' no es mutable: se declaró con 'la' (inmutable)
       │ sugerencia: Usa 'el constante' para hacerlo mutable
```

---

## [C###] — Errores de FFI (llamadas a C)

| Código | Significado | Solución |
|--------|-------------|----------|
| `C001` | Función no encontrada | Revisa el nombre de la función externa |
| `C002` | Error de linkage | Falta una biblioteca en el linker |

---

## [M###] — Errores de módulos

| Código | Significado | Solución |
|--------|-------------|----------|
| `M001` | Visibilidad privada | El símbolo existe pero es privado. Marca la función como `el función` para hacerla pública |
| `M002` | Símbolo no encontrado | El nombre no existe en el módulo. Revisa la ruta del import |

---

## [W###] — Advertencias (no impiden compilar)

| Código | Significado | Solución |
|--------|-------------|----------|
| *Próximamente* | — | — |

---

## Consejos para evitar errores

### 1. Lee el error completo

Los errores de Falcato incluyen **línea, columna y sugerencia**. No solo mires
el código — lee el mensaje completo.

### 2. Error de tipo más común: olvidar el tipo

```falcato
// ❌ Error: ¿qué tipo es 'x'?
el x = 10;

// ✅ Correcto
el x: Entero32 = 10;
```

### 3. Error de ownership más común: mutable vs inmutable

```falcato
// ❌ Error: 'nombre' es inmutable
la nombre: Palabra = "Ana";
nombre = "Luis";

// ✅ Correcto: declarar como mutable
el nombre: Palabra = "Ana";
nombre = "Luis";
```

### 4. Error de sintaxis más común: olvidar punto y coma

```falcato
// ❌ Error: falta ;
si x > 5 { decir("hola") }

// ✅ Correcto
si x > 5 { decir("hola"); }
```

### 5. Error de memoria más común: no liberar

```falcato
// ❌ Fuga de memoria
el t: Texto = texto_desde("Hola");
decir(t);
// falta t.liberar()

// ✅ Correcto
el t: Texto = texto_desde("Hola");
decir(t);
t.liberar();
```

### 6. Si ves T004 con una sugerencia "quizás quisiste decir"

```falcato
// ❌ Error: typo en nombre de variable
el temperatura: Entero32 = 25;
imprimir_linea(tenperatura);  // T004: 'tenperatura' no declarado

// ✅ El compilador sugiere el nombre correcto
//    "¿Quizás quisiste decir 'temperatura'?"
```

---

## Si nada funciona

1. Busca tu código en la carpeta `ejemplos/` — hay 70+ programas que puedes
   usar como referencia
2. Revisa [GUIA.md](GUIA.md) — el capítulo relevante explica el concepto
3. Revisa [REFERENCIA.md](REFERENCIA.md) — las firmas de las funciones
4. Si crees que es un error del compilador, reporta el código `[I###]`
   en [github.com/CerebroCanibalus/falcato/issues](https://github.com/CerebroCanibalus/falcato/issues)
