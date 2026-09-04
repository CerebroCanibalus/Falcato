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

/// Busca si `sub` aparece dentro de `desc_texto`.
/// Retorna 1 si contiene, 0 si no.
///
/// # Safety
/// - Ambos descriptores deben ser válidos
#[no_mangle]
pub unsafe extern "C" fn falcato_texto_contiene(desc_texto: i64, desc_sub: i64) -> i32 {
    if desc_texto == 0 || desc_sub == 0 {
        return 0;
    }

    let t_ptr = leer_campo(desc_texto, OFFSET_PTR) as *const u8;
    let t_len = leer_campo(desc_texto, OFFSET_LEN) as usize;
    let s_ptr = leer_campo(desc_sub, OFFSET_PTR) as *const u8;
    let s_len = leer_campo(desc_sub, OFFSET_LEN) as usize;

    if s_len == 0 {
        return 1; // Substring vacío siempre está contenido
    }
    if s_len > t_len {
        return 0;
    }

    // Búsqueda O(n*m) simple — suficiente para v0
    let limite = t_len - s_len;
    'outer: for i in 0..=limite {
        for j in 0..s_len {
            if *t_ptr.add(i + j) != *s_ptr.add(j) {
                continue 'outer;
            }
        }
        return 1; // Encontrado
    }
    0
}

/// Verifica si `desc_texto` empieza con `desc_prefijo`.
/// Retorna 1 si es así, 0 si no.
///
/// # Safety
/// - Descriptores deben ser válidos
#[no_mangle]
pub unsafe extern "C" fn falcato_texto_empieza_con(desc_texto: i64, desc_prefijo: i64) -> i32 {
    if desc_texto == 0 || desc_prefijo == 0 { return 0; }

    let t_ptr = leer_campo(desc_texto, OFFSET_PTR) as *const u8;
    let t_len = leer_campo(desc_texto, OFFSET_LEN) as usize;
    let p_ptr = leer_campo(desc_prefijo, OFFSET_PTR) as *const u8;
    let p_len = leer_campo(desc_prefijo, OFFSET_LEN) as usize;

    if p_len == 0 { return 1; } // prefijo vacío siempre coincide
    if p_len > t_len { return 0; }

    for i in 0..p_len {
        if *t_ptr.add(i) != *p_ptr.add(i) {
            return 0;
        }
    }
    1
}

/// Verifica si `desc_texto` termina con `desc_sufijo`.
/// Retorna 1 si es así, 0 si no.
///
/// # Safety
/// - Descriptores deben ser válidos
#[no_mangle]
pub unsafe extern "C" fn falcato_texto_termina_con(desc_texto: i64, desc_sufijo: i64) -> i32 {
    if desc_texto == 0 || desc_sufijo == 0 { return 0; }

    let t_ptr = leer_campo(desc_texto, OFFSET_PTR) as *const u8;
    let t_len = leer_campo(desc_texto, OFFSET_LEN) as usize;
    let s_ptr = leer_campo(desc_sufijo, OFFSET_PTR) as *const u8;
    let s_len = leer_campo(desc_sufijo, OFFSET_LEN) as usize;

    if s_len == 0 { return 1; }
    if s_len > t_len { return 0; }

    let start = t_len - s_len;
    for i in 0..s_len {
        if *t_ptr.add(start + i) != *s_ptr.add(i) {
            return 0;
        }
    }
    1
}

/// Convierte texto a vector de bytes.
/// Escribe cada byte como un Entero8 en el vector de salida.
///
/// # Safety
/// - Descriptores deben ser válidos
#[no_mangle]
pub unsafe extern "C" fn falcato_texto_a_bytes(desc_texto: i64, desc_out: i64) {
    if desc_texto == 0 || desc_out == 0 { return; }

    let t_ptr = leer_campo(desc_texto, OFFSET_PTR) as *const u8;
    let t_len = leer_campo(desc_texto, OFFSET_LEN) as usize;

    // Vector de bytes: cada elemento es 1 byte (Entero8)
    // Layout del vector: ptr -> array de i8, len, cap
    let array_size = t_len;
    let array_ptr = malloc(array_size);
    if array_ptr.is_null() { return; }

    if t_len > 0 && !t_ptr.is_null() {
        memcpy(array_ptr, t_ptr as *const c_void, t_len);
    }

    let old_ptr = leer_campo(desc_out, OFFSET_PTR);
    if old_ptr != 0 { free(old_ptr as *mut c_void); }

    escribir_campo(desc_out, OFFSET_PTR, array_ptr as i64);
    escribir_campo(desc_out, OFFSET_LEN, t_len as i64);
    escribir_campo(desc_out, OFFSET_CAP, t_len as i64);
}

