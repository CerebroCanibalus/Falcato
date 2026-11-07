# libEst — Librería Estándar de Falcato (borrador de diseño)

> **Estado:** 📋 Borrador para validación
> **Versión objetivo:** 0.8.0 (núcleo) → 0.9.x (red/JSON/gráficos)
> **Base:** P-001 (tipos fragmentados + verbos consistentes + namespaces `::`), P-002 (`::`), P-005 (reducir builtins), P-017 (regla unificadora de API)
> **Filosofía:** *"La morfología española ES el sistema de tipos."* Cada dominio es un namespace `::`; cada verbo es consistente entre dominios.

---

## 1. Principios de diseño

### 1.1 Regla unificadora de API (P-017)

> **"Toda API debe ser consistente con su familia. Si hay dos formas de hacer lo mismo, es un bug de diseño, no una feature."**

- **Coerción uniforme:** TODAS las funciones aceptan `Texto` y `Palabra` por igual. No hay razón para que `archivo::escribir` sea distinto de `archivo::agregar`.
- **Errores en vez de silencio:** si no puedes hacerlo bien, da error `[S###]` en compile-time. Un vacío silencioso es infinitamente peor.
- **Verbos consistentes:** `abrir`, `leer`, `escribir`, `cerrar`, `conectar`, `enviar`, `recibir`, `crear`, `borrar`, `buscar`, `contiene`, `longitud` son los MISMOS en todos los dominios.

### 1.2 Nomenclatura

- **Namespace:** `dominio::accion` (P-002, `::` confirmado).
- **Tipos:** sustantivos fragmentados por dominio (`Archivo`, `Conexion`, `HttpCliente`, `Socket`, `Directorio`, `Ventana`, `Imagen`).
- **Verbos:** infinitivo consistente (`abrir`, `leer`, `escribir`, `cerrar`).
- **Conjugación como azúcar OPCIONAL:** `abrir(archivo)` expande a `archivo::abrir(archivo)`. El LLM siempre puede usar la forma explícita.

### 1.3 Auto-import (P-003)

**Opción A — prelude pequeño (Rust):** solo `imprimir`, `imprimir_linea`, `salir` auto-importados. El resto requiere `usar libEst::*`.

```falcato
usar libEst::texto::*;
usar libEst::archivo::*;
usar libEst::red::http::*;
```

### 1.4 Método vs función (P-004)

- **Método** cuando `t` es el sujeto natural y la operación ES sobre `t`: `t.longitud()`, `t.contiene("hola")`.
- **Función en libEst** cuando hay DOS operandos del mismo tipo, o cuando `t` no es "sujeto": `texto::unir(a, b)`.

### 1.5 Convenciones de sintaxis (fieles al lenguaje real)

> **Verificado contra el compiler (v0.7.5):** estas firmas usan la sintaxis REAL de Falcato, no inventada.

- **`Resultado<T>` es azúcar** para `Resultado<T, Entero32>` (tipo de error por defecto). Cuando el error es otro tipo, se escribe explícito: `Resultado<T, Texto>`.
- **`Option<T>`** (no `Opcion<T>`). Variantes: `Algo(valor)` / `Nada`.
- **`Resultado.Exito(x)` / `Resultado.Error(e)`** son las variantes.
- **Genéricos explícitos** en la llamada: `vector::nuevo<Entero32>()`, `vector::agregar<Entero32>(v, 10)`.
- **Strings:** `Palabra` = literal (string estático), `Texto` = string dinámico (heap). Ambos en español, sin anglicismos.
- **Funciones:** `función` / `funcion` / `fn`. **Structs:** `estructural`. **Enums:** `enumeración` / `enumeracion`. **Imports:** `usar`. **Alias:** `apodo`. **Traits:** `rasgo`.
- **Métodos:** `x.metodo(args)` desugarea a llamada built-in según el tipo del receptor.

---

## 2. Árbol de la libEst

```
libEst/
├── núcleo/            # Tipos base, prelude
│   ├── texto.fc       # Texto, Palabra
│   ├── numeros.fc     # Entero*, Natural*, Real, Flotante*
│   ├── logico.fc      # Booleano, Lógico
│   ├── opcion.fc      # Option, Resultado
│   ├── rango.fc       # Rangos (P-012)
│   └── unidad.fc      # Unidades (P-012 F4)
├── colecciones/
│   ├── vector.fc      # Vector
│   ├── diccionario.fc # Diccionario
│   ├── conjunto.fc    # Conjunto
│   ├── lista.fc       # Lista enlazada
│   ├── cola.fc        # Cola FIFO
│   ├── pila.fc        # Pila LIFO
│   └── par.fc         # Pares clave-valor
├── archivo/
│   ├── archivo.fc     # Archivo (abrir/leer/escribir/cerrar)
│   ├── directorio.fc  # Directorio (listar/crear/borrar)
│   ├── ruta.fc        # Ruta (unir/absoluta/relativa)
│   └── formato.fc     # CSV, INI, TOML (parseo)
├── red/
│   ├── tcp.fc         # Socket TCP
│   ├── udp.fc         # Socket UDP
│   ├── tls.fc         # Conexión TLS
│   ├── http.fc        # Cliente HTTP
│   ├── servidor.fc    # Servidor HTTP
│   ├── dns.fc         # Resolución DNS
│   └── json.fc        # JSON (serializar/deserializar)
├── tiempo/
│   ├── reloj.fc       # Reloj, instante, duración
│   ├── fecha.fc       # Fecha/hora legible
│   └── temporizador.fc# Timers, sleep
├── proceso/
│   ├── proceso.fc     # Crear/esperar/matar procesos
│   ├── hilo.fc        # Threads
│   ├── canal.fc       # Canales mpsc
│   └── sincronizar.fc # Mutex, semáforo, barrera
├── sistema/
│   ├── entorno.fc     # Variables de entorno
│   ├── argumentos.fc  # Argumentos CLI
│   ├── consola.fc     # I/O consola, color, cursor
│   ├── aleatorio.fc   # RNG
│   ├── cripto.fc      # Hash, HMAC, aleatorio seguro
│   └── memoria.fc     # sizeof, alineación, alloc
├── matemáticas/
│   ├── aritmetica.fc  # abs, max, min, redondeo
│   ├── trig.fc        # seno, coseno, tangente, ...
│   ├── exponencial.fc # exp, log, log10, potencia, raíz
│   ├── complejo.fc    # Números complejos
│   ├── vectorial.fc   # Vectores 2D/3D, producto punto/cruz
│   ├── matriz.fc      # Matrices, determinante, inversa
│   └── estadistica.fc # media, mediana, desviación, varianza
├── visual/            # ⭐ CAPA VISUAL Y GRÁFICA
│   ├── ventana.fc     # Ventanas nativas
│   ├── control.fc     # Botones, cajas, etiquetas, listas
│   ├── layout.fc      # Contenedores, anchors, constraints
│   ├── evento.fc      # Dispatch de eventos, binding
│   ├── color.fc       # Colores, paletas, conversión
│   ├── geometria.fc   # Punto, Rect, tamaño
│   ├── lienzo.fc      # Canvas 2D (líneas, rects, círculos, texto)
│   ├── imagen.fc      # Bitmap, PNG, JPEG, redimensionar
│   ├── fuente.fc      # Tipografías
│   ├── animacion.fc   # Timers de animación, easing
│   ├── sonido.fc      # ⭐ Audio: WAV, buffers, mezcla, efectos (DAW R9.3)
│   └── terminal_ui.fc # TUI (cajas, colores, input en terminal)
└── compat/            # Aliases de builtins viejos (2-3 releases)
    └── compat.fc      # texto_agregar_texto → texto::agregar, etc.
```

---

## 3. Núcleo (`libEst::núcleo`)

### 3.1 Texto (`libEst::texto`)

