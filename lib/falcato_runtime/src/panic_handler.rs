//! 3.1b - Manejador de panico/crash para Falcato (runtime Capa A).
//!
//! Objetivo: cuando el programa crashea con 0xC0000005 / 0xC0000374 / SIGSEGV,
//! imprimir función+línea o al menos dirección+código en vez de "exit 322122...".
//!
//! Windows: SetUnhandledExceptionFilter + VectoredExceptionHandler.
//! POSIX:   signal(SIGSEGV/SIGABRT/SIGILL/SIGFPE) → handler.
//!
//! Nota Day-0: este módulo es Capa A (C Runtime). Cero cfg(target_os) en codegen.
//! El handler se auto-instala via `ctor` (init_array) y también es callable
//! explícitamente desde el prólogo de `principal` por si el ctor no corrió
//! (staticlib sin CRT init).

use std::ffi::c_void;

#[cfg(target_os = "windows")]
mod windows_handler {
    use super::*;
    use std::sync::Once;

    static INSTALADO: Once = Once::new();

    // --- WinAPI raw declarations (sin crate windows-sys) ---

    type PEXCEPTION_POINTERS = *mut c_void;
    type LONG = i32;
    // EXCEPTION_EXECUTE_HANDLER = 1, EXCEPTION_CONTINUE_SEARCH = 0
    const EXCEPTION_EXECUTE_HANDLER: LONG = 1;

    // STD_ERROR_HANDLE = -12
    const STD_ERROR_HANDLE: i32 = -12;
    const STD_OUTPUT_HANDLE: i32 = -11;

    extern "system" {
        fn SetUnhandledExceptionFilter(lpTopLevelExceptionFilter: extern "system" fn(PEXCEPTION_POINTERS) -> LONG) -> usize;
        fn GetStdHandle(nStdHandle: i32) -> *mut c_void;
        fn WriteFile(
            hFile: *mut c_void,
            lpBuffer: *const c_void,
            nNumberOfBytesToWrite: u32,
            lpNumberOfBytesWritten: *mut u32,
            lpOverlapped: *mut c_void,
        ) -> i32;
        fn ExitProcess(uExitCode: u32) -> !;
        fn FlushFileBuffers(hFile: *mut c_void) -> i32;
    }

    // Mensajes para códigos comunes
    fn nombre_excepcion(code: u32) -> &'static str {
        match code {
            0xC0000005 => "ACCESS_VIOLATION (0xC0000005) - puntero nulo / use-after-free / buffer overflow",
            0xC0000374 => "HEAP_CORRUPTION (0xC0000374) - doble free / heap buffer overflow / Texto/Vector corrupción",
            0xC0000094 => "INTEGER_DIVIDE_BY_ZERO (0xC0000094) - división por cero",
            0xC0000095 => "INTEGER_OVERFLOW (0xC0000095)",
            0xC00000FD => "STACK_OVERFLOW (0xC00000FD) - recursión infinita",
            0xC0000409 => "STACK_BUFFER_OVERRUN (0xC0000409) - canario de stack roto",
            _ => "EXCEPCION_DESCONOCIDA",
        }
    }

    extern "system" fn handler_top_level(exc: PEXCEPTION_POINTERS) -> LONG {
        unsafe {
            // EXCEPTION_POINTERS = { PEXCEPTION_RECORD ExceptionRecord; PCONTEXT ContextRecord }
            // Leer ExceptionRecord pointer (offset 0)
            let rec_ptr = *(exc as *const usize) as *const u8;
            if rec_ptr.is_null() {
                write_stderr(b"[PANIC] Crash sin ExceptionRecord\n");
                return EXCEPTION_EXECUTE_HANDLER;
            }
            // EXCEPTION_RECORD layout (64-bit):
            // 0: ExceptionCode (u32)
            // 4: ExceptionFlags (u32)
            // 8: ExceptionRecord (ptr)
            // 16: ExceptionAddress (ptr)
            let code = *(rec_ptr as *const u32);
            let addr = *(rec_ptr.add(16) as *const usize);

            // Flush stdout antes de reportar (asegura que prints previos no se pierdan - 3.1a)
            let h_out = GetStdHandle(STD_OUTPUT_HANDLE);
            if !h_out.is_null() {
                FlushFileBuffers(h_out);
            }

            let nombre = nombre_excepcion(code);
            let mut buf: [u8; 512] = [0; 512];
            let msg = format_panic(code, addr, nombre, &mut buf);
            write_stderr(msg);
            // Sugerencia accionable
            write_stderr(b"  [SUGERENCIA] stdout ya fue flusheado (imprimir_linea). Revisa la ultima linea impresa para localizar el crash.\n");
            write_stderr(b"  [SUGERENCIA] Si el crash es HEAP_CORRUPTION, revisa doble-free / Vector<Texto> / texto_agregar_texto.\n");
            write_stderr(b"  [SUGERENCIA] Compila con --emit-clif para ver IR de cada funcion.\n");
            // Terminar con el código original para que el caller vea 0xC000... pero ya informamos
            ExitProcess(code);
        }
    }

    fn format_panic<'a>(code: u32, addr: usize, nombre: &str, buf: &'a mut [u8]) -> &'a [u8] {
        // Formateo sin alloc (no_std friendly, pero estamos en std)
        // Usamos format! con alloc pequeño - ok en handler (no es async-signal-safe pero es Windows SEH)
        let s = format!(
            "\n[FALCATO PANIC] {} en 0x{:016X} (codigo 0x{:08X})\n  Funcion: desconocida (sin simbolos - usa --emit-clif + addr2line para mapear)\n",
            nombre, addr, code
        );
        let bytes = s.as_bytes();
        let n = bytes.len().min(buf.len() - 1);
        buf[..n].copy_from_slice(&bytes[..n]);
        &buf[..n]
    }

    fn write_stderr(msg: &[u8]) {
        unsafe {
            let h = GetStdHandle(STD_ERROR_HANDLE);
            if h.is_null() {
                return;
            }
            let mut written: u32 = 0;
            WriteFile(h, msg.as_ptr() as *const c_void, msg.len() as u32, &mut written, std::ptr::null_mut());
        }
    }

    /// Instala el handler. Idempotente (Once).
    pub fn instalar() {
        INSTALADO.call_once(|| unsafe {
            SetUnhandledExceptionFilter(handler_top_level);
        });
    }

    // Auto-instalar al cargar el binario (init_array / .CRT$XCU)
    // Usamos `ctor` via link_section sin depender del crate `ctor`.
    #[used]
    #[allow(non_upper_case_globals)]
    #[cfg(target_os = "windows")]
    #[link_section = ".CRT$XCU"]
    static ctor_windows: unsafe extern "C" fn() = {
        unsafe extern "C" fn ctor() {
            instalar();
        }
        ctor
    };
}

