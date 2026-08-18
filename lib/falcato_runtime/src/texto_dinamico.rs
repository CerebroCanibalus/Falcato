//! # Texto dinámico — operaciones de mutación eficiente sobre strings
//!
//! Operaciones nativas sobre el descriptor de Texto (ptr, len, cap) que permiten
//! manipulación eficiente sin copias innecesarias.
//!
//! Layout del descriptor (24 bytes):
//! - offset 0: ptr (i64) — puntero a los datos
//! - offset 8: len (i64) — longitud actual
//! - offset 16: cap (i64) — capacidad asignada
//!
//! API expuesta (C ABI):
//! - `falcato_texto_agregar_texto(desc, frag_desc)` — append con realloc eficiente
//! - `falcato_texto_poner_byte(desc, i, b)` — mutación in-place del heap
//! - `falcato_texto_puntero(desc) -> i64` — ptr interno del Texto
//! - `falcato_texto_desde_bytes(ptr, n, desc_out)` — construir Texto desde buffer crudo

use std::ffi::c_void;

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
}

// Offsets del descriptor de Texto (deben coincidir con codegen/mod.rs)
const OFFSET_PTR: isize = 0;
const OFFSET_LEN: isize = 8;
const OFFSET_CAP: isize = 16;

/// Lee un campo i64 del descriptor en el offset dado.
unsafe fn leer_campo(desc: i64, offset: isize) -> i64 {
    let ptr = (desc as *mut u8).offset(offset) as *const i64;
    *ptr
}

/// Escribe un campo i64 en el descriptor en el offset dado.
unsafe fn escribir_campo(desc: i64, offset: isize, valor: i64) {
    let ptr = (desc as *mut u8).offset(offset) as *mut i64;
    *ptr = valor;
}

/// Agrega un fragmento de texto al final del texto base con realloc eficiente.
/// Si la capacidad no es suficiente, realloc al doble o al tamaño necesario.
///
/// # Safety
/// - `desc` debe ser un puntero válido a un descriptor de Texto (24 bytes)
/// - `frag_desc` debe ser un puntero válido a un descriptor de Texto (24 bytes)
#[no_mangle]
pub unsafe extern "C" fn falcato_texto_agregar_texto(desc: i64, frag_desc: i64) {
    if desc == 0 || frag_desc == 0 {
        return;
    }

    // Leer campos del descriptor base
    let base_ptr = leer_campo(desc, OFFSET_PTR) as *mut u8;
    let base_len = leer_campo(desc, OFFSET_LEN) as usize;
    let mut base_cap = leer_campo(desc, OFFSET_CAP) as usize;

    // Leer campos del fragmento
    let frag_ptr = leer_campo(frag_desc, OFFSET_PTR) as *const u8;
    let frag_len = leer_campo(frag_desc, OFFSET_LEN) as usize;

    if frag_len == 0 {
        return; // Nada que agregar
    }

    let nueva_len = base_len + frag_len;

    // Si no hay capacidad suficiente, realloc
    if nueva_len + 1 > base_cap {
        // Nueva capacidad: el doble o el tamaño necesario, lo que sea mayor
        let mut nueva_cap = if base_cap == 0 { 16 } else { base_cap * 2 };
        while nueva_cap < nueva_len + 1 {
            nueva_cap *= 2;
        }

        let nuevo_ptr = realloc(base_ptr as *mut c_void, nueva_cap);
        if nuevo_ptr.is_null() {
            return; // OOM — no hacer nada
        }

        // Actualizar descriptor
        escribir_campo(desc, OFFSET_PTR, nuevo_ptr as i64);
        escribir_campo(desc, OFFSET_CAP, nueva_cap as i64);
        let nuevo_ptr_u8 = nuevo_ptr as *mut u8;
        memcpy(
            nuevo_ptr_u8.add(base_len) as *mut c_void,
            frag_ptr as *const c_void,
            frag_len,
        );
    } else {
        // Hay capacidad suficiente, solo copiar
        memcpy(
            base_ptr.add(base_len) as *mut c_void,
            frag_ptr as *const c_void,
            frag_len,
        );
    }

    // Actualizar longitud y null-terminator
    escribir_campo(desc, OFFSET_LEN, nueva_len as i64);
    let ptr_final = leer_campo(desc, OFFSET_PTR) as *mut u8;
    *ptr_final.add(nueva_len) = 0; // null-terminator
}

/// Pone un byte en la posición i del texto (mutación in-place).
/// Si i >= len, no hace nada (bounds check).
///
/// # Safety
/// - `desc` debe ser un puntero válido a un descriptor de Texto (24 bytes)
/// - `i` debe ser >= 0
/// - `b` debe ser un byte válido (0-255)
#[no_mangle]
pub unsafe extern "C" fn falcato_texto_poner_byte(desc: i64, i: i32, b: i32) {
    if desc == 0 || i < 0 {
        return;
    }

    let ptr = leer_campo(desc, OFFSET_PTR) as *mut u8;
    let len = leer_campo(desc, OFFSET_LEN) as usize;
    let idx = i as usize;

    if idx >= len {
        return; // Fuera de bounds
    }

    *ptr.add(idx) = b as u8;
}

/// Devuelve el puntero interno del texto (para pasar a funciones C como tcp_escribir).
///
/// # Safety
/// - `desc` debe ser un puntero válido a un descriptor de Texto (24 bytes)
#[no_mangle]
pub unsafe extern "C" fn falcato_texto_puntero(desc: i64) -> i64 {
    if desc == 0 {
        return 0;
    }
    leer_campo(desc, OFFSET_PTR)
}

/// Construye un descriptor de Texto desde un buffer crudo (ptr, n).
/// Copia los datos a un nuevo buffer malloc'ed y null-termina.
///
/// # Safety
/// - `ptr` debe ser un puntero válido a n bytes
/// - `desc_out` debe ser un puntero válido a un descriptor de Texto (24 bytes)
#[no_mangle]
pub unsafe extern "C" fn falcato_texto_desde_bytes(ptr: i64, n: i32, desc_out: i64) {
    if desc_out == 0 || n < 0 {
        return;
    }

    let len = n as usize;
    let cap = len + 1; // +1 para null-terminator

    let nuevo_ptr = malloc(cap);
    if nuevo_ptr.is_null() {
        return; // OOM
    }

    if len > 0 && ptr != 0 {
        memcpy(nuevo_ptr, ptr as *const c_void, len);
    }

    // Null-terminator
    *(nuevo_ptr as *mut u8).add(len) = 0;

    // Escribir descriptor
    escribir_campo(desc_out, OFFSET_PTR, nuevo_ptr as i64);
    escribir_campo(desc_out, OFFSET_LEN, len as i64);
    escribir_campo(desc_out, OFFSET_CAP, cap as i64);
}
