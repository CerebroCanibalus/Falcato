//! # Conversión de texto a números (R7.5 Fase 2)
//!
//! API expuesta (C ABI):
//! - `falcato_texto_a_entero(ptr, len) -> i64` — parsea un entero con signo.
//!   0 si no es número válido (o string vacío).
//! - `falcato_texto_a_natural(ptr, len) -> i64` — parsea un entero sin signo.
//!   -1 si no es número válido.
//! - `falcato_texto_a_flotante(ptr, len) -> f64` — parsea un flotante. 0.0 si no.
//! - `falcato_texto_a_booleano(ptr, len) -> i64` — 1 si "true"/"verdadero"/"sí"/"1",
//!   0 en cualquier otro caso.
//!
//! Reciben `(ptr, len)` — NO un descriptor Texto completo — para que el codegen
//! pueda llamarlas directo extrayendo ptr+len del descriptor (mismo contrato que
//! `falcato_texto_comparar`).
//!
//! Seguridad: el input viene de argv (no confiable). Se respeta `len` explícito
//! (nunca se asume null terminator) y se usa `str::from_utf8` con lossy.

use std::slice;

#[no_mangle]
pub unsafe extern "C" fn falcato_texto_a_entero(ptr: *const u8, len: i64) -> i64 {
    if ptr.is_null() || len <= 0 {
        return 0;
    }
    let bytes = slice::from_raw_parts(ptr, len as usize);
    let s = String::from_utf8_lossy(bytes);
    s.trim().parse::<i64>().unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn falcato_texto_a_natural(ptr: *const u8, len: i64) -> i64 {
    if ptr.is_null() || len <= 0 {
        return -1;
    }
    let bytes = slice::from_raw_parts(ptr, len as usize);
    let s = String::from_utf8_lossy(bytes);
    s.trim().parse::<u64>().map(|v| v as i64).unwrap_or(-1)
}

#[no_mangle]
pub unsafe extern "C" fn falcato_texto_a_flotante(ptr: *const u8, len: i64) -> f64 {
    if ptr.is_null() || len <= 0 {
        return 0.0;
    }
    let bytes = slice::from_raw_parts(ptr, len as usize);
    let s = String::from_utf8_lossy(bytes);
    s.trim().parse::<f64>().unwrap_or(0.0)
}

#[no_mangle]
pub unsafe extern "C" fn falcato_texto_a_booleano(ptr: *const u8, len: i64) -> i64 {
    if ptr.is_null() || len <= 0 {
        return 0;
    }
    let bytes = slice::from_raw_parts(ptr, len as usize);
    let s = String::from_utf8_lossy(bytes).to_lowercase();
    let s = s.trim();
    if s == "1" || s == "true" || s == "verdadero" || s == "si" || s == "sí" || s == "yes" {
        1
    } else {
        0
    }
}
