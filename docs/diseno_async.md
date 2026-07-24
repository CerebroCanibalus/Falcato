# Diseño de Async en Falcato — Fase 18

> Estado: **FINAL** — listo para implementación
> Fecha: 2026-07-23
> Autor: General Beria + Falcato Agent

---

## 1. Motivación

Falcato es un lenguaje de sistemas. Los sistemas modernos necesitan concurrencia:
servidores, pipelines de datos, I/O multiplexado. Pero el async en lenguajes
existentes (Rust, C++) es notoriamente complejo.

**Problema con Rust async:**
- `Pin<Box<dyn Future>>` — el programador pelea con el type system
- Lifetimes en futuros — `'a` se vuelve inmanejable con `.await`
- `Send + 'static` bounds en spawn — fricción constante
- Colored functions — `async fn` vs `fn` infecta todo el call graph
- No hay cancelación estructurada — tareas huérfanas por diseño
- Recursión async requiere `Box::pin` manual

**Ventaja Falcato (ya implementada):**
- `&yo T` (self-referential) → resuelve `Pin` sin ceremony
- `región { }` (arena allocation) → memoria determinística para tareas
- `puro`/`muta(campo)`/`lee(campo)` → el compiler razona seguridad entre tasks
- Borrow checker gradual → Nivel 0 para prototipos, Nivel 2 para producción

**Tesis:** Falcato puede ofrecer async con la ergonomía de Go y la seguridad
de Rust, aprovechando gramática española (futuro verbal = async, subjuntivo =
fallible) y features de ownership ya implementadas.

---

## 2. Decisiones de diseño (FINALES)

### 2.1 Keyword: `fut función`

```falcato
fut función descargar(la url: Texto) -> Resultado<Texto, Entero32> {
    la respuesta = esperar http::obtener(url)?;
    retornar Resultado.Exito(esperar respuesta.leer_todo()?);
}
```

`fut` es keyword. Adjetivo antes del sustantivo, como en español natural.

### 2.2 Colored functions: modelo híbrido

- `esperar` **solo** dentro de `fut función` (semántica limpia, un significado)
- `fut función principal()` funciona sin ceremony (el runtime arranca el executor)
- Para llamar async desde sync: `bloquear(expr)` — API explícita, no `esperar`
- Error `[T080]` si `esperar` aparece fuera de `fut función`

```falcato
// OK: esperar dentro de fut función
fut función proceso() -> Entero32 {
    retornar esperar operacion();
}

// OK: principal puede ser fut gratis
fut función principal() -> Entero32 {
    la x = esperar proceso();
    retornar x;
}

// OK: sync llama async con bloquear() explícito
función sync_helper() -> Entero32 {
    retornar bloquear(proceso());  // bloquea el thread actual
}

// ERROR [T080]: esperar fuera de fut función
función malo() -> Entero32 {
    retornar esperar proceso();  // ← error
}
```

### 2.3 Stack: el compiler decide (stackless por defecto)

**Regla:** el compiler analiza cada `fut función` y elige el modelo óptimo:

| Condición | Modelo | Overhead |
|-----------|--------|----------|
| Sin recursión, variables predecibles | **Stackless** (state machine) | CERO (como Rust) |
| Recursión directa o indirecta | **Stackful** (stack dinámico) | ~1-2 ns/llamada |
| Buffers locales grandes (>4KB) | **Stackful** | ~1-2 ns/llamada |
| Default (90% de casos) | **Stackless** | CERO |

**Stackless (state machine):**
```falcato
// El compiler genera internamente:
// estructural DescargarFuturo { estado: Entero32, url: Texto, resp: Respuesta }
// Tamaño: ~96 bytes. Cero stack. Cache-friendly.
fut función descargar(la url: Texto) -> Texto {
    la resp = esperar http::obtener(url);
    la cuerpo = esperar resp.leer();
    retornar cuerpo;
}
```

