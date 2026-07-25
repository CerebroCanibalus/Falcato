# 📗 Falcato — Referencia de funciones built-in

> Todas las funciones que vienen incluidas en el lenguaje.
> **📖 Primero lee:** [GUIA.md](GUIA.md) — tutorial desde cero

---

## 📚 Documentación relacionada

| Guía | Descripción |
|------|-------------|
| [📖 GUIA.md](GUIA.md) | Tutorial completo desde cero |
| [⚙️ INSTALL.md](INSTALL.md) | Instalación |
| [📗 REFERENCIA.md](REFERENCIA.md) | **← Estás aquí** |
| [🚨 ERRORES.md](ERRORES.md) | Códigos de error y soluciones |

---

## I/O (entrada/salida)

| Función | Firma real | Qué hace |
|---------|------------|----------|
| `imprimir` | `(cualquier tipo) -> Vacío` | Escribe en pantalla, sin salto de línea |
| `imprimir_linea` | `(cualquier tipo) -> Vacío` | Escribe en pantalla, con salto de línea |
| `decir` | `(cualquier tipo) -> Vacío` | Igual que `imprimir_linea` (alias) |

> Aceptan: `Entero32`, `Entero8`, `Entero64`, `Natural8/16/32/64`, `Flotante64`, `Booleano`,
> `Palabra` y `Texto`. El compilador elige el formato automáticamente.

**Ejemplo:**
```falcato
imprimir("Hola ");
imprimir_linea("mundo");
decir(42);               // números
decir(3.14);             // decimales
decir(verdadero);        // booleanos
```

---

## Texto (strings dinámicos)

| Función | Firma | Qué hace |
|---------|-------|----------|
| `texto_nuevo` | `() -> Texto` | Crea un Texto vacío |
| `texto_desde` | `(Palabra) -> Texto` | Crea Texto desde una Palabra |
| `texto_agregar` | `(Texto, Palabra) -> Vacío` | Agrega texto al final |
| `texto_longitud` | `(Texto) -> Entero32` | Cuántos bytes tiene |
| `texto_tam` | `(Texto) -> Entero32` | Alias de `texto_longitud` |
| `texto_liberar` | `(Texto) -> Vacío` | Libera la memoria (¡obligatorio!) |
| `texto_concatenar` | `(Texto, Texto) -> Texto` | Une dos Textos en uno nuevo |
| `texto_subtexto` | `(Texto, Entero32, Entero32) -> Texto` | Extrae desde `inicio` hasta `fin` |
| `texto_comparar` | `(Texto, Texto) -> Entero32` | Compara byte a byte (0=iguales) |
| `texto_obtener_byte` | `(Texto, Entero32) -> Entero8` | Byte en la posición indicada |

**Forma preferida (métodos):**
```falcato
t.agregar("hola");
t.tam();
t.liberar();
t[0];            // texto_obtener_byte
a + b;           // texto_concatenar
t[0..5];         // texto_subtexto
```

---

## Vector\<T\> (arrays dinámicos)

| Función | Firma | Qué hace |
|---------|-------|----------|
| `vector_nuevo` | `::<T>() -> Vector<T>` | Crea un vector vacío del tipo T |
| `vector_agregar` | `::<T>(Vector<T>, T) -> Vacío` | Añade un elemento al final |
| `vector_obtener` | `::<T>(Vector<T>, Entero32) -> T` | Obtiene elemento por índice |
| `vector_longitud` | `::<T>(Vector<T>) -> Entero32` | Cuántos elementos tiene |
| `vector_tam` | `::<T>(Vector<T>) -> Entero32` | Alias de `vector_longitud` |
| `vector_liberar` | `::<T>(Vector<T>) -> Vacío` | Libera la memoria |

**Forma preferida (métodos):**
```falcato
v.agregar(42);
v.tam();
v[0];
v.liberar();
```

---

## Archivos

| Función | Firma | Qué hace |
|---------|-------|----------|
| `archivo_leer` | `(Palabra) -> Texto` | Lee archivo completo a Texto |
| `archivo_escribir` | `(Palabra, Texto) -> Entero32` | Escribe (0=ok, -1=error) |
| `archivo_existe` | `(Palabra) -> Booleano` | Verifica si el archivo existe |

**Ejemplo:**
```falcato
el contenido = archivo_leer("datos.txt");
si contenido.tam() > 0 {
    decir(contenido);
}
contenido.liberar();
```

---

## Matemáticas

| Función | Firma | Qué hace |
|---------|-------|----------|
| `abs` | `(Entero32) -> Entero32` | Valor absoluto |
| `max` | `(Entero32, Entero32) -> Entero32` | Máximo de dos números |
| `min` | `(Entero32, Entero32) -> Entero32` | Mínimo de dos números |
| `raiz` | `(Flotante64) -> Flotante64` | Raíz cuadrada |
| `potencia` | `(Flotante64, Flotante64) -> Flotante64` | Potencia (base^exp) |
| `tamaño_de` | `::<T>() -> Entero64` | Tamaño en bytes de T (comptime) |

**Ejemplo:**
```falcato
el r = raiz(25.0);       // 5.0
el p = potencia(2.0, 10.0); // 1024.0
el tam = tamaño_de::<Entero32>(); // 4
```

