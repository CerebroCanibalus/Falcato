//! # Entrada estándar (stdin) — leer entrada pipeada o tecleada
//!
//! API expuesta (C ABI):
//! - `falcato_entrada_leer() -> *mut c_char` — lee TODO stdin hasta EOF,
//!   devuelve buffer heap (caller libera con free) o NULL en error.

use std::ffi::c_void;

// ============================================================
// Windows
// ============================================================
#[cfg(target_os = "windows")]
mod imp {
    use std::ffi::c_char;
    use std::ffi::c_void;
    use std::ptr;

    const STD_INPUT_HANDLE: i32 = -10;

    extern "system" {
        fn GetStdHandle(n_std_handle: i32) -> *mut c_void;
        fn ReadFile(
            h_file: *mut c_void,
            lp_buffer: *mut u8,
            n_number_of_bytes_to_read: u32,
            lp_number_of_bytes_read: *mut u32,
            lp_overlapped: *mut c_void,
        ) -> i32;
    }

    extern "C" {
        fn malloc(size: usize) -> *mut c_void;
        fn free(ptr: *mut c_void);
    }

    pub unsafe fn entrada_leer() -> *mut c_char {
        let input = GetStdHandle(STD_INPUT_HANDLE);
        if input.is_null() {
            return ptr::null_mut();
        }

        // Leer en chunks hasta EOF
        let mut buffer: Vec<u8> = Vec::new();
        let mut tmp = [0u8; 4096];
        loop {
            let mut leidos: u32 = 0;
            let ok = ReadFile(
                input,
                tmp.as_mut_ptr(),
                tmp.len() as u32,
                &mut leidos,
                ptr::null_mut(),
            );
            if ok == 0 || leidos == 0 {
                break;
            }
            buffer.extend_from_slice(&tmp[..leidos as usize]);
        }

        // Copiar a heap C con null terminator
        let len = buffer.len();
        let out = malloc(len + 1) as *mut u8;
        if out.is_null() {
            return ptr::null_mut();
        }
        if len > 0 {
            ptr::copy_nonoverlapping(buffer.as_ptr(), out, len);
        }
        *out.add(len) = 0;
        out as *mut c_char
    }
}

// ============================================================
// POSIX (Linux/macOS)
// ============================================================
#[cfg(not(target_os = "windows"))]
mod imp {
    use std::ffi::c_char;
    use std::ffi::c_void;
    use std::ptr;

    const STDIN_FILENO: i32 = 0;

    extern "C" {
        fn read(fd: i32, buf: *mut u8, count: usize) -> isize;
        fn malloc(size: usize) -> *mut c_void;
    }

    pub unsafe fn entrada_leer() -> *mut c_char {
        let mut buffer: Vec<u8> = Vec::new();
        let mut tmp = [0u8; 4096];
        loop {
            let n = read(STDIN_FILENO, tmp.as_mut_ptr(), tmp.len());
            if n <= 0 {
                break;
            }
            buffer.extend_from_slice(&tmp[..n as usize]);
        }
        let len = buffer.len();
        let out = malloc(len + 1) as *mut u8;
        if out.is_null() {
            return ptr::null_mut();
        }
        if len > 0 {
            ptr::copy_nonoverlapping(buffer.as_ptr(), out, len);
        }
        *out.add(len) = 0;
        out as *mut c_char
    }
}

pub use imp::*;