**Stackful (stack dinámico estilo Go):**
```falcato
// Recursión → no puede ser state machine (tamaño infinito)
// Stack: 8KB inicial, crece ×2 cuando se llena
fut función recorrer_arbol(el nodo: &Nodo) -> Entero64 {
    la suma = nodo.valor;
    si nodo.izq está {
        suma = suma + esperar recorrer_arbol(nodo.izq);
    }
    retornar suma;
}
```

**Override explícito (raro):**
```falcato
#[stackful]       // forzar stack (debugging, stack traces completos)
fut función mi_tarea() { ... }

#[stackless]      // forzar state machine (error si hay recursión)
fut función mi_tarea() { ... }
```

**Para CPU-bound (inferencia, kernels):** el async NO se toca.
```falcato
// Hot path: sync + puro + auto-vectorizable. Cero overhead async.
puro función forward(la entrada: &[Flotante32; 784]) -> [Flotante32; 128] { ... }

// I/O path: async stackless para servir requests
fut función servir(req: Request) -> Response {
    la modelo = esperar cargar_modelo("mnist.bin");  // stackless, ~128B
    retornar Response::nuevo(forward(req.datos, modelo.pesos));
}
```

### 2.4 `esperar`: expresión y sentencia

```falcato
// Como expresión:
la x = esperar operacion();

// Como sentencia (descarta resultado):
esperar dormir(1000);
```

### 2.5 Executor: single-thread default, multi-thread opt-in

```falcato
// Default: single-thread cooperativo (sin data races por construcción)
fut función principal() {
    lanzar tarea_a();  // mismo thread, scheduling cooperativo
    lanzar tarea_b();
    esperar tarea_a;
}

// Multi-thread: opt-in explícito
fut función principal() con_executor(hilos: 4) {
    lanzar tarea_a();  // puede correr en cualquier thread → requiere Send
    lanzar tarea_b();
}

// Aislamiento: región con executor propio
región crítico con_executor {
    // Executor independiente. Tareas aquí NO necesitan ser Send.
    // Si este executor muere, el principal sigue vivo.
    lanzar tarea_delicada();
}
```

**Reglas:**
1. `lanzar` fuera de región → executor del contexto (global por defecto)
2. `región X { }` sin annotation → usa el executor global (solo cancelación)
3. `región X con_executor { }` → crea executor propio (aislamiento real)
4. `con_executor(hilos: N)` → executor multi-thread (requiere Send en tareas)
5. Single-thread: tareas `!Send` permitidas, sin data races por construcción
6. Multi-thread: compiler verifica Send-ness (inferida desde efectos)

---

## 3. Sintaxis completa

### 3.1 Funciones async

```falcato
fut función descargar(la url: Texto) -> Resultado<Texto, Entero32> {
    la respuesta: Respuesta = esperar http::obtener(url)?;
    la cuerpo: Texto = esperar respuesta.leer_todo()?;
    retornar Resultado.Exito(cuerpo);
}
```

### 3.2 Await

```falcato
la dato: Entero32 = esperar operacion_lenta();
la dato: Entero32 = esperar operacion_fallible()?;
esperar dormir(1000);  // como sentencia
```

### 3.3 Spawn

```falcato
la tarea: Tarea<Entero32> = lanzar calcular(42);

región servidor {
    lanzar manejar_conexion(conn1);
    lanzar manejar_conexion(conn2);
    // Al salir: todas las tareas se cancelan
}
```

### 3.4 Select

```falcato
seleccionar {
    la dato = esperar canal_a.recibir() => {
        procesar(dato);
    }
    la _ = esperar temporizador::dormir(5000) => {
        imprimir_linea("timeout");
    }
}
```

### 3.5 Canales

```falcato
el (tx, rx): (Enviador<Entero32>, Receptor<Entero32>) = canal::nuevo::<Entero32>();
esperar tx.enviar(42);
la valor: Entero32 = esperar rx.recibir();
```

