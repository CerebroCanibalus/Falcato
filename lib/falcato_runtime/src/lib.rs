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