/// Codifica bytes a Base64.
///
/// # Safety
/// - Descriptores deben ser válidos
#[no_mangle]
pub unsafe extern "C" fn falcato_texto_codificar_base64(desc_texto: i64, desc_out: i64) {
    if desc_texto == 0 || desc_out == 0 { return; }

    let t_ptr = leer_campo(desc_texto, OFFSET_PTR) as *const u8;
    let t_len = leer_campo(desc_texto, OFFSET_LEN) as usize;

    if t_ptr.is_null() || t_len == 0 {
        texto_desde_buffer(b"", desc_out);
        return;
    }

    let input = core::slice::from_raw_parts(t_ptr, t_len);
    let mut output = Vec::with_capacity((t_len + 2) / 3 * 4);

    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };

        let triple = (b0 << 16) | (b1 << 8) | b2;

        output.push(TABLE[(triple >> 18 & 0x3F) as usize]);
        output.push(TABLE[(triple >> 12 & 0x3F) as usize]);

        if chunk.len() > 1 {
            output.push(TABLE[(triple >> 6 & 0x3F) as usize]);
        } else {
            output.push(b'=');
        }

        if chunk.len() > 2 {
            output.push(TABLE[(triple & 0x3F) as usize]);
        } else {
            output.push(b'=');
        }
    }

    texto_desde_buffer(&output, desc_out);
}

/// Decodifica Base64 a bytes.
///
/// # Safety
/// - Descriptores deben ser válidos
#[no_mangle]
pub unsafe extern "C" fn falcato_texto_decodificar_base64(desc_texto: i64, desc_out: i64) {
    if desc_texto == 0 || desc_out == 0 { return; }

    let t_ptr = leer_campo(desc_texto, OFFSET_PTR) as *const u8;
    let t_len = leer_campo(desc_texto, OFFSET_LEN) as usize;

    if t_ptr.is_null() || t_len == 0 {
        texto_desde_buffer(b"", desc_out);
        return;
    }

    let input = core::slice::from_raw_parts(t_ptr, t_len);

    // Build decode table
    let mut table = [0u8; 256];
    for (i, &c) in b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/".iter().enumerate() {
        table[c as usize] = i as u8;
    }

    let mut output = Vec::with_capacity(t_len * 3 / 4);
    let mut buf: u32 = 0;
    let mut bits: i32 = 0;

    for &c in input {
        if c == b'=' {
            break;
        }
        if c == b' ' || c == b'\n' || c == b'\r' || c == b'\t' {
            continue;
        }
        let val = table[c as usize];
        buf = (buf << 6) | val as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            output.push((buf >> bits) as u8);
        }
    }

    texto_desde_buffer(&output, desc_out);
}