```falcato
// Creación
texto::nuevo() -> Texto                          // texto vacío
texto::desde(palabra: Palabra) -> Texto          // copia de literal
texto::repetir(texto: Texto, n: Entero32) -> Texto

// Inspección
texto::longitud(t: Texto) -> Entero32            // nº de caracteres (UTF-8)
texto::esta_vacio(t: Texto) -> Booleano
texto::contiene(t: Texto, sub: Texto) -> Booleano
texto::empieza_con(t: Texto, prefijo: Texto) -> Booleano
texto::termina_con(t: Texto, sufijo: Texto) -> Booleano
texto::indice_de(t: Texto, sub: Texto) -> Option<Entero32>
texto::ultimo_indice_de(t: Texto, sub: Texto) -> Option<Entero32>

// Manipulación
texto::agregar(t: Texto, otro: Texto) -> Texto   // concat
texto::unir(a: Texto, b: Texto) -> Texto         // 2 operandos → función
texto::insertar(t: Texto, indice: Entero32, sub: Texto) -> Texto
texto::reemplazar(t: Texto, de: Texto, a: Texto) -> Texto
texto::recortar(t: Texto) -> Texto               // trim espacios
texto::recortar_izq(t: Texto) -> Texto
texto::recortar_der(t: Texto) -> Texto
texto::mayusculas(t: Texto) -> Texto
texto::minusculas(t: Texto) -> Texto
texto::invertir(t: Texto) -> Texto

// Segmentación
texto::dividir(t: Texto, sep: Texto) -> Vector<Texto>
texto::subtexto(t: Texto, inicio: Entero32, fin: Entero32) -> Texto
texto::caracter(t: Texto, indice: Entero32) -> Texto  // 1 char

// Conversión
texto::a_entero(t: Texto) -> Resultado<Entero64>
texto::a_natural(t: Texto) -> Resultado<Natural64>
texto::a_flotante(t: Texto) -> Resultado<Real>
texto::a_booleano(t: Texto) -> Resultado<Booleano>
texto::a_bytes(t: Texto) -> Vector<Entero8>
texto::desde_bytes(bytes: Vector<Entero8>) -> Resultado<Texto>
texto::formatear(plantilla: Texto, args: ...) -> Texto  // sprintf seguro

// Codificación
texto::codificar_utf8(t: Texto) -> Vector<Entero8>
texto::decodificar_utf8(bytes: Vector<Entero8>) -> Resultado<Texto>
texto::codificar_base64(t: Texto) -> Texto
texto::decodificar_base64(t: Texto) -> Resultado<Texto>
texto::codificar_url(t: Texto) -> Texto
texto::decodificar_url(t: Texto) -> Resultado<Texto>
```

### 3.2 Números (`libEst::numeros`)

```falcato
numeros::min(a: Entero64, b: Entero64) -> Entero64
numeros::max(a: Entero64, b: Entero64) -> Entero64
numeros::abs(n: Entero64) -> Entero64
numeros::redondear(n: Real) -> Entero64
numeros::piso(n: Real) -> Entero64
numeros::techo(n: Real) -> Entero64
numeros::truncar(n: Real) -> Entero64
numeros::es_par(n: Entero64) -> Booleano
numeros::es_impar(n: Entero64) -> Booleano
numeros::es_primo(n: Entero64) -> Booleano
numeros::mcd(a: Entero64, b: Entero64) -> Entero64
numeros::mcm(a: Entero64, b: Entero64) -> Entero64
numeros::potencia(base: Real, exp: Real) -> Real
numeros::raiz(n: Real) -> Real
numeros::aleatorio_entre(min: Entero64, max: Entero64) -> Entero64
```

### 3.3 Option y Resultado (`libEst::opcion`)

> **Nota de sintaxis:** el lenguaje usa `Option<T>` (no `Option<T>`) y `Resultado<T, E>` con **dos** parámetros de tipo. `Resultado.Exito(x)` / `Resultado.Error(e)` son las variantes.

```falcato
opcion::es_alguno(o: Option<T>) -> Booleano
opcion::es_ninguno(o: Option<T>) -> Booleano
opcion::valor(o: Option<T>) -> T                    // panic si Ninguno
opcion::valor_o(o: Option<T>, defecto: T) -> T
opcion::valor_o_obtener(o: Option<T>, f: fn() -> T) -> T

resultado::es_exito(r: Resultado<T, E>) -> Booleano
resultado::es_error(r: Resultado<T, E>) -> Booleano
resultado::valor(r: Resultado<T, E>) -> T           // panic si Error
resultado::error(r: Resultado<T, E>) -> Texto       // mensaje de error
resultado::valor_o(r: Resultado<T, E>, defecto: T) -> T
resultado::propagar(r: Resultado<T, E>) -> T        // equivalente a `?`
```

---

## 4. Colecciones (`libEst::colecciones`)

### 4.1 Vector (`libEst::vector`)

```falcato
vector::nuevo<T>() -> Vector<T>
vector::desde<T>(items: [T]) -> Vector<T>
vector::longitud(v: Vector<T>) -> Entero32
vector::esta_vacio(v: Vector<T>) -> Booleano
vector::agregar(v: Vector<T>, item: T)              // push
vector::insertar(v: Vector<T>, indice: Entero32, item: T)
vector::eliminar(v: Vector<T>, indice: Entero32) -> T
vector::eliminar_ultimo(v: Vector<T>) -> T          // pop
vector::obtener(v: Vector<T>, indice: Entero32) -> Option<T>
vector::establecer(v: Vector<T>, indice: Entero32, item: T)
vector::contiene(v: Vector<T>, item: T) -> Booleano
vector::indice_de(v: Vector<T>, item: T) -> Option<Entero32>
vector::ordenar(v: Vector<T>)                       // sort in-place
vector::invertir(v: Vector<T>)
vector::limpiar(v: Vector<T>)
vector::clonar(v: Vector<T>) -> Vector<T>
vector::unir(a: Vector<T>, b: Vector<T>) -> Vector<T>
vector::subvector(v: Vector<T>, inicio: Entero32, fin: Entero32) -> Vector<T>
vector::primer(v: Vector<T>) -> Option<T>
vector::ultimo(v: Vector<T>) -> Option<T>
vector::suma(v: Vector<Entero64>) -> Entero64
vector::promedio(v: Vector<Real>) -> Real
```

### 4.2 Diccionario (`libEst::diccionario`)

```falcato
diccionario::nuevo<K, V>() -> Diccionario<K, V>
diccionario::longitud(d: Diccionario<K, V>) -> Entero32
diccionario::insertar(d: Diccionario<K, V>, clave: K, valor: V)
diccionario::obtener(d: Diccionario<K, V>, clave: K) -> Option<V>
diccionario::existe(d: Diccionario<K, V>, clave: K) -> Booleano
diccionario::eliminar(d: Diccionario<K, V>, clave: K) -> Option<V>
diccionario::claves(d: Diccionario<K, V>) -> Vector<K>
diccionario::valores(d: Diccionario<K, V>) -> Vector<V>
diccionario::contiene_clave(d: Diccionario<K, V>, clave: K) -> Booleano
diccionario::limpiar(d: Diccionario<K, V>)
```

### 4.3 Conjunto (`libEst::conjunto`)

```falcato
conjunto::nuevo<T>() -> Conjunto<T>
conjunto::longitud(c: Conjunto<T>) -> Entero32
conjunto::insertar(c: Conjunto<T>, item: T)
conjunto::contiene(c: Conjunto<T>, item: T) -> Booleano
conjunto::eliminar(c: Conjunto<T>, item: T) -> Booleano
conjunto::union(a: Conjunto<T>, b: Conjunto<T>) -> Conjunto<T>
conjunto::interseccion(a: Conjunto<T>, b: Conjunto<T>) -> Conjunto<T>
conjunto::diferencia(a: Conjunto<T>, b: Conjunto<T>) -> Conjunto<T>
conjunto::elementos(c: Conjunto<T>) -> Vector<T>
```

### 4.4 Lista, Cola, Pila (`libEst::colecciones`)

```falcato
// Lista enlazada
lista::nueva<T>() -> Lista<T>
lista::agregar_al_frente(l: Lista<T>, item: T)
lista::agregar_al_final(l: Lista<T>, item: T)
lista::longitud(l: Lista<T>) -> Entero32
lista::primer(l: Lista<T>) -> Option<T>
lista::ultimo(l: Lista<T>) -> Option<T>
lista::eliminar_primer(l: Lista<T>) -> Option<T>
lista::eliminar_ultimo(l: Lista<T>) -> Option<T>

// Cola FIFO
cola::nueva<T>() -> Cola<T>
cola::encolar(c: Cola<T>, item: T)
cola::desencolar(c: Cola<T>) -> Option<T>
cola::frente(c: Cola<T>) -> Option<T>
cola::longitud(c: Cola<T>) -> Entero32
cola::esta_vacia(c: Cola<T>) -> Booleano

// Pila LIFO
pila::nueva<T>() -> Pila<T>
pila::empujar(p: Pila<T>, item: T)
pila::sacar(p: Pila<T>) -> Option<T>
pila::cima(p: Pila<T>) -> Option<T>
pila::longitud(p: Pila<T>) -> Entero32
pila::esta_vacia(p: Pila<T>) -> Booleano
```

