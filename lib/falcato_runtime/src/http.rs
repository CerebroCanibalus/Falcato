//! # HTTP Client — cliente HTTP/HTTPS mínimo para Cid
//!
//! Implementación sobre TCP del runtime. Soporta GET y POST básicos.
//! Parsing HTTP minimal: solo lee status code y body.

use std::ffi::c_void;

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn send(socket: i64, buf: *const c_void, len: i32, flags: i32) -> i32;
    fn recv(socket: i64, buf: *mut c_void, len: i32, flags: i32) -> i32;
    fn closesocket(socket: i64) -> i32;
    // TCP builtins del runtime
    fn falcato_tcp_conectar(host: i64, puerto: i32) -> i64;
}

// Offsets del descriptor de Texto
const OFFSET_PTR: isize = 0;
const OFFSET_LEN: isize = 8;
const OFFSET_CAP: isize = 16;

unsafe fn leer_campo(desc: i64, offset: isize) -> i64 {
    let ptr = (desc as *mut u8).offset(offset) as *const i64;
    *ptr
}

unsafe fn escribir_campo(desc: i64, offset: isize, valor: i64) {
    let ptr = (desc as *mut u8).offset(offset) as *mut i64;
    *ptr = valor;
}

/// Crea un descriptor de Texto desde un buffer Rust.
unsafe fn texto_desde_buffer(data: &[u8], desc_out: i64) {
    let len = data.len();
    let cap = len + 1;
    let ptr = malloc(cap);
    if ptr.is_null() { return; }
    if len > 0 {
        memcpy(ptr, data.as_ptr() as *const c_void, len);
    }
    *(ptr as *mut u8).add(len) = 0;
    escribir_campo(desc_out, OFFSET_PTR, ptr as i64);
    escribir_campo(desc_out, OFFSET_LEN, len as i64);
    escribir_campo(desc_out, OFFSET_CAP, cap as i64);
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|window| window == needle)
}

fn parse_status_code(response: &[u8]) -> i32 {
    if let Some(pos) = find_subsequence(response, b" ") {
        let after_space = &response[pos + 1..];
        let mut code: i32 = 0;
        for &b in after_space.iter().take(3) {
            if b >= b'0' && b <= b'9' {
                code = code * 10 + (b - b'0') as i32;
            } else {
                break;
            }
        }
        return code;
    }
    -1
}

/// HTTP GET — conecta por TCP, envía request, lee respuesta.
/// Escribe el body en `desc_body_out`. Retorna status code o -1 si hay error.
///
/// # Safety
/// - Descriptores deben ser válidos
#[no_mangle]
pub unsafe extern "C" fn falcato_http_get(
    desc_host: i64,
    puerto: i32,
    desc_path: i64,
    desc_body_out: i64,
) -> i32 {
    if desc_host == 0 || desc_path == 0 || desc_body_out == 0 {
        return -1;
    }

    let host_ptr = leer_campo(desc_host, OFFSET_PTR) as *const u8;
    let host_len = leer_campo(desc_host, OFFSET_LEN) as usize;
    let path_ptr = leer_campo(desc_path, OFFSET_PTR) as *const u8;
    let path_len = leer_campo(desc_path, OFFSET_LEN) as usize;

    // Conectar TCP
    let fd = falcato_tcp_conectar(desc_host, puerto);
    if fd < 0 {
        return -1;
    }

    // Construir request HTTP/1.1
    let mut request = Vec::new();
    request.extend_from_slice(b"GET ");
    request.extend_from_slice(core::slice::from_raw_parts(path_ptr, path_len));
    request.extend_from_slice(b" HTTP/1.1\r\nHost: ");
    request.extend_from_slice(core::slice::from_raw_parts(host_ptr, host_len));
    request.extend_from_slice(b"\r\nConnection: close\r\nAccept: */*\r\n\r\n");

    // Enviar request
    let sent = send(fd, request.as_ptr() as *const c_void, request.len() as i32, 0);
    if sent < 0 {
        closesocket(fd);
        return -1;
    }

    // Leer respuesta completa
    let buf_len = 1024 * 1024; // 1MB máximo
    let buf = malloc(buf_len);
    if buf.is_null() {
        closesocket(fd);
        return -1;
    }

    let mut total: usize = 0;
    let mut headers_completos = false;
    let mut content_length: usize = 0;

    while total < buf_len - 1 {
        let por_leer = if buf_len - total > 4096 { 4096 } else { buf_len - total };
        let recibido = recv(fd, buf.add(total) as *mut c_void, por_leer as i32, 0);

        if recibido <= 0 {
            break;
        }
        total += recibido as usize;

        // Verificar si ya tenemos los headers completos
        if !headers_completos {
            let buf_slice = core::slice::from_raw_parts(buf as *const u8, total);
            if let Some(pos) = find_subsequence(buf_slice, b"\r\n\r\n") {
                headers_completos = true;
                let headers = &buf_slice[pos + 4..];

                // Parsear Content-Length
                for line in headers.split(|&b| b == b'\n') {
                    let line = if line.ends_with(b"\r") { &line[..line.len()-1] } else { line };
                    if line.len() > 15 && line[..15].eq_ignore_ascii_case(b"Content-Length:") {
                        let val = &line[15..];
                        // trim spaces/tabs manually for &[u8]
                        let val = {
                            let mut start = 0;
                            while start < val.len() && (val[start] == b' ' || val[start] == b'\t') {
                                start += 1;
                            }
                            &val[start..]
                        };
                        let mut cl: usize = 0;
                        for &b in val {
                            if b >= b'0' && b <= b'9' {
                                cl = cl * 10 + (b - b'0') as usize;
                            }
                        }
                        content_length = cl;
                    }
                }
            }
        }

        // Si tenemos headers y Content-Length, verificar si ya tenemos todo el body
        if headers_completos && content_length > 0 {
            let body_start = find_subsequence(core::slice::from_raw_parts(buf as *const u8, total), b"\r\n\r\n")
                .map(|p| p + 4)
                .unwrap_or(0);
            let body_actual = total - body_start;
            if body_actual >= content_length {
                break;
            }
        }
    }

    closesocket(fd);

    if total <= 0 {
        free(buf);
        return -1;
    }

    // Parsear status code
    let response = core::slice::from_raw_parts(buf as *const u8, total);
    let status_code = parse_status_code(response);

    // Extraer body
    if let Some(body_start) = find_subsequence(response, b"\r\n\r\n") {
        let body = &response[body_start + 4..];
        texto_desde_buffer(body, desc_body_out);
    } else {
        texto_desde_buffer(b"", desc_body_out);
    }

    free(buf);
    status_code
}

