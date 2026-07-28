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