#[cfg(not(target_os = "windows"))]
mod posix_handler {
    use super::*;
    use std::sync::Once;

    static INSTALADO: Once = Once::new();

    const SIGSEGV: i32 = 11;
    const SIGABRT: i32 = 6;
    const SIGILL: i32 = 4;
    const SIGFPE: i32 = 8;
    const STDERR_FD: i32 = 2;

    extern "C" {
        fn signal(sig: i32, handler: extern "C" fn(i32)) -> usize;
        fn write(fd: i32, buf: *const u8, count: usize) -> isize;
        fn _exit(status: i32) -> !;
        fn fsync(fd: i32) -> i32;
    }

    extern "C" fn handler_sig(sig: i32) {
        unsafe {
            // Flush stdout
            fsync(1);
            let msg = match sig {
                SIGSEGV => b"\n[FALCATO PANIC] SIGSEGV (11) - segfault / puntero nulo / heap corruption\n  Funcion: desconocida (sin simbolos)\n",
                SIGABRT => b"\n[FALCATO PANIC] SIGABRT (6) - abort / double-free detectado por allocator\n  Funcion: desconocida\n",
                SIGILL => b"\n[FALCATO PANIC] SIGILL (4) - instruccion ilegal / heap corruption\n",
                SIGFPE => b"\n[FALCATO PANIC] SIGFPE (8) - division por cero / overflow\n",
                _ => b"\n[FALCATO PANIC] Senal desconocida\n",
            };
            write(STDERR_FD, msg.as_ptr(), msg.len());
            let sug = b"  [SUGERENCIA] stdout flusheado. Revisa ultima linea impresa.\n";
            write(STDERR_FD, sug.as_ptr(), sug.len());
            _exit(128 + sig);
        }
    }

    pub fn instalar() {
        INSTALADO.call_once(|| unsafe {
            signal(SIGSEGV, handler_sig);
            signal(SIGABRT, handler_sig);
            signal(SIGILL, handler_sig);
            signal(SIGFPE, handler_sig);
        });
    }

    // Auto-instalar via .init_array (sin crate ctor)
    #[used]
    #[allow(non_upper_case_globals)]
    #[link_section = ".init_array"]
    static ctor_posix: unsafe extern "C" fn() = {
        unsafe extern "C" fn ctor() {
            instalar();
        }
        ctor
    };
}

#[cfg(target_os = "windows")]
pub use windows_handler::instalar as instalar_manejador;

#[cfg(not(target_os = "windows"))]
pub use posix_handler::instalar as instalar_manejador;

/// API C para llamar desde el prólogo de `principal` (codegen).
/// También es auto-instalado, pero la llamada explícita asegura que se instale
/// aunque el CRT no corra ctors (staticlib).
#[no_mangle]
pub extern "C" fn falcato_instalar_manejador_panico() {
    instalar_manejador();
}