---

## 5. Archivo (`libEst::archivo`)

### 5.1 Archivo (`libEst::archivo`)

```falcato
// Apertura y cierre
archivo::abrir(ruta: Texto) -> Resultado<Archivo>          // lectura
archivo::abrir_escritura(ruta: Texto) -> Resultado<Archivo>
archivo::abrir_agregar(ruta: Texto) -> Resultado<Archivo>
archivo::crear(ruta: Texto) -> Resultado<Archivo>          // truncar
archivo::cerrar(a: Archivo)

// Lectura
archivo::leer(a: Archivo) -> Resultado<Texto>              // todo el archivo
archivo::leer_linea(a: Archivo) -> Resultado<Option<Texto>>
archivo::leer_lineas(a: Archivo) -> Resultado<Vector<Texto>>
archivo::leer_bytes(a: Archivo) -> Resultado<Vector<Entero8>>
archivo::leer_hasta(a: Archivo, n: Entero32) -> Resultado<Texto>

// Escritura
archivo::escribir(a: Archivo, contenido: Texto) -> Resultado
archivo::escribir_linea(a: Archivo, linea: Texto) -> Resultado
archivo::escribir_bytes(a: Archivo, bytes: Vector<Entero8>) -> Resultado
archivo::agregar(a: Archivo, contenido: Texto) -> Resultado

// Posición
archivo::posicion(a: Archivo) -> Entero64
archivo::mover(a: Archivo, offset: Entero64) -> Resultado
archivo::al_inicio(a: Archivo)
archivo::al_final(a: Archivo)
archivo::longitud(a: Archivo) -> Entero64

// Estado
archivo::existe(ruta: Texto) -> Booleano
archivo::es_archivo(ruta: Texto) -> Booleano
archivo::es_directorio(ruta: Texto) -> Booleano
archivo::tamano(ruta: Texto) -> Resultado<Entero64>
archivo::borrar(ruta: Texto) -> Resultado
archivo::renombrar(de: Texto, a: Texto) -> Resultado
archivo::copiar(de: Texto, a: Texto) -> Resultado
archivo::mover(de: Texto, a: Texto) -> Resultado
```

### 5.2 Directorio (`libEst::directorio`)

```falcato
directorio::abrir(ruta: Texto) -> Resultado<Directorio>
directorio::listar(d: Directorio) -> Resultado<Vector<Texto>>
directorio::crear(ruta: Texto) -> Resultado
directorio::crear_recursivo(ruta: Texto) -> Resultado
directorio::borrar(ruta: Texto) -> Resultado
directorio::borrar_recursivo(ruta: Texto) -> Resultado
directorio::actual(ruta: Texto) -> Resultado<Texto>        // cwd
directorio::cambiar(ruta: Texto) -> Resultado
directorio::existe(ruta: Texto) -> Booleano
directorio::es_vacio(ruta: Texto) -> Booleano
directorio::tamano(ruta: Texto) -> Resultado<Entero64>     // recursivo
```

### 5.3 Ruta (`libEst::ruta`)

```falcato
ruta::unir(a: Texto, b: Texto) -> Texto
ruta::absoluta(ruta: Texto) -> Resultado<Texto>
ruta::relativa(ruta: Texto, base: Texto) -> Resultado<Texto>
ruta::nombre_archivo(ruta: Texto) -> Texto
ruta::extension(ruta: Texto) -> Texto
ruta::sin_extension(ruta: Texto) -> Texto
ruta::directorio_padre(ruta: Texto) -> Texto
ruta::es_absoluta(ruta: Texto) -> Booleano
ruta::normalizar(ruta: Texto) -> Texto
ruta::separador() -> Texto
```

### 5.4 Formato (`libEst::formato`)

```falcato
formato::csv_parsear(texto: Texto) -> Resultado<Vector<Vector<Texto>>>
formato::csv_serializar(datos: Vector<Vector<Texto>>) -> Texto
formato::ini_parsear(texto: Texto) -> Resultado<Diccionario<Texto, Texto>>
formato::toml_parsear(texto: Texto) -> Resultado<Diccionario<Texto, Texto>>
```

---

## 6. Red (`libEst::red`)

### 6.1 TCP (`libEst::red::tcp`)

```falcato
tcp::conectar(host: Texto, puerto: Entero32) -> Resultado<Conexion>
tcp::escuchar(puerto: Entero32) -> Resultado<Servidor>
tcp::aceptar(s: Servidor) -> Resultado<Conexion>
tcp::enviar(c: Conexion, datos: Texto) -> Resultado
tcp::enviar_bytes(c: Conexion, datos: Vector<Entero8>) -> Resultado
tcp::recibir(c: Conexion) -> Resultado<Texto>
tcp::recibir_bytes(c: Conexion, n: Entero32) -> Resultado<Vector<Entero8>>
tcp::cerrar(c: Conexion)
tcp::cerrar_servidor(s: Servidor)
tcp::direccion_local(c: Conexion) -> Texto
tcp::direccion_remota(c: Conexion) -> Texto
```

### 6.2 UDP (`libEst::red::udp`)

```falcato
udp::abrir(puerto: Entero32) -> Resultado<Socket>
udp::enviar(s: Socket, host: Texto, puerto: Entero32, datos: Texto) -> Resultado
udp::enviar_bytes(s: Socket, host: Texto, puerto: Entero32, datos: Vector<Entero8>) -> Resultado
udp::recibir(s: Socket) -> Resultado<Texto>
udp::recibir_bytes(s: Socket, n: Entero32) -> Resultado<Vector<Entero8>>
udp::cerrar(s: Socket)
```

### 6.3 TLS (`libEst::red::tls`)

```falcato
tls::conectar(host: Texto, puerto: Entero32) -> Resultado<Conexion>
tls::enviar(c: Conexion, datos: Texto) -> Resultado
tls::recibir(c: Conexion) -> Resultado<Texto>
tls::cerrar(c: Conexion)
tls::verificar_certificado(c: Conexion) -> Resultado<Booleano>
```

### 6.4 HTTP (`libEst::red::http`) — desbloquea Cid

```falcato
http::get(url: Texto) -> Resultado<Respuesta>
http::post(url: Texto, cuerpo: Texto) -> Resultado<Respuesta>
http::put(url: Texto, cuerpo: Texto) -> Resultado<Respuesta>
http::borrar(url: Texto) -> Resultado<Respuesta>
http::cabecera(r: Respuesta, nombre: Texto) -> Option<Texto>
http::cuerpo(r: Respuesta) -> Texto
http::estado(r: Respuesta) -> Entero32
http::es_exito(r: Respuesta) -> Booleano
http::con_cabecera(req: Peticion, nombre: Texto, valor: Texto) -> Peticion
http::con_tiempo_limite(req: Peticion, ms: Entero32) -> Peticion
```

### 6.5 Servidor HTTP (`libEst::red::servidor`)

```falcato
servidor::nuevo(puerto: Entero32) -> Resultado<Servidor>
servidor::manejar(s: Servidor, ruta: Texto, handler: fn(Peticion) -> Respuesta)
servidor::iniciar(s: Servidor) -> Resultado
servidor::detener(s: Servidor)
servidor::respuesta_texto(cuerpo: Texto) -> Respuesta
servidor::respuesta_json(cuerpo: Texto) -> Respuesta
servidor::respuesta_error(estado: Entero32, mensaje: Texto) -> Respuesta
```

### 6.6 DNS (`libEst::red::dns`)

```falcato
dns::resolver(host: Texto) -> Resultado<Texto>          // primera IP
dns::resolver_todas(host: Texto) -> Resultado<Vector<Texto>>
```

### 6.7 JSON (`libEst::red::json`) — desbloquea Cid

```falcato
json::serializar(valor: T) -> Resultado<Texto>
json::deserializar<T>(texto: Texto) -> Resultado<T>
json::parsear(texto: Texto) -> Resultado<ValorJson>
json::obtener(v: ValorJson, ruta: Texto) -> Option<ValorJson>
json::es_objeto(v: ValorJson) -> Booleano
json::es_array(v: ValorJson) -> Booleano
json::es_texto(v: ValorJson) -> Booleano
json::es_numero(v: ValorJson) -> Booleano
json::es_booleano(v: ValorJson) -> Booleano
json::es_nulo(v: ValorJson) -> Booleano
json::a_texto(v: ValorJson) -> Option<Texto>
json::a_entero(v: ValorJson) -> Option<Entero64>
json::a_booleano(v: ValorJson) -> Option<Booleano>
json::a_array(v: ValorJson) -> Option<Vector<ValorJson>>
json::a_objeto(v: ValorJson) -> Option<Diccionario<Texto, ValorJson>>
```

