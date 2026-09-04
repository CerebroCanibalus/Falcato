//! # Diccionario — operaciones de extracción y limpieza
//!
//! Extiende los builtins de diccionario existentes con claves(), valores(), limpiar().
//! Usa el mismo layout que Vector (ptr, len, cap) con buckets de diccionario.

use std::ffi::c_void;

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
}

const OFFSET_PTR: isize = 0;
const OFFSET_LEN: isize = 8;
const OFFSET_CAP: isize = 16;

unsafe fn leer_campo(desc: i64, offset: isize) -> i64 {
    let ptr = (desc as *mut u8).offset(offset) as *const i64;
    *ptr
}

unsafe fn escribir_campo(desc: i64, offset: isize, valor: i64) {
    let ptr = (desc as *mut u8).offset(offset) as *mut i64;
    *ptr = valor;
}

unsafe fn texto_desde_buffer(data: &[u8], desc_out: i64) {
    let len = data.len();
    let cap = len + 1;
    let ptr = malloc(cap);
    if ptr.is_null() { return; }
    if len > 0 {
        memcpy(ptr, data.as_ptr() as *const c_void, len);
    }
    *(ptr as *mut u8).add(len) = 0;
    escribir_campo(desc_out, OFFSET_PTR, ptr as i64);
    escribir_campo(desc_out, OFFSET_LEN, len as i64);
    escribir_campo(desc_out, OFFSET_CAP, cap as i64);
}

/// Extrae las claves de un diccionario como Vector<Texto>.
/// Los buckets del diccionario tienen layout: [occupied:1][hash:4][padding:3][key_ptr:8][key_len:8][val...]
/// Para MVP, iteramos los buckets y extraemos las claves como strings.
///
/// # Safety
/// - `desc_dict` debe ser un descriptor válido de Diccionario
/// - `desc_out` debe ser un descriptor válido de Vector (inicializado)
/// - `key_size` y `val_size` son los tamaños de los tipos K y V en bytes
#[no_mangle]
pub unsafe extern "C" fn falcato_diccionario_claves(
    desc_dict: i64,
    desc_out: i64,
    key_size: i32,
    val_size: i32,
) {
    if desc_dict == 0 || desc_out == 0 { return; }

    let dict_ptr = leer_campo(desc_dict, OFFSET_PTR) as *const u8;
    let dict_len = leer_campo(desc_dict, OFFSET_LEN) as usize;
    let dict_cap = leer_campo(desc_dict, OFFSET_CAP) as usize;

    if dict_ptr.is_null() || dict_cap == 0 { return; }

    // Calcular stride del bucket: 8 (occupied+hash+padding) + key_size + val_size
    let bucket_stride = ((8 + key_size as usize + val_size as usize + 7) / 8) * 8;

    // Allocar array de punteros a descriptores de Texto
    let array_size = dict_len * 8;
    let array_ptr = malloc(array_size);
    if array_ptr.is_null() { return; }

    let mut count = 0;
    for i in 0..dict_cap {
        let bucket = dict_ptr.add(i * bucket_stride);

        // Verificar si el bucket está ocupado (byte 0 != 0)
        if *bucket == 0 { continue; }

        // La clave está después de occupied(1) + hash(4) + padding(3) = 8 bytes
        // Para claves que son Texto (descriptor): key_ptr y key_len
        // Para claves que son enteros: el valor directo
        let key_addr = bucket.add(8);

        // Crear descriptor de Texto para la clave
        let desc_key = malloc(24);
        if desc_key.is_null() { continue; }

        if key_size == 8 {
            // Clave es un Texto (descriptor de 8 bytes = puntero)
            let key_desc = *(key_addr as *const i64);
            if key_desc != 0 {
                let kptr = leer_campo(key_desc, OFFSET_PTR);
                let klen = leer_campo(key_desc, OFFSET_LEN);
                let kcap = leer_campo(key_desc, OFFSET_CAP);

                // Copiar el descriptor
                let new_buf = malloc(kcap as usize);
                if !new_buf.is_null() && kptr != 0 {
                    memcpy(new_buf, kptr as *const c_void, klen as usize);
                    *(new_buf as *mut u8).add(klen as usize) = 0;
                }
                escribir_campo(desc_key, OFFSET_PTR, new_buf as i64);
                escribir_campo(desc_key, OFFSET_LEN, klen);
                escribir_campo(desc_key, OFFSET_CAP, kcap);
            }
        } else if key_size == 4 {
            // Clave es un Entero32
            let val = *(key_addr as *const i32);
            let text = format!("{}", val);
            texto_desde_buffer(text.as_bytes(), desc_key);
        } else if key_size == 8 {
            // Clave es un Entero64
            let val = *(key_addr as *const i64);
            let text = format!("{}", val);
            texto_desde_buffer(text.as_bytes(), desc_key);
        }

        *(array_ptr as *mut i64).add(count) = desc_key as i64;
        count += 1;
    }

    // Actualizar vector de salida
    let old_ptr = leer_campo(desc_out, OFFSET_PTR);
    if old_ptr != 0 { free(old_ptr as *mut c_void); }

    escribir_campo(desc_out, OFFSET_PTR, array_ptr as i64);
    escribir_campo(desc_out, OFFSET_LEN, count as i64);
    escribir_campo(desc_out, OFFSET_CAP, count as i64);
}

