//! # TLS en macOS — módulo Rust sobre el shim de Network.framework
//!
//! Mismo patrón que Windows (`tls_schannel_windows.rs`) y Linux
//! (`tls_openssl_posix.rs`):
//! - `TlsContext` en el heap y handle `i64` opaco (`Box::into_raw` / `from_raw`).
//! - Errores por valor de retorno (`0` / `-1`), sin panic.
//! - API síncrona: el bloqueo real lo hace el shim ObjC con `dispatch_semaphore`.
//!
//! Diferencia con las otras plataformas: acá el FFI no va a la librería nativa
//! directamente, sino al shim (`falcato_nw_tls_*`), porque Network.framework es
//! asíncrona. El handshake, el SNI y la validación del certificado los sigue
//! haciendo el sistema.

use std::ffi::c_void;

/// Contexto de conexión TLS (handle opaco para el codegen).
pub struct TlsContext {
    handle: *mut c_void,
}

extern "C" {
    fn falcato_nw_tls_conectar(host: *const u8, host_len: usize, puerto: i32) -> *mut c_void;
    fn falcato_nw_tls_escribir(handle: *mut c_void, datos: *const u8, n: i32) -> i32;
    fn falcato_nw_tls_leer(handle: *mut c_void, buf: *mut u8, n: i32) -> i32;
    fn falcato_nw_tls_datos_disponibles(handle: *mut c_void) -> i32;
    fn falcato_nw_tls_cerrar(handle: *mut c_void);
}

/// Desglosa un descriptor de `Texto` `{ ptr, len, cap }` (lo pasa el codegen).
unsafe fn desglosar_texto(desc: *const u8) -> (*const u8, usize) {
    let ptr = *(desc as *const i64) as *const u8;
    let len = *((desc as *const i64).add(1)) as usize;
    (ptr, len)
}

/// Conecta a un servidor TLS. `host` es el descriptor de Texto del codegen.
pub unsafe fn tls_connect_impl(host: *const u8, puerto: i32) -> i64 {
    if host.is_null() || puerto <= 0 {
        return 0;
    }
    let (ptr, len) = desglosar_texto(host);
    if ptr.is_null() || len == 0 {
        return 0;
    }
    let handle = falcato_nw_tls_conectar(ptr, len, puerto);
    if handle.is_null() {
        return 0;
    }
    Box::into_raw(Box::new(TlsContext { handle })) as i64
}

/// Escribe datos cifrados. Devuelve bytes escritos o -1.
pub unsafe fn tls_write_impl(conn: i64, datos: *const u8, n: i32) -> i32 {
    if conn == 0 || datos.is_null() || n <= 0 {
        return -1;
    }
    let ctx = &*(conn as *const TlsContext);
    falcato_nw_tls_escribir(ctx.handle, datos, n)
}

/// Lee datos descifrados. Devuelve bytes leídos, 0 en EOF, -1 en error.
pub unsafe fn tls_read_impl(conn: i64, buf: *mut u8, n: i32) -> i32 {
    if conn == 0 || buf.is_null() || n <= 0 {
        return -1;
    }
    let ctx = &*(conn as *const TlsContext);
    falcato_nw_tls_leer(ctx.handle, buf, n)
}

/// ¿Hay datos disponibles? (conservador en macOS, ver el shim).
pub unsafe fn tls_datos_disponibles_impl(conn: i64) -> i32 {
    if conn == 0 {
        return 0;
    }
    let ctx = &*(conn as *const TlsContext);
    falcato_nw_tls_datos_disponibles(ctx.handle)
}

/// Cierra la conexión y libera el contexto.
pub unsafe fn tls_close_impl(conn: i64) {
    if conn == 0 {
        return;
    }
    let ctx = Box::from_raw(conn as *mut TlsContext);
    falcato_nw_tls_cerrar(ctx.handle);
}
