//! # JSON — parser y serializador mínimo para MCP/Cid
//!
//! Soporta: null, boolean, number, string, array, object.
//! Parsing recursivo descendente. Serialización directa.

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

/// Parser JSON minimalista.
/// Tipos: 0=null, 1=bool, 2=int, 3=float, 4=string, 5=array, 6=object
struct JsonParser<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> JsonParser<'a> {
    fn new(input: &'a [u8]) -> Self {
        Self { input, pos: 0 }
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() {
            match self.input[self.pos] {
                b' ' | b'\t' | b'\n' | b'\r' => self.pos += 1,
                _ => break,
            }
        }
    }

    fn peek(&self) -> Option<u8> {
        self.input.get(self.pos).copied()
    }

    fn next(&mut self) -> Option<u8> {
        let b = self.input.get(self.pos).copied();
        if b.is_some() { self.pos += 1; }
        b
    }

    fn parse_value(&mut self) -> Option<(Vec<u8>, i32)> {
        self.skip_whitespace();
        match self.peek()? {
            b'n' => self.parse_null(),
            b't' | b'f' => self.parse_bool(),
            b'"' => self.parse_string(),
            b'-' | b'0'..=b'9' => self.parse_number(),
            b'[' => self.parse_array(),
            b'{' => self.parse_object(),
            _ => None,
        }
    }

    fn parse_null(&mut self) -> Option<(Vec<u8>, i32)> {
        let rest = &self.input[self.pos..];
        if rest.len() >= 4 && &rest[..4] == b"null" {
            self.pos += 4;
            Some((b"null".to_vec(), 0))
        } else {
            None
        }
    }

    fn parse_bool(&mut self) -> Option<(Vec<u8>, i32)> {
        let rest = &self.input[self.pos..];
        if rest.len() >= 4 && &rest[..4] == b"true" {
            self.pos += 4;
            Some((b"true".to_vec(), 1))
        } else if rest.len() >= 5 && &rest[..5] == b"false" {
            self.pos += 5;
            Some((b"false".to_vec(), 1))
        } else {
            None
        }
    }

    fn parse_string(&mut self) -> Option<(Vec<u8>, i32)> {
        self.next()?; // skip opening "
        let mut result = Vec::new();
        result.push(b'"');
        loop {
            match self.next()? {
                b'"' => {
                    result.push(b'"');
                    return Some((result, 4));
                }
                b'\\' => {
                    result.push(b'\\');
                    result.push(self.next()?);
                }
                c => {
                    result.push(c);
                }
            }
        }
    }

    fn parse_number(&mut self) -> Option<(Vec<u8>, i32)> {
        let start = self.pos;
        let mut is_float = false;

        if self.peek() == Some(b'-') { self.pos += 1; }

        while self.pos < self.input.len() && self.input[self.pos].is_ascii_digit() {
            self.pos += 1;
        }

        if self.pos < self.input.len() && self.input[self.pos] == b'.' {
            is_float = true;
            self.pos += 1;
            while self.pos < self.input.len() && self.input[self.pos].is_ascii_digit() {
                self.pos += 1;
            }
        }

        if self.pos < self.input.len() && (self.input[self.pos] == b'e' || self.input[self.pos] == b'E') {
            is_float = true;
            self.pos += 1;
            if self.pos < self.input.len() && (self.input[self.pos] == b'+' || self.input[self.pos] == b'-') {
                self.pos += 1;
            }
            while self.pos < self.input.len() && self.input[self.pos].is_ascii_digit() {
                self.pos += 1;
            }
        }

        let num_str = &self.input[start..self.pos];
        Some((num_str.to_vec(), if is_float { 3 } else { 2 }))
    }

    fn parse_array(&mut self) -> Option<(Vec<u8>, i32)> {
        self.next()?; // skip [
        self.skip_whitespace();

        let mut result = Vec::new();
        result.push(b'[');

        if self.peek() == Some(b']') {
            self.next();
            result.push(b']');
            return Some((result, 5));
        }

        loop {
            self.skip_whitespace();
            let (val, _tipo) = self.parse_value()?;
            result.extend_from_slice(&val);
            self.skip_whitespace();

            match self.peek()? {
                b',' => { result.push(b','); self.next(); }
                b']' => { result.push(b']'); self.next(); return Some((result, 5)); }
                _ => return None,
            }
        }
    }

    fn parse_object(&mut self) -> Option<(Vec<u8>, i32)> {
        self.next()?; // skip {
        self.skip_whitespace();

        let mut result = Vec::new();
        result.push(b'{');

        if self.peek() == Some(b'}') {
            self.next();
            result.push(b'}');
            return Some((result, 6));
        }

        loop {
            self.skip_whitespace();
            let (key, _tipo) = self.parse_string()?;
            result.extend_from_slice(&key);
            self.skip_whitespace();

            if self.next()? != b':' { return None; }
            result.push(b':');
            self.skip_whitespace();

            let (val, _tipo) = self.parse_value()?;
            result.extend_from_slice(&val);
            self.skip_whitespace();

            match self.peek()? {
                b',' => { result.push(b','); self.next(); }
                b'}' => { result.push(b'}'); self.next(); return Some((result, 6)); }
                _ => return None,
            }
        }
    }
}

/// Parsea un texto JSON y escribe el resultado en `desc_out`.
/// Retorna el tipo: 0=null, 1=bool, 2=int, 3=float, 4=string, 5=array, 6=object, -1=error.
///
/// # Safety
/// - `desc_json` y `desc_out` deben ser descriptores válidos de Texto
#[no_mangle]
pub unsafe extern "C" fn falcato_json_parsear(desc_json: i64, desc_out: i64) -> i32 {
    if desc_json == 0 || desc_out == 0 { return -1; }

    let ptr = leer_campo(desc_json, OFFSET_PTR) as *const u8;
    let len = leer_campo(desc_json, OFFSET_LEN) as usize;

    if ptr.is_null() || len == 0 { return -1; }

    let input = core::slice::from_raw_parts(ptr, len);
    let mut parser = JsonParser::new(input);

    match parser.parse_value() {
        Some((json, tipo)) => {
            texto_desde_buffer(&json, desc_out);
            tipo
        }
        None => -1,
    }
}