/// Extrae los valores de un diccionario como Vector<Texto> (serializados).
/// Para MVP, serializa cada valor como texto.
///
/// # Safety
/// - Descriptores deben ser válidos
#[no_mangle]
pub unsafe extern "C" fn falcato_diccionario_valores(
    desc_dict: i64,
    desc_out: i64,
    key_size: i32,
    val_size: i32,
) {
    if desc_dict == 0 || desc_out == 0 { return; }

    let dict_ptr = leer_campo(desc_dict, OFFSET_PTR) as *const u8;
    let dict_len = leer_campo(desc_dict, OFFSET_LEN) as usize;
    let dict_cap = leer_campo(desc_dict, OFFSET_CAP) as usize;

    if dict_ptr.is_null() || dict_cap == 0 { return; }

    let bucket_stride = ((8 + key_size as usize + val_size as usize + 7) / 8) * 8;

    let array_size = dict_len * 8;
    let array_ptr = malloc(array_size);
    if array_ptr.is_null() { return; }

    let mut count = 0;
    for i in 0..dict_cap {
        let bucket = dict_ptr.add(i * bucket_stride);
        if *bucket == 0 { continue; }

        // El valor está después de la clave
        let val_addr = bucket.add(8 + key_size as usize);

        let desc_val = malloc(24);
        if desc_val.is_null() { continue; }

        // Serializar valor según tipo
        if val_size == 8 {
            // Valor es un Texto (descriptor)
            let val_desc = *(val_addr as *const i64);
            if val_desc != 0 {
                let vptr = leer_campo(val_desc, OFFSET_PTR);
                let vlen = leer_campo(val_desc, OFFSET_LEN);
                let vcap = leer_campo(val_desc, OFFSET_CAP);
                let new_buf = malloc(vcap as usize);
                if !new_buf.is_null() && vptr != 0 {
                    memcpy(new_buf, vptr as *const c_void, vlen as usize);
                    *(new_buf as *mut u8).add(vlen as usize) = 0;
                }
                escribir_campo(desc_val, OFFSET_PTR, new_buf as i64);
                escribir_campo(desc_val, OFFSET_LEN, vlen);
                escribir_campo(desc_val, OFFSET_CAP, vcap);
            }
        } else if val_size == 4 {
            let val = *(val_addr as *const i32);
            let text = format!("{}", val);
            texto_desde_buffer(text.as_bytes(), desc_val);
        } else {
            let val = *(val_addr as *const i64);
            let text = format!("{}", val);
            texto_desde_buffer(text.as_bytes(), desc_val);
        }

        *(array_ptr as *mut i64).add(count) = desc_val as i64;
        count += 1;
    }

    let old_ptr = leer_campo(desc_out, OFFSET_PTR);
    if old_ptr != 0 { free(old_ptr as *mut c_void); }

    escribir_campo(desc_out, OFFSET_PTR, array_ptr as i64);
    escribir_campo(desc_out, OFFSET_LEN, count as i64);
    escribir_campo(desc_out, OFFSET_CAP, count as i64);
}

/// Vacía el diccionario sin deallocar la memoria.
/// Marca todos los buckets como no ocupados.
///
/// # Safety
/// - `desc_dict` debe ser un descriptor válido
#[no_mangle]
pub unsafe extern "C" fn falcato_diccionario_limpiar(
    desc_dict: i64,
    key_size: i32,
    val_size: i32,
) {
    if desc_dict == 0 { return; }

    let dict_ptr = leer_campo(desc_dict, OFFSET_PTR) as *mut u8;
    let dict_cap = leer_campo(desc_dict, OFFSET_CAP) as usize;

    if dict_ptr.is_null() || dict_cap == 0 { return; }

    let bucket_stride = ((8 + key_size as usize + val_size as usize + 7) / 8) * 8;

    for i in 0..dict_cap {
        let bucket = dict_ptr.add(i * bucket_stride);
        // Marcar como no ocupado
        *bucket = 0;
    }

    // Resetear len a 0
    escribir_campo(desc_dict, OFFSET_LEN, 0);
}

/// Extrae los elementos de un conjunto como Vector<Texto>.
/// Conjunto es un Diccionario<T, Booleano>, así que extraemos las claves.
///
/// # Safety
/// - Descriptores deben ser válidos
#[no_mangle]
pub unsafe extern "C" fn falcato_conjunto_elementos(
    desc_set: i64,
    desc_out: i64,
    key_size: i32,
) {
    // Conjunto es Diccionario<T, Booleano> con val_size=1
    falcato_diccionario_claves(desc_set, desc_out, key_size, 1);
}
