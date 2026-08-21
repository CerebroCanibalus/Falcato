//! # Memoria Debug — Lente graduable (Niveles 0-3)
//! 
//! Filosofía: Falcato es lenguaje de sistemas. La memoria no se esconde,
//! se expone con zoom que el usuario elige.
//!
//! ```
//! falcato compila app.fc                          # nivel 0 — Silencio (0 costo)
//! falcato compila app.fc --depurar-memoria        # nivel 1 — Guardián
//! falcato compila app.fc --depurar-memoria=2      # nivel 2 — Cirujano
//! falcato compila app.fc --depurar-memoria=3      # nivel 3 — Enfermizo
//! ```
//!
//! Cada nivel es un módulo (<400 LOC) con responsabilidad única.
//! Los bugs se arreglan sin tocar los otros niveles.

pub mod log;
pub mod nivel1;
pub mod nivel2;
pub mod nivel3;

use std::sync::atomic::{AtomicU8, Ordering};

/// Nivel global — 0..3. 0 = sin instrumentación.
static NIVEL: AtomicU8 = AtomicU8::new(0);

/// Inicializa el nivel desde env var `FALCATO_MEMORIA` o flag `--depurar-memoria`.
/// Llamado por `falcato_memoria_init` desde el prólogo de `principal`.
#[no_mangle]
pub unsafe extern "C" fn falcato_memoria_init(nivel: u8) {
    let n = nivel.min(3);
    NIVEL.store(n, Ordering::SeqCst);
    if n >= 1 { nivel1::init(); }
    if n >= 2 { nivel2::init(); }
    if n >= 3 { nivel3::init(); }
}

/// Nivel actual (0..3). Lectura sin lock.
#[inline]
pub fn nivel_actual() -> u8 { NIVEL.load(Ordering::Relaxed) }

/// Helpers expuestos al codegen — file:line vienen del Span del compiler.
#[no_mangle]
pub unsafe extern "C" fn falcato_memoria_rastrear_alloc(ptr: i64, bytes: i64, archivo: *const u8, linea: u32) {
    let n = nivel_actual();
    if n >= 1 { nivel1::rastrear_alloc(ptr, bytes, archivo, linea); }
    if n >= 2 { nivel2::pintar_alloc(ptr, bytes); }
    if n >= 3 { nivel3::timeline_alloc(ptr, bytes, archivo, linea); }
}

#[no_mangle]
pub unsafe extern "C" fn falcato_memoria_rastrear_free(ptr: i64, archivo: *const u8, linea: u32) {
    let n = nivel_actual();
    if n >= 1 { let ok = nivel1::rastrear_free(ptr, archivo, linea); if !ok { return; } }
    if n >= 2 { nivel2::verificar_canario(ptr, archivo, linea); nivel2::encuarentenar(ptr); }
    if n >= 3 { nivel3::timeline_free(ptr, archivo, linea); }
}

// Builtins de inspección — siempre disponibles, más detalle con nivel alto.
#[no_mangle]
pub unsafe extern "C" fn falcato_memoria_usada() -> i64 { nivel1::memoria_usada() }

#[no_mangle]
pub unsafe extern "C" fn falcato_memoria_volcar(ptr: i64, n: i32) {
    let nivel = nivel_actual();
    nivel3::volcar(ptr, n, nivel);
}

#[no_mangle]
pub unsafe extern "C" fn falcato_memoria_rastrear(ptr: i64) {
    let nivel = nivel_actual();
    if nivel == 0 {
        log::imprimir("[MEM] sin traza — compila con --depurar-memoria=2 para ver ficha completa");
        return;
    }
    nivel1::ficha(ptr);
    if nivel >= 2 { nivel2::ficha_canario(ptr); }
    if nivel >= 3 { nivel3::ficha_timeline(ptr); }
}

#[no_mangle]
pub unsafe extern "C" fn falcato_memoria_canario_verificar(ptr: i64) -> i32 {
    if nivel_actual() < 2 { return -1; } // sin canarios en nivel 0/1
    if nivel2::canario_ok(ptr) { 1 } else { 0 }
}