/// Divide `desc_texto` por `desc_sep` y escribe el resultado en `desc_vector_out`.
/// `desc_vector_out` debe ser un descriptor de Vector ya inicializado (vector_nuevo).
///
/// Layout del Vector descriptor (coincide con codegen):
/// - offset 0: ptr (i64) — puntero a array de punteros
/// - offset 8: len (i64) — cantidad actual
/// - offset 16: cap (i64) — capacidad
///
/// # Safety
/// - Todos los descriptores deben ser válidos
#[no_mangle]
pub unsafe extern "C" fn falcato_texto_dividir(desc_texto: i64, desc_sep: i64, desc_vector_out: i64) {
    if desc_texto == 0 || desc_sep == 0 || desc_vector_out == 0 {
        return;
    }

    let t_ptr = leer_campo(desc_texto, OFFSET_PTR) as *const u8;
    let t_len = leer_campo(desc_texto, OFFSET_LEN) as usize;
    let s_ptr = leer_campo(desc_sep, OFFSET_PTR) as *const u8;
    let s_len = leer_campo(desc_sep, OFFSET_LEN) as usize;

    // Separador vacío: devolver cada carácter como elemento
    if s_len == 0 {
        // TODO: implementar split por caracteres
        return;
    }

    // Vector descriptor offsets (mismos que Texto: ptr, len, cap)
    const VEC_OFFSET_PTR: isize = 0;
    const VEC_OFFSET_LEN: isize = 8;
    const VEC_OFFSET_CAP: isize = 16;
    const TAM_PUNTERO: isize = 8; // puntero de 64 bits

    // Contar ocurrencias del separador
    let mut count: usize = 0;
    let mut i: usize = 0;
    while i <= t_len {
        if i + s_len <= t_len {
            let mut coincide = true;
            for j in 0..s_len {
                if *t_ptr.add(i + j) != *s_ptr.add(j) {
                    coincide = false;
                    break;
                }
            }
            if coincide {
                count += 1;
                i += s_len;
                continue;
            }
        }
        i += 1;
    }
    // count = número de separadores. Partes = count + 1 (si hay al menos 1 separador)
    // Si no hay separadores, 1 parte (el texto completo)
    let num_partes = if count == 0 { 1 } else { count + 1 };

    // Allocar array de punteros a descriptores de Texto
    let array_size = (num_partes as isize) * TAM_PUNTERO;
    let array_ptr = malloc(array_size as usize);
    if array_ptr.is_null() { return; }

    // Para cada parte, crear un descriptor de Texto en el heap
    for idx in 0..num_partes {
        // Encontrar inicio y fin de esta parte
        let (part_start, part_end) = if idx == 0 {
            // Primera parte: desde el inicio hasta el primer separador (o todo si no hay)
            let mut pos = 0;
            let mut found = false;
            while pos + s_len <= t_len {
                let mut coincide = true;
                for j in 0..s_len {
                    if *t_ptr.add(pos + j) != *s_ptr.add(j) {
                        coincide = false;
                        break;
                    }
                }
                if coincide {
                    found = true;
                    break;
                }
                pos += 1;
            }
            if found { (0, pos) } else { (0, t_len) }
        } else {
            // Partes siguientes: buscar desde el final de la parte anterior + separador
            // Necesitamos recorrer de nuevo
            let mut pos = 0;
            let mut parte_actual = 0;
            let mut found_start = false;
            let mut start = 0;

            while pos + s_len <= t_len {
                let mut coincide = true;
                for j in 0..s_len {
                    if *t_ptr.add(pos + j) != *s_ptr.add(j) {
                        coincide = false;
                        break;
                    }
                }
                if coincide {
                    if parte_actual == idx - 1 {
                        start = pos + s_len;
                        found_start = true;
                    }
                    if parte_actual == idx {
                        return; // No debería llegar aquí
                    }
                    parte_actual += 1;
                    pos += s_len;
                    continue;
                }
                pos += 1;
            }

            if found_start {
                // Fin: buscar el siguiente separador o el final
                let mut end = start;
                while end + s_len <= t_len {
                    let mut coincide = true;
                    for j in 0..s_len {
                        if *t_ptr.add(end + j) != *s_ptr.add(j) {
                            coincide = false;
                            break;
                        }
                    }
                    if coincide { break; }
                    end += 1;
                }
                (start, end)
            } else {
                (0, 0) // No debería pasar
            }
        };

        let part_len = part_end - part_start;

        // Crear descriptor de Texto para esta parte
        let desc_ptr = malloc(24); // 3 campos de 8 bytes
        if desc_ptr.is_null() { continue; }

        // Allocar buffer para el contenido
        let buf_cap = part_len + 1;
        let buf = malloc(buf_cap);
        if buf.is_null() {
            free(desc_ptr);
            continue;
        }

        // Copiar datos
        if part_len > 0 {
            memcpy(buf, t_ptr.add(part_start) as *const c_void, part_len);
        }
        *(buf as *mut u8).add(part_len) = 0; // null terminator

        // Escribir descriptor
        escribir_campo(desc_ptr as i64, OFFSET_PTR, buf as i64);
        escribir_campo(desc_ptr as i64, OFFSET_LEN, part_len as i64);
        escribir_campo(desc_ptr as i64, OFFSET_CAP, buf_cap as i64);

        // Guardar puntero al descriptor en el array
        *(array_ptr as *mut i64).add(idx) = desc_ptr as i64;
    }

    // Actualizar descriptor del vector
    // Si el vector ya tiene datos, liberarlos primero
    let vec_len = leer_campo(desc_vector_out, VEC_OFFSET_LEN);
    if vec_len > 0 {
        let old_array = leer_campo(desc_vector_out, VEC_OFFSET_PTR) as *mut i64;
        for i in 0..vec_len as usize {
            let desc = *old_array.add(i);
            if desc != 0 {
                let ptr = leer_campo(desc, OFFSET_PTR);
                if ptr != 0 { free(ptr as *mut c_void); }
                free(desc as *mut c_void);
            }
        }
        free(old_array as *mut c_void);
    }

    escribir_campo(desc_vector_out, VEC_OFFSET_PTR, array_ptr as i64);
    escribir_campo(desc_vector_out, VEC_OFFSET_LEN, num_partes as i64);
    escribir_campo(desc_vector_out, VEC_OFFSET_CAP, num_partes as i64);
}

