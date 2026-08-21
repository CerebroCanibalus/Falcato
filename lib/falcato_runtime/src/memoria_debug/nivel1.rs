//! # Nivel 1 — Guardián (~250 LOC)
//! 
//! Responsabilidad ÚNICA: detectar lo fatal sin falsos positivos.
//! - doble_free
//! - free de puntero no alocado
//! - leak al salir (reporte final)
//! 
//! Sin canarios, sin pintura, sin timeline. Solo HashMap + Mutex.
//! Costo ~3%. Si este nivel falla, el bug está aquí — no en nivel2/3.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use super::log;

#[derive(Clone)]
struct Ficha {
    bytes: i64,
    archivo: String,
    linea: u32,
    liberado: bool,
    archivo_free: Option<String>,
    linea_free: Option<u32>,
}
static MAPA: OnceLock<Mutex<HashMap<i64, Ficha>>> = OnceLock::new();
static USADA: OnceLock<Mutex<i64>> = OnceLock::new();

fn mapa() -> &'static Mutex<HashMap<i64, Ficha>> {
    MAPA.get_or_init(|| Mutex::new(HashMap::new()))
}
fn usada() -> &'static Mutex<i64> {
    USADA.get_or_init(|| Mutex::new(0))
}

pub fn init() {
    let _ = mapa(); let _ = usada();
    log::imprimir("[MEM] nivel 1 — Guardián activo");
}

pub fn rastrear_alloc(ptr: i64, bytes: i64, archivo_p: *const u8, linea: u32) {
    if ptr == 0 { return; }
    let archivo = if archivo_p.is_null() { "desconocido".to_string() } else {
        unsafe { std::ffi::CStr::from_ptr(archivo_p as *const i8).to_string_lossy().into_owned() }
    };
    mapa().lock().unwrap().insert(ptr, Ficha { bytes, archivo: archivo.clone(), linea, liberado: false, archivo_free: None, linea_free: None });
    *usada().lock().unwrap() += bytes;
    log::incrementar(&format!("alloc {}b {}", bytes, archivo));
}

/// Retorna false si fue doble_free (para abortar cadena de niveles).
 pub fn rastrear_free(ptr: i64, archivo_p: *const u8, linea: u32) -> bool {
    if ptr == 0 { return true; }
    let archivo = if archivo_p.is_null() { "desconocido".to_string() } else {
        unsafe { std::ffi::CStr::from_ptr(archivo_p as *const i8).to_string_lossy().into_owned() }
    };
    let mut m = mapa().lock().unwrap();
    match m.get_mut(&ptr) {
        None => {
            log::imprimir(&format!("[MEM] free inválido en {}:{} — 0x{:x} nunca fue alocado", archivo, linea, ptr as u64));
            false
        }
        Some(f) if f.liberado => {
            log::imprimir(&format!(
                "[MEM] doble liberación en {}:{} — 0x{:x} ({}b) alocado en {}:{} liberado primero en {}:{}",
                archivo, linea, ptr as u64, f.bytes, f.archivo, f.linea,
                f.archivo_free.as_deref().unwrap_or("?"), f.linea_free.unwrap_or(0)
            ));
            false
        }
        Some(f) => {
            f.liberado = true;
            f.archivo_free = Some(archivo.clone());
            f.linea_free = Some(linea);
            *usada().lock().unwrap() -= f.bytes;
            true
        }
    }
}

pub fn memoria_usada() -> i64 { *usada().lock().unwrap() }

pub fn ficha(ptr: i64) {
    let m = mapa().lock().unwrap();
    if let Some(f) = m.get(&ptr) {
        log::imprimir_directo(&format!(
            "[MEM] ficha 0x{:x}: {}b alocado en {}:{}, estado: {}",
            ptr as u64, f.bytes, f.archivo, f.linea,
            if f.liberado { format!("liberado en {}:{}", f.archivo_free.as_deref().unwrap_or("?"), f.linea_free.unwrap_or(0)) } else { "vivo".to_string() }
        ));
    } else {
        log::imprimir_directo(&format!("[MEM] ficha 0x{:x}: no rastreado (nivel 1)", ptr as u64));
    }
}

/// Reporte de leaks al salir — llamado desde atexit.
 pub fn reporte_leaks() {
    let m = mapa().lock().unwrap();
    let vivos: Vec<_> = m.iter().filter(|(_, f)| !f.liberado).collect();
    if vivos.is_empty() { log::imprimir_directo("[MEM] sin leaks — 0 bloques vivos"); return; }
    log::imprimir_directo(&format!("[MEM] {} leaks detectados:", vivos.len()));
    // Agrupar por sitio para no spamear
    let mut por_sitio: HashMap<String, usize> = HashMap::new();
    for (_, f) in &vivos { *por_sitio.entry(format!("{}:{}", f.archivo, f.linea)).or_insert(0) += 1; }
    for (sitio, n) in por_sitio.iter().take(10) {
        log::imprimir_directo(&format!("  {} — {} bloques", sitio, n));
    }
    if por_sitio.len() > 10 { log::imprimir_directo(&format!("  ... y {} sitios más (ver --depurar-memoria=3)", por_sitio.len()-10)); }
}
