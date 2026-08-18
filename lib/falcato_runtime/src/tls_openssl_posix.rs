//! # TLS/HTTPS — Implementación POSIX con OpenSSL
//!
//! Implementación de conexiones TLS usando OpenSSL.
//! ~350 LOC

use std::ffi::{c_void, CStr};
use std::ptr;

// Contexto de conexión TLS
pub struct TlsContext {
    socket: i32,
    ssl: *mut c_void,  // SSL*
    ctx: *mut c_void,  // SSL_CTX*
}

extern "C" {
    fn socket(domain: i32, sock_type: i32, protocol: i32) -> i32;
    fn connect(sockfd: i32, addr: *const c_void, addrlen: u32) -> i32;
    fn close(fd: i32) -> i32;
    fn send(sockfd: i32, buf: *const c_void, len: usize, flags: i32) -> isize;
    fn recv(sockfd: i32, buf: *mut c_void, len: usize, flags: i32) -> isize;
    fn getaddrinfo(
        node: *const u8,
        service: *const u8,
        hints: *const c_void,
        result: *mut *mut c_void,
    ) -> i32;
    fn freeaddrinfo(res: *mut c_void);

    // OpenSSL functions
    fn SSL_library_init() -> i32;
    fn SSL_load_error_strings();
    fn TLS_client_method() -> *mut c_void;
    fn SSL_CTX_new(method: *mut c_void) -> *mut c_void;
    fn SSL_CTX_free(ctx: *mut c_void);
    fn SSL_new(ctx: *mut c_void) -> *mut c_void;
    fn SSL_free(ssl: *mut c_void);
    fn SSL_set_fd(ssl: *mut c_void, fd: i32) -> i32;
    fn SSL_connect(ssl: *mut c_void) -> i32;
    fn SSL_write(ssl: *mut c_void, buf: *const c_void, num: i32) -> i32;
    fn SSL_read(ssl: *mut c_void, buf: *mut c_void, num: i32) -> i32;
    fn SSL_shutdown(ssl: *mut c_void) -> i32;
    fn SSL_set_tlsext_host_name(ssl: *mut c_void, name: *const u8) -> i32;
}

const AF_INET: i32 = 2;
const SOCK_STREAM: i32 = 1;
const IPPROTO_TCP: i32 = 6;

static mut OPENSSL_INITIALIZED: bool = false;

/// Inicializa OpenSSL (una sola vez).
unsafe fn init_openssl() {
    if !OPENSSL_INITIALIZED {
        SSL_library_init();
        SSL_load_error_strings();
        OPENSSL_INITIALIZED = true;
    }
}

/// Conecta a un servidor TLS.
pub unsafe fn tls_connect_impl(host: *const u8, puerto: i32) -> i64 {
    if host.is_null() || puerto <= 0 {
        return 0;
    }

    init_openssl();

    // Resolver DNS
    let puerto_str = format!("{}\0", puerto);
    let mut hints: [u8; 48] = [0; 48];
    *(hints.as_mut_ptr() as *mut i32) = AF_INET;
    *((hints.as_mut_ptr() as *mut i32).offset(1)) = SOCK_STREAM;
    *((hints.as_mut_ptr() as *mut i32).offset(2)) = IPPROTO_TCP;

    let mut result: *mut c_void = ptr::null_mut();
    let ret = getaddrinfo(host, puerto_str.as_ptr(), hints.as_ptr() as *const c_void, &mut result);
    if ret != 0 || result.is_null() {
        return 0;
    }

    // Crear socket y conectar
    let addr_info = &*(result as *const [u8; 32]);
    let sock = socket(AF_INET, SOCK_STREAM, IPPROTO_TCP);
    if sock < 0 {
        freeaddrinfo(result);
        return 0;
    }

    let addr = addr_info.as_ptr().offset(16) as *const c_void;
    if connect(sock, addr, 16) != 0 {
        close(sock);
        freeaddrinfo(result);
        return 0;
    }
    freeaddrinfo(result);

    // Crear contexto SSL
    let method = TLS_client_method();
    let ctx = SSL_CTX_new(method);
    if ctx.is_null() {
        close(sock);
        return 0;
    }

    let ssl = SSL_new(ctx);
    if ssl.is_null() {
        SSL_CTX_free(ctx);
        close(sock);
        return 0;
    }

    // Configurar SNI
    SSL_set_tlsext_host_name(ssl, host);

    // Asociar socket con SSL
    if SSL_set_fd(ssl, sock) != 1 {
        SSL_free(ssl);
        SSL_CTX_free(ctx);
        close(sock);
        return 0;
    }

    // Handshake TLS
    if SSL_connect(ssl) != 1 {
        SSL_free(ssl);
        SSL_CTX_free(ctx);
        close(sock);
        return 0;
    }

    // Crear contexto
    let tls_ctx = Box::new(TlsContext {
        socket: sock,
        ssl,
        ctx,
    });

    Box::into_raw(tls_ctx) as i64
}

/// Escribe datos a través de la conexión TLS.
pub unsafe fn tls_write_impl(conn: i64, datos: *const u8, n: i32) -> i32 {
    if conn == 0 || datos.is_null() || n <= 0 {
        return -1;
    }
    let ctx = &*(conn as *const TlsContext);
    SSL_write(ctx.ssl, datos as *const c_void, n)
}

/// Lee datos de la conexión TLS.
pub unsafe fn tls_read_impl(conn: i64, buf: *mut u8, n: i32) -> i32 {
    if conn == 0 || buf.is_null() || n <= 0 {
        return -1;
    }
    let ctx = &*(conn as *const TlsContext);
    SSL_read(ctx.ssl, buf as *mut c_void, n)
}

/// Verifica si hay datos disponibles.
pub unsafe fn tls_datos_disponibles_impl(conn: i64) -> i32 {
    if conn == 0 {
        return 0;
    }
    // TODO: Implementar con select/poll
    0
}

/// Cierra la conexión TLS.
pub unsafe fn tls_close_impl(conn: i64) {
    if conn == 0 {
        return;
    }
    let ctx = Box::from_raw(conn as *mut TlsContext);
    SSL_shutdown(ctx.ssl);
    SSL_free(ctx.ssl);
    SSL_CTX_free(ctx.ctx);
    close(ctx.socket);
}