### 3.6 Join

```falcato
la (a, b, c) = esperar todos(tarea1, tarea2, tarea3);
la resultado: Entero32 = esperar tarea;
```

### 3.7 Blocking desde sync

```falcato
función sync_main() -> Entero32 {
    la dato = bloquear(descargar("http://..."));
    retornar 0;
}
```

---

## 4. Arquitectura del runtime

### 4.1 Componentes

```
┌──────────────────────────────────────────────────────────┐
│                      Executor                             │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │  Run Queue (cola de tareas listas)                 │  │
│  └────────────────────────┬───────────────────────────┘  │
│                           │                              │
│  ┌────────────────────────▼───────────────────────────┐  │
│  │  Scheduler                                         │  │
│  │  - Single-thread: round-robin cooperativo          │  │
│  │  - Multi-thread: work-stealing entre threads       │  │
│  └────────────────────────┬───────────────────────────┘  │
│                           │                              │
│  ┌────────────────────────▼───────────────────────────┐  │
│  │  Reactor (I/O multiplexing)                        │  │
│  │  - Windows: IOCP                                   │  │
│  │  - Linux: epoll / io_uring (futuro)                │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │  Timer Wheel (temporizadores eficientes)           │  │
│  └────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────┘
```

### 4.2 Tarea (representación interna)

```falcato
estructural TareaInterna {
    id: Entero64,
    estado: EstadoTarea,
    // Stackless: futuro como struct en heap
    futuro_ptr: *mut Entero8,    // puntero a la state machine
    futuro_size: Entero64,
    // Stackful: stack propio
    stack_ptr: *mut Entero8,     // base del stack (si stackful)
    stack_limit: *mut Entero8,   // límite actual (para growth check)
    stack_cap: Entero64,         // capacidad actual
    // Metadata
    waker: Waker,
    region_id: Entero64,         // 0 = global
    es_stackful: Booleano,
}

enumeración EstadoTarea {
    Lista,
    Suspendida(waker_id: Entero64),
    Completada,
    Cancelada,
}
```

### 4.3 Waker

```falcato
estructural Waker {
    tarea_id: Entero64,
    executor_ptr: *mut Executor,
}
// waker.despertar() → encola la tarea en run_queue
```

### 4.4 Stack dinámico (para tareas stackful)

```
// Crecimiento estilo Go (contiguous stack):
// 1. Stack de 8KB se llena
// 2. Runtime alloc 16KB
// 3. memcpy(8KB viejo → 16KB nuevo)
// 4. Actualizar punteros internos (offset-based)
// 5. Liberar bloque viejo
//
// Encogimiento: en GC/idle, si stack usa <25% de cap → shrink
```

### 4.5 Reactor (I/O)

**Windows (IOCP) — MVP:**
- `CreateIoCompletionPort` para sockets y archivos
- Overlapped I/O para operaciones async
- `GetQueuedCompletionStatus` en el loop del reactor

**Linux (epoll/io_uring) — futuro:**
- `epoll_create1` + `epoll_ctl` + `epoll_wait`
- `io_uring` para zero-copy (fase posterior)

---

## 5. Semántica y type system

### 5.1 Tipo Futuro<T>

```falcato
// Todo `fut función` retorna implícitamente Futuro<T>
fut función calcular(x: Entero32) -> Entero32 { ... }
// Tipo real: calcular : (Entero32) -> Futuro<Entero32>

rasgo Futuro {
    tipo Salida;
    // poll(self: *mut Self, waker: &Waker) -> EstadoFuturo<Salida>
}

enumeración EstadoFuturo<T> {
    Listo(valor: T),
    Pendiente,
}
```

### 5.2 Interacción con Resultado

```falcato
fut función leer_archivo(la ruta: Palabra) -> Resultado<Texto, IoError> {
    la fd = esperar io::abrir(ruta)?;
    la datos = esperar io::leer_todo(fd)?;
    retornar Resultado.Exito(datos);
}
```