---

## 7. Tiempo (`libEst::tiempo`)

### 7.1 Reloj (`libEst::tiempo::reloj`)

```falcato
reloj::ahora() -> Instante
reloj::unix() -> Entero64
reloj::milisegundos() -> Entero64
reloj::nanosegundos() -> Entero64
reloj::duracion(inicio: Instante, fin: Instante) -> Duracion
reloj::segundos(d: Duracion) -> Real
reloj::milisegundos(d: Duracion) -> Entero64
reloj::nanosegundos(d: Duracion) -> Entero64
```

### 7.2 Fecha (`libEst::tiempo::fecha`)

```falcato
fecha::ahora() -> Texto                    // "2026-08-28 17:30:45"
fecha::hoy() -> Texto                      // "2026-08-28"
fecha::hora() -> Texto                     // "17:30:45"
fecha::anio(unix: Entero64) -> Entero32
fecha::mes(unix: Entero64) -> Entero32
fecha::dia(unix: Entero64) -> Entero32
fecha::hora_del_dia(unix: Entero64) -> Entero32
fecha::minuto(unix: Entero64) -> Entero32
fecha::segundo(unix: Entero64) -> Entero32
fecha::dia_semana(unix: Entero64) -> Texto  // "lunes"...
fecha::formatear(unix: Entero64, formato: Texto) -> Texto  // strftime
fecha::desde_texto(texto: Texto) -> Resultado<Entero64>
```

### 7.3 Temporizador (`libEst::tiempo::temporizador`)

```falcato
temporizador::dormir(ms: Entero32)
temporizador::nuevo(ms: Entero32, callback: fn()) -> Temporizador
temporizador::cancelar(t: Temporizador)
temporizador::repetir(ms: Entero32, callback: fn()) -> Temporizador
```

---

## 8. Proceso (`libEst::proceso`)

### 8.1 Proceso (`libEst::proceso`)

```falcato
proceso::crear(comando: Texto) -> Resultado<Proceso>
proceso::crear_con_args(comando: Texto, args: Vector<Texto>) -> Resultado<Proceso>
proceso::esperar(p: Proceso) -> Resultado<Entero32>       // exit code
proceso::matar(p: Proceso) -> Resultado
proceso::esta_corriendo(p: Proceso) -> Booleano
proceso::salida_estandar(p: Proceso) -> Resultado<Texto>
proceso::salida_error(p: Proceso) -> Resultado<Texto>
proceso::entrada_estandar(p: Proceso, datos: Texto) -> Resultado
proceso::id(p: Proceso) -> Entero64
proceso::actual_id() -> Entero64
```

### 8.2 Hilo (`libEst::proceso::hilo`)

```falcato
hilo::crear(f: fn()) -> Resultado<Hilo>
hilo::unir(h: Hilo) -> Resultado
hilo::dormir(ms: Entero32)
hilo::actual_id() -> Entero64
hilo::numero() -> Entero32                 // nº de hilos del sistema
```

### 8.3 Canal (`libEst::proceso::canal`)

```falcato
canal::nuevo<T>() -> Canal<T>
canal::enviar(c: Canal<T>, item: T) -> Resultado
canal::recibir(c: Canal<T>) -> Resultado<T>
canal::intentar_recibir(c: Canal<T>) -> Resultado<Option<T>>
canal::cerrar(c: Canal<T>)
canal::longitud(c: Canal<T>) -> Entero32
```

### 8.4 Sincronización (`libEst::proceso::sincronizar`)

```falcato
sincronizar::mutex_nuevo() -> Mutex
sincronizar::bloquear(m: Mutex)
sincronizar::desbloquear(m: Mutex)
sincronizar::sem_nuevo(valor_inicial: Entero32) -> Semaforo
sincronizar::sem_esperar(s: Semaforo)
sincronizar::sem_senal(s: Semaforo)
sincronizar::barrera_nueva(n: Entero32) -> Barrera
sincronizar::barrera_esperar(b: Barrera)
```

---

## 9. Sistema (`libEst::sistema`)

### 9.1 Entorno (`libEst::sistema::entorno`)

```falcato
entorno::obtener(nombre: Texto) -> Option<Texto>
entorno::establecer(nombre: Texto, valor: Texto) -> Resultado
entorno::eliminar(nombre: Texto)
entorno::todos() -> Diccionario<Texto, Texto>
```

### 9.2 Argumentos (`libEst::sistema::argumentos`)

```falcato
argumentos::todos() -> Vector<Texto>
argumentos::cantidad() -> Entero32
argumentos::obtener(indice: Entero32) -> Option<Texto>
argumentos::programa() -> Texto
argumentos::tiene(flag: Texto) -> Booleano
argumentos::valor_de(flag: Texto) -> Option<Texto>
```

### 9.3 Consola (`libEst::sistema::consola`)

```falcato
consola::imprimir(texto: Texto)
consola::imprimir_linea(texto: Texto)
consola::imprimir_error(texto: Texto)
consola::leer_linea() -> Resultado<Texto>
consola::leer_caracter() -> Resultado<Texto>
consola::limpiar()
consola::color_texto(color: Color)
consola::color_fondo(color: Color)
consola::restablecer_color()
consola::mover_cursor(x: Entero32, y: Entero32)
consola::ocultar_cursor(ocultar: Booleano)
consola::dimensiones() -> (Entero32, Entero32)   // ancho, alto
consola::modo_raw(activo: Booleano) -> Resultado
```

### 9.4 Aleatorio (`libEst::sistema::aleatorio`)

```falcato
aleatorio::entero() -> Entero64
aleatorio::entero_entre(min: Entero64, max: Entero64) -> Entero64
aleatorio::real() -> Real                    // [0, 1)
aleatorio::real_entre(min: Real, max: Real) -> Real
aleatorio::booleano() -> Booleano
aleatorio::elegir<T>(items: Vector<T>) -> Option<T>
aleatorio::barajar<T>(items: Vector<T>) -> Vector<T>
aleatorio::semilla(s: Entero64)
```

### 9.5 Cripto (`libEst::sistema::cripto`)

```falcato
cripto::hash_md5(texto: Texto) -> Texto
cripto::hash_sha1(texto: Texto) -> Texto
cripto::hash_sha256(texto: Texto) -> Texto
cripto::hash_sha512(texto: Texto) -> Texto
cripto::hmac_sha256(clave: Texto, mensaje: Texto) -> Texto
cripto::aleatorio_seguro(n: Entero32) -> Vector<Entero8>
cripto::constante_tiempo_igual(a: Texto, b: Texto) -> Booleano
```

### 9.6 Memoria (`libEst::sistema::memoria`)

```falcato
memoria::tamano_de<T>() -> Entero32
memoria::alineacion_de<T>() -> Entero32
memoria::asignar(n: Entero32) -> *mut Entero8
memoria::liberar(ptr: *mut Entero8)
memoria::copiar(origen: *mut Entero8, destino: *mut Entero8, n: Entero32)
memoria::cero(ptr: *mut Entero8, n: Entero32)
```

---

## 10. Matemáticas (`libEst::matematicas`)

### 10.1 Aritmética (`libEst::matematicas::aritmetica`)

```falcato
aritmetica::abs(n: Real) -> Real
aritmetica::max(a: Real, b: Real) -> Real
aritmetica::min(a: Real, b: Real) -> Real
aritmetica::redondear(n: Real) -> Entero64
aritmetica::piso(n: Real) -> Entero64
aritmetica::techo(n: Real) -> Entero64
aritmetica::truncar(n: Real) -> Entero64
aritmetica::signo(n: Real) -> Entero32
aritmetica::clamp(n: Real, min: Real, max: Real) -> Real
aritmetica::es_finito(n: Real) -> Booleano
aritmetica::es_nan(n: Real) -> Booleano
```

### 10.2 Trigonometría (`libEst::matematicas::trig`)

