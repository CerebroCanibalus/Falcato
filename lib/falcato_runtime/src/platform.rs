//! Platform abstraction — tipos comunes para mutex y semaphore.
//! La implementación difiere entre Win32 y POSIX.

#[cfg(target_os = "windows")]
#[allow(dead_code)]
mod imp {
    use std::ffi::c_void;

    extern "system" {
        fn CloseHandle(hObject: *mut c_void) -> i32;
    }

    #[allow(dead_code)]
    pub type Mutex = *mut c_void;
    #[allow(dead_code)]
    pub type Semaphore = *mut c_void;

    pub unsafe fn mutex_init() -> *mut c_void {
        extern "system" {
            fn CreateMutexW(
                lpMutexAttributes: *mut c_void,
                bInitialOwner: i32,
                lpName: *mut c_void,
            ) -> *mut c_void;
        }
        CreateMutexW(core::ptr::null_mut(), 0, core::ptr::null_mut())
    }

    pub unsafe fn sem_init(initial_count: i32, max_count: i32) -> *mut c_void {
        extern "system" {
            fn CreateSemaphoreW(
                lpSemaphoreAttributes: *mut c_void,
                lInitialCount: i32,
                lMaximumCount: i32,
                lpName: *mut c_void,
            ) -> *mut c_void;
        }
        CreateSemaphoreW(core::ptr::null_mut(), initial_count, max_count, core::ptr::null_mut())
    }

    pub unsafe fn mutex_lock(mtx: *mut c_void) -> i32 {
        extern "system" {
            fn WaitForSingleObject(hHandle: *mut c_void, dwMilliseconds: u32) -> i32;
        }
        WaitForSingleObject(mtx, u32::MAX) // INFINITE
    }

    pub unsafe fn mutex_unlock(mtx: *mut c_void) -> i32 {
        extern "system" {
            fn ReleaseMutex(hMutex: *mut c_void) -> i32;
        }
        ReleaseMutex(mtx)
    }

    pub unsafe fn sem_wait(sem: *mut c_void) -> i32 {
        extern "system" {
            fn WaitForSingleObject(hHandle: *mut c_void, dwMilliseconds: u32) -> i32;
        }
        WaitForSingleObject(sem, u32::MAX)
    }

    pub unsafe fn sem_trywait(sem: *mut c_void) -> i32 {
        extern "system" {
            fn WaitForSingleObject(hHandle: *mut c_void, dwMilliseconds: u32) -> i32;
        }
        WaitForSingleObject(sem, 0) // timeout=0 → no espera
    }

    pub unsafe fn sem_post(sem: *mut c_void) -> i32 {
        extern "system" {
            fn ReleaseSemaphore(
                hSemaphore: *mut c_void,
                lReleaseCount: i32,
                lpPreviousCount: *mut i32,
            ) -> i32;
        }
        ReleaseSemaphore(sem, 1, core::ptr::null_mut())
    }

    pub unsafe fn mutex_destroy(mtx: *mut c_void) {
        CloseHandle(mtx);
    }

    pub unsafe fn sem_destroy(sem: *mut c_void) {
        CloseHandle(sem);
    }
}

#[cfg(not(target_os = "windows"))]
mod imp {
    use std::ffi::c_void;

    extern "C" {
        fn pthread_mutex_init(mtx: *mut c_void, attr: *const c_void) -> i32;
        fn pthread_mutex_lock(mtx: *mut c_void) -> i32;
        fn pthread_mutex_unlock(mtx: *mut c_void) -> i32;
        fn pthread_mutex_destroy(mtx: *mut c_void) -> i32;
        // Nombres Rust internos sys_sem_* (sin colisión E0428 con los wrappers);
        // el símbolo C real es sem_* (link_name).
        #[link_name = "sem_init"]
        fn sys_sem_init(sem: *mut c_void, pshared: i32, value: u32) -> i32;
        #[link_name = "sem_wait"]
        fn sys_sem_wait(sem: *mut c_void) -> i32;
        #[link_name = "sem_trywait"]
        fn sys_sem_trywait(sem: *mut c_void) -> i32;
        #[link_name = "sem_post"]
        fn sys_sem_post(sem: *mut c_void) -> i32;
        #[link_name = "sem_destroy"]
        fn sys_sem_destroy(sem: *mut c_void) -> i32;
        fn malloc(size: usize) -> *mut c_void;
        fn free(ptr: *mut c_void);
    }

    // Tamaños de structs del sistema por plataforma
    // macOS: pthread_mutex_t=64, sem_t=8 (puntero opaco)
    // Linux glibc x86_64: pthread_mutex_t=40, sem_t=32
    #[cfg(target_os = "macos")]
    const PTHREAD_MUTEX_SIZE: usize = 64;
    #[cfg(target_os = "macos")]
    const SEM_T_SIZE: usize = 8;

    #[cfg(not(target_os = "macos"))]
    const PTHREAD_MUTEX_SIZE: usize = 40; // x86_64 Linux
    #[cfg(not(target_os = "macos"))]
    const SEM_T_SIZE: usize = 32;         // x86_64 Linux

    pub unsafe fn mutex_init() -> *mut c_void {
        let mtx = malloc(PTHREAD_MUTEX_SIZE);
        if !mtx.is_null() {
            pthread_mutex_init(mtx, core::ptr::null());
        }
        mtx
    }

    pub unsafe fn sem_init(initial_count: i32, _max_count: i32) -> *mut c_void {
        let sem = malloc(SEM_T_SIZE);
        if !sem.is_null() {
            sys_sem_init(sem, 0, initial_count as u32);
        }
        sem
    }

    pub unsafe fn mutex_lock(mtx: *mut c_void) -> i32 {
        pthread_mutex_lock(mtx)
    }

    pub unsafe fn mutex_unlock(mtx: *mut c_void) -> i32 {
        pthread_mutex_unlock(mtx)
    }

    pub unsafe fn sem_wait(sem: *mut c_void) -> i32 {
        sys_sem_wait(sem)
    }

    pub unsafe fn sem_trywait(sem: *mut c_void) -> i32 {
        sys_sem_trywait(sem)
    }

    pub unsafe fn sem_post(sem: *mut c_void) -> i32 {
        sys_sem_post(sem)
    }

    pub unsafe fn mutex_destroy(mtx: *mut c_void) {
        pthread_mutex_destroy(mtx);
        free(mtx);
    }

    pub unsafe fn sem_destroy(sem: *mut c_void) {
        sys_sem_destroy(sem);
        free(sem);
    }
}

pub use imp::*;
