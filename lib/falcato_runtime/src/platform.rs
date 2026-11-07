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
        #[allow(dead_code)]
        fn pthread_cond_init(cond: *mut c_void, attr: *const c_void) -> i32;
        #[allow(dead_code)]
        fn pthread_cond_destroy(cond: *mut c_void) -> i32;
        #[allow(dead_code)]
        fn pthread_cond_wait(cond: *mut c_void, mtx: *mut c_void) -> i32;
        #[allow(dead_code)]
        fn pthread_cond_signal(cond: *mut c_void) -> i32;
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

    // macOS: `sem_init(3)` devuelve ENOSYS (Darwin no soporta semáforos sin
    // nombre). Se implementa un semáforo contador con mutex + cond sobre un
    // bloque propio, manteniendo la misma API de puntero opaco.
    #[cfg(target_os = "macos")]
    #[repr(C)]
    struct SemDarwin {
        mtx: *mut c_void,   // pthread_mutex_t (64 bytes)
        cond: *mut c_void,  // pthread_cond_t (48 bytes)
        count: i32,
    }

    #[cfg(target_os = "macos")]
    const PTHREAD_COND_SIZE: usize = 48;

    pub unsafe fn sem_init(initial_count: i32, _max_count: i32) -> *mut c_void {
        #[cfg(target_os = "macos")]
        {
            let sem = malloc(core::mem::size_of::<SemDarwin>());
            if sem.is_null() {
                return sem;
            }
            let mtx = malloc(PTHREAD_MUTEX_SIZE);
            let cond = malloc(PTHREAD_COND_SIZE);
            if mtx.is_null() || cond.is_null() {
                free(mtx);
                free(cond);
                free(sem);
                return core::ptr::null_mut();
            }
            pthread_mutex_init(mtx, core::ptr::null());
            pthread_cond_init(cond, core::ptr::null());
            let s = &mut *(sem as *mut SemDarwin);
            s.mtx = mtx;
            s.cond = cond;
            s.count = initial_count;
            return sem;
        }
        #[cfg(not(target_os = "macos"))]
        {
            let sem = malloc(SEM_T_SIZE);
            if !sem.is_null() {
                sys_sem_init(sem, 0, initial_count as u32);
            }
            sem
        }
    }

    pub unsafe fn mutex_lock(mtx: *mut c_void) -> i32 {
        pthread_mutex_lock(mtx)
    }

    pub unsafe fn mutex_unlock(mtx: *mut c_void) -> i32 {
        pthread_mutex_unlock(mtx)
    }

    pub unsafe fn sem_wait(sem: *mut c_void) -> i32 {
        #[cfg(target_os = "macos")]
        {
            let s = &mut *(sem as *mut SemDarwin);
            pthread_mutex_lock(s.mtx);
            while s.count <= 0 {
                pthread_cond_wait(s.cond, s.mtx);
            }
            s.count -= 1;
            pthread_mutex_unlock(s.mtx);
            return 0;
        }
        #[cfg(not(target_os = "macos"))]
        sys_sem_wait(sem)
    }

    pub unsafe fn sem_trywait(sem: *mut c_void) -> i32 {
        #[cfg(target_os = "macos")]
        {
            let s = &mut *(sem as *mut SemDarwin);
            pthread_mutex_lock(s.mtx);
            let ok = if s.count > 0 {
                s.count -= 1;
                0
            } else {
                -1
            };
            pthread_mutex_unlock(s.mtx);
            return ok;
        }
        #[cfg(not(target_os = "macos"))]
        sys_sem_trywait(sem)
    }

    pub unsafe fn sem_post(sem: *mut c_void) -> i32 {
        #[cfg(target_os = "macos")]
        {
            let s = &mut *(sem as *mut SemDarwin);
            pthread_mutex_lock(s.mtx);
            s.count += 1;
            pthread_cond_signal(s.cond);
            pthread_mutex_unlock(s.mtx);
            return 0;
        }
        #[cfg(not(target_os = "macos"))]
        sys_sem_post(sem)
    }

    pub unsafe fn mutex_destroy(mtx: *mut c_void) {
        pthread_mutex_destroy(mtx);
        free(mtx);
    }

    pub unsafe fn sem_destroy(sem: *mut c_void) {
        #[cfg(target_os = "macos")]
        {
            let s = &mut *(sem as *mut SemDarwin);
            pthread_cond_destroy(s.cond);
            pthread_mutex_destroy(s.mtx);
            free(s.cond);
            free(s.mtx);
            free(sem);
            return;
        }
        #[cfg(not(target_os = "macos"))]
        {
            sys_sem_destroy(sem);
            free(sem);
        }
    }
}

pub use imp::*;
