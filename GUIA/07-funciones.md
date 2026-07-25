# 07 — Funciones

← [06: Bucles](06-bucles.md) | [Indice](../GUIA.md) | [Siguiente: Texto y Palabra →](08-texto.md)

---

Las funciones son la herramienta más básica para organizar código. Código sin funciones es como una casa sin habitaciones — todo amontonado.

## Anatomía de una función

```falcato
función sumar(el a: Entero32, el b: Entero32) -> Entero32 {
    retornar a + b;
}
```

- **`función`** — palabra clave (también `fn` o `funcion`)
- **`sumar`** — nombre
- **`(el a: Entero32, el b: Entero32)`** — parámetros (artículo + nombre + tipo)
- **`-> Entero32`** — tipo de retorno
- **`{ retornar a + b; }`** — cuerpo

## Parámetros: el vs la

El artículo del parámetro dice qué puede hacer la función con él:

```falcato
// 'la' → solo lectura (prestado)
fn saludar(la nombre: Palabra) -> Vacio {
    decir("Hola, {nombre}");
    // nombre = "otro";  // Error: 'la' es inmutable
}

// 'el' → la función es dueña (puede consumir)
fn consumir_texto(el msg: Texto) -> Vacio {
    decir(msg);
    msg.liberar();  // podemos liberarlo porque somos dueños
}

// ¿Cuál usar?
//   la → solo necesitas leerlo
//   el → necesitas modificar o liberar el dato original
```

### Pasar por referencia (&T / &mut T)

```falcato
// Referencia inmutable — puedes leer pero no cambiar
fn mostrar(la datos: &Texto) {
    decir("El texto dice: ");
    decir(datos);
    // la función solo lee, el dueño original conserva el dato
}

// Referencia mutable — puedes modificar el original
fn añadir_exclamacion(el msg: &mut Texto) {
    msg.agregar("!");
}
```

**Regla práctica:** parámetros pequeños (Entero32, Booleano) → `el`. Parámetros grandes o que no modificas → `la`. Parámetros que modificas sin tomar propiedad → `&mut`.

## Retornar valores

```falcato
// Un valor
fn cuadrado(x: Entero32) -> Entero32 {
    retornar x * x;
}

// Sin retorno → Vacio (se puede omitir)
fn log(la msg: Palabra) {    // → Vacio implícito
    decir("[LOG] {msg}");
}

// Retornar un Texto (transferencia de propiedad)
fn crear_saludo(la nombre: Palabra) -> Texto {
    el resultado: Texto = texto_desde("Hola, ");
    resultado.agregar(nombre);
    retornar resultado;  // quien llama recibe la propiedad
}

fn main() {
    el saludo = crear_saludo("Ana");
    decir(saludo);
    saludo.liberar();  // quien creó el Texto... ¡ah, no! Fue crear_saludo
    // Pero te pasó la propiedad. Tú eres el nuevo dueño. Tú liberas.
}
```

### Múltiples puntos de retorno

```falcato
fn clasificar_edad(edad: Entero32) -> Palabra {
    si edad < 0 { retornar "inválida"; }    // salida temprana
    si edad < 12 { retornar "niño"; }
    si edad < 18 { retornar "adolescente"; }
    si edad < 65 { retornar "adulto"; }
    retornar "jubilado";  // última salida
}
```

Los retornos tempranos son útiles para casos borde: verificas y te vas rápido.

## Llamar una función

```falcato
función principal() -> Entero32 {
    el resultado = sumar(3, 4);       // 7
    saludar("Ana");                   // "Hola, Ana"
    el saludo = crear_saludo("Luis");
    decir(saludo);
    saludo.liberar();
    retornar 0;
}
```

### Funciones como bloques de construcción

El truco para que las funciones sean útiles es **componerlas**:

```falcato
fn calcular_precio(la precio_base: Entero32) -> Entero32 {
    retornar sumar(precio_base, calcular_iva(precio_base));
}

fn calcular_iva(precio: Entero32) -> Entero32 {
    retornar precio * 21 / 100;  // 21% IVA
}

fn calcular_descuento(la precio: Entero32, la cliente_fiel: Booleano) -> Entero32 {
    si cliente_fiel está {
        retornar precio - (precio * 10 / 100);  // 10% descuento
    }
    retornar precio;
}

fn main() -> Entero32 {
    el base = 100;
    el con_iva = calcular_precio(base);
    el final = calcular_descuento(con_iva, verdadero);
    decir("Total: {final}");  // 100 + 21 = 121 - 12 = 109
    retornar 0;
}
```

## Recursión (una función que se llama a sí misma)

```falcato
fn factorial(n: Entero32) -> Entero32 {
    si n <= 1 { retornar 1; }
    retornar n * factorial(n - 1);  // se llama a sí misma
}

decir("5! = {factorial(5)}");  // 120
```

La recursión es útil para árboles, laberintos, y problemas que se dividen en partes más pequeñas. Pero cuidado: cada llamada gasta stack.

## Formas de escribir "función"

```falcato
función suma(...) { }   // con tilde (recomendada)
funcion suma(...) { }   // sin tilde (si tu teclado no tiene)
fn suma(...) { }        // corta (como Rust)
```

## devolver y retornar

```falcato
función suma(a: Entero32, b: Entero32) -> Entero32 {
    devolver a + b;     // alias de retornar
}
```

Ambos hacen lo mismo. Usa el que te sea más natural.

## Genéricos

```falcato
función maximo<T que Comparable>(el a: T, el b: T) -> T {
    si a > b { retornar a; } sino { retornar b; }
}

maximo(3, 5);        // T = Entero32 → 5
maximo(3.14, 2.71);  // T = Flotante64 → 3.14
maximo("gato", "perro"); // T = Palabra → "perro"
```

`<T que Comparable>` = "T debe poderse comparar con `<`, `>`, etc."

También genéricos numéricos:

```falcato
fn longitud<N: Entero32>(la nums: [Entero32; N]) -> Entero32 {
    retornar N;  // N es el tamaño del array, conocido en compilación
}
```

## Errores típicos

```falcato
// Error: tipo de retorno incorrecto
fn sumar(a: Entero32, b: Entero32) -> Palabra {
    retornar a + b;  // a + b es Entero32, no Palabra
}

// Error: parámetro 'el' movido dos veces
fn procesar(el msg: Texto) {
    msg.liberar();
    decir(msg);  // Error: msg ya no existe (se liberó)
}

// Error: olvidar retornar cuando se prometió
fn siempre_error() -> Entero32 {
    // Error: debe retornar un Entero32
}

// Bien
fn siempre_error() -> Entero32 {
    retornar 42;
}

// Error: falta tipo en parámetro
fn rara(x) -> Entero32 {  // ¿qué tipo es x?
    retornar x;
}

// Bien
fn rara(x: Entero32) -> Entero32 {
    retornar x;
}
```

## Función vs procedimiento

| | Devuelve algo | Solo ejecuta |
|---|---------------|--------------|
| En Falcato | `fn x() -> Tipo` | `fn x() -> Vacio` (o sin `->`) |
| Se usa para | Calcular un valor | Hacer algo (imprimir, guardar) |

```falcato
// Función (devuelve un valor)
fn cuadrado(x: Entero32) -> Entero32 { retornar x * x; }

// Procedimiento (no devuelve nada útil)
fn mostrar_error(la msg: Palabra) {
    decir("[ERROR] {msg}");
}
```

---

← [06: Bucles](06-bucles.md) | [Indice](../GUIA.md) | [Siguiente: Texto y Palabra →](08-texto.md)
