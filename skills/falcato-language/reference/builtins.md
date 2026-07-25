# Falcato — Built-in Functions Reference

Loaded on-demand when detailed function signatures are needed.

## Texto (heap string, 24 bytes: ptr+I64, len+I64, cap+I64)

| Function | Signature | Description |
|----------|-----------|-------------|
| `texto_nuevo` | `() -> Texto` | Create empty Texto |
| `texto_desde` | `(Palabra) -> Texto` | Create Texto from &str |
| `texto_agregar` | `(Texto, Palabra) -> Vacio` | Append text |
| `texto_longitud` | `(Texto) -> Entero32` | Byte length |
| `texto_liberar` | `(Texto) -> Vacio` | Free memory |
| `texto_concatenar` | `(Texto, Texto) -> Texto` | New concatenated Texto |
| `texto_subtexto` | `(Texto, Entero32, Entero32) -> Texto` | Slice [inicio, fin) |
| `texto_comparar` | `(Texto, Texto) -> Entero32` | Compare (0=equal) |
| `texto_obtener_byte` | `(Texto, Entero32) -> Entero8` | Byte at index |

**Method syntax:** `t.agregar("x")` `t.tam()` `t.liberar()` `t[0]` `a+b` `t[0..5]`

## Vector<T> (heap dynamic array, 24 bytes)

| Function | Signature | Description |
|----------|-----------|-------------|
| `vector_nuevo` | `::<T>() -> Vector<T>` | Create empty vector |
| `vector_agregar` | `::<T>(Vector<T>, T) -> Vacio` | Push element |
| `vector_obtener` | `::<T>(Vector<T>, Entero32) -> T` | Get by index |
| `vector_longitud` | `::<T>(Vector<T>) -> Entero32` | Element count |
| `vector_liberar` | `::<T>(Vector<T>) -> Vacio` | Free memory |

**Method syntax:** `v.agregar(x)` `v.tam()` `v.obtener(i)` `v[0]` `v.liberar()`

## File I/O

| Function | Signature | Description |
|----------|-----------|-------------|
| `archivo_leer` | `(Palabra) -> Texto` | Read file to Texto (liberar!) |
| `archivo_escribir` | `(Palabra, Texto) -> Entero32` | Write file (0=ok) |
| `archivo_existe` | `(Palabra) -> Booleano` | File exists? |

## Math

| Function | Signature | Description |
|----------|-----------|-------------|
| `abs` | `(Entero32) -> Entero32` | Absolute value |
| `max` | `(Entero32, Entero32) -> Entero32` | Maximum |
| `min` | `(Entero32, Entero32) -> Entero32` | Minimum |
| `raiz` | `(Flotante64) -> Flotante64` | Square root |
| `potencia` | `(Flotante64, Flotante64) -> Flotante64` | Power |
| `tamano_de` | `::<T>() -> Entero64` | Sizeof (comptime) |

## Bitwise Methods (on integers)

| Method | Args | Effect |
|--------|------|--------|
| `x.poner_bit(n)` | 1 | `x |= (1 << n)` |
| `x.quitar_bit(n)` | 1 | `x &= ~(1 << n)` |
| `x.alternar_bit(n)` | 1 | `x ^= (1 << n)` |
| `x.extraer_bits(off, cnt)` | 2 | Extract bit field |
| `x.unos()` | 0 | Popcount |
| `x.ceros_izquierda()` | 0 | Count leading zeros |

## Channels + TCP

| Function | Signature | Description |
|----------|-----------|-------------|
| `canal_nuevo` | `(Entero32) -> Entero64` | Create channel (capacity) |
| `canal_enviar` | `(Entero64, Entero32) -> Vacio` | Send |
| `canal_recibir` | `(Entero64) -> Entero32` | Recv (blocking) |
| `canal_intentar` | `(Entero64) -> Entero32` | Try recv (non-blocking) |
| `canal_cerrar` | `(Entero64) -> Vacio` | Destroy |
| `tcp_vincular` | `(Entero32) -> Entero64` | Bind + listen |
| `tcp_aceptar` | `(Entero64) -> Entero64` | Accept |
| `tcp_leer` | `(Entero64, Entero64, Entero32) -> Entero32` | Recv |
| `tcp_escribir` | `(Entero64, Entero64, Entero32) -> Entero32` | Send |
| `tcp_cerrar` | `(Entero64) -> Vacio` | Close |

## I/O (polymorphic — accepts int, string, float, bool)

| Function | Description |
|----------|-------------|
| `imprimir(x)` | Print without newline |
| `imprimir_linea(x)` | Print with newline |
| `decir(x)` | Alias of imprimir_linea |