/// HTTP POST — igual que GET pero con body en el request.
///
/// # Safety
/// - Todos los descriptores deben ser válidos
#[no_mangle]
pub unsafe extern "C" fn falcato_http_post(
    desc_host: i64,
    puerto: i32,
    desc_path: i64,
    desc_body_req: i64,
    desc_body_out: i64,
) -> i32 {
    if desc_host == 0 || desc_path == 0 || desc_body_out == 0 {
        return -1;
    }

    let host_ptr = leer_campo(desc_host, OFFSET_PTR) as *const u8;
    let host_len = leer_campo(desc_host, OFFSET_LEN) as usize;
    let path_ptr = leer_campo(desc_path, OFFSET_PTR) as *const u8;
    let path_len = leer_campo(desc_path, OFFSET_LEN) as usize;

    // Body del request
    let (body_ptr, body_len) = if desc_body_req != 0 {
        let p = leer_campo(desc_body_req, OFFSET_PTR) as *const u8;
        let l = leer_campo(desc_body_req, OFFSET_LEN) as usize;
        (p, l)
    } else {
        (core::ptr::null(), 0)
    };

    let fd = falcato_tcp_conectar(desc_host, puerto);
    if fd < 0 {
        return -1;
    }

    // Construir request
    let mut request = Vec::new();
    request.extend_from_slice(b"POST ");
    request.extend_from_slice(core::slice::from_raw_parts(path_ptr, path_len));
    request.extend_from_slice(b" HTTP/1.1\r\nHost: ");
    request.extend_from_slice(core::slice::from_raw_parts(host_ptr, host_len));
    request.extend_from_slice(b"\r\nContent-Type: application/json\r\nContent-Length: ");
    request.extend_from_slice(body_len.to_string().as_bytes());
    request.extend_from_slice(b"\r\nConnection: close\r\n\r\n");
    if body_len > 0 {
        request.extend_from_slice(core::slice::from_raw_parts(body_ptr, body_len));
    }

    let sent = send(fd, request.as_ptr() as *const c_void, request.len() as i32, 0);
    if sent < 0 {
        closesocket(fd);
        return -1;
    }

    let buf_len = 1024 * 1024;
    let buf = malloc(buf_len);
    if buf.is_null() {
        closesocket(fd);
        return -1;
    }

    let mut total: usize = 0;
    while total < buf_len - 1 {
        let por_leer = if buf_len - total > 4096 { 4096 } else { buf_len - total };
        let recibido = recv(fd, buf.add(total) as *mut c_void, por_leer as i32, 0);
        if recibido <= 0 { break; }
        total += recibido as usize;
    }

    closesocket(fd);

    if total <= 0 {
        free(buf);
        return -1;
    }

    let response = core::slice::from_raw_parts(buf as *const u8, total);
    let status_code = parse_status_code(response);

    if let Some(body_start) = find_subsequence(response, b"\r\n\r\n") {
        let body = &response[body_start + 4..];
        texto_desde_buffer(body, desc_body_out);
    } else {
        texto_desde_buffer(b"", desc_body_out);
    }

    free(buf);
    status_code
}
