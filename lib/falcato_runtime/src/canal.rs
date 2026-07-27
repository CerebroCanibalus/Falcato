//! # Canales — ring buffer con mutex + 2 semáforos
//!
//! Layout del struct (idéntico en todas las plataformas):
//! ```c
//! struct FalcatoChannel {
//!     void* mutex;        // offset 0 — HANDLE (Win) o *pthread_mutex_t (POSIX)
//!     void* sem_signal;   // offset 8 — semáforo de señal (datos disponibles)
//!     void* sem_space;    // offset 16 — semáforo de espacio (buffer vacío)
//!     i32   head;         // offset 24 — índice de escritura
//!     i32   tail;         // offset 28 — índice de lectura
//!     i32   capacity;     // offset 32 — capacidad máxima
//!     i32   elem_size;    // offset 36 — tamaño de cada elemento
//!     void* buffer;       // offset 40 — datos circulares
//! };
//! ```

use crate::platform;
use std::ffi::c_void;
use core::ptr;

const CHANNEL_HEADER_SIZE: usize = 48; // 6 × 8 bytes (ptrs) + 4 × 4 bytes (i32)

#[repr(C)]
struct FalcatoChannel {
    mutex: *mut c_void,
    sem_signal: *mut c_void,
    sem_space: *mut c_void,
    head: i32,
    tail: i32,
    capacity: i32,
    elem_size: i32,
    buffer: *mut u8,
}

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
}

pub unsafe fn falcato_channel_new(capacity: i32, elem_size: i32) -> *mut c_void {
    if capacity <= 0 || elem_size <= 0 {
        return ptr::null_mut();
    }

    // malloc canal struct + buffer
    let buf_size = (capacity as usize) * (elem_size as usize);
    let total = CHANNEL_HEADER_SIZE + buf_size;
    let mem = malloc(total) as *mut u8;
    if mem.is_null() {
        return ptr::null_mut();
    }

    let ch = &mut *(mem as *mut FalcatoChannel);

    // Inicializar primitivas de sync
    ch.mutex = platform::mutex_init();
    if ch.mutex.is_null() {
        free(mem as *mut c_void);
        return ptr::null_mut();
    }
    ch.sem_signal = platform::sem_init(0, capacity);
    if ch.sem_signal.is_null() {
        platform::mutex_destroy(ch.mutex);
        free(mem as *mut c_void);
        return ptr::null_mut();
    }
    ch.sem_space = platform::sem_init(capacity, capacity);
    if ch.sem_space.is_null() {
        platform::sem_destroy(ch.sem_signal);
        platform::mutex_destroy(ch.mutex);
        free(mem as *mut c_void);
        return ptr::null_mut();
    }

    ch.head = 0;
    ch.tail = 0;
    ch.capacity = capacity;
    ch.elem_size = elem_size;
    ch.buffer = mem.add(CHANNEL_HEADER_SIZE);

    mem as *mut c_void
}

pub unsafe fn falcato_channel_send(ch: *mut c_void, data: *const c_void) -> i32 {
    if ch.is_null() || data.is_null() {
        return -1;
    }
    let ch = &mut *(ch as *mut FalcatoChannel);

    // Esperar espacio disponible
    let wait = platform::sem_wait(ch.sem_space);
    if wait != 0 {
        return -1;
    }

    // Lock mutex
    platform::mutex_lock(ch.mutex);

    // Escribir en ring buffer
    let offset = (ch.head as usize) * (ch.elem_size as usize);
    let dst = ch.buffer.add(offset);
    memcpy(dst as *mut c_void, data, ch.elem_size as usize);
    ch.head = (ch.head + 1) % ch.capacity;

    // Unlock mutex
    platform::mutex_unlock(ch.mutex);

    // Señalar que hay datos
    platform::sem_post(ch.sem_signal);
    0
}

pub unsafe fn falcato_channel_recv(ch: *mut c_void, data: *mut c_void) -> i32 {
    if ch.is_null() || data.is_null() {
        return -1;
    }
    let ch = &mut *(ch as *mut FalcatoChannel);

    // Esperar datos disponibles
    let wait = platform::sem_wait(ch.sem_signal);
    if wait != 0 {
        return -1;
    }

    // Lock mutex
    platform::mutex_lock(ch.mutex);

    // Leer de ring buffer
    let offset = (ch.tail as usize) * (ch.elem_size as usize);
    let src = ch.buffer.add(offset);
    memcpy(data, src as *const c_void, ch.elem_size as usize);
    ch.tail = (ch.tail + 1) % ch.capacity;

    // Unlock mutex
    platform::mutex_unlock(ch.mutex);

    // Señalar que hay espacio
    platform::sem_post(ch.sem_space);
    0
}

pub unsafe fn falcato_channel_try_recv(ch: *mut c_void, data: *mut c_void) -> i32 {
    if ch.is_null() || data.is_null() {
        return -1;
    }
    let ch = &mut *(ch as *mut FalcatoChannel);

    // Non-blocking wait for signal (timeout=0)
    let wait = platform::sem_trywait(ch.sem_signal);
    if wait != 0 {
        // No data available
        return -1;
    }

    // Lock mutex
    platform::mutex_lock(ch.mutex);

    // Read from ring buffer
    let offset = (ch.tail as usize) * (ch.elem_size as usize);
    let src = ch.buffer.add(offset);
    memcpy(data, src as *const c_void, ch.elem_size as usize);
    ch.tail = (ch.tail + 1) % ch.capacity;

    // Unlock mutex
    platform::mutex_unlock(ch.mutex);

    // Signal that space is available
    platform::sem_post(ch.sem_space);
    0
}

pub unsafe fn falcato_channel_close(ch: *mut c_void) {
    if ch.is_null() {
        return;
    }
    let ch = &*(ch as *const FalcatoChannel);
    platform::mutex_destroy(ch.mutex);
    platform::sem_destroy(ch.sem_signal);
    platform::sem_destroy(ch.sem_space);
    free(ch as *const _ as *mut c_void);
}
