//! # TLS/HTTPS — Conexiones seguras
//!
//! API pública para conexiones TLS/HTTPS usando Schannel (Windows) o OpenSSL (POSIX).
//!
//! API expuesta (C ABI):
//! - `falcato_tls_conectar(host, puerto) -> i64` — establece conexión TLS
//! - `falcato_tls_escribir(conn, datos, n) -> i32` — escribe datos cifrados
//! - `falcato_tls_leer(conn, buf, n) -> i32` — lee datos descifrados
//! - `falcato_tls_datos_disponibles(conn) -> i32` — verifica si hay datos sin bloquear
//! - `falcato_tls_cerrar(conn)` — cierra conexión TLS
//!
//! ## Notas de seguridad
//! - Validación de certificados automática (certs del sistema en Windows, CA bundle en POSIX)
//! - SNI (Server Name Indication) enviado en handshake
//! - ALPN negociado para HTTP/2 si el servidor lo soporta

use std::ffi::c_void;

// ============================================================
// Windows — Schannel
// ============================================================
#[cfg(target_os = "windows")]
#[path = "tls_schannel_windows.rs"]
mod tls_schannel_windows;

#[cfg(target_os = "windows")]
use tls_schannel_windows::*;

// ============================================================
// POSIX — OpenSSL
// ============================================================
#[cfg(not(target_os = "windows"))]
#[path = "tls_openssl_posix.rs"]
mod tls_openssl_posix;

#[cfg(not(target_os = "windows"))]
use tls_openssl_posix::*;

/// Conecta a un servidor TLS/HTTPS.
/// Retorna handle de conexión (0 = error).
#[no_mangle]
pub unsafe extern "C" fn falcato_tls_conectar(host: *const u8, puerto: i32) -> i64 {
    tls_connect_impl(host, puerto)
}

/// Escribe datos a través de la conexión TLS.
/// Retorna número de bytes escritos (-1 = error).
#[no_mangle]
pub unsafe extern "C" fn falcato_tls_escribir(conn: i64, datos: *const u8, n: i32) -> i32 {
    tls_write_impl(conn, datos, n)
}

/// Lee datos de la conexión TLS.
/// Retorna número de bytes leídos (0 = EOF, -1 = error).
#[no_mangle]
pub unsafe extern "C" fn falcato_tls_leer(conn: i64, buf: *mut u8, n: i32) -> i32 {
    tls_read_impl(conn, buf, n)
}

/// Verifica si hay datos disponibles para leer sin bloquear.
/// Retorna 1 si hay datos, 0 si no.
#[no_mangle]
pub unsafe extern "C" fn falcato_tls_datos_disponibles(conn: i64) -> i32 {
    tls_datos_disponibles_impl(conn)
}

/// Cierra la conexión TLS.
#[no_mangle]
pub unsafe extern "C" fn falcato_tls_cerrar(conn: i64) {
    tls_close_impl(conn)
}
