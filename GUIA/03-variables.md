# 03 — Variables: el, la, un, los, las

← [02: Tu primer programa](02-tu-primer-programa.md) | [Indice](../GUIA.md) | [Siguiente: Operaciones →](04-operaciones.md)

---

Las variables en Falcato se declaran con un **artículo**. Como en español de verdad: `el`, `la`, `un`, `los`, `las`.

Cada artículo dice **quién es el dueño** del dato. No es decoración — el compilador lo usa para evitar errores de memoria.

## De un vistazo

| Artículo | Dueño | ¿Se puede cambiar? | ¿Varias variables pueden apuntar al mismo dato? |
|----------|-------|-------------------|--------------------------------------------------|
| `el` | Tú (único dueño) | Sí | No — solo tú lo tienes |
| `la` | Otro (prestado) | No | Sí — solo lectura |
| `un` | Quizás nadie | Depende | Es opcional, puede estar vacío |
| `los` | Varios (contado de referencias) | Sí | Sí — todos son dueños |
| `las` | Varios (préstamo compartido) | No | Sí — solo lectura compartida |

## `el` — tuyo, mutable

Eres el **único dueño**. Puedes leer, modificar, mover o liberar el dato.
Nadie más tiene acceso directo.

**Cuándo se usa en la vida real:**

```falcato
// Un contador que incrementa
el puntos: Entero32 = 0;
puntos = puntos + 100;
puntos = puntos - 30;  // se modifica normal

// Un buffer que se llena dinámicamente
el nombre: Texto = texto_desde("Ana");
nombre.agregar(" Martínez");    // modificas el Texto
imprimir_linea(nombre);         // "Ana Martínez"
nombre.liberar();               // tú lo creaste, tú lo liberas

// Una variable que cambia según condiciones
el temperatura: Entero32;
si es_invierno {
    temperatura = 15;
} sino {
    temperatura = 35;
}
// temperatura está inicializada en ambas ramas → seguro
```

**Regla mental:** "Este dato es **mío**, hago lo que quiero con él."

## `la` — prestado, inmutable

El dato **no es tuyo**. Te lo prestaron para leerlo. No puedes modificarlo ni liberarlo.

**Cuándo se usa en la vida real:**

```falcato
// Constantes de configuración que nunca cambian
la NOMBRE_APP: Palabra = "Mi Programa";
la VERSION: Palabra = "1.0";
la TASA_IVA: Flotante64 = 0.21;
// NOMBRE_APP = "Otra Cosa";  // Error: la es inmutable

// Datos que recibes y solo necesitas leer
fn saludar(la nombre: Palabra) {
    imprimir_linea("Hola, " + nombre);
    // No puedes modificar 'nombre' — no es tuyo
}

// Un valor fijo que usas en varios cálculos
la gravedad: Flotante64 = 9.81;
la altura: Flotante64 = 10.0;
la energia: Flotante64 = masa * gravedad * altura;

// Texto prestado — lees pero no liberas
fn mostrar_mensaje(la msg: Texto) -> Entero32 {
    el len = msg.tam();        // ok, solo lees
    imprimir_linea(msg);       // ok
    // msg.liberar();          // Error: no es tuyo, no puedes liberarlo
    retornar len;
}
```

**Regla mental:** "Me prestaron esto para **leer**. Lo devuelvo como lo recibí."

## `un` — opcional, quizás existe, quizás no

El dato **puede existir o no**. Como un regalo que quizás te dieron, quizás no.

Es una alternativa ligera a `Resultado<T,E>` — útil cuando la ausencia de valor **no es un error**, solo es que no hay nada.

**Cuándo se usa en la vida real:**

```falcato
// Buscar un usuario por ID — puede no existir
fn buscar_usuario(la id: Entero32) -> un Palabra {
    si id es 1 { retornar admin; }
    si id es 2 { retornar invitado; }
    // No retornamos nada — usuario no existe
}

// Parsear un número de un string — puede fallar
fn leer_entero(la texto: Palabra) -> un Entero32 {
    // Si el texto no es un número válido, retorna vacío
    si es_numero_valido(texto) {
        retornar convertir_a_entero(texto);
    }
    // No retornamos nada automáticamente
}

// Configuración opcional
un timeout_ms: Entero32;    // cero significa "sin timeout"
tiempo_espera(timeout_ms);

// Un buffer temporal que quizás se necesita
fn procesar(la datos: Palabra) {
    un buffer: Texto;
    si datos.tam() > 1000 {
        buffer = texto_desde(datos);
        // procesar...
        buffer.liberar();
    }
    // Si datos.tam() <= 1000, buffer nunca se creó — y está bien
}
```