```falcato
trig::seno(angulo: Real) -> Real
trig::coseno(angulo: Real) -> Real
trig::tangente(angulo: Real) -> Real
trig::arcseno(x: Real) -> Real
trig::arccoseno(x: Real) -> Real
trig::arctangente(x: Real) -> Real
trig::arctangente2(y: Real, x: Real) -> Real
trig::seno_hiperbolico(x: Real) -> Real
trig::coseno_hiperbolico(x: Real) -> Real
trig::tangente_hiperbolica(x: Real) -> Real
trig::grados_a_radianes(grados: Real) -> Real
trig::radianes_a_grados(radianes: Real) -> Real
trig::seno_2pi(fase: Real) -> Real          // para osciladores
trig::coseno_2pi(fase: Real) -> Real
```

### 10.3 Exponencial (`libEst::matematicas::exponencial`)

```falcato
exponencial::exp(x: Real) -> Real
exponencial::log(x: Real) -> Real           // log natural
exponencial::log10(x: Real) -> Real
exponencial::log2(x: Real) -> Real
exponencial::potencia(base: Real, exp: Real) -> Real
exponencial::raiz(x: Real) -> Real          // sqrt
exponencial::raiz_cubica(x: Real) -> Real
```

### 10.4 Complejo (`libEst::matematicas::complejo`)

```falcato
complejo::nuevo(real: Real, imag: Real) -> Complejo
complejo::real(c: Complejo) -> Real
complejo::imaginario(c: Complejo) -> Real
complejo::modulo(c: Complejo) -> Real
complejo::argumento(c: Complejo) -> Real
complejo::sumar(a: Complejo, b: Complejo) -> Complejo
complejo::restar(a: Complejo, b: Complejo) -> Complejo
complejo::multiplicar(a: Complejo, b: Complejo) -> Complejo
complejo::dividir(a: Complejo, b: Complejo) -> Complejo
complejo::conjugado(c: Complejo) -> Complejo
```

### 10.5 Vectorial (`libEst::matematicas::vectorial`)

```falcato
vectorial::nuevo2(x: Real, y: Real) -> Vector2
vectorial::nuevo3(x: Real, y: Real, z: Real) -> Vector3
vectorial::longitud2(v: Vector2) -> Real
vectorial::longitud3(v: Vector3) -> Real
vectorial::normalizar2(v: Vector2) -> Vector2
vectorial::normalizar3(v: Vector3) -> Vector3
vectorial::producto_punto2(a: Vector2, b: Vector2) -> Real
vectorial::producto_punto3(a: Vector3, b: Vector3) -> Real
vectorial::producto_cruz(a: Vector3, b: Vector3) -> Vector3
vectorial::sumar2(a: Vector2, b: Vector2) -> Vector2
vectorial::restar2(a: Vector2, b: Vector2) -> Vector2
vectorial::escalar2(v: Vector2, s: Real) -> Vector2
vectorial::distancia2(a: Vector2, b: Vector2) -> Real
vectorial::distancia3(a: Vector3, b: Vector3) -> Real
```

### 10.6 Matriz (`libEst::matematicas::matriz`)

```falcato
matriz::nueva(filas: Entero32, columnas: Entero32) -> Matriz
matriz::identidad(n: Entero32) -> Matriz
matriz::multiplicar(a: Matriz, b: Matriz) -> Resultado<Matriz>
matriz::sumar(a: Matriz, b: Matriz) -> Resultado<Matriz>
matriz::transponer(m: Matriz) -> Matriz
matriz::determinante(m: Matriz) -> Resultado<Real>
matriz::inversa(m: Matriz) -> Resultado<Matriz>
matriz::obtener(m: Matriz, fila: Entero32, col: Entero32) -> Real
matriz::establecer(m: Matriz, fila: Entero32, col: Entero32, valor: Real)
matriz::filas(m: Matriz) -> Entero32
matriz::columnas(m: Matriz) -> Entero32
```

### 10.7 Estadística (`libEst::matematicas::estadistica`)

```falcato
estadistica::media(datos: Vector<Real>) -> Real
estadistica::mediana(datos: Vector<Real>) -> Real
estadistica::moda(datos: Vector<Real>) -> Vector<Real>
estadistica::varianza(datos: Vector<Real>) -> Real
estadistica::desviacion(datos: Vector<Real>) -> Real
estadistica::min(datos: Vector<Real>) -> Real
estadistica::max(datos: Vector<Real>) -> Real
estadistica::suma(datos: Vector<Real>) -> Real
estadistica::producto(datos: Vector<Real>) -> Real
```

---

## 11. ⭐ Visual y Gráfico (`libEst::visual`)

> **Filosofía:** Cero abstracciones gratuitas. Ownership-driven. LLM-friendly.
> Integra y extiende `docs/diseno_gui.md` (GUI-1). Cada error tiene código `[V###]`, span y sugerencia concreta.

### 11.1 Ventana (`libEst::visual::ventana`)

```falcato
// Tipos
estructural Ventana { hwnd: *mut Entero32 }
estructural ClaseVentana { ... }
estructural Punto { x: Entero32, y: Entero32 }
estructural Rect { izquierda: Entero32, superior: Entero32, derecha: Entero32, inferior: Entero32 }
estructural Tamano { ancho: Entero32, alto: Entero32 }

// Creación
ventana::clase_nueva(nombre: Texto, proc: fn(Ventana, Mensaje, Entero64, Entero64) -> Entero64) -> ClaseVentana
ventana::clase_registrar(c: ClaseVentana) -> Resultado
ventana::nueva(clase: ClaseVentana, titulo: Texto, ancho: Entero32, alto: Entero32) -> Resultado<Ventana>
ventana::mostrar(v: Ventana)
ventana::ocultar(v: Ventana)
ventana::cerrar(v: Ventana)
ventana::destruir(v: Ventana)
ventana::bucle_mensajes(v: Ventana) -> Entero32
ventana::salir(v: Ventana)

// Propiedades
ventana::titulo(v: Ventana) -> Texto
ventana::establecer_titulo(v: Ventana, titulo: Texto)
ventana::posicion(v: Ventana) -> Punto
ventana::mover(v: Ventana, p: Punto)
ventana::tamano(v: Ventana) -> Tamano
ventana::redimensionar(v: Ventana, t: Tamano)
ventana::rect_cliente(v: Ventana) -> Rect
ventana::es_visible(v: Ventana) -> Booleano
ventana::esta_maximizada(v: Ventana) -> Booleano
ventana::esta_minimizada(v: Ventana) -> Booleano
ventana::maximizar(v: Ventana)
ventana::minimizar(v: Ventana)
ventana::restaurar(v: Ventana)
ventana::enfocar(v: Ventana)
ventana::al_frente(v: Ventana)

// Estilos (bitfields)
ventana::estilo(v: Ventana) -> EstiloVentana
ventana::establecer_estilo(v: Ventana, estilo: EstiloVentana)
```

### 11.2 Controles (`libEst::visual::control`)

