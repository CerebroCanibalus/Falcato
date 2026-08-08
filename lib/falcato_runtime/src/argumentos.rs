//! # Argumentos de línea de comandos (argv) — R7.5
//!
//! API expuesta (C ABI):
//! - `falcato_argumentos() -> *mut c_void` — devuelve un descriptor
//!   `Vector<Texto>` de Falcato construido en heap: `{ptr: i64, len: i64,
//!   cap: i64}` donde `ptr` apunta a un array de descriptores `Texto`
//!   (`{ptr, len, cap}` cada uno) y cada Texto apunta a un string malloc'ed.
//!   El caller NO libera el descriptor directamente: `vector_liberar` /
//!   `texto_liberar` de Falcato lo recorren y liberan.
//!
//! El layout del descriptor lo comparte con el codegen (OFFSET_PTR=0,
//! OFFSET_LEN=8, OFFSET_CAP=16). El runtime NO sabe el layout de Falcato:
//! lo construye con el mismo contrato de memoria que usa `entrada_leer`.
//!
//! Seguridad: argv viene del SO, no es confiable. Se copian TODOS los strings
//! a heap propio con su longitud explícita (nunca se confía en el contenido).

use std::ffi::c_char;
use std::ffi::c_void;
use std::ptr;

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
}

// Layout de descriptor compartido con codegen (24 bytes cada uno)
const OFF_PTR: usize = 0;
const OFF_LEN: usize = 8;
const OFF_CAP: usize = 16;
const TAM_DESCRIPTOR: usize = 24;

// ============================================================
// Windows — GetCommandLineW + CommandLineToArgvW (shell32)
// ============================================================
#[cfg(target_os = "windows")]
mod imp {
    use super::*;

    extern "system" {
        fn GetCommandLineW() -> *const u16;
        fn CommandLineToArgvW(
            lp_cmd_line: *const u16,
            p_num_args: *mut i32,
        ) -> *mut *mut u16;
        fn LocalFree(h_mem: *mut c_void) -> *mut c_void;
    }

    /// Convierte un string UTF-16 a un buffer UTF-8 heap (malloc'ed).
    /// Devuelve (ptr, len) sin null terminator adicional (el Texto usa len).
    unsafe fn utf16_a_utf8(wide: *const u16) -> (*mut u8, usize) {
        if wide.is_null() {
            return (ptr::null_mut(), 0);
        }
        // Calcular longitud UTF-16
        let mut n = 0usize;
        while *wide.add(n) != 0 {
            n += 1;
        }
        let slice = std::slice::from_raw_parts(wide, n);
        let utf8 = String::from_utf16_lossy(slice).into_bytes();
        let len = utf8.len();
        // malloc(len + 1): incluye null terminator para puts/printf (%s)
        let out = malloc(len + 1) as *mut u8;
        if out.is_null() {
            return (ptr::null_mut(), 0);
        }
        if len > 0 {
            ptr::copy_nonoverlapping(utf8.as_ptr(), out, len);
        }
        *out.add(len) = 0;
        (out, len)
    }

    pub unsafe fn argumentos() -> *mut c_void {
        let mut argc: i32 = 0;
        let argvw = CommandLineToArgvW(GetCommandLineW(), &mut argc);
        if argvw.is_null() {
            return ptr::null_mut();
        }
        let count = argc.max(0) as usize;

        // Array de PUNTEROS a descriptores Texto (count * 8 bytes).
        // El layout de Vector<Texto> en codegen guarda punteros (8 bytes por
        // elemento), no los descriptores inline de 24 bytes.
        let tam_puntero = 8usize;
        let arr = malloc(count.saturating_mul(tam_puntero)) as *mut u8;
        if count > 0 && arr.is_null() {
            LocalFree(argvw as *mut c_void);
            return ptr::null_mut();
        }

        for i in 0..count {
            let (data, len) = utf16_a_utf8(*argvw.add(i));
            let cap = len + 1; // incluye null terminator
            // Crear descriptor Texto: {ptr, len, cap}
            let desc = malloc(TAM_DESCRIPTOR) as *mut u8;
            if desc.is_null() {
                continue;
            }
            ptr::write(desc.add(OFF_PTR) as *mut i64, data as i64);
            ptr::write(desc.add(OFF_LEN) as *mut i64, len as i64);
            ptr::write(desc.add(OFF_CAP) as *mut i64, cap as i64);
            // Guardar el PUNTERO al descriptor en el array
            ptr::write(arr.add(i * tam_puntero) as *mut i64, desc as i64);
        }

        LocalFree(argvw as *mut c_void);

        // Descriptor Vector<Texto>: {ptr=arr, len=count, cap=count}
        let vec_desc = malloc(TAM_DESCRIPTOR) as *mut u8;
        if vec_desc.is_null() {
            return ptr::null_mut();
        }
        ptr::write(vec_desc.add(OFF_PTR) as *mut i64, arr as i64);
        ptr::write(vec_desc.add(OFF_LEN) as *mut i64, count as i64);
        ptr::write(vec_desc.add(OFF_CAP) as *mut i64, count as i64);
        vec_desc as *mut c_void
    }
}

