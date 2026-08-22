//! # Falcato Runtime Library (Capa A)
//!
//! Librería estática que se linkea al binario generado por Falcato.
//! Contiene operaciones multi-paso que no se pueden expresar como
//! simples llamadas C-ABI (canales, executor, threads).
//!
//! Cero dependencias externas — usa raw extern declarations.
//!
//! Cada plataforma tiene su implementación en modules separados.
//! La selección es vía `#[cfg(target_os = "...")]`.

mod platform;

mod canal;
mod executor;
mod threading;
mod proceso;
mod terminal;
mod entrada;
mod tiempo;
mod dht;
mod argumentos;
mod convertir;
mod tcp_cliente;
mod texto_dinamico;
mod conversion_numerica;
mod archivo_avanzado;
mod tls;
mod panic_handler;
mod memoria_debug;
mod perfil;

use std::ffi::c_void;

// ============================================================
// Channel API — canales productor-consumidor
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn falcato_channel_new(capacity: i32, elem_size: i32) -> *mut c_void {
    canal::falcato_channel_new(capacity, elem_size)
}

#[no_mangle]
pub unsafe extern "C" fn falcato_channel_send(ch: *mut c_void, data: *const c_void) -> i32 {
    canal::falcato_channel_send(ch, data)
}

#[no_mangle]
pub unsafe extern "C" fn falcato_channel_recv(ch: *mut c_void, data: *mut c_void) -> i32 {
    canal::falcato_channel_recv(ch, data)
}

#[no_mangle]
pub unsafe extern "C" fn falcato_channel_try_recv(ch: *mut c_void, data: *mut c_void) -> i32 {
    canal::falcato_channel_try_recv(ch, data)
}

#[no_mangle]
pub unsafe extern "C" fn falcato_channel_close(ch: *mut c_void) {
    canal::falcato_channel_close(ch)
}

// ============================================================
// Executor API — thread pool
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn falcato_executor_new(num_threads: i32, queue_capacity: i32) -> *mut c_void {
    executor::falcato_executor_new(num_threads, queue_capacity)
}

#[no_mangle]
pub unsafe extern "C" fn falcato_executor_submit(
    exec: *mut c_void,
    task_fn: unsafe extern "C" fn(*mut c_void) -> i32,
    arg: *mut c_void,
) -> i32 {
    executor::falcato_executor_submit(exec, task_fn, arg)
}

#[no_mangle]
pub unsafe extern "C" fn falcato_executor_cancel(exec: *mut c_void) {
    executor::falcato_executor_cancel(exec)
}

#[no_mangle]
pub unsafe extern "C" fn falcato_executor_close(exec: *mut c_void) {
    executor::falcato_executor_close(exec)
}

// ============================================================
// Thread API — creación directa de threads (fallback sin executor)
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn falcato_thread_run(
    thread_fn: unsafe extern "C" fn(*mut c_void) -> i32,
    arg: *mut c_void,
) -> *mut c_void {
    threading::thread_run(thread_fn, arg)
}

#[no_mangle]
pub unsafe extern "C" fn falcato_thread_join(handle: *mut c_void) -> i32 {
    threading::thread_join(handle)
}

// ============================================================
// Proceso API — creación de procesos con captura de salida
// ============================================================

/// Lanza un proceso con el comando dado (vía shell del sistema), capturando
/// stdout+stderr en un pipe. Devuelve un Handle opaco o NULL si falla.
#[no_mangle]
pub unsafe extern "C" fn falcato_proceso_crear(comando: *const i8) -> *mut c_void {
    proceso::proceso_crear(comando as *const std::ffi::c_char)
}

/// Espera a que el proceso termine. Devuelve el exit code del proceso.
#[no_mangle]
pub unsafe extern "C" fn falcato_proceso_esperar(handle: *mut c_void) -> i32 {
    proceso::proceso_esperar(handle)
}

/// Devuelve un puntero a la salida capturada (malloc'ed, con null terminator).
/// El caller debe liberarlo con `free`.
#[no_mangle]
pub unsafe extern "C" fn falcato_proceso_leer_salida(handle: *mut c_void) -> *mut i8 {
    proceso::proceso_leer_salida(handle) as *mut i8
}