```falcato
// Botón
control::boton_nuevo(padre: Ventana, texto: Texto, rect: Rect) -> Resultado<Boton>
control::boton_al_click(b: Boton, cb: fn())
control::boton_texto(b: Boton) -> Texto
control::boton_establecer_texto(b: Boton, texto: Texto)
control::boton_habilitar(b: Boton, habilitado: Booleano)
control::boton_es_habilitado(b: Boton) -> Booleano
control::boton_mostrar(b: Boton)
control::boton_ocultar(b: Boton)
control::boton_eliminar(b: Boton)

// Etiqueta
control::etiqueta_nueva(padre: Ventana, texto: Texto, rect: Rect) -> Resultado<Etiqueta>
control::etiqueta_texto(e: Etiqueta) -> Texto
control::etiqueta_establecer_texto(e: Etiqueta, texto: Texto)
control::etiqueta_color(e: Etiqueta, color: Color)
control::etiqueta_fuente(e: Etiqueta, fuente: Fuente)
control::etiqueta_eliminar(e: Etiqueta)

// Caja de texto
control::caja_nueva(padre: Ventana, rect: Rect) -> Resultado<CajaTexto>
control::caja_texto(c: CajaTexto) -> Texto
control::caja_establecer_texto(c: CajaTexto, texto: Texto)
control::caja_es_solo_lectura(c: CajaTexto) -> Booleano
control::caja_establecer_solo_lectura(c: CajaTexto, solo: Booleano)
control::caja_es_multilinea(c: CajaTexto) -> Booleano
control::caja_establecer_multilinea(c: CajaTexto, multi: Booleano)
control::caja_limpiar(c: CajaTexto)
control::caja_al_cambiar(c: CajaTexto, cb: fn(Texto))
control::caja_eliminar(c: CajaTexto)

// Área de texto (multilínea)
control::area_nueva(padre: Ventana, rect: Rect) -> Resultado<AreaTexto>
control::area_texto(a: AreaTexto) -> Texto
control::area_establecer_texto(a: AreaTexto, texto: Texto)
control::area_agregar_linea(a: AreaTexto, linea: Texto)
control::area_limpiar(a: AreaTexto)
control::area_eliminar(a: AreaTexto)

// Lista
control::lista_nueva(padre: Ventana, rect: Rect) -> Resultado<Lista>
control::lista_agregar(l: Lista, item: Texto)
control::lista_eliminar(l: Lista, indice: Entero32)
control::lista_limpiar(l: Lista)
control::lista_seleccionada(l: Lista) -> Option<Entero32>
control::lista_item(l: Lista, indice: Entero32) -> Option<Texto>
control::lista_longitud(l: Lista) -> Entero32
control::lista_al_seleccionar(l: Lista, cb: fn(Entero32))
control::lista_eliminar(l: Lista)

// Barra de progreso
control::barra_nueva(padre: Ventana, rect: Rect) -> Resultado<Barra>
control::barra_valor(b: Barra) -> Entero32
control::barra_establecer_valor(b: Barra, valor: Entero32)
control::barra_rango(b: Barra, min: Entero32, max: Entero32)
control::barra_eliminar(b: Barra)

// Casilla de verificación
control::casilla_nueva(padre: Ventana, texto: Texto, rect: Rect) -> Resultado<Casilla>
control::casilla_marcada(c: Casilla) -> Booleano
control::casilla_establecer_marcada(c: Casilla, marcada: Booleano)
control::casilla_al_cambiar(c: Casilla, cb: fn(Booleano))
control::casilla_eliminar(c: Casilla)

// Menú
control::menu_nuevo() -> Menu
control::menu_agregar_item(m: Menu, texto: Texto, id: Entero32)
control::menu_agregar_separador(m: Menu)
control::menu_agregar_submenu(m: Menu, texto: Texto, sub: Menu)
control::ventana_establecer_menu(v: Ventana, m: Menu)
```

### 11.3 Layout (`libEst::visual::layout`)

```falcato
// Contenedores
layout::contenedor_nuevo(padre: Ventana, tipo: TipoLayout) -> Contenedor
layout::agregar(c: Contenedor, control: Control)
layout::recalcular(c: Contenedor, rect: Rect)
layout::tipo(c: Contenedor) -> TipoLayout
layout::establecer_tipo(c: Contenedor, tipo: TipoLayout)
layout::espaciado(c: Contenedor, px: Entero32)
layout::margen(c: Contenedor, px: Entero32)

// Anchors / constraints
layout::anclar(control: Control, anclaje: Anclaje)
layout::anclar_izquierda(control: Control, valor: Entero32)
layout::anclar_derecha(control: Control, valor: Entero32)
layout::anclar_superior(control: Control, valor: Entero32)
layout::anclar_inferior(control: Control, valor: Entero32)
layout::centrar_horizontal(control: Control)
layout::centrar_vertical(control: Control)
layout::centrar(control: Control)
layout::rellenar(control: Control)

// Tipos de layout
enumeracion TipoLayout { Vertical, Horizontal, Cuadricula, Apilado }
estructural Anclaje { izquierda: Entero32, derecha: Entero32, superior: Entero32, inferior: Entero32 }
```

### 11.4 Eventos (`libEst::visual::evento`)

```falcato
// Dispatch
evento::procesar(v: Ventana) -> Booleano     // procesa 1 mensaje pendiente
evento::procesar_todos(v: Ventana)           // procesa todos los pendientes
evento::enviar(v: Ventana, msg: Mensaje, w: Entero64, l: Entero64) -> Entero64
evento::publicar(v: Ventana, msg: Mensaje, w: Entero64, l: Entero64)

// Binding
evento::al_click(control: Control, cb: fn())
evento::al_tecla(v: Ventana, cb: fn(Tecla))
evento::al_raton_mover(v: Ventana, cb: fn(Punto))
evento::al_raton_click(v: Ventana, cb: fn(Punto, BotonRaton))
evento::al_redimensionar(v: Ventana, cb: fn(Tamano))
evento::al_cerrar(v: Ventana, cb: fn())
evento::al_pintar(v: Ventana, cb: fn(DC))

// Mensajes (enumeración)
enumeracion Mensaje {
    Crear = 1, Destruir = 2, Cerrar = 16, Pintar = 15, Tamano = 5,
    ClickIzquierdo = 513, SoltarIzquierdo = 514, Mover = 512,
    TeclaAbajo = 256, TeclaArriba = 257, Comando = 273,
}
enumeracion Tecla { ... }                    // teclas especiales
enumeracion BotonRaton { Izquierdo, Derecho, Medio }
```

### 11.5 Color (`libEst::visual::color`)

```falcato
// Tipos
estructural Color { r: Entero8, g: Entero8, b: Entero8, a: Entero8 }

// Constantes
color::ROJO -> Color
color::VERDE -> Color
color::AZUL -> Color
color::NEGRO -> Color
color::BLANCO -> Color
color::GRIS -> Color
color::AMARILLO -> Color
color::NARANJA -> Color
color::MORADO -> Color
color::CIAN -> Color
color::MAGENTA -> Color
color::TRANSPARENTE -> Color

// Creación
color::nuevo(r: Entero8, g: Entero8, b: Entero8) -> Color
color::nuevo_con_alfa(r: Entero8, g: Entero8, b: Entero8, a: Entero8) -> Color
color::desde_hex(hex: Entero32) -> Color     // 0xRRGGBB
color::desde_texto(nombre: Texto) -> Resultado<Color>

// Manipulación
color::rojo(c: Color) -> Entero8
color::verde(c: Color) -> Entero8
color::azul(c: Color) -> Entero8
color::alfa(c: Color) -> Entero8
color::con_alfa(c: Color, a: Entero8) -> Color
color::a_hex(c: Color) -> Entero32
color::a_texto(c: Color) -> Texto
color::mezclar(a: Color, b: Color, t: Real) -> Color   // interpolación
color::aclarar(c: Color, factor: Real) -> Color
color::oscurecer(c: Color, factor: Real) -> Color
color::complementario(c: Color) -> Color
color::es_claro(c: Color) -> Booleano
color::es_oscuro(c: Color) -> Booleano
```

### 11.6 Geometría (`libEst::visual::geometria`)

```falcato
// Punto
geometria::punto_nuevo(x: Entero32, y: Entero32) -> Punto
geometria::punto_x(p: Punto) -> Entero32
geometria::punto_y(p: Punto) -> Entero32
geometria::punto_sumar(a: Punto, b: Punto) -> Punto
geometria::punto_restar(a: Punto, b: Punto) -> Punto
geometria::punto_distancia(a: Punto, b: Punto) -> Real

// Rect
geometria::rect_nuevo(x: Entero32, y: Entero32, ancho: Entero32, alto: Entero32) -> Rect
geometria::rect_x(r: Rect) -> Entero32
geometria::rect_y(r: Rect) -> Entero32
geometria::rect_ancho(r: Rect) -> Entero32
geometria::rect_alto(r: Rect) -> Entero32
geometria::rect_izquierda(r: Rect) -> Entero32
geometria::rect_superior(r: Rect) -> Entero32
geometria::rect_derecha(r: Rect) -> Entero32
geometria::rect_inferior(r: Rect) -> Entero32
geometria::rect_contiene(r: Rect, p: Punto) -> Booleano
geometria::rect_intersecta(a: Rect, b: Rect) -> Booleano
geometria::rect_interseccion(a: Rect, b: Rect) -> Option<Rect>
geometria::rect_unir(a: Rect, b: Rect) -> Rect
geometria::rect_centro(r: Rect) -> Punto
geometria::rect_desplazar(r: Rect, dx: Entero32, dy: Entero32) -> Rect
geometria::rect_redimensionar(r: Rect, ancho: Entero32, alto: Entero32) -> Rect
geometria::rect_es_vacio(r: Rect) -> Booleano
```

### 11.7 Lienzo 2D (`libEst::visual::lienzo`) — ⭐ el corazón gráfico

