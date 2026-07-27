//! # Executor — thread pool con ring buffer de tareas
//!
//! Layout del struct:
//! ```c
//! struct FalcatoExecutor {
//!     void* mutex;           // offset 0 — lock del pool
//!     void* sem_tasks;       // offset 8 — tareas pendientes
//!     void* cancel_event;    // offset 16 — señal de cancelación
//!     void** handles;        // offset 24 — array de handles de threads
//!     void* queue_buffer;    // offset 32 — tareas pendientes (ring buffer)
//!     i32   head;            // offset 40
//!     i32   tail;            // offset 44
//!     i32   count;           // offset 48
//!     i32   capacity;        // offset 52
//!     i32   shutdown;        // offset 56 — flag atómico (0=run, 1=shutdown)
//!     i32   active_tasks;    // offset 60
//!     i32   num_workers;     // offset 64
//!     i32   _pad;            // offset 68 — padding a 72
//! };
//! ```

use crate::platform;
use std::ffi::c_void;
use core::ptr;

const EXECUTOR_SIZE: usize = 72;

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn memset(dst: *mut c_void, val: i32, n: usize) -> *mut c_void;
}

/// Task entry en el ring buffer
#[repr(C)]
struct TaskEntry {
    func: unsafe extern "C" fn(*mut c_void) -> i32,
    arg: *mut c_void,
}

/// Loop interno del worker thread. Procesa tareas hasta shutdown.
pub unsafe extern "C" fn falcato_executor_worker_loop(param: *mut c_void) -> i32 {
    let exec = param as *mut FalcatoExecutor;

    loop {
        // Esperar tarea o shutdown
        platform::sem_wait((*exec).sem_tasks);

        // Check shutdown flag
        let n_shutdown: *mut i32 = &raw mut (*exec).shutdown;
        if *n_shutdown != 0 {
            break;
        }

        // Lock
        platform::mutex_lock((*exec).mutex);

        // Dequeue
        if (*exec).count <= 0 {
            platform::mutex_unlock((*exec).mutex);
            continue;
        }

        let entry_ptr = ((*exec).queue_buffer as *mut TaskEntry).add((*exec).tail as usize);
        let func = (*entry_ptr).func;
        let arg = (*entry_ptr).arg;
        (*exec).tail = ((*exec).tail + 1) % (*exec).capacity;
        (*exec).count -= 1;
        (*exec).active_tasks += 1;

        // Unlock antes de ejecutar (no hold lock durante trabajo)
        platform::mutex_unlock((*exec).mutex);

        // Ejecutar tarea
        func(arg);

        // Decrement active después de ejecutar
        platform::mutex_lock((*exec).mutex);
        (*exec).active_tasks -= 1;
        platform::mutex_unlock((*exec).mutex);
    }

    0
}

#[repr(C)]
struct FalcatoExecutor {
    mutex: *mut c_void,
    sem_tasks: *mut c_void,
    cancel_event: *mut c_void,
    handles: *mut *mut c_void,
    queue_buffer: *mut c_void,
    head: i32,
    tail: i32,
    count: i32,
    capacity: i32,
    shutdown: i32,
    active_tasks: i32,
    num_workers: i32,
}