/// Libera el handle del proceso (después de proceso_esperar/proceso_leer_salida).
#[no_mangle]
pub unsafe extern "C" fn falcato_proceso_cerrar(handle: *mut c_void) {
    proceso::proceso_cerrar(handle);
}

// ============================================================
// Proceso API bidireccional — pipes para diálogo en vivo (MCP servers)
// ============================================================

/// Lanza un proceso con stdin/stdout/stderr pipes separados para diálogo en vivo.
/// Devuelve un Handle opaco o NULL si falla.
#[no_mangle]
pub unsafe extern "C" fn falcato_proceso_crear_con_pipes(comando: *const i8) -> *mut c_void {
    proceso::proceso_crear_con_pipes(comando as *const std::ffi::c_char)
}

/// Escribe datos a stdin del proceso. Devuelve bytes escritos o -1 si error.
#[no_mangle]
pub unsafe extern "C" fn falcato_proceso_escribir(handle: *mut c_void, datos: *const u8, n: u32) -> i32 {
    proceso::proceso_escribir(handle, datos, n)
}

/// Lee stdout del proceso chunk por chunk (no bloquea hasta EOF). Devuelve bytes leídos o 0 si EOF.
#[no_mangle]
pub unsafe extern "C" fn falcato_proceso_leer_salida_chunk(handle: *mut c_void, buf: *mut u8, n: u32) -> i32 {
    proceso::proceso_leer_salida_chunk(handle, buf, n)
}

/// Lee stderr del proceso chunk por chunk. Devuelve bytes leídos o 0 si EOF.
#[no_mangle]
pub unsafe extern "C" fn falcato_proceso_leer_error_chunk(handle: *mut c_void, buf: *mut u8, n: u32) -> i32 {
    proceso::proceso_leer_error_chunk(handle, buf, n)
}

/// Cierra stdin del proceso (envía EOF al hijo).
#[no_mangle]
pub unsafe extern "C" fn falcato_proceso_cerrar_entrada(handle: *mut c_void) {
    proceso::proceso_cerrar_entrada(handle)
}

/// Verifica si hay datos disponibles en stdout sin bloquear. Devuelve 1 si hay datos, 0 si no.
/// El parámetro `ms` es el timeout para esperar (0 = no bloquear).
#[no_mangle]
pub unsafe extern "C" fn falcato_proceso_listo_para_leer(handle: *mut c_void, ms: u32) -> i32 {
    proceso::proceso_listo_para_leer(handle, ms)
}

/// Libera el handle del proceso bidireccional (cierra todos los pipes).
#[no_mangle]
pub unsafe extern "C" fn falcato_proceso_cerrar_bidireccional(handle: *mut c_void) {
    proceso::proceso_cerrar_bidireccional(handle)
}

// ============================================================
// Terminal API — modo raw y lectura de teclas (TUI)
// ============================================================

/// Activa (1) o desactiva (0) el modo raw de terminal.
/// En Windows también activa ENABLE_VIRTUAL_TERMINAL_PROCESSING (ANSI).
/// Devuelve 1 si OK, 0 si error.
#[no_mangle]
pub unsafe extern "C" fn falcato_terminal_modo_raw(activo: i32) -> i32 {
    terminal::terminal_modo_raw(activo)
}

/// Lee una tecla bloqueante. Devuelve el código de tecla (ver terminal.rs).
#[no_mangle]
pub unsafe extern "C" fn falcato_terminal_leer_tecla() -> i32 {
    terminal::terminal_leer_tecla()
}

// ============================================================
// Entrada estándar (stdin) — R7.3
// ============================================================

/// Lee TODO stdin hasta EOF. Devuelve buffer malloc'ed con null terminator
/// (caller libera con free) o NULL en error.
#[no_mangle]
pub unsafe extern "C" fn falcato_entrada_leer() -> *mut i8 {
    entrada::entrada_leer() as *mut i8
}

// ============================================================
// Tiempo (reloj de pared) — R7.4
// ============================================================

/// Segundos desde Unix epoch (1970-01-01 UTC).
#[no_mangle]
pub unsafe extern "C" fn falcato_fecha_unix() -> i64 {
    tiempo::fecha_unix()
}

/// Milisegundos desde Unix epoch.
#[no_mangle]
pub unsafe extern "C" fn falcato_fecha_ms() -> i64 {
    tiempo::fecha_ms()
}