### 5.3 Interacción con ownership

```falcato
// Futuro captura argumentos por valor (move)
fut función procesar(el dato: Vector<Entero32>) -> Entero64 {
    // `dato` se MOVIÓ al futuro — caller pierde ownership
    la suma: Entero64 = 0;
    para x en dato { suma = suma + x; }
    retornar suma;
}

// Para compartir: referencia explícita
fut función leer_compartido(las datos: &Vector<Entero32>) -> Entero32 {
    // `las` = shared borrowed — el futuro NO es dueño
    retornar datos.longitud();
}
```

### 5.4 Send-ness inferida

```falcato
// Compiler INFIERE send-ness desde efectos:
fut función tarea_segura() -> Entero32 {
    // Solo datos locales + funciones puras → Send automático
    retornar 42;
}

fut función tarea_peligrosa(el fd: Entero32) -> Entero32 {
    // Lee de fd (estado compartido) → NO Send
    retornar esperar io::leer(fd);
}

// En executor multi-thread:
fut función principal() con_executor(hilos: 4) {
    lanzar tarea_segura();      // OK: Send inferido
    lanzar tarea_peligrosa(3);  // ERROR [T083]: no es Send
    inseguro lanzar tarea_peligrosa(3);  // override explícito
}

// En executor single-thread: ambas OK (no migran entre threads)
```

### 5.5 Cancelación estructurada

```falcato
región servidor {
    el listener = esperar TcpListener::vincular("0.0.0.0:8080")?;
    bucle {
        la (conn, addr) = esperar listener.aceptar()?;
        lanzar manejar_conexion(conn);  // hija de la región
    }
    // Al salir: cancela todas las tareas hijas, espera graceful, libera memoria
}

// Cancelación explícita:
la tarea = lanzar trabajo_largo();
tarea.cancelar();
```

---

## 6. Codegen con Cranelift

### 6.1 Stackless (state machine) — default

```falcato
// Fuente:
fut función foo(x: Entero32) -> Entero32 {
    la a = x + 1;
    la b = esperar bar(a);
    retornar b + 2;
}

// Codegen genera:
// 1. Struct FooFuturo { estado: i32, x: i32, a: i32, b: i32 }
// 2. Función foo_poll(self: *mut FooFuturo, waker: *mut Waker) -> i32:
//      match self.estado:
//        0 → { self.a = self.x + 1; self.estado = 1; }
//        1 → { match bar_poll(&self.bar_futuro, waker):
//                Listo(v) → { self.b = v; self.estado = 2; }
//                Pendiente → return PENDIENTE; }
//        2 → { return LISTO(self.b + 2); }
// 3. Función foo(x: i32) -> *mut FooFuturo:
//      alloc FooFuturo { estado: 0, x, a: 0, b: 0 }
```

### 6.2 Stackful (stack dinámico) — recursión

```falcato
// Fuente:
fut función factorial(n: Entero64) -> Entero64 {
    si n <= 1 { retornar 1; }
    retornar n * esperar factorial(n - 1);
}

// Codegen genera:
// 1. Función factorial(n: i64, ctx: *mut TareaContexto) -> i64
//    con stack overflow check en prólogo:
//      cmp rsp, [ctx.stack_limit]
//      jb  .crecer
// 2. En cada `esperar`: guarda contexto, retorna Pendiente
// 3. Al reanudar: restaura contexto, continúa
```

### 6.3 `esperar` — desugaring

```falcato
// Superficie:
la y = esperar bar(x);

// Desugaring (stackless):
// self.bar_futuro = bar(x);
// self.estado = N;
// return PENDIENTE;  // primera vez
// // al reanudar:
// la y = match poll(&mut self.bar_futuro, waker) {
//     Listo(v) => v,
//     Pendiente => return PENDIENTE,
// };

// Desugaring (stackful):
// let __fut = bar(x, ctx);
// loop {
//     match __fut.poll(waker) {
//         Listo(v) => { y = v; break; }
//         Pendiente => { suspender_tarea(ctx); }
//     }
// }
```

