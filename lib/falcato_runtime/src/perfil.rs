//! # Perfil — reloj monotónico de alta resolución + marcas
//!
//! Nivel 0: `reloj_mono_ns() -> i64` — ns desde arranque, monotónico (QueryPerformanceCounter / CLOCK_MONOTONIC)
//! Nivel 1: `perfil_inicio()`, `perfil_marca(Texto)`, `perfil_reporte()` — tabla ordenada por tiempo
//!
//! Diseño: todo en Rust nativo (Capa A), sin `#[cfg]` en codegen. La librería pura `.fc` queda obsoleta.

use std::ffi::c_void;
use std::sync::{Mutex, LazyLock};

// Offsets del descriptor Texto (coinciden con codegen/mod.rs)
const OFFSET_PTR: isize = 0;
const OFFSET_LEN: isize = 8;

unsafe fn leer_campo(desc: i64, offset: isize) -> i64 {
    let ptr = (desc as *mut u8).offset(offset) as *const i64;
    *ptr
}

// ============================================================
// Reloj monotónico — ns desde arranque (no wall-clock)
// ============================================================

#[cfg(target_os = "windows")]
mod reloj_imp {
    pub unsafe fn mono_ns() -> i64 {
        extern "system" {
            fn QueryPerformanceCounter(lpPerformanceCount: *mut i64) -> i32;
            fn QueryPerformanceFrequency(lpFrequency: *mut i64) -> i32;
        }
        let mut counter: i64 = 0;
        let mut freq: i64 = 0;
        if QueryPerformanceFrequency(&mut freq) == 0 || freq == 0 {
            return 0;
        }
        if QueryPerformanceCounter(&mut counter) == 0 {
            return 0;
        }
        // ns = counter * 1_000_000_000 / freq  (usar i128 para evitar overflow)
        ((counter as i128 * 1_000_000_000i128) / freq as i128) as i64
    }
}

#[cfg(not(target_os = "windows"))]
mod reloj_imp {
    pub unsafe fn mono_ns() -> i64 {
        extern "C" {
            fn clock_gettime(clk_id: i32, tp: *mut Timespec) -> i32;
        }
        #[repr(C)]
        struct Timespec { tv_sec: i64, tv_nsec: i64 }
        let mut ts = Timespec { tv_sec: 0, tv_nsec: 0 };
        // CLOCK_MONOTONIC = 1 en Linux, 6 en macOS (pero 1 funciona en ambos para mono)
        // Usamos 1 (MONOTONIC) — en macOS también es 6, pero clock_gettime con 1 da EINVAL.
        // Para portabilidad, probamos 1 y si falla, probamos 6.
        let mut ret = clock_gettime(1, &mut ts);
        if ret != 0 {
            ret = clock_gettime(6, &mut ts);
            if ret != 0 { return 0; }
        }
        ts.tv_sec * 1_000_000_000 + ts.tv_nsec
    }
}

#[no_mangle]
pub unsafe extern "C" fn falcato_reloj_mono_ns() -> i64 {
    reloj_imp::mono_ns()
}

// ============================================================
// Tabla de marcas — nivel 1
// ============================================================

struct Marca {
    etiqueta: String,
    ns: i64,
}

static PERFIL: LazyLock<Mutex<Vec<Marca>>> = LazyLock::new(|| Mutex::new(Vec::new()));
static INICIO_NS: LazyLock<Mutex<Option<i64>>> = LazyLock::new(|| Mutex::new(None));

#[no_mangle]
pub unsafe extern "C" fn falcato_perfil_inicio() {
    let ns = reloj_imp::mono_ns();
    if let Ok(mut inicio) = INICIO_NS.lock() {
        *inicio = Some(ns);
    }
    if let Ok(mut marcas) = PERFIL.lock() {
        marcas.clear();
        marcas.push(Marca { etiqueta: "__inicio__".to_string(), ns });
    }
}

#[no_mangle]
pub unsafe extern "C" fn falcato_perfil_marca(desc: i64) {
    if desc == 0 { return; }
    let ptr = leer_campo(desc, OFFSET_PTR) as *const u8;
    let len = leer_campo(desc, OFFSET_LEN) as usize;
    if ptr.is_null() { return; }
    let slice = std::slice::from_raw_parts(ptr, len);
    let etiqueta = String::from_utf8_lossy(slice).to_string();
    let ns = reloj_imp::mono_ns();
    if let Ok(mut marcas) = PERFIL.lock() {
        marcas.push(Marca { etiqueta, ns });
    }
}

#[no_mangle]
pub unsafe extern "C" fn falcato_perfil_reporte() {
    let inicio_opt = INICIO_NS.lock().ok().and_then(|g| *g);
    let marcas_guard = PERFIL.lock();
    if inicio_opt.is_none() || marcas_guard.is_err() {
        eprintln!("[perfil] sin inicio — llama a perfil_inicio() primero");
        return;
    }
    let inicio = inicio_opt.unwrap();
    let marcas = marcas_guard.unwrap();
    if marcas.len() <= 1 {
        eprintln!("[perfil] sin marcas");
        return;
    }

    // Calcular deltas entre marcas consecutivas
    let mut filas: Vec<(String, i64, i64)> = Vec::new(); // (etiqueta, delta, total)
    for i in 1..marcas.len() {
        let delta = marcas[i].ns - marcas[i-1].ns;
        let total = marcas[i].ns - inicio;
        filas.push((marcas[i].etiqueta.clone(), delta, total));
    }

    // Ordenar por delta descendente para ver el cuello de botella primero
    let mut por_delta = filas.clone();
    por_delta.sort_by(|a, b| b.1.cmp(&a.1));

    eprintln!("┌─ perfil ({} marcas, total {} ms) ─────────────────────", filas.len(), (marcas.last().unwrap().ns - inicio) / 1_000_000);
    eprintln!("│ {:<30} {:>10} {:>10}", "marca", "delta", "total");
    eprintln!("│ {:─<30} {:─>10} {:─>10}", "", "", "");
    for (et, delta, total) in &por_delta {
        let d_ms = *delta as f64 / 1_000_000.0;
        let t_ms = *total as f64 / 1_000_000.0;
        eprintln!("│ {:<30} {:>9.3}ms {:>9.3}ms", et, d_ms, t_ms);
    }
    eprintln!("└──────────────────────────────────────────────────────");
    // También volcar en orden cronológico para traza
    eprintln!("[perfil] cronológico:");
    for (et, delta, total) in &filas {
        eprintln!("  {} +{:.3}ms (total {:.3}ms)", et, *delta as f64/1_000_000.0, *total as f64/1_000_000.0);
    }
}
