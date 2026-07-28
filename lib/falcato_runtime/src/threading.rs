//! # Threading — creación y unión de threads del OS
//!
//! Abstracción portable sobre CreateThread (Win32) y pthread_create (POSIX).

#[cfg(target_os = "windows")]
mod imp {
    use std::ffi::c_void;

    pub unsafe fn thread_run(
        thread_fn: unsafe extern "C" fn(*mut c_void) -> i32,
        arg: *mut c_void,
    ) -> *mut c_void {
        extern "system" {
            fn CreateThread(
                lpThreadAttributes: *mut c_void,
                dwStackSize: usize,
                lpStartAddress: unsafe extern "C" fn(*mut c_void) -> i32,
                lpParameter: *mut c_void,
                dwCreationFlags: u32,
                lpThreadId: *mut u32,
            ) -> *mut c_void;
        }
        CreateThread(core::ptr::null_mut(), 0, thread_fn, arg, 0, core::ptr::null_mut())
    }

    pub unsafe fn thread_join(handle: *mut c_void) -> i32 {
        extern "system" {
            fn WaitForSingleObject(hHandle: *mut c_void, dwMilliseconds: u32) -> i32;
            fn CloseHandle(hObject: *mut c_void) -> i32;
        }
        let ret = WaitForSingleObject(handle, u32::MAX);
        CloseHandle(handle);
        ret
    }
}

#[cfg(not(target_os = "windows"))]
mod imp {
    use std::ffi::c_void;

    extern "C" {
        fn pthread_create(
            thread: *mut usize,
            attr: *const c_void,
            start_routine: unsafe extern "C" fn(*mut c_void) -> i32,
            arg: *mut c_void,
        ) -> i32;

        fn pthread_join(thread: usize, retval: *mut *mut c_void) -> i32;
        fn malloc(size: usize) -> *mut c_void;
        fn free(ptr: *mut c_void);
    }

    pub unsafe fn thread_run(
        thread_fn: unsafe extern "C" fn(*mut c_void) -> i32,
        arg: *mut c_void,
    ) -> *mut c_void {
        let tid = malloc(core::mem::size_of::<usize>()) as *mut usize;
        if tid.is_null() {
            return core::ptr::null_mut();
        }
        let ret = pthread_create(tid, core::ptr::null(), thread_fn, arg);
        if ret != 0 {
            free(tid as *mut c_void);
            return core::ptr::null_mut();
        }
        tid as *mut c_void
    }

    pub unsafe fn thread_join(handle: *mut c_void) -> i32 {
        let tid = *(handle as *mut usize);
        let ret = pthread_join(tid, core::ptr::null_mut());
        free(handle);
        ret
    }
}

pub use imp::*;