### 6.4 `lanzar` — codegen

```falcato
// Superficie:
la tarea = lanzar foo(42);

// Codegen:
// 1. Crear Tarea { futuro: alloc(FooFuturo::new(42)), estado: Lista }
// 2. Encolar en run_queue del executor actual
// 3. Retornar handle Tarea<Entero32>
```

### 6.5 `seleccionar` — codegen

```falcato
// Superficie:
seleccionar {
    la a = esperar fut_a => { manejar(a); }
    la b = esperar fut_b => { manejar(b); }
}

// Codegen:
// 1. Poll fut_a → si Listo, ejecutar brazo a, retornar
// 2. Poll fut_b → si Listo, ejecutar brazo b, retornar
// 3. Si ambos Pendiente: registrar waker combinado, suspender
// 4. Al despertar: re-poll ambos (spurious wakeup safe)
```

---

## 7. Sub-fases de implementación

### 18A — MVP: sintaxis + executor single-thread + stackless básico

**Objetivo:** `fut función` + `esperar` + `lanzar` + `dormir()` funcionan end-to-end.

**Entregables:**
- [ ] Lexer: keywords `fut`, `esperar`, `lanzar`, `bloquear`, `seleccionar`
- [ ] AST: `ModoVerbal::Futuro`, `Expresion::Esperar`, `Expresion::Lanzar`,
      `Expresion::Bloquear`, `Expresion::Seleccionar`
- [ ] Parser: `fut función`, `esperar expr`, `lanzar expr`, `bloquear(expr)`
- [ ] Semántica:
  - `esperar` solo dentro de `fut función` → [T080]
  - `lanzar` requiere `Futuro<T>` → [T081]
  - Tipo de retorno de `fut función` → [T082]
  - Send-ness en multi-thread → [T083]
- [ ] Codegen stackless: state machine para futuros simples
- [ ] Runtime mínimo (C o Rust como lib estática):
  - Executor single-thread con run queue
  - `dormir(ms)` como primer futuro real (timer)
  - `bloquear()` como bridge sync→async
- [ ] Ejemplo: `async_simple.fc`

**Criterio de éxito:**
```falcato
fut función contador(la nombre: Palabra, veces: Entero32) {
    para i en 0..veces {
        imprimir_linea("{nombre}: {i}");
        esperar dormir(200);
    }
}

fut función principal() -> Entero32 {
    lanzar contador("A", 5);
    lanzar contador("B", 5);
    esperar dormir(1500);
    retornar 0;
}
```
→ Imprime "A: 0", "B: 0", "A: 1", "B: 1"... alternando.

### 18B — I/O no bloqueante + stack dinámico

**Objetivo:** TcpStream, Timer reales. Stackful para recursión.

**Entregables:**
- [ ] Runtime: IOCP reactor (Windows)
- [ ] Tipo `TcpListener`, `TcpStream` con métodos async
- [ ] `Temporizador` con `dormir(ms)` real (IOCP timer, no busy-wait)
- [ ] Stack dinámico: 8KB inicial, crece ×2, encoge en idle
- [ ] Detección automática stackless/stackful en codegen
- [ ] `#[stackful]` / `#[stackless]` como override
- [ ] Ejemplo: `echo_server.fc`

**Dependencia:** 18A completo.

### 18C — Scheduling + concurrencia

**Objetivo:** Canales, select, cancelación, multi-thread.