pub unsafe fn falcato_executor_new(num_threads: i32, queue_capacity: i32) -> *mut c_void {
    if num_threads <= 0 || queue_capacity <= 0 {
        return ptr::null_mut();
    }

    let mem = malloc(EXECUTOR_SIZE) as *mut u8;
    if mem.is_null() {
        return ptr::null_mut();
    }
    memset(mem as *mut c_void, 0, EXECUTOR_SIZE);

    let exec = &mut *(mem as *mut FalcatoExecutor);

    // Inicializar sync primitives
    exec.mutex = platform::mutex_init();
    if exec.mutex.is_null() {
        free(mem as *mut c_void);
        return ptr::null_mut();
    }

    exec.sem_tasks = platform::sem_init(0, queue_capacity);
    if exec.sem_tasks.is_null() {
        platform::mutex_destroy(exec.mutex);
        free(mem as *mut c_void);
        return ptr::null_mut();
    }

    exec.cancel_event = platform::sem_init(0, 1);
    if exec.cancel_event.is_null() {
        platform::sem_destroy(exec.sem_tasks);
        platform::mutex_destroy(exec.mutex);
        free(mem as *mut c_void);
        return ptr::null_mut();
    }

    // Ring buffer de tareas
    let queue_bytes = (queue_capacity as usize) * core::mem::size_of::<TaskEntry>();
    exec.queue_buffer = malloc(queue_bytes);
    if exec.queue_buffer.is_null() {
        platform::sem_destroy(exec.cancel_event);
        platform::sem_destroy(exec.sem_tasks);
        platform::mutex_destroy(exec.mutex);
        free(mem as *mut c_void);
        return ptr::null_mut();
    }
    memset(exec.queue_buffer, 0, queue_bytes);

    exec.head = 0;
    exec.tail = 0;
    exec.count = 0;
    exec.capacity = queue_capacity;
    exec.shutdown = 0;
    exec.active_tasks = 0;
    exec.num_workers = num_threads;

    // Crear threads workers
    let handles_size = (num_threads as usize) * core::mem::size_of::<*mut c_void>();
    exec.handles = malloc(handles_size) as *mut *mut c_void;
    if exec.handles.is_null() {
        free(exec.queue_buffer);
        platform::sem_destroy(exec.cancel_event);
        platform::sem_destroy(exec.sem_tasks);
        platform::mutex_destroy(exec.mutex);
        free(mem as *mut c_void);
        return ptr::null_mut();
    }

    for i in 0..num_threads as usize {
        let h = crate::threading::thread_run(falcato_executor_worker_loop, mem as *mut c_void);
        *exec.handles.add(i) = h;
    }

    mem as *mut c_void
}

pub unsafe fn falcato_executor_submit(
    exec: *mut c_void,
    task_fn: unsafe extern "C" fn(*mut c_void) -> i32,
    arg: *mut c_void,
) -> i32 {
    if exec.is_null() {
        return -1;
    }
    let exec = &mut *(exec as *mut FalcatoExecutor);

    // Lock mutex
    platform::mutex_lock(exec.mutex);

    // Check shutdown
    if exec.shutdown != 0 {
        platform::mutex_unlock(exec.mutex);
        return -1;
    }

    // Check full
    if exec.count >= exec.capacity {
        platform::mutex_unlock(exec.mutex);
        return -1;
    }

    // Enqueue
    let entry = &mut *(exec.queue_buffer as *mut TaskEntry).add(exec.head as usize);
    entry.func = task_fn;
    entry.arg = arg;
    exec.head = (exec.head + 1) % exec.capacity;
    exec.count += 1;

    platform::mutex_unlock(exec.mutex);

    // Signal task available
    platform::sem_post(exec.sem_tasks);
    0
}

pub unsafe fn falcato_executor_cancel(exec: *mut c_void) {
    if exec.is_null() {
        return;
    }
    let exec = &mut *(exec as *mut FalcatoExecutor);

    // Set shutdown flag
    exec.shutdown = 1;

    // Signal all waiting workers (one extra post per worker)
    for _ in 0..exec.num_workers {
        platform::sem_post(exec.sem_tasks);
    }
}

pub unsafe fn falcato_executor_close(exec: *mut c_void) {
    if exec.is_null() {
        return;
    }
    let exec = &mut *(exec as *mut FalcatoExecutor);

    // Cancel first (signal shutdown)
    exec.shutdown = 1;
    for _ in 0..exec.num_workers {
        platform::sem_post(exec.sem_tasks);
    }

    // Wait for all workers to finish
    for i in 0..exec.num_workers as usize {
        let h = *exec.handles.add(i);
        if !h.is_null() {
            crate::threading::thread_join(h);
        }
    }

    // Cleanup
    free(exec.handles as *mut c_void);
    free(exec.queue_buffer);
    platform::sem_destroy(exec.cancel_event);
    platform::sem_destroy(exec.sem_tasks);
    platform::mutex_destroy(exec.mutex);
    free(exec as *const _ as *mut c_void);
}