/// Reemplaza todas las ocurrencias de `desc_de` por `desc_a` en `desc_texto`.
/// Escribe el resultado en `desc_out` (nuevo descriptor).
///
/// # Safety
/// - Todos los descriptores deben ser válidos
#[no_mangle]
pub unsafe extern "C" fn falcato_texto_reemplazar(desc_texto: i64, desc_de: i64, desc_a: i64, desc_out: i64) {
    if desc_texto == 0 || desc_de == 0 || desc_a == 0 || desc_out == 0 {
        return;
    }

    let t_ptr = leer_campo(desc_texto, OFFSET_PTR) as *const u8;
    let t_len = leer_campo(desc_texto, OFFSET_LEN) as usize;
    let de_ptr = leer_campo(desc_de, OFFSET_PTR) as *const u8;
    let de_len = leer_campo(desc_de, OFFSET_LEN) as usize;
    let a_ptr = leer_campo(desc_a, OFFSET_PTR) as *const u8;
    let a_len = leer_campo(desc_a, OFFSET_LEN) as usize;

    if de_len == 0 || t_len == 0 {
        // Sin reemplazo, copiar original
        let cap = t_len + 1;
        let nuevo_ptr = malloc(cap);
        if nuevo_ptr.is_null() { return; }
        if t_len > 0 {
            memcpy(nuevo_ptr, t_ptr as *const c_void, t_len);
        }
        *(nuevo_ptr as *mut u8).add(t_len) = 0;
        escribir_campo(desc_out, OFFSET_PTR, nuevo_ptr as i64);
        escribir_campo(desc_out, OFFSET_LEN, t_len as i64);
        escribir_campo(desc_out, OFFSET_CAP, cap as i64);
        return;
    }

    // Primera pasada: contar ocurrencias
    let mut ocurrencias: usize = 0;
    let mut i: usize = 0;
    while i <= t_len - de_len {
        let mut coincide = true;
        for j in 0..de_len {
            if *t_ptr.add(i + j) != *de_ptr.add(j) {
                coincide = false;
                break;
            }
        }
        if coincide {
            ocurrencias += 1;
            i += de_len;
        } else {
            i += 1;
        }
    }

    // Calcular nueva longitud
    let nueva_len = t_len + ocurrencias * (a_len.saturating_sub(de_len));
    let cap = nueva_len + 1;
    let nuevo_ptr = malloc(cap);
    if nuevo_ptr.is_null() { return; }

    // Segunda pasada: copiar con reemplazos
    let mut dst: usize = 0;
    let mut src: usize = 0;
    while src < t_len {
        if src <= t_len - de_len {
            let mut coincide = true;
            for j in 0..de_len {
                if *t_ptr.add(src + j) != *de_ptr.add(j) {
                    coincide = false;
                    break;
                }
            }
            if coincide {
                if a_len > 0 {
                    memcpy(nuevo_ptr.add(dst) as *mut c_void, a_ptr as *const c_void, a_len);
                }
                dst += a_len;
                src += de_len;
                continue;
            }
        }
        *(nuevo_ptr as *mut u8).add(dst) = *t_ptr.add(src);
        dst += 1;
        src += 1;
    }
    *(nuevo_ptr as *mut u8).add(dst) = 0;

    escribir_campo(desc_out, OFFSET_PTR, nuevo_ptr as i64);
    escribir_campo(desc_out, OFFSET_LEN, dst as i64);
    escribir_campo(desc_out, OFFSET_CAP, cap as i64);
}