**Entregables:**
- [ ] `canal<T>` (mpsc): `canal::nuevo()`, `tx.enviar()`, `rx.recibir()`
- [ ] `seleccionar { }` — select sobre múltiples futuros
- [ ] `todos(f1, f2, ...)` — join N futuros
- [ ] `ceder()` — yield explícito
- [ ] Cancelación: `tarea.cancelar()`, `región` con cancelación
- [ ] `con_executor(hilos: N)` — multi-thread work-stealing
- [ ] `región X con_executor { }` — executor aislado
- [ ] Send-ness inferida desde efectos
- [ ] Ejemplo: `productor_consumidor.fc`

**Dependencia:** 18B completo.

### 18D — Optimización

**Objetivo:** Escalar a millones de tareas.

**Entregables:**
- [ ] Stackless optimizado: arena allocation en `región`
- [ ] Work-stealing scheduler refinado
- [ ] Stack pooling (reutilizar stacks de tareas completadas)
- [ ] Timer wheel eficiente (O(1) insert/cancel)
- [ ] `lanzar_bloqueante()` — para CPU work en thread pool separado
- [ ] Benchmarks: 100K+ tareas concurrentes
- [ ] Linux: epoll reactor

**Dependencia:** 18C completo.

---

## 8. Interacción con features existentes

### 8.1 `&yo` resuelve Pin

```falcato
// Futuro self-referential (state machine con self-ref)
estructural MiFuturo {
    estado: Entero32,
    dato: &yo Texto,  // OK en Falcato, requiere Pin<Box<>> en Rust
}
```

### 8.2 `región` como arena de tareas

```falcato
región conexión {
    el buffer: Vector<Entero8> = vector_nuevo();
    lanzar procesar(buffer);
}
// buffer Y la tarea se liberan aquí — sin leak posible
```

### 8.3 Efectos en async

```falcato
// puro → ejecutable en cualquier thread, paralelizable
puro fut función calcular_pi(iteraciones: Entero64) -> Flotante64 { ... }

// muta → NO paralelizable, requiere acceso exclusivo
muta(contador) fut función incrementar(el contador: &mut Entero64) { ... }
```

### 8.4 Subjuntivo en async

```falcato
fut función operación() -> Resultado<Entero32, Error> {
    si conexión fuese cerrada {
        // Cold path (subjuntivo) — codegen ya optimiza como rama fría
        retornar Resultado.Error(Error::ConexionPerdida);
    }
    retornar Resultado.Exito(42);
}
```

---

## 9. Comparación con otros lenguajes

| Feature | Rust | Go | Erlang | Falcato |
|---------|------|-----|--------|---------|
| Modelo | Stackless | Stackful (2KB+grow) | Stackful (2KB+grow) | **Ambos** (compiler decide) |
| Pin/Unpin | Manual | N/A | N/A | `&yo` lo resuelve |
| Cancelación | Manual | context.Context | kill/monitor | **Estructurada** (`región`) |
| Send-ness | Annotation | N/A (GC) | N/A (message passing) | **Inferida** desde efectos |
| Colored functions | Sí (infecta) | No | No | **Híbrido** (`bloquear()`) |
| Stack inicial | 0 (state machine) | 2 KB | ~2 KB | 0 (stackless) / 8KB (stackful) |
| Recursión async | Box::pin manual | Automática | Automática | **Automática** (stackful) |
| Select | tokio::select! | select{} | receive{} | `seleccionar { }` |
| Executor | Library (Tokio) | Runtime integrado | BEAM integrado | **Integrado** (single default) |
| CPU-bound escape | spawn_blocking | GOMAXPROCS | dirty scheduler | `lanzar_bloqueante()` |

---

## 10. Códigos de error

| Código | Mensaje | Contexto |
|--------|---------|----------|
| [T080] | `esperar` solo puede usarse dentro de `fut función` | `esperar` en función sync |
| [T081] | `lanzar` requiere una expresión de tipo `Futuro<T>` | `lanzar 42` |
| [T082] | `fut función` requiere tipo de retorno compatible con `Futuro<T>` | Retorno incorrecto |
| [T083] | Tarea no es `Send`: no puede lanzarse en executor multi-thread | `lanzar` en `con_executor(hilos: N)` |
| [T084] | `bloquear()` dentro de `fut función` causaría deadlock | `bloquear` en async |
| [T085] | `seleccionar` requiere al menos un brazo con `esperar` | `seleccionar { }` vacío |
| [O010] | Futuro captura referencia que no vive suficiente | Lifetime en async |

