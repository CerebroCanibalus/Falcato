//! # Conversión numérica — número ↔ texto
//!
//! Funciones nativas para serializar números a Texto (simetría con texto_a_entero).
//!
//! API expuesta (C ABI):
//! - `falcato_entero_a_texto(n: i64, desc_out: i64)` — convierte entero a texto
//! - `falcato_flotante_a_texto(f: f64, desc_out: i64)` — convierte flotante a texto
//! - `falcato_booleano_a_texto(b: i32, desc_out: i64)` — convierte booleano a texto

use std::ffi::c_void;

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn snprintf(buffer: *mut u8, size: usize, format: *const u8, ...) -> i32;
}

// Offsets del descriptor de Texto (deben coincidir con codegen/mod.rs)
const OFFSET_PTR: isize = 0;
const OFFSET_LEN: isize = 8;
const OFFSET_CAP: isize = 16;

/// Escribe un campo i64 en el descriptor en el offset dado.
unsafe fn escribir_campo(desc: i64, offset: isize, valor: i64) {
    let ptr = (desc as *mut u8).offset(offset) as *mut i64;
    *ptr = valor;
}

/// Convierte un entero (i64) a texto.
/// Formato: "%lld" (decimal con signo).
///
/// # Safety
/// - `desc_out` debe ser un puntero válido a un descriptor de Texto (24 bytes)
#[no_mangle]
pub unsafe extern "C" fn falcato_entero_a_texto(n: i64, desc_out: i64) {
    if desc_out == 0 {
        return;
    }

    // Buffer temporal para snprintf (20 bytes es suficiente para i64: -9223372036854775808)
    let mut buffer = [0u8; 32];
    let format = b"%lld\0";

    let len = snprintf(
        buffer.as_mut_ptr(),
        buffer.len(),
        format.as_ptr(),
        n,
    );

    if len < 0 {
        return; // Error de formato
    }

    let len_usize = len as usize;
    let cap = len_usize + 1; // +1 para null-terminator

    // Allocar buffer permanente
    let ptr = malloc(cap);
    if ptr.is_null() {
        return; // OOM
    }

    // Copiar datos
    std::ptr::copy_nonoverlapping(buffer.as_ptr(), ptr as *mut u8, len_usize);

    // Null-terminator
    *(ptr as *mut u8).add(len_usize) = 0;

    // Escribir descriptor
    escribir_campo(desc_out, OFFSET_PTR, ptr as i64);
    escribir_campo(desc_out, OFFSET_LEN, len_usize as i64);
    escribir_campo(desc_out, OFFSET_CAP, cap as i64);
}

/// Convierte un flotante (f64) a texto.
/// Formato: "%.17g" (precisión round-trip, notación científica si necesario).
///
/// # Safety
/// - `desc_out` debe ser un puntero válido a un descriptor de Texto (24 bytes)
/// - `f_bits` son los bits de un f64 reinterpretados como i64
#[no_mangle]
pub unsafe extern "C" fn falcato_flotante_a_texto(f_bits: i64, desc_out: i64) {
    if desc_out == 0 {
        return;
    }

    // Reinterpretar los bits como f64
    let f = f64::from_bits(f_bits as u64);

    // Buffer temporal para snprintf (32 bytes es suficiente para f64 con %.17g)
    let mut buffer = [0u8; 64];
    let format = b"%.17g\0";

    let len = snprintf(
        buffer.as_mut_ptr(),
        buffer.len(),
        format.as_ptr(),
        f,
    );

    if len < 0 {
        return; // Error de formato
    }

    let len_usize = len as usize;
    let cap = len_usize + 1; // +1 para null-terminator

    // Allocar buffer permanente
    let ptr = malloc(cap);
    if ptr.is_null() {
        return; // OOM
    }

    // Copiar datos
    std::ptr::copy_nonoverlapping(buffer.as_ptr(), ptr as *mut u8, len_usize);

    // Null-terminator
    *(ptr as *mut u8).add(len_usize) = 0;

    // Escribir descriptor
    escribir_campo(desc_out, OFFSET_PTR, ptr as i64);
    escribir_campo(desc_out, OFFSET_LEN, len_usize as i64);
    escribir_campo(desc_out, OFFSET_CAP, cap as i64);
}

/// Convierte un booleano (i32) a texto.
/// Formato: "verdadero" (b != 0) o "falso" (b == 0).
///
/// # Safety
/// - `desc_out` debe ser un puntero válido a un descriptor de Texto (24 bytes)
#[no_mangle]
pub unsafe extern "C" fn falcato_booleano_a_texto(b: i32, desc_out: i64) {
    if desc_out == 0 {
        return;
    }

    let texto: &[u8] = if b != 0 {
        b"verdadero"
    } else {
        b"falso"
    };
    let len = texto.len();
    let cap = len + 1; // +1 para null-terminator

    // Allocar buffer permanente
    let ptr = malloc(cap);
    if ptr.is_null() {
        return; // OOM
    }

    // Copiar datos
    std::ptr::copy_nonoverlapping(texto.as_ptr(), ptr as *mut u8, len);

    // Null-terminator
    *(ptr as *mut u8).add(len) = 0;

    // Escribir descriptor
    escribir_campo(desc_out, OFFSET_PTR, ptr as i64);
    escribir_campo(desc_out, OFFSET_LEN, len as i64);
    escribir_campo(desc_out, OFFSET_CAP, cap as i64);
}