---

## Enteros — métodos bitwise

| Método | Args | Qué hace |
|--------|------|----------|
| `x.poner_bit(n)` | 1 | Pone el bit n en 1: `x \|= (1 << n)` |
| `x.quitar_bit(n)` | 1 | Pone el bit n en 0: `x &= ~(1 << n)` |
| `x.alternar_bit(n)` | 1 | Cambia el bit n: `x ^= (1 << n)` |
| `x.extraer_bits(off, cnt)` | 2 | Extrae `cnt` bits desde `off` |
| `x.unos()` | 0 | Cuenta bits en 1 (popcount) |
| `x.ceros_izquierda()` | 0 | Cuenta ceros a la izquierda (clz) |

---

## Async / Hilos

| Función | Firma | Qué hace |
|---------|-------|----------|
| `dormir` | `(Entero32) -> Vacío` | Duerme el hilo actual (ms) |
| `lanzar` | `(expr) -> Vacío` | Lanza una función en otro hilo |
| `esperar` | `(fut expr) -> T` | Espera a que un futuro termine |
| `bloquear` | `(expr) -> T` | Puente sync→async |
| `cancelar` | `() -> Vacío` | Cancela tareas pendientes en executor |

### Canales

| Función | Firma | Qué hace |
|---------|-------|----------|
| `canal_nuevo` | `(Entero32) -> Entero64` | Crea canal con capacidad |
| `canal_enviar` | `(Entero64, Entero32) -> Vacío` | Envía un valor |
| `canal_recibir` | `(Entero64) -> Entero32` | Recibe (bloqueante) |
| `canal_intentar` | `(Entero64) -> Entero32` | Recibe (no bloqueante) |
| `canal_cerrar` | `(Entero64) -> Vacío` | Destruye el canal |

### TCP

| Función | Firma | Qué hace |
|---------|-------|----------|
| `tcp_vincular` | `(Entero32) -> Entero64` | Crea server en puerto |
| `tcp_aceptar` | `(Entero64) -> Entero64` | Acepta conexión |
| `tcp_leer` | `(Entero64, Entero64, Entero32) -> Entero32` | Recibe datos |
| `tcp_escribir` | `(Entero64, Entero64, Entero32) -> Entero32` | Envía datos |
| `tcp_cerrar` | `(Entero64) -> Vacío` | Cierra conexión |

---

## Testing

| Función | Firma | Qué hace |
|---------|-------|----------|
| `afirmar` | `(Booleano) -> Vacío` | Assert en tiempo de ejecución |

**Ejemplo:**
```falcato
prueba "suma básica" {
    afirmar(sumar(2, 2) es 4);
}
```

---

## Keywords y sintaxis

### Declaraciones

| Keyword | Para qué |
|---------|----------|
| `función` / `funcion` / `fn` | Declarar una función |
| `estructural` | Declarar un struct |
| `enumeración` | Declarar un enum |
| `rasgo` | Declarar un trait/interface |
| `implementar` | Implementar un trait |
| `módulo` | Declarar un módulo |
| `usar` | Importar símbolos |
| `prueba` | Declarar un test |

### Control de flujo

| Keyword | Para qué |
|---------|----------|
| `si` / `entonces` / `sino` | Condicional |
| `mientras` | Bucle |
| `para` / `en` | Iteración |
| `coincidir` / `emparejar` | Pattern matching |
| `retornar` / `devolver` | Retornar valor |
| `seleccionar` | Select sobre canales |
| `con_executor` | Thread pool |

### Modos verbales (innovación Falcato)

| Palabra | Significado |
|---------|-------------|
| `es` | Identidad permanente (==) |
| `está` | Estado temporal |
| `fuese` | Subjuntivo (caso improbable) |

### Artículos (ownership)

| Artículo | Significado |
|----------|-------------|
| `el` | Dueño, mutable |
| `la` | Prestado, inmutable |
| `un` | Opcional |
| `los` | Compartido, mutable |
| `las` | Compartido, inmutable |

### Async

| Keyword | Para qué |
|---------|----------|
| `fut` | Función asíncrona |
| `esperar` | Await (dentro de fut) |
| `lanzar` | Spawn en thread |
| `bloquear` | Bridge sync→async |

### Ownership

| Keyword | Para qué |
|---------|----------|
| `mover` | Transferir ownership |
| `copiar` | Clonar explícitamente |
| `prestar` | Borrow explícito |
| `verificado` | Nivel 1 de borrow checker |
| `estricto` | Nivel 2 (máximo) |
| `región` / `region` | Arena allocation |
| `puro` | Función sin efectos secundarios |
| `muta(campo)` | Anotación de efecto |
| `lee(campo)` | Anotación de efecto |

### Otros

| Palabra | Para qué |
|---------|----------|
| `todos` | Inicializar array con un valor |
| `inseguro` | Bloque FFI (unsafe) |
| `como` | Binding en pattern matching |
| `mut` | Referencia mutable |
| `yo` | Self en referencias (`&yo T`) |
| `tipo` | Declaración de tipo (alias) |
| `verdadero` / `falso` | Booleanos |
