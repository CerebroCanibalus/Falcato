//! # Shims de impresión con ABI fija (macOS / Apple AArch64)
//!
//! En Apple Silicon los argumentos variádicos de una función (`printf(fmt, ...)`)
//! se pasan **por la pila**, no por registros. Cranelift no modela funciones
//! variádicas: el codegen declara `printf` como una función normal `(i64, i64)`
//! y los args van en `x1`/`x2`, así que la libc de macOS lee pila sin inicializar
//! → basura o SIGSEGV.
//!
//! Solución: el codegen sigue llamando `printf`; el registry de macOS remapea el
//! nombre a estos shims, que tienen firma fija (no variádica) y por dentro llaman
//! a `printf` desde Rust — rustc/LLVM sí emiten la ABI variádica correcta.
//!
//! Solo se remapean los formatos que el codegen genera:
//! `%d`, `%lld`, `%u`, `%llu`, `%c`, `%s`, `%f`, `%.17g`.

use std::ffi::c_char;

extern "C" {
    fn printf(fmt: *const c_char, ...) -> i32;
}

/// Conversión detectada en el string de formato.
#[derive(Clone, Copy, PartialEq)]
enum Conv {
    Texto,
    Entero32,
    Entero64,
    Natural32,
    Natural64,
    Caracter,
    Flotante64,
    Desconocida,
}

/// Extrae la conversión del printf-format. Los formatos vienen de literales
/// creados por el codegen, pero nunca se asume: si no se reconoce, se trata el
/// valor como entero de 64 bits.
unsafe fn clasificar(fmt: *const c_char) -> Conv {
    if fmt.is_null() {
        return Conv::Desconocida;
    }
    let bytes = std::slice::from_raw_parts(fmt as *const u8, 32);
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] != b'%' {
            if bytes[i] == 0 {
                return Conv::Desconocida;
            }
            i += 1;
            continue;
        }
        i += 1;
        // Saltar flags/ancho/precisión: - + espacio # 0 dígitos . dígitos
        while i < bytes.len() {
            match bytes[i] {
                b'-' | b'+' | b' ' | b'#' | b'0' | b'.' | b'1'..=b'9' | b'\'' => i += 1,
                _ => break,
            }
        }
        // Modificadores de longitud: l, ll, h, hh, z, j, t
        let mut es_largo = false;
        while i < bytes.len() && matches!(bytes[i], b'l' | b'h' | b'z' | b'j' | b't') {
            if bytes[i] == b'l' {
                es_largo = true;
            }
            i += 1;
        }
        if i >= bytes.len() {
            return Conv::Desconocida;
        }
        return match bytes[i] {
            b's' => Conv::Texto,
            b'c' => Conv::Caracter,
            b'f' | b'F' | b'e' | b'E' | b'g' | b'G' | b'a' | b'A' => Conv::Flotante64,
            b'd' | b'i' => {
                if es_largo {
                    Conv::Entero64
                } else {
                    Conv::Entero32
                }
            }
            b'u' => {
                if es_largo {
                    Conv::Natural64
                } else {
                    Conv::Natural32
                }
            }
            b'p' | b'x' | b'X' | b'o' => Conv::Entero64,
            _ => Conv::Desconocida,
        };
    }
    Conv::Desconocida
}

/// Reemplazo no variádico de `printf(fmt, val)` para valores enteros/texto.
///
/// Firma fija `(i64, i64) -> i32`: el valor viaja como bits de 64 bits y se
/// reinterpreta según la conversión del formato.
#[no_mangle]
pub unsafe extern "C" fn falcato_printf(fmt: *const c_char, bits: u64) -> i32 {
    match clasificar(fmt) {
        Conv::Texto => printf(fmt, bits as *const c_char),
        Conv::Entero32 => printf(fmt, bits as i32),
        Conv::Entero64 => printf(fmt, bits as i64),
        Conv::Natural32 => printf(fmt, bits as u32),
        Conv::Natural64 => printf(fmt, bits as u64),
        Conv::Caracter => printf(fmt, bits as i32),
        Conv::Flotante64 => printf(fmt, f64::from_bits(bits)),
        Conv::Desconocida => printf(fmt, bits as i64),
    }
}

/// Reemplazo no variádico de `printf(fmt, val)` para valores flotantes.
///
/// El codegen POSIX pasa los double por registros FP (`F64`), así que la firma
/// mantiene ese tipo en vez de bits.
#[no_mangle]
pub unsafe extern "C" fn falcato_printf_f64(fmt: *const c_char, val: f64) -> i32 {
    printf(fmt, val)
}