/// Convierte todo el texto a mayúsculas (ASCII). Escribe en `desc_out`.
///
/// # Safety
/// - Descriptores deben ser válidos
#[no_mangle]
pub unsafe extern "C" fn falcato_texto_mayusculas(desc_texto: i64, desc_out: i64) {
    if desc_texto == 0 || desc_out == 0 { return; }

    let t_ptr = leer_campo(desc_texto, OFFSET_PTR) as *const u8;
    let t_len = leer_campo(desc_texto, OFFSET_LEN) as usize;
    let cap = t_len + 1;
    let nuevo_ptr = malloc(cap);
    if nuevo_ptr.is_null() { return; }

    let dst = nuevo_ptr as *mut u8;
    for i in 0..t_len {
        let b = *t_ptr.add(i);
        if b >= b'a' && b <= b'z' {
            *dst.add(i) = b - 32; // ASCII: a-z -> A-Z
        } else {
            *dst.add(i) = b;
        }
    }
    *dst.add(t_len) = 0;

    escribir_campo(desc_out, OFFSET_PTR, nuevo_ptr as i64);
    escribir_campo(desc_out, OFFSET_LEN, t_len as i64);
    escribir_campo(desc_out, OFFSET_CAP, cap as i64);
}

/// Convierte todo el texto a minúsculas (ASCII). Escribe en `desc_out`.
///
/// # Safety
/// - Descriptores deben ser válidos
#[no_mangle]
pub unsafe extern "C" fn falcato_texto_minusculas(desc_texto: i64, desc_out: i64) {
    if desc_texto == 0 || desc_out == 0 { return; }

    let t_ptr = leer_campo(desc_texto, OFFSET_PTR) as *const u8;
    let t_len = leer_campo(desc_texto, OFFSET_LEN) as usize;
    let cap = t_len + 1;
    let nuevo_ptr = malloc(cap);
    if nuevo_ptr.is_null() { return; }

    let dst = nuevo_ptr as *mut u8;
    for i in 0..t_len {
        let b = *t_ptr.add(i);
        if b >= b'A' && b <= b'Z' {
            *dst.add(i) = b + 32; // ASCII: A-Z -> a-z
        } else {
            *dst.add(i) = b;
        }
    }
    *dst.add(t_len) = 0;

    escribir_campo(desc_out, OFFSET_PTR, nuevo_ptr as i64);
    escribir_campo(desc_out, OFFSET_LEN, t_len as i64);
    escribir_campo(desc_out, OFFSET_CAP, cap as i64);
}

/// Recorta espacios en blanco al inicio y final. Escribe en `desc_out`.
///
/// # Safety
/// - Descriptores deben ser válidos
#[no_mangle]
pub unsafe extern "C" fn falcato_texto_recortar(desc_texto: i64, desc_out: i64) {
    if desc_texto == 0 || desc_out == 0 { return; }

    let t_ptr = leer_campo(desc_texto, OFFSET_PTR) as *const u8;
    let t_len = leer_campo(desc_texto, OFFSET_LEN) as usize;

    // Buscar inicio (primer no-space)
    let mut inicio: usize = 0;
    while inicio < t_len && (*t_ptr.add(inicio) == b' ' || *t_ptr.add(inicio) == b'\t' || *t_ptr.add(inicio) == b'\n' || *t_ptr.add(inicio) == b'\r') {
        inicio += 1;
    }

    // Buscar fin (último no-space)
    let mut fin: usize = t_len;
    while fin > inicio && (*t_ptr.add(fin - 1) == b' ' || *t_ptr.add(fin - 1) == b'\t' || *t_ptr.add(fin - 1) == b'\n' || *t_ptr.add(fin - 1) == b'\r') {
        fin -= 1;
    }

    let nueva_len = fin - inicio;
    let cap = nueva_len + 1;
    let nuevo_ptr = malloc(cap);
    if nuevo_ptr.is_null() { return; }

    if nueva_len > 0 {
        memcpy(nuevo_ptr, t_ptr.add(inicio) as *const c_void, nueva_len);
    }
    *(nuevo_ptr as *mut u8).add(nueva_len) = 0;

    escribir_campo(desc_out, OFFSET_PTR, nuevo_ptr as i64);
    escribir_campo(desc_out, OFFSET_LEN, nueva_len as i64);
    escribir_campo(desc_out, OFFSET_CAP, cap as i64);
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
