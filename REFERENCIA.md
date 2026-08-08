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

## Diccionario\<K,V\> (mapa clave→valor)

Hash map con open addressing y resize automático. Claves y valores viven en heap.

| Función | Firma | Qué hace |
|---------|-------|----------|
| `diccionario_nuevo` | `::<K,V>() -> Diccionario<K,V>` | Crea un diccionario vacío |
| `diccionario_insertar` | `::<K,V>(Dict, K, V) -> Entero64` | Inserta clave→valor (devuelve puntero para chaining) |
| `diccionario_obtener` | `::<K,V>(Dict, K) -> V` | Valor de la clave (⚠️ usa `existe()` antes — si no existe devuelve basura) |
| `diccionario_existe` | `::<K,V>(Dict, K) -> Booleano` | ¿Existe la clave? |
| `diccionario_eliminar` | `::<K,V>(Dict, K) -> Booleano` | Elimina la clave (true si estaba) |
| `diccionario_longitud` | `::<K,V>(Dict) -> Entero32` | Cuántas entradas tiene |
| `diccionario_liberar` | `::<K,V>(Dict) -> Vacío` | Libera la memoria |

**Forma preferida (métodos):**
```falcato
el d: Diccionario<Texto, Entero32> = diccionario_nuevo<Texto, Entero32>();
d.insertar(Texto.desde("edad"), 30);
si d.existe(Texto.desde("edad")) {
    el v: Entero32 = d.obtener(Texto.desde("edad"));
    decir(v);
}
d.eliminar(Texto.desde("edad"));
d.tam();   // diccionario_longitud
d.liberar();
```

> **⚠️ Tipo compuesto como valor:** `Diccionario<Texto, Vector<Texto>>` aún rompe el
> verifier de Cranelift (`blockN is not sealed`) — bug conocido en
> `builtin_diccionario_insertar`, pendiente. Usa valores simples por ahora.

---

## Conjunto\<T\> (set de valores únicos)

Wrapper de `Diccionario` (claves sin valores). Valores en heap.

| Función | Firma | Qué hace |
|---------|-------|----------|
| `conjunto_nuevo` | `::<T>() -> Conjunto<T>` | Crea un conjunto vacío |
| `conjunto_insertar` | `::<T>(Conj, T) -> Vacío` | Añade un valor (si no existe) |
| `conjunto_contiene` | `::<T>(Conj, T) -> Booleano` | ¿Está el valor? |
| `conjunto_eliminar` | `::<T>(Conj, T) -> Booleano` | Elimina un valor (true si estaba) |
| `conjunto_longitud` | `::<T>(Conj) -> Entero32` | Cuántos valores tiene |
| `conjunto_liberar` | `::<T>(Conj) -> Vacío` | Libera la memoria |

**Forma preferida (métodos):**
```falcato
el c: Conjunto<Entero32> = conjunto_nuevo<Entero32>();
c.insertar(10);
si c.contiene(10) { decir("está"); }
c.eliminar(10);
c.tam();   // conjunto_longitud
c.liberar();
```

---

## Argumentos de línea de comandos

| Función | Firma | Qué hace |
|---------|-------|----------|
| `argumentos` | `() -> Vector<Texto>` | argv crudo estilo C (args[0]=ejecutable) |

**Forma tipada (recomendada):** `función principal(el args: Struct)` — el compilador
genera el parseo de `--etiqueta valor`, validación y `--ayuda` automáticos.
Ver [GUIA 16](GUIA/16-argumentos.md).

**Librería avanzada:** `librerias/args_avanzados.fc` — subcomandos, repetición,
posicionales (`args_tiene`, `args_obtener`, `args_todos`, `args_subcomando`,
`args_posicionales`, `args_cuenta`). Ver [GUIA 16](GUIA/16-argumentos.md).

---

## Conversión texto → número