// ============================================================
// POSIX (Linux/macOS) — __argc / __argv (glibc) o _NSGetArgc/_NSGetArgv
// ============================================================
#[cfg(not(target_os = "windows"))]
mod imp {
    use super::*;

    #[cfg(target_os = "macos")]
    extern "C" {
        fn _NSGetArgc() -> *const i32;
        fn _NSGetArgv() -> *const *const *const c_char;
    }

    #[cfg(not(target_os = "macos"))]
    extern "C" {
        // glibc / musl exponen estas globales del CRT
        static __argc: i32;
        static __argv: *const *const c_char;
    }

    /// Copia un string C a heap propio con null terminator, devuelve (ptr, len).
    unsafe fn copiar_str(s: *const c_char) -> (*mut u8, usize) {
        if s.is_null() {
            return (ptr::null_mut(), 0);
        }
        let mut n = 0usize;
        while *s.add(n) != 0 {
            n += 1;
        }
        let len = n;
        // malloc(len + 1): incluye null terminator para puts/printf (%s)
        let out = malloc(len + 1) as *mut u8;
        if out.is_null() {
            return (ptr::null_mut(), 0);
        }
        if len > 0 {
            ptr::copy_nonoverlapping(s as *const u8, out, len);
        }
        *out.add(len) = 0;
        (out, len)
    }

    pub unsafe fn argumentos() -> *mut c_void {
        #[cfg(target_os = "macos")]
        let (argc_ptr, argv_ptr): (*const i32, *const *const *const c_char) =
            (_NSGetArgc(), _NSGetArgv());
        #[cfg(not(target_os = "macos"))]
        let (argc_ptr, argv_ptr): (*const i32, *const *const *const c_char) =
            (&__argc as *const i32, &__argv as *const *const *const c_char);

        if argv_ptr.is_null() {
            return ptr::null_mut();
        }
        let count = (*argc_ptr).max(0) as usize;

        // Array de PUNTEROS a descriptores Texto (8 bytes por elemento)
        let tam_puntero = 8usize;
        let arr = malloc(count.saturating_mul(tam_puntero)) as *mut u8;
        if count > 0 && arr.is_null() {
            return ptr::null_mut();
        }

        for i in 0..count {
            let (data, len) = copiar_str(*argv_ptr.add(i));
            let desc = malloc(TAM_DESCRIPTOR) as *mut u8;
            if desc.is_null() {
                continue;
            }
            ptr::write(desc.add(OFF_PTR) as *mut i64, data as i64);
            ptr::write(desc.add(OFF_LEN) as *mut i64, len as i64);
            ptr::write(desc.add(OFF_CAP) as *mut i64, (len + 1) as i64);
            ptr::write(arr.add(i * tam_puntero) as *mut i64, desc as i64);
        }

        let vec_desc = malloc(TAM_DESCRIPTOR) as *mut u8;
        if vec_desc.is_null() {
            return ptr::null_mut();
        }
        ptr::write(vec_desc.add(OFF_PTR) as *mut i64, arr as i64);
        ptr::write(vec_desc.add(OFF_LEN) as *mut i64, count as i64);
        ptr::write(vec_desc.add(OFF_CAP) as *mut i64, count as i64);
        vec_desc as *mut c_void
    }
}

pub use imp::*;