/// Serializa un valor JSON a texto.
/// `desc_valor` es el texto JSON crudo (ya serializado o literal).
/// Simplemente copia el valor a `desc_out`.
///
/// # Safety
/// - Descriptores deben ser válidos
#[no_mangle]
pub unsafe extern "C" fn falcato_json_serializar(desc_valor: i64, desc_out: i64) {
    if desc_valor == 0 || desc_out == 0 { return; }

    let ptr = leer_campo(desc_valor, OFFSET_PTR) as *const u8;
    let len = leer_campo(desc_valor, OFFSET_LEN) as usize;

    if ptr.is_null() || len == 0 {
        texto_desde_buffer(b"null", desc_out);
        return;
    }

    let data = core::slice::from_raw_parts(ptr, len);
    texto_desde_buffer(data, desc_out);
}

/// Escapa un string para JSON (agrega comillas al inicio/final).
///
/// # Safety
/// - Descriptores deben ser válidos
#[no_mangle]
pub unsafe extern "C" fn falcato_json_escapar(desc_texto: i64, desc_out: i64) {
    if desc_texto == 0 || desc_out == 0 { return; }

    let ptr = leer_campo(desc_texto, OFFSET_PTR) as *const u8;
    let len = leer_campo(desc_texto, OFFSET_LEN) as usize;

    let input = if ptr.is_null() || len == 0 { b"" } else { core::slice::from_raw_parts(ptr, len) };

    let mut result = Vec::new();
    result.push(b'"');
    for &b in input {
        match b {
            b'"' => result.extend_from_slice(b"\\\""),
            b'\\' => result.extend_from_slice(b"\\\\"),
            b'\n' => result.extend_from_slice(b"\\n"),
            b'\r' => result.extend_from_slice(b"\\r"),
            b'\t' => result.extend_from_slice(b"\\t"),
            0x08 => result.extend_from_slice(b"\\b"),
            0x0C => result.extend_from_slice(b"\\f"),
            b if b < 0x20 => {
                result.extend_from_slice(format!("\\u{:04x}", b).as_bytes());
            }
            b => result.push(b),
        }
    }
    result.push(b'"');

    texto_desde_buffer(&result, desc_out);
}

/// Extrae un campo de un objeto JSON por nombre de clave.
/// Retorna 1 si encontró, 0 si no.
///
/// # Safety
/// - Descriptores deben ser válidos
#[no_mangle]
pub unsafe extern "C" fn falcato_json_obtener(
    desc_json: i64,
    desc_clave: i64,
    desc_out: i64,
) -> i32 {
    if desc_json == 0 || desc_clave == 0 || desc_out == 0 { return 0; }

    let json_ptr = leer_campo(desc_json, OFFSET_PTR) as *const u8;
    let json_len = leer_campo(desc_json, OFFSET_LEN) as usize;
    let clave_ptr = leer_campo(desc_clave, OFFSET_PTR) as *const u8;
    let clave_len = leer_campo(desc_clave, OFFSET_LEN) as usize;

    if json_ptr.is_null() || json_len == 0 { return 0; }
    if clave_ptr.is_null() || clave_len == 0 { return 0; }

    let json = core::slice::from_raw_parts(json_ptr, json_len);
    let clave = core::slice::from_raw_parts(clave_ptr, clave_len);

    // Buscar "clave": en el JSON
    let mut search = Vec::new();
    search.push(b'"');
    search.extend_from_slice(clave);
    search.extend_from_slice(b"\":");

    if let Some(pos) = find_subsequence(json, &search) {
        let after = &json[pos + search.len()..];
        // Encontrar el valor (skip whitespace)
        let mut start = 0;
        while start < after.len() && (after[start] == b' ' || after[start] == b'\t') {
            start += 1;
        }
        let after = &after[start..];

        // Extraer valor hasta coma o llave/corchete balanceado
        let mut depth_obj = 0i32;
        let mut depth_arr = 0i32;
        let mut in_string = false;
        let mut end = 0;

        for (i, &b) in after.iter().enumerate() {
            if in_string {
                if b == b'"' && (i == 0 || after[i-1] != b'\\') {
                    in_string = false;
                }
                continue;
            }
            match b {
                b'"' => in_string = true,
                b'{' => depth_obj += 1,
                b'}' => depth_obj -= 1,
                b'[' => depth_arr += 1,
                b']' => depth_arr -= 1,
                b',' if depth_obj == 0 && depth_arr == 0 => { end = i; break; }
                b'}' if depth_obj < 0 => { end = i; break; }
                b']' if depth_arr < 0 => { end = i; break; }
                _ => {}
            }
            end = i + 1;
        }

        let value = &after[..end];
        // Trim whitespace
        let value = trim_bytes(value);
        texto_desde_buffer(value, desc_out);
        1
    } else {
        0
    }
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|window| window == needle)
}

fn trim_bytes(s: &[u8]) -> &[u8] {
    let mut start = 0;
    while start < s.len() && (s[start] == b' ' || s[start] == b'\t' || s[start] == b'\n' || s[start] == b'\r') {
        start += 1;
    }
    let mut end = s.len();
    while end > start && (s[end-1] == b' ' || s[end-1] == b'\t' || s[end-1] == b'\n' || s[end-1] == b'\r') {
        end -= 1;
    }
    &s[start..end]
}