---

## 11. Ejemplos objetivo

### 18A: async_simple.fc
```falcato
fut función contador(la nombre: Palabra, veces: Entero32) {
    para i en 0..veces {
        imprimir_linea("{nombre}: {i}");
        esperar dormir(200);
    }
}

fut función principal() -> Entero32 {
    lanzar contador("A", 5);
    lanzar contador("B", 5);
    esperar dormir(1500);
    retornar 0;
}
```

### 18B: echo_server.fc
```falcato
fut función manejar_cliente(el conn: TcpStream) {
    el buffer: [Entero8; 1024] = todos 0;
    bucle {
        la n = esperar conn.leer(&mut buffer)?;
        si n es 0 { romper; }
        esperar conn.escribir(&buffer[0..n])?;
    }
}

fut función principal() -> Resultado<Entero32, Entero32> {
    el listener = esperar TcpListener::vincular("127.0.0.1:9000")?;
    imprimir_linea("Escuchando en :9000");
    bucle {
        la (conn, addr) = esperar listener.aceptar()?;
        imprimir_linea("Conexión de {addr}");
        lanzar manejar_cliente(conn);
    }
}
```

### 18C: productor_consumidor.fc
```falcato
fut función productor(el tx: Enviador<Entero32>) {
    para i en 0..10 {
        esperar tx.enviar(i * 10);
        esperar dormir(100);
    }
}

fut función consumidor(el rx: Receptor<Entero32>) {
    bucle {
        seleccionar {
            la valor = esperar rx.recibir() => {
                imprimir_linea("Recibido: {valor}");
            }
            la _ = esperar dormir(5000) => {
                imprimir_linea("Timeout — saliendo");
                romper;
            }
        }
    }
}

fut función principal() {
    el (tx, rx) = canal::nuevo::<Entero32>();
    lanzar productor(tx);
    esperar consumidor(rx);
}
```

### 18D: high_concurrency.fc
```falcato
fut función worker(id: Entero32, el rx: Receptor<Trabajo>) {
    bucle {
        la trabajo = esperar rx.recibir();
        la resultado = procesar(trabajo);  // CPU-bound, sync
        esperar dormir(1);  // yield
    }
}

fut función principal() con_executor(hilos: 4) {
    el (tx, rx) = canal::nuevo::<Trabajo>();
    para i en 0..1000 {
        lanzar worker(i, rx.clonar());
    }
    para trabajo en trabajos {
        esperar tx.enviar(trabajo);
    }
}
```

---

## 12. Dependencias del compilador

```toml
# Runtime como librería estática (Rust compilado a .lib)
# Se linkea automáticamente cuando el código usa `fut función`
[dependencies]
# Runtime interno — no expuesto al usuario
# Opción A: runtime en Rust → libfalcato_rt.a
# Opción B: runtime en C → más portable
# Decisión: Rust (ya tenemos toolchain, seguridad de memoria)
```

El runtime se distribuye como `libfalcato_rt.a`. El linker lo incluye
automáticamente cuando detecta `fut función` en el código.

---

## 13. Timeline estimado

| Sub-fase | Esfuerzo | Entregable clave |
|----------|----------|------------------|
| 18A | 3-5 sesiones | `async_simple.fc` funciona |
| 18B | 2-3 sesiones | `echo_server.fc` funciona |
| 18C | 3-4 sesiones | `productor_consumidor.fc` funciona |
| 18D | 5+ sesiones | 100K tareas concurrentes |

**Total: 13-17 sesiones de trabajo enfocado.**

---

*"El futuro no se espera — se construye."*