| Función | Firma | Qué hace |
|---------|-------|----------|
| `texto_a_entero` | `(Texto) -> Entero64` | Parsea a Entero64 (0 si falla) |
| `texto_a_natural` | `(Texto) -> Natural64` | Parsea a Natural64 (0 si falla) |
| `texto_a_flotante` | `(Texto) -> Flotante64` | Parsea a Flotante64 (0.0 si falla) |
| `texto_a_booleano` | `(Texto) -> Booleano` | `"true"`, `"1"`, `"sí"` → verdadero |
| `como_entero32` | `(Entero64) -> Entero32` | Convierte a Entero32 (trunca) |

> Usados internamente por el parseo tipado de `principal(el args: Struct)`.

---

## Sistema (procesos, terminal, stdin, fecha)

Primitivas del runtime para CLIs y agentes (R7).

| Función | Firma | Qué hace |
|---------|-------|----------|
| `proceso_crear` | `(Palabra) -> Entero64` | Lanza un proceso; devuelve handle (0=error) |
| `proceso_esperar` | `(Entero64) -> Entero32` | Espera al proceso; devuelve exit code |
| `proceso_leer_salida` | `(Entero64) -> Texto` | stdout+stderr capturados |
| `proceso_cerrar` | `(Entero64) -> Vacío` | Cierra el handle |
| `terminal_modo_raw` | `(Entero32) -> Entero32` | 1=modo raw (sin echo, sin Enter; activa ANSI VT), 0=restaura |
| `terminal_leer_tecla` | `() -> Entero32` | Lee una tecla sin Enter (flechas = 0x100+) |
| `entrada_leer` | `() -> Texto` | Lee TODO el stdin hasta EOF |
| `fecha_unix` | `() -> Entero64` | Segundos desde epoch |
| `fecha_ms` | `() -> Entero64` | Milisegundos desde epoch |

**Ejemplo (procesos):**
```falcato
el h: Entero64 = proceso_crear("falcato verifica app.fc");
el codigo: Entero32 = proceso_esperar(h);
el salida: Texto = proceso_leer_salida(h);
decir(salida);
salida.liberar();
proceso_cerrar(h);
```

**Ejemplo (terminal TUI):**
```falcato
terminal_modo_raw(1);
el tecla: Entero32 = terminal_leer_tecla();
si tecla es 27 { decir("ESC"); }          // 27 = escape
terminal_modo_raw(0);
```

> `fecha_unix`/`fecha_ms` son reloj de pared real; `timestamp` mide ms desde boot
> (intervalos).

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

## GUI / FFI — System pointers

| Función | Firma | Qué hace |
|---------|-------|----------|
| `direccion_de(fn)` | `(nombre función) -> Entero64` | Obtiene la dirección en memoria de una función (para WNDPROC, callbacks Win32) |
| `dir_de(fn)` | `(nombre función) -> Entero64` | Alias de `direccion_de` |
| `texto_a_puntero(s)` | `(Palabra) -> Entero64` | Convierte un literal string a puntero (para `GetModuleHandle`, `LoadCursor`, etc.) |
| `como_entero64(x)` | `(Entero32) -> Entero64` | Convierte `Entero32` a `Entero64` sin pérdida de signo |

> **Nota sobre punteros Win32:** En Falcato, los `HANDLE`, `HWND`, `HINSTANCE` y punteros
> a funciones se representan como `Entero64` (8 bytes, mismo tamaño que un puntero
> en x86_64). Las funciones Win32 se declaran como `inseguro función` para llamadas
> FFI.
>
> Para structs Win32 complejos (WNDCLASSEXA, MSG, etc.), se usa el
> [patrón Trampolín C](docs/diseno_gui.md#37-el-patrón-trampolín-c):
> un archivo `.c` precompilado a `.obj` que el compilador linkea automáticamente.
>
> Ver: [`docs/diseno_gui.md`](docs/diseno_gui.md) — diseño completo del sistema gráfico.

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
