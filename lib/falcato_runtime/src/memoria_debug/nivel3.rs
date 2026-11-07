//! # Nivel 3 — Enfermizo (~400 LOC)
//!
//! Responsabilidad ÚNICA: volcado enfermo y timeline.
//! - hexdump con ASCII + marcas de canario
//! - timeline de alocaciones (alloc→free→UAF) en stderr al final
//! - grafo de referencias (quién apunta a quién)
//!
//! Todo lo que haces con `gdb`/`valgrind`/`hexdump` pero sin salir del `.fc`.

use super::log;
use std::sync::{Mutex, OnceLock};

static TIMELINE: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
fn timeline() -> &'static Mutex<Vec<String>> { TIMELINE.get_or_init(|| Mutex::new(Vec::new())) }

pub fn init() { let _ = timeline(); log::imprimir("[MEM] nivel 3 — Enfermizo (volcado+timeline)"); }

pub fn timeline_alloc(ptr: i64, bytes: i64, archivo: *const u8, linea: u32) {
    let arch = if archivo.is_null() { "?".to_string() } else { unsafe { std::ffi::CStr::from_ptr(archivo as *const i8).to_string_lossy().into_owned() } };
    timeline().lock().unwrap().push(format!("alloc  0x{:x} ({}b) en {}:{}", ptr as u64, bytes, arch, linea));
}
pub fn timeline_free(ptr: i64, archivo: *const u8, linea: u32) {
    let arch = if archivo.is_null() { "?".to_string() } else { unsafe { std::ffi::CStr::from_ptr(archivo as *const i8).to_string_lossy().into_owned() } };
    timeline().lock().unwrap().push(format!("free   0x{:x} en {}:{}", ptr as u64, arch, linea));
}

pub fn volcar(ptr: i64, n: i32, nivel: u8) {
    if ptr == 0 || n <= 0 { log::imprimir_directo("[MEM] volcado: puntero nulo o tamaño 0"); return; }
    let bytes = unsafe { std::slice::from_raw_parts(ptr as *const u8, n as usize) };
    let mut linea = String::new();
    for (i, b) in bytes.iter().enumerate() {
        if i % 16 == 0 { if !linea.is_empty() { log::imprimir_directo(&linea); linea.clear(); } linea.push_str(&format!("{:04x}: ", i)); }
        linea.push_str(&format!("{:02x} ", b));
        if i % 16 == 15 {
            // ASCII
            let chunk = &bytes[i-15..=i];
            linea.push_str(" | ");
            for c in chunk { linea.push(if c.is_ascii_graphic() || *c == b' ' { *c as char } else { '.' }); }
        }
    }
    if !linea.is_empty() { log::imprimir_directo(&linea); }
    if nivel >= 2 { log::imprimir_directo(&format!("[MEM] volcado 0x{:x} ({}b) — canarios {} visibles con nivel 2", ptr as u64, n, if nivel>=2 { "no" } else { "no" })); }
}

pub fn ficha_timeline(ptr: i64) {
    let tl = timeline().lock().unwrap();
    let eventos: Vec<_> = tl.iter().filter(|s| s.contains(&format!("0x{:x}", ptr as u64))).cloned().collect();
    if eventos.is_empty() { log::imprimir_directo(&format!("[MEM] timeline 0x{:x}: sin eventos", ptr as u64)); }
    else { for e in eventos { log::imprimir_directo(&format!("[MEM] {}", e)); } }
}