// ============================================================
// Argumentos de línea de comandos (argv) — R7.5
// ============================================================

/// Devuelve un descriptor `Vector<Texto>` de Falcato construido en heap:
/// `{ptr: i64, len: i64, cap: i64}` donde `ptr` apunta a un array de
/// descriptores `Texto` y cada uno a un string malloc'ed. El caller usa
/// `vector_liberar`/`texto_liberar` de Falcato para liberar.
#[no_mangle]
pub unsafe extern "C" fn falcato_argumentos() -> *mut c_void {
    argumentos::argumentos()
}

// ============================================================
// DHT distribuido (R8.2) — índice P2P de paquetes
// ============================================================

/// Crea un nodo DHT (puerto 0 = efímero). Devuelve Handle o NULL.
#[no_mangle]
pub unsafe extern "C" fn falcato_dht_nuevo(puerto: u16) -> *mut c_void {
    dht::dht_nuevo(puerto)
}

/// Publica clave→valor firmado (ed25519). Devuelve 1 si OK, 0 si error.
#[no_mangle]
pub unsafe extern "C" fn falcato_dht_publicar(
    handle: *mut c_void,
    clave: *const u8,
    clave_len: usize,
    valor: *const u8,
    valor_len: usize,
) -> i32 {
    dht::dht_publicar(handle, clave, clave_len, valor, valor_len)
}

/// Consulta la clave. Devuelve buffer heap (caller libera con free) o NULL.
#[no_mangle]
pub unsafe extern "C" fn falcato_dht_consultar(
    handle: *mut c_void,
    clave: *const u8,
    clave_len: usize,
) -> *mut u8 {
    dht::dht_consultar(handle, clave, clave_len)
}

/// Conecta el nodo a un peer conocido (bootstrap). Devuelve 1 si OK.
#[no_mangle]
pub unsafe extern "C" fn falcato_dht_bootstrap(
    handle: *mut c_void,
    direccion: *const i8,
    puerto: u16,
) -> i32 {
    dht::dht_bootstrap(handle, direccion, puerto)
}

/// Libera el nodo DHT.
#[no_mangle]
pub unsafe extern "C" fn falcato_dht_cerrar(handle: *mut c_void) {
    dht::dht_cerrar(handle);
}

// ============================================================
// TCP Cliente + DNS — conexión a servidores y resolución de nombres
// ============================================================

/// Conecta a host:puerto. host puede ser IP ("127.0.0.1") o nombre ("example.com").
/// Devuelve socket handle o 0 si falla.
#[no_mangle]
pub unsafe extern "C" fn falcato_tcp_conectar(host: *const i8, puerto: i32) -> i64 {
    #[cfg(target_os = "windows")]
    {
        tcp_cliente::tcp_conectar(host as *const std::ffi::c_char, puerto) as i64
    }
    #[cfg(not(target_os = "windows"))]
    {
        tcp_cliente::tcp_conectar(host as *const std::ffi::c_char, puerto)
    }
}

/// Resuelve nombre de host a IP (string). Devuelve buffer malloc'ed (caller libera) o NULL.
#[no_mangle]
pub unsafe extern "C" fn falcato_dns_resolver(host: *const i8) -> *mut i8 {
    tcp_cliente::dns_resolver(host as *const std::ffi::c_char) as *mut i8
}

/// Establece timeout de lectura/escritura en milisegundos.
#[no_mangle]
pub unsafe extern "C" fn falcato_tcp_establecer_timeout(sock: i64, ms: i32) {
    #[cfg(target_os = "windows")]
    {
        tcp_cliente::tcp_establecer_timeout(sock as usize, ms)
    }
    #[cfg(not(target_os = "windows"))]
    {
        tcp_cliente::tcp_establecer_timeout(sock, ms)
    }
}

/// Verifica si hay datos disponibles para leer sin bloquear. Devuelve 1 si hay datos, 0 si no.
#[no_mangle]
pub unsafe extern "C" fn falcato_tcp_datos_disponibles(sock: i64) -> i32 {
    #[cfg(target_os = "windows")]
    {
        tcp_cliente::tcp_datos_disponibles(sock as usize)
    }
    #[cfg(not(target_os = "windows"))]
    {
        tcp_cliente::tcp_datos_disponibles(sock)
    }
}