**Regla mental:** "Este dato **quizás está, quizás no**. No pasa nada si no."

## `los` — dueño compartido, mutable entre varios

Varios hilos o funciones comparten la **propiedad** del dato. Cuando todos terminan de usarlo, se libera automáticamente.

**Cuándo se usa en la vida real:**

```falcato
// Recurso compartido entre varios hilos
los contador_global: Entero32 = 0;

fn hilo_trabajador() {
    // Varios hilos pueden modificar contador_global
    contador_global = contador_global + 1;
}

// Cache compartido — todos pueden leer y escribir
los cache: Texto = texto_nuevo();

fn guardar_en_cache(la valor: Palabra) {
    cache.agregar(valor);
}

fn leer_cache() -> Palabra {
    // 'los' permite acceso desde cualquier parte
    retornar cache;  // esto hace una copia para el dueño
}
```

**Regla mental:** "Esto es de **todos**. Nadie lo borra mientras alguien lo use."

## `las` — préstamo compartido, solo lectura entre varios

Varias partes pueden **leer el mismo dato simultáneamente** pero ninguna puede modificarlo.

**Cuándo se usa en la vida real:**

```falcato
// Un log centralizado que todos leen
las registro_global: Texto;

fn reportar_estado() {
    // Todos pueden leer, nadie puede escribir directo
    imprimir_linea("Estado actual del sistema:");
    // registro_global es solo lectura
}

// Configuración global compartida
las config_sistema: Palabra = "produccion";

fn verificar_modo() -> Palabra {
    // Todos pueden consultar la configuración
    retornar config_sistema;
}
```

**Regla mental:** "Todos **leen** el mismo periódico. Nadie puede garabatearlo."

## ¿Cómo elegir el artículo correcto?

Esta tabla te guía según lo que **necesitas hacer**:

| Situación | Artículo | Ejemplo |
|-----------|----------|---------|
| Voy a cambiar este dato frecuentemente | `el` | contadores, buffers, acumuladores |
| Solo necesito leerlo, no me pertenece | `la` | parámetros de función, config |
| Este valor puede no existir | `un` | búsquedas, parseos, opcionales |
| Varios hilos necesitan modificarlo | `los` | cachés compartidos, estado global |
| Todos leen el mismo dato, nadie escribe | `las` | logs compartidos, config global |

## Comparativa visual: el mismo programa en 5 artículos

```falcato
// el — contador personal
fn mi_contador() {
    el c: Entero32 = 0;
    c = c + 1;          // solo yo lo cambio
    imprimir_linea(c);  // 1
}

// la — contador prestado
fn ver_contador(la c: Entero32) {
    imprimir_linea(c);  // solo leo
    // c = c + 1;       // error: no puedo
}

// un — contador opcional
fn buscar_contador(la id: Entero32) -> un Entero32 {
    si id > 0 { retornar id * 10; }
    // si id es 0, retorna vacío
}

// los — contador compartido entre hilos
los c_compartido: Entero32 = 0;

fn hilo_a() { c_compartido = c_compartido + 1; }
fn hilo_b() { c_compartido = c_compartido + 1; }
// ambos modifican el mismo contador

// las — contador de solo lectura global
las c_global: Entero32 = 100;

fn mostrar_global() {
    imprimir_linea(c_global);  // todos leen
}
```

## Errores típicos

```falcato
// Error: 'la' no se puede modificar
la x: Entero32 = 10;
x = 20;          // [O001] 'la' es inmutable

// Error: 'un' no se puede usar sin verificar
un y: Entero32;
imprimir_linea(y);  // [T001] puede no tener valor

// Bien: 'el' se puede modificar
el z: Entero32 = 10;
z = 20;          // ok
```

## Recuerda

- **`el`**: es tuyo, cámbialo, bórralo, haz lo que quieras
- **`la`**: te lo prestaron, solo lee
- **`un`**: quizás hay algo, quizás no — trátalo con cuidado
- **`los`**: es de todos, todos pueden cambiarlo
- **`las`**: todos lo leen, nadie lo cambia

---

← [02: Tu primer programa](02-tu-primer-programa.md) | [Indice](../GUIA.md) | [Siguiente: Operaciones →](04-operaciones.md)
