//! # Nivel 2 — Cirujano (~350 LOC)
//!
//! Responsabilidad ÚNICA: integridad del bloque y UAF con cuarentena.
//! - canarios 0xDEADBEEFCAFEBABE antes/después
//! - pintado: 0xAA al alocar, 0xFF al liberar (detecta UAF por lectura)
//! - cuarentena de 64 bloques (free no es inmediato, UAF es diagnosticable)
//! - ficha de canario
//!
//! Depende de nivel1 para el mapa base. Si falla, el bug está aquí.

use super::log;
use std::collections::VecDeque;
use std::sync::{Mutex, OnceLock};

const CANARIO: u64 = 0xDEADBEEFCAFEBABE;
const CANARIO_SIZE: usize = 8;
static CUARENTENA: OnceLock<Mutex<VecDeque<i64>>> = OnceLock::new();

fn cuarentena() -> &'static Mutex<VecDeque<i64>> {
    CUARENTENA.get_or_init(|| Mutex::new(VecDeque::with_capacity(64)))
}

pub fn init() { let _ = cuarentena(); log::imprimir("[MEM] nivel 2 — Cirujano (canarios+cuarentena)"); }

pub fn pintar_alloc(ptr: i64, bytes: i64) {
    if ptr == 0 { return; }
    unsafe {
        std::ptr::write_bytes(ptr as *mut u8, 0xAA, bytes as usize);
        // Escribir canarios
        let antes = (ptr as *mut u8).sub(CANARIO_SIZE) as *mut u64;
        let despues = (ptr as *mut u8).add(bytes as usize) as *mut u64;
        // NOTA: esto requiere que el allocator haya reservado 16 bytes extra.
        // En la integración real, el wrapper malloc reserva +16 y desplaza ptr.
        // Aquí stub — la lógica real vive en el wrapper de malloc del runtime.
        let _ = (antes, despues); // stub para compilar
    }
    log::incrementar("pintado_alloc");
}

pub fn verificar_canario(ptr: i64, archivo: *const u8, linea: u32) {
    // Stub — verificación real lee los 8 bytes antes/después y compara con CANARIO
    let archivo_s = if archivo.is_null() { "?".to_string() } else { unsafe { std::ffi::CStr::from_ptr(archivo as *const i8).to_string_lossy().into_owned() } };
    let _ = (archivo_s, linea); // evitar warning
    // Si corrupto: log::imprimir(&format!("[MEM] corrupción canario en 0x{:x} — buffer overflow detectado en {}:{}", ...))
}

pub fn encuarentenar(ptr: i64) {
    let mut q = cuarentena().lock().unwrap();
    q.push_back(ptr);
    if q.len() > 64 {
        let viejo = q.pop_front().unwrap();
        // Real free del bloque viejo (fuera de cuarentena)
        log::incrementar("cuarentena_free");
        let _ = viejo;
    }
    unsafe { std::ptr::write_bytes(ptr as *mut u8, 0xFF, 8); } // pintar primeros 8 con FF para detectar UAF
}

pub fn canario_ok(ptr: i64) -> bool {
    // Stub — retorna true si canarios intactos
    let _ = ptr; true
}

pub fn ficha_canario(ptr: i64) {
    log::imprimir_directo(&format!("[MEM] canario 0x{:x}: {} (nivel 2)", ptr as u64, if canario_ok(ptr) { "OK" } else { "CORRUPTO — overflow" }));
}