```falcato
// Tipos
estructural Lienzo { ... }                  // canvas 2D (GDI / Direct2D)
estructural Lapiz { ... }
estructural Brocha { ... }
estructural Ruta { ... }

// Creación
lienzo::nuevo(ancho: Entero32, alto: Entero32) -> Lienzo
lienzo::desde_ventana(v: Ventana) -> Lienzo
lienzo::desde_dc(dc: DC) -> Lienzo
lienzo::liberar(l: Lienzo)

// Estado
lienzo::ancho(l: Lienzo) -> Entero32
lienzo::alto(l: Lienzo) -> Entero32
lienzo::color_relleno(l: Lienzo, color: Color)
lienzo::color_linea(l: Lienzo, color: Color)
lienzo::grosor_linea(l: Lienzo, px: Entero32)
lienzo::fuente(l: Lienzo, fuente: Fuente)
lienzo::opacidad(l: Lienzo, alfa: Real)     // 0.0 - 1.0

// Primitivas
lienzo::limpiar(l: Lienzo, color: Color)
lienzo::pixel(l: Lienzo, x: Entero32, y: Entero32, color: Color)
lienzo::linea(l: Lienzo, x1: Entero32, y1: Entero32, x2: Entero32, y2: Entero32)
lienzo::rectangulo(l: Lienzo, x: Entero32, y: Entero32, ancho: Entero32, alto: Entero32)
lienzo::rectangulo_relleno(l: Lienzo, x: Entero32, y: Entero32, ancho: Entero32, alto: Entero32)
lienzo::rectangulo_redondeado(l: Lienzo, x: Entero32, y: Entero32, ancho: Entero32, alto: Entero32, radio: Entero32)
lienzo::circulo(l: Lienzo, cx: Entero32, cy: Entero32, radio: Entero32)
lienzo::circulo_relleno(l: Lienzo, cx: Entero32, cy: Entero32, radio: Entero32)
lienzo::elipse(l: Lienzo, cx: Entero32, cy: Entero32, rx: Entero32, ry: Entero32)
lienzo::elipse_rellena(l: Lienzo, cx: Entero32, cy: Entero32, rx: Entero32, ry: Entero32)
lienzo::arco(l: Lienzo, cx: Entero32, cy: Entero32, radio: Entero32, inicio: Real, fin: Real)
lienzo::poligono(l: Lienzo, puntos: Vector<Punto>)
lienzo::poligono_relleno(l: Lienzo, puntos: Vector<Punto>)
lienzo::polilinea(l: Lienzo, puntos: Vector<Punto>)
lienzo::curva(l: Lienzo, puntos: Vector<Punto>)   // bezier suavizada

// Texto
lienzo::texto(l: Lienzo, x: Entero32, y: Entero32, texto: Texto)
lienzo::texto_centrado(l: Lienzo, x: Entero32, y: Entero32, texto: Texto)
lienzo::texto_en_rect(l: Lienzo, rect: Rect, texto: Texto, alineacion: Alineacion)
lienzo::medir_texto(l: Lienzo, texto: Texto) -> Tamano

// Transformaciones
lienzo::trasladar(l: Lienzo, dx: Entero32, dy: Entero32)
lienzo::escalar(l: Lienzo, sx: Real, sy: Real)
lienzo::rotar(l: Lienzo, grados: Real)
lienzo::guardar(l: Lienzo)                    // push transform
lienzo::restaurar(l: Lienzo)                  // pop transform
lienzo::restablecer_transformacion(l: Lienzo)

// Recorte
lienzo::recortar(l: Lienzo, rect: Rect)
lienzo::recortar_circulo(l: Lienzo, cx: Entero32, cy: Entero32, radio: Entero32)
lienzo::sin_recorte(l: Lienzo)

// Gradientes
lienzo::gradiente_lineal(l: Lienzo, x1: Entero32, y1: Entero32, x2: Entero32, y2: Entero32, a: Color, b: Color)
lienzo::gradiente_radial(l: Lienzo, cx: Entero32, cy: Entero32, radio: Entero32, a: Color, b: Color)

// Sombra
lienzo::sombra(l: Lienzo, rect: Rect, color: Color, desenfoque: Entero32)
```

### 11.8 Imagen (`libEst::visual::imagen`)

```falcato
// Tipos
estructural Imagen { ... }                  // bitmap en memoria

// Creación
imagen::nueva(ancho: Entero32, alto: Entero32) -> Imagen
imagen::desde_archivo(ruta: Texto) -> Resultado<Imagen>   // PNG/JPEG/BMP
imagen::desde_bytes(bytes: Vector<Entero8>) -> Resultado<Imagen>
imagen::desde_lienzo(l: Lienzo) -> Imagen

// Propiedades
imagen::ancho(i: Imagen) -> Entero32
imagen::alto(i: Imagen) -> Entero32
imagen::pixel(i: Imagen, x: Entero32, y: Entero32) -> Color
imagen::establecer_pixel(i: Imagen, x: Entero32, y: Entero32, color: Color)

// Manipulación
imagen::redimensionar(i: Imagen, ancho: Entero32, alto: Entero32) -> Imagen
imagen::recortar(i: Imagen, rect: Rect) -> Imagen
imagen::rotar(i: Imagen, grados: Real) -> Imagen
imagen::voltear_horizontal(i: Imagen) -> Imagen
imagen::voltear_vertical(i: Imagen) -> Imagen
imagen::invertir_colores(i: Imagen) -> Imagen
imagen::escala_grises(i: Imagen) -> Imagen
imagen::brillo(i: Imagen, factor: Real) -> Imagen
imagen::contraste(i: Imagen, factor: Real) -> Imagen
imagen::desenfocar(i: Imagen, radio: Entero32) -> Imagen

// Dibujo
imagen::dibujar(i: Imagen, l: Lienzo, x: Entero32, y: Entero32)
imagen::dibujar_escalada(i: Imagen, l: Lienzo, rect: Rect)
imagen::dibujar_con_alfa(i: Imagen, l: Lienzo, x: Entero32, y: Entero32, alfa: Real)

// Guardar
imagen::guardar_png(i: Imagen, ruta: Texto) -> Resultado
imagen::guardar_jpeg(i: Imagen, ruta: Texto, calidad: Entero32) -> Resultado
imagen::guardar_bmp(i: Imagen, ruta: Texto) -> Resultado
imagen::a_bytes_png(i: Imagen) -> Resultado<Vector<Entero8>>
```

### 11.9 Fuente (`libEst::visual::fuente`)

```falcato
// Tipos
estructural Fuente { ... }

// Creación
fuente::nueva(nombre: Texto, tamano: Entero32) -> Fuente
fuente::nueva_con_estilo(nombre: Texto, tamano: Entero32, estilo: EstiloFuente) -> Fuente
fuente::sistema() -> Fuente
fuente::monoespaciada(tamano: Entero32) -> Fuente

// Propiedades
fuente::nombre(f: Fuente) -> Texto
fuente::tamano(f: Fuente) -> Entero32
fuente::estilo(f: Fuente) -> EstiloFuente
fuente::es_negrita(f: Fuente) -> Booleano
fuente::es_cursiva(f: Fuente) -> Booleano
fuente::es_subrayada(f: Fuente) -> Booleano

// Estilos
enumeracion EstiloFuente { Normal, Negrita, Cursiva, NegritaCursiva, Subrayada }
```

### 11.10 Animación (`libEst::visual::animacion`)

```falcato
// Tipos
estructural Animacion { ... }
estructural Fotograma { ... }

// Creación
animacion::nueva(duracion_ms: Entero32, cb: fn(Real)) -> Animacion   // t: 0.0-1.0
animacion::repetir(duracion_ms: Entero32, cb: fn(Real)) -> Animacion
animacion::invertir(a: Animacion)
animacion::detener(a: Animacion)
animacion::pausar(a: Animacion)
animacion::reanudar(a: Animacion)
animacion::esta_corriendo(a: Animacion) -> Booleano
animacion::progreso(a: Animacion) -> Real

// Easing
animacion::lineal(t: Real) -> Real
animacion::suave(t: Real) -> Real            // ease-in-out
animacion::entrada(t: Real) -> Real          // ease-in
animacion::salida(t: Real) -> Real           // ease-out
animacion::rebote(t: Real) -> Real
animacion::elastico(t: Real) -> Real
animacion::con_easing(a: Animacion, easing: fn(Real) -> Real) -> Animacion

// Interpolación
animacion::interpolar(inicio: Real, fin: Real, t: Real) -> Real
animacion::interpolar_color(a: Color, b: Color, t: Real) -> Color
animacion::interpolar_punto(a: Punto, b: Punto, t: Real) -> Punto
```

### 11.11 Sonido (`libEst::visual::sonido`) — ⭐ audio para la DAW (R9.3)

> **Filosofía:** buffers de audio como `Vector<Real>` (muestras PCM), ownership-driven. Conecta con la DAW R9.3.0 (WAV, buffers, mezcla, secuenciador, efectos).

```falcato
// Tipos
estructural Audio { muestras: Vector<Real>, canales: Entero32, frecuencia: Entero32 }
estructural Secuenciador { ... }
estructural Efecto { ... }

// Creación
sonido::nuevo(canales: Entero32, frecuencia: Entero32) -> Audio
sonido::desde_archivo(ruta: Texto) -> Resultado<Audio>        // WAV
sonido::desde_bytes(bytes: Vector<Entero8>) -> Resultado<Audio>
sonido::silencio(duracion_ms: Entero32, canales: Entero32, frecuencia: Entero32) -> Audio
sonido::tono(frecuencia: Real, duracion_ms: Entero32, canales: Entero32, frecuencia_muestra: Entero32) -> Audio
sonido::tono_envolvente(frecuencia: Real, duracion_ms: Entero32, ataque: Real, caida: Real, ...) -> Audio

// Propiedades
sonido::canales(a: Audio) -> Entero32
sonido::frecuencia(a: Audio) -> Entero32
sonido::duracion_ms(a: Audio) -> Entero32
sonido::longitud_muestras(a: Audio) -> Entero32
sonido::muestra(a: Audio, indice: Entero32) -> Real
sonido::establecer_muestra(a: Audio, indice: Entero32, valor: Real)

// Mezcla
sonido::mezclar(a: Audio, b: Audio) -> Resultado<Audio>       // suma con clamp
sonido::mezclar_con_volumen(a: Audio, b: Audio, volumen_b: Real) -> Resultado<Audio>
sonido::concatenar(a: Audio, b: Audio) -> Resultado<Audio>
sonido::superponer(a: Audio, b: Audio, offset_ms: Entero32) -> Resultado<Audio>

// Edición
sonido::recortar(a: Audio, inicio_ms: Entero32, fin_ms: Entero32) -> Audio
sonido::volumen(a: Audio, factor: Real) -> Audio
sonido::normalizar(a: Audio) -> Audio
sonido::invertir(a: Audio) -> Audio
sonido::reversa(a: Audio) -> Audio
sonido::cambiar_frecuencia(a: Audio, nueva_frecuencia: Entero32) -> Audio
sonido::cambiar_velocidad(a: Audio, factor: Real) -> Audio

// Efectos
sonido::eco(a: Audio, retraso_ms: Entero32, retroalimentacion: Real) -> Audio
sonido::reverberacion(a: Audio, tamano: Real, mezcla: Real) -> Audio
sonido::fade_in(a: Audio, duracion_ms: Entero32) -> Audio
sonido::fade_out(a: Audio, duracion_ms: Entero32) -> Audio
sonido::distorsion(a: Audio, cantidad: Real) -> Audio
sonido::filtro_pasa_bajos(a: Audio, frecuencia_corte: Real) -> Audio
sonido::filtro_pasa_altos(a: Audio, frecuencia_corte: Real) -> Audio
sonido::compresor(a: Audio, umbral: Real, ratio: Real) -> Audio

// Secuenciador (DAW)
sonido::secuenciador_nuevo(frecuencia: Entero32) -> Secuenciador
sonido::secuenciador_agregar_nota(s: Secuenciador, inicio_ms: Entero32, duracion_ms: Entero32, frecuencia: Real, volumen: Real)
sonido::secuenciador_agregar_pista(s: Secuenciador, pista: Entero32)
sonido::secuenciador_render(s: Secuenciador) -> Audio
sonido::secuenciador_limpiar(s: Secuenciador)

// Guardar
sonido::guardar_wav(a: Audio, ruta: Texto) -> Resultado
sonido::a_bytes_wav(a: Audio) -> Resultado<Vector<Entero8>>
sonido::reproducir(a: Audio) -> Resultado            // reproducción nativa
sonido::reproducir_async(a: Audio) -> Resultado      // no bloquea
sonido::detener_reproduccion()
```

### 11.12 Terminal UI (`libEst::visual::terminal_ui`)

```falcato
// Cajas y bordes
terminal_ui::caja(x: Entero32, y: Entero32, ancho: Entero32, alto: Entero32, titulo: Texto)
terminal_ui::linea_horizontal(x: Entero32, y: Entero32, ancho: Entero32)
terminal_ui::linea_vertical(x: Entero32, y: Entero32, alto: Entero32)

// Texto con estilo
terminal_ui::texto_en(x: Entero32, y: Entero32, texto: Texto)
terminal_ui::texto_color(texto: Texto, color: Color)
terminal_ui::texto_negrita(texto: Texto) -> Texto
terminal_ui::texto_subrayado(texto: Texto) -> Texto
terminal_ui::texto_parpadeante(texto: Texto) -> Texto

// Input
terminal_ui::preguntar(prompt: Texto) -> Resultado<Texto>
terminal_ui::preguntar_oculto(prompt: Texto) -> Resultado<Texto>   // contraseña
terminal_ui::confirmar(prompt: Texto) -> Booleano
terminal_ui::elegir(prompt: Texto, opciones: Vector<Texto>) -> Resultado<Entero32>

// Barras
terminal_ui::barra_progreso(progreso: Real, ancho: Entero32) -> Texto
terminal_ui::barra_carga(progreso: Real, ancho: Entero32) -> Texto

// Tablas
terminal_ui::tabla(cabeceras: Vector<Texto>, filas: Vector<Vector<Texto>>) -> Texto
```

---

## 12. Compat (`libEst::compat`) — aliases de builtins viejos

> Los nombres actuales se mantienen como aliases durante 2-3 releases (P-005).

```falcato
// texto
compat::texto_agregar_texto = texto::agregar
compat::texto_agregar = texto::agregar
compat::texto_longitud = texto::longitud
compat::texto_contiene = texto::contiene
compat::texto_dividir = texto::dividir
compat::texto_desde = texto::desde

// archivo
compat::archivo_leer = archivo::leer
compat::archivo_escribir = archivo::escribir
compat::archivo_agregar = archivo::agregar
compat::archivo_existe = archivo::existe
compat::archivo_borrar = archivo::borrar
compat::archivo_renombrar = archivo::renombrar
compat::archivo_listar = directorio::listar

// red
compat::tcp_conectar = red::tcp::conectar
compat::tls_conectar = red::tls::conectar

// proceso
compat::proceso_crear = proceso::crear

// http / json (cuando existan)
compat::http_get = red::http::get
compat::json_serializar = red::json::serializar
```

---

## 13. Orden de implementación (roadmap)

| Fase | Contenido | Versión | Desbloquea |
|------|-----------|---------|-----------|
| **F1** | `::` namespace + organización + `libEst::compat` | 0.8.0 | Todo lo demás |
| **F2** | `núcleo` (texto, numeros, opcion) + `colecciones` | 0.8.0 | Programas básicos |
| **F3** | `archivo` + `tiempo` + `sistema` | 0.8.0 | I/O completo |
| **F4** | `matematicas` | 0.8.0 | Cálculo |
| **F5** | `red::tcp/udp/tls` + `proceso` | 0.8.1 | Red básica |
| **F6** | `red::http` + `red::json` | 0.9.0 | **Cid** |
| **F7** | `visual::terminal_ui` | 0.9.0 | TUI |
| **F8** | `visual::ventana/control/layout/evento` | 0.9.x | GUI nativa |
| **F9** | `visual::lienzo/color/geometria/fuente` | 0.9.x | Gráficos 2D |
| **F10** | `visual::imagen` + `visual::animacion` | 0.9.x | Imágenes, animación |
| **F11** | `visual::sonido` (WAV, buffers, mezcla, efectos) | 0.9.x | **DAW R9.3** |
| **F12** | `red::servidor` + `red::dns` | 1.0 | Backend completo |

---

## 14. Criterios de éxito

1. **Un LLM puede generar un programa no-trivial** (leer archivo, parsear JSON, hacer HTTP, dibujar en una ventana) **sin alucinar APIs** — cada función tiene firma clara y consistente.
2. **Cero inconsistencias de API** (P-017): todas las funciones de una familia aceptan los mismos tipos.
3. **Cero comportamientos silenciosos**: todo fallo es un error `[S###]` o `[V###]` con span y sugerencia.
4. **Compatibilidad**: el código de hoy compila con `libEst::compat` durante 2-3 releases.
5. **Ownership-driven**: los recursos gráficos (GDI, imágenes, fuentes) se liberan automáticamente por el artículo (`el`/`la`/`los`).
