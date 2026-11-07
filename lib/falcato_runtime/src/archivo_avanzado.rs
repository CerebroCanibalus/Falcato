//! # Archivos avanzados + entorno
//!
//! Operaciones nativas para manipulación de archivos y variables de entorno.
//!
//! API expuesta (C ABI):
//! - `falcato_archivo_agregar(ruta: i64, texto: i64)` — append a archivo
//! - `falcato_archivo_borrar(ruta: i64)` — eliminar archivo
//! - `falcato_archivo_renombrar(vieja: i64, nueva: i64)` — mover/renombrar
//! - `falcato_archivo_listar(dir: i64, desc_out: i64)` — listar directorio → Vector<Texto>
//! - `falcato_archivo_escribir_bytes(ruta: i64, ptr: i64, n: i32)` — escribir bytes crudos
//! - `falcato_entorno_obtener(nombre: i64, desc_out: i64)` — variable de entorno → Texto
//! - `falcato_directorio_actual(desc_out: i64)` — cwd → Texto
//! - `falcato_aleatorio() -> i64` — número aleatorio

use std::ffi::c_void;

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn strlen(s: *const u8) -> usize;
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

// ============================================================
// Windows
// ============================================================
#[cfg(target_os = "windows")]
mod imp {
    use super::*;
    use std::ffi::c_void;

    const GENERIC_READ: u32 = 0x80000000;
    const GENERIC_WRITE: u32 = 0x40000000;
    const FILE_APPEND_DATA: u32 = 0x0004;
    const FILE_SHARE_READ: u32 = 0x00000001;
    const OPEN_EXISTING: u32 = 3;
    const FILE_ATTRIBUTE_NORMAL: u32 = 0x80;
    const INVALID_HANDLE_VALUE: *mut c_void = -1isize as *mut c_void;

    #[repr(C)]
    struct WIN32_FIND_DATAW {
        dw_file_attributes: u32,
        ft_creation_time: [u32; 2],
        ft_last_access_time: [u32; 2],
        ft_last_write_time: [u32; 2],
        n_file_size_high: u32,
        n_file_size_low: u32,
        dw_reserved0: u32,
        dw_reserved1: u32,
        c_file_name: [u16; 260],
        c_alternate_file_name: [u16; 14],
    }

    extern "system" {
        fn CreateFileW(
            lp_file_name: *const u16,
            dw_desired_access: u32,
            dw_share_mode: u32,
            lp_security_attributes: *mut c_void,
            dw_disposition: u32,
            dw_flags_and_attributes: u32,
            h_template_file: *mut c_void,
        ) -> *mut c_void;
        fn WriteFile(
            h_file: *mut c_void,
            lp_buffer: *const c_void,
            n_number_of_bytes_to_write: u32,
            lp_number_of_bytes_written: *mut u32,
            lp_overlapped: *mut c_void,
        ) -> i32;
        fn CloseHandle(h_object: *mut c_void) -> i32;
        fn DeleteFileW(lp_file_name: *const u16) -> i32;
        fn MoveFileW(lp_existing_file_name: *const u16, lp_new_file_name: *const u16) -> i32;
        fn FindFirstFileW(lp_file_name: *const u16, lp_find_file_data: *mut WIN32_FIND_DATAW) -> *mut c_void;
        fn FindNextFileW(h_find_file: *mut c_void, lp_find_file_data: *mut WIN32_FIND_DATAW) -> i32;
        fn FindClose(h_find_file: *mut c_void) -> i32;
        fn GetEnvironmentVariableW(lp_name: *const u16, lp_buffer: *mut u16, n_size: u32) -> u32;
        fn GetCurrentDirectoryW(n_buffer_length: u32, lp_buffer: *mut u16) -> u32;
        fn MultiByteToWideChar(
            code_page: u32,
            dw_flags: u32,
            lp_multi_byte_str: *const u8,
            cb_multi_byte: i32,
            lp_wide_char_str: *mut u16,
            cch_wide_char: i32,
        ) -> i32;
        fn WideCharToMultiByte(
            code_page: u32,
            dw_flags: u32,
            lp_wide_char_str: *const u16,
            cch_wide_char: i32,
            lp_multi_byte_str: *mut u8,
            cb_multi_byte: i32,
            lp_default_char: *const u8,
            lp_used_default_char: *mut i32,
        ) -> i32;
        fn rand() -> i32;
    }

    const CP_UTF8: u32 = 65001;

    /// Convierte string UTF-8 (ptr, len) a UTF-16 null-terminated.
    /// Retorna puntero a buffer allocado (caller debe free).
    unsafe fn utf8_a_utf16(ptr: *const u8, len: usize) -> *mut u16 {
        let needed = MultiByteToWideChar(CP_UTF8, 0, ptr, len as i32, std::ptr::null_mut(), 0);
        if needed <= 0 {
            return std::ptr::null_mut();
        }
        let buf = malloc((needed as usize + 1) * 2) as *mut u16;
        if buf.is_null() {
            return std::ptr::null_mut();
        }
        MultiByteToWideChar(CP_UTF8, 0, ptr, len as i32, buf, needed);
        *buf.offset(needed as isize) = 0; // null-terminate
        buf
    }

    /// Convierte string UTF-16 null-terminated a UTF-8.
    /// Retorna puntero a buffer allocado (caller debe free).
    unsafe fn utf16_a_utf8(ptr: *const u16) -> (*mut u8, usize) {
        let needed = WideCharToMultiByte(CP_UTF8, 0, ptr, -1, std::ptr::null_mut(), 0, std::ptr::null(), std::ptr::null_mut());
        if needed <= 0 {
            return (std::ptr::null_mut(), 0);
        }
        let buf = malloc(needed as usize) as *mut u8;
        if buf.is_null() {
            return (std::ptr::null_mut(), 0);
        }
        WideCharToMultiByte(CP_UTF8, 0, ptr, -1, buf, needed, std::ptr::null(), std::ptr::null_mut());
        (buf, (needed - 1) as usize) // -1 porque no contamos el null
    }

    /// Append texto a archivo.
    #[no_mangle]
    pub unsafe extern "C" fn falcato_archivo_agregar(ruta_desc: i64, texto_desc: i64) {
        if ruta_desc == 0 || texto_desc == 0 {
            return;
        }
        let ruta_ptr = leer_campo(ruta_desc, OFFSET_PTR) as *const u8;
        let ruta_len = leer_campo(ruta_desc, OFFSET_LEN) as usize;
        let texto_ptr = leer_campo(texto_desc, OFFSET_PTR) as *const u8;
        let texto_len = leer_campo(texto_desc, OFFSET_LEN) as usize;

        let ruta_w = utf8_a_utf16(ruta_ptr, ruta_len);
        if ruta_w.is_null() {
            return;
        }

        let handle = CreateFileW(
            ruta_w,
            FILE_APPEND_DATA,
            FILE_SHARE_READ,
            std::ptr::null_mut(),
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            std::ptr::null_mut(),
        );
        free(ruta_w as *mut c_void);

        if handle == INVALID_HANDLE_VALUE {
            return;
        }

        let mut escritos: u32 = 0;
        WriteFile(handle, texto_ptr as *const c_void, texto_len as u32, &mut escritos, std::ptr::null_mut());
        CloseHandle(handle);
    }

    /// Borrar archivo.
    #[no_mangle]
    pub unsafe extern "C" fn falcato_archivo_borrar(ruta_desc: i64) {
        if ruta_desc == 0 {
            return;
        }
        let ruta_ptr = leer_campo(ruta_desc, OFFSET_PTR) as *const u8;
        let ruta_len = leer_campo(ruta_desc, OFFSET_LEN) as usize;

        let ruta_w = utf8_a_utf16(ruta_ptr, ruta_len);
        if ruta_w.is_null() {
            return;
        }

        DeleteFileW(ruta_w);
        free(ruta_w as *mut c_void);
    }

    /// Renombrar/mover archivo.
    #[no_mangle]
    pub unsafe extern "C" fn falcato_archivo_renombrar(vieja_desc: i64, nueva_desc: i64) {
        if vieja_desc == 0 || nueva_desc == 0 {
            return;
        }
        let vieja_ptr = leer_campo(vieja_desc, OFFSET_PTR) as *const u8;
        let vieja_len = leer_campo(vieja_desc, OFFSET_LEN) as usize;
        let nueva_ptr = leer_campo(nueva_desc, OFFSET_PTR) as *const u8;
        let nueva_len = leer_campo(nueva_desc, OFFSET_LEN) as usize;

        let vieja_w = utf8_a_utf16(vieja_ptr, vieja_len);
        let nueva_w = utf8_a_utf16(nueva_ptr, nueva_len);
        if vieja_w.is_null() || nueva_w.is_null() {
            if !vieja_w.is_null() { free(vieja_w as *mut c_void); }
            if !nueva_w.is_null() { free(nueva_w as *mut c_void); }
            return;
        }

        MoveFileW(vieja_w, nueva_w);
        free(vieja_w as *mut c_void);
        free(nueva_w as *mut c_void);
    }

    /// Escribir bytes crudos a archivo.
    #[no_mangle]
    pub unsafe extern "C" fn falcato_archivo_escribir_bytes(ruta_desc: i64, datos_ptr: i64, n: i32) {
        if ruta_desc == 0 || datos_ptr == 0 || n <= 0 {
            return;
        }
        let ruta_ptr = leer_campo(ruta_desc, OFFSET_PTR) as *const u8;
        let ruta_len = leer_campo(ruta_desc, OFFSET_LEN) as usize;

        let ruta_w = utf8_a_utf16(ruta_ptr, ruta_len);
        if ruta_w.is_null() {
            return;
        }

        let handle = CreateFileW(
            ruta_w,
            GENERIC_WRITE,
            0,
            std::ptr::null_mut(),
            2, // CREATE_ALWAYS
            FILE_ATTRIBUTE_NORMAL,
            std::ptr::null_mut(),
        );
        free(ruta_w as *mut c_void);

        if handle == INVALID_HANDLE_VALUE {
            return;
        }

        let mut escritos: u32 = 0;
        WriteFile(handle, datos_ptr as *const c_void, n as u32, &mut escritos, std::ptr::null_mut());
        CloseHandle(handle);
    }

    /// Obtener variable de entorno.
    #[no_mangle]
    pub unsafe extern "C" fn falcato_entorno_obtener(nombre_desc: i64, desc_out: i64) {
        if nombre_desc == 0 || desc_out == 0 {
            return;
        }
        let nombre_ptr = leer_campo(nombre_desc, OFFSET_PTR) as *const u8;
        let nombre_len = leer_campo(nombre_desc, OFFSET_LEN) as usize;

        let nombre_w = utf8_a_utf16(nombre_ptr, nombre_len);
        if nombre_w.is_null() {
            return;
        }

        let mut buf = [0u16; 4096];
        let ret = GetEnvironmentVariableW(nombre_w, buf.as_mut_ptr(), 4096);
        free(nombre_w as *mut c_void);

        if ret == 0 || ret >= 4096 {
            return; // No existe o buffer insuficiente
        }

        let (utf8_ptr, utf8_len) = utf16_a_utf8(buf.as_ptr());
        if utf8_ptr.is_null() {
            return;
        }

        let cap = utf8_len + 1;
        escribir_campo(desc_out, OFFSET_PTR, utf8_ptr as i64);
        escribir_campo(desc_out, OFFSET_LEN, utf8_len as i64);
        escribir_campo(desc_out, OFFSET_CAP, cap as i64);
    }

    /// Obtener directorio actual.
    #[no_mangle]
    pub unsafe extern "C" fn falcato_directorio_actual(desc_out: i64) {
        if desc_out == 0 {
            return;
        }

        let mut buf = [0u16; 4096];
        let ret = GetCurrentDirectoryW(4096, buf.as_mut_ptr());
        if ret == 0 || ret >= 4096 {
            return;
        }

        let (utf8_ptr, utf8_len) = utf16_a_utf8(buf.as_ptr());
        if utf8_ptr.is_null() {
            return;
        }

        let cap = utf8_len + 1;
        escribir_campo(desc_out, OFFSET_PTR, utf8_ptr as i64);
        escribir_campo(desc_out, OFFSET_LEN, utf8_len as i64);
        escribir_campo(desc_out, OFFSET_CAP, cap as i64);
    }

    /// Número aleatorio.
    #[no_mangle]
    pub unsafe extern "C" fn falcato_aleatorio() -> i64 {
        rand() as i64
    }

    /// Listar directorio — retorna Vector<Texto> como descriptor.
    /// El descriptor de salida tiene: ptr = array de descriptores Texto, len = count, cap = capacity.
    #[no_mangle]
    pub unsafe extern "C" fn falcato_archivo_listar(dir_desc: i64, desc_out: i64) {
        if dir_desc == 0 || desc_out == 0 {
            return;
        }
        let dir_ptr = leer_campo(dir_desc, OFFSET_PTR) as *const u8;
        let dir_len = leer_campo(dir_desc, OFFSET_LEN) as usize;

        // Construir patrón "dir\*"
        let mut patron = Vec::with_capacity(dir_len + 3);
        patron.extend_from_slice(std::slice::from_raw_parts(dir_ptr, dir_len));
        patron.push(b'\\');
        patron.push(b'*');

        let patron_w = utf8_a_utf16(patron.as_ptr(), patron.len());
        if patron_w.is_null() {
            return;
        }

        let mut find_data: WIN32_FIND_DATAW = std::mem::zeroed();
        let handle = FindFirstFileW(patron_w, &mut find_data);
        free(patron_w as *mut c_void);

        if handle == INVALID_HANDLE_VALUE {
            return;
        }

        // Coleccionar nombres
        let mut nombres: Vec<(*mut u8, usize)> = Vec::new();
        loop {
            // Saltar "." y ".."
            let c0 = find_data.c_file_name[0];
            if c0 == b'.' as u16 {
                let c1 = find_data.c_file_name[1];
                if c1 == 0 || (c1 == b'.' as u16 && find_data.c_file_name[2] == 0) {
                    if FindNextFileW(handle, &mut find_data) == 0 {
                        break;
                    }
                    continue;
                }
            }

            let (utf8_ptr, utf8_len) = utf16_a_utf8(find_data.c_file_name.as_ptr());
            if !utf8_ptr.is_null() {
                nombres.push((utf8_ptr, utf8_len));
            }

            if FindNextFileW(handle, &mut find_data) == 0 {
                break;
            }
        }
        FindClose(handle);

          // Construir array de PUNTEROS a descriptores Texto en heap
          // Convención: Vector<Texto> contiene punteros a descriptores individuales
          let count = nombres.len();
          let tam_puntero = 8usize; // sizeof(puntero)
          let array_size = count * tam_puntero;
          let array_ptr = malloc(array_size) as *mut u8;
          if array_ptr.is_null() {
              // Liberar nombres
              for (ptr, _) in nombres {
                  free(ptr as *mut c_void);
              }
              return;
          }

          // Crear un descriptor individual para cada nombre y guardar puntero en el array
          for (i, (nombre_ptr, nombre_len)) in nombres.iter().enumerate() {
              // Crear descriptor de Texto en heap (24 bytes)
              let desc_ptr = malloc(24) as *mut u8;
              if desc_ptr.is_null() {
                  free(*nombre_ptr as *mut c_void);
                  continue;
              }
              let cap = *nombre_len + 1;
              // Escribir ptr, len, cap en el descriptor
              *(desc_ptr as *mut i64) = *nombre_ptr as i64;
              *(desc_ptr.offset(8) as *mut i64) = *nombre_len as i64;
              *(desc_ptr.offset(16) as *mut i64) = cap as i64;
              // Guardar puntero al descriptor en el array
              *(array_ptr.offset((i * tam_puntero) as isize) as *mut i64) = desc_ptr as i64;
          }

        // Escribir descriptor del Vector<Texto>
        escribir_campo(desc_out, OFFSET_PTR, array_ptr as i64);
        escribir_campo(desc_out, OFFSET_LEN, count as i64);
        escribir_campo(desc_out, OFFSET_CAP, count as i64);
    }
}

// ============================================================
// POSIX (Linux/macOS)
// ============================================================
#[cfg(not(target_os = "windows"))]
mod imp {
    use super::*;

    extern "C" {
        fn open(pathname: *const u8, flags: i32, mode: u32) -> i32;
        fn write(fd: i32, buf: *const c_void, count: usize) -> isize;
        fn close(fd: i32) -> i32;
        fn unlink(pathname: *const u8) -> i32;
        fn rename(oldpath: *const u8, newpath: *const u8) -> i32;
        fn getenv(name: *const u8) -> *const u8;
        fn getcwd(buf: *mut u8, size: usize) -> *mut u8;
        fn rand() -> i32;
        fn opendir(name: *const u8) -> *mut c_void;
        fn readdir(dirp: *mut c_void) -> *mut c_void;
        fn closedir(dirp: *mut c_void) -> i32;
    }

    // struct dirent tiene d_name en offset 19 (Linux) o variable (macOS)
    // Usamos un offset seguro
    #[repr(C)]
    struct Dirent {
        d_ino: u64,
        d_off: i64,
        d_reclen: u16,
        d_type: u8,
        d_name: [u8; 256],
    }

    const O_WRONLY: i32 = 1;
    const O_CREAT: i32 = 64;
    const O_TRUNC: i32 = 512;
    const O_APPEND: i32 = 1024;

    #[no_mangle]
    pub unsafe extern "C" fn falcato_archivo_agregar(ruta_desc: i64, texto_desc: i64) {
        if ruta_desc == 0 || texto_desc == 0 {
            return;
        }
        let ruta_ptr = leer_campo(ruta_desc, OFFSET_PTR) as *const u8;
        let ruta_len = leer_campo(ruta_desc, OFFSET_LEN) as usize;
        let texto_ptr = leer_campo(texto_desc, OFFSET_PTR) as *const u8;
        let texto_len = leer_campo(texto_desc, OFFSET_LEN) as usize;

        // Null-terminar ruta
        let ruta_buf = malloc(ruta_len + 1) as *mut u8;
        if ruta_buf.is_null() { return; }
        memcpy(ruta_buf as *mut c_void, ruta_ptr as *const c_void, ruta_len);
        *ruta_buf.add(ruta_len) = 0;

        let fd = open(ruta_buf, O_WRONLY | O_APPEND | O_CREAT, 0o644);
        free(ruta_buf as *mut c_void);

        if fd < 0 { return; }
        write(fd, texto_ptr as *const c_void, texto_len);
        close(fd);
    }

    #[no_mangle]
    pub unsafe extern "C" fn falcato_archivo_borrar(ruta_desc: i64) {
        if ruta_desc == 0 { return; }
        let ruta_ptr = leer_campo(ruta_desc, OFFSET_PTR) as *const u8;
        let ruta_len = leer_campo(ruta_desc, OFFSET_LEN) as usize;

        let ruta_buf = malloc(ruta_len + 1) as *mut u8;
        if ruta_buf.is_null() { return; }
        memcpy(ruta_buf as *mut c_void, ruta_ptr as *const c_void, ruta_len);
        *ruta_buf.add(ruta_len) = 0;

        unlink(ruta_buf);
        free(ruta_buf as *mut c_void);
    }

    #[no_mangle]
    pub unsafe extern "C" fn falcato_archivo_renombrar(vieja_desc: i64, nueva_desc: i64) {
        if vieja_desc == 0 || nueva_desc == 0 { return; }
        let vieja_ptr = leer_campo(vieja_desc, OFFSET_PTR) as *const u8;
        let vieja_len = leer_campo(vieja_desc, OFFSET_LEN) as usize;
        let nueva_ptr = leer_campo(nueva_desc, OFFSET_PTR) as *const u8;
        let nueva_len = leer_campo(nueva_desc, OFFSET_LEN) as usize;

        let vieja_buf = malloc(vieja_len + 1) as *mut u8;
        let nueva_buf = malloc(nueva_len + 1) as *mut u8;
        if vieja_buf.is_null() || nueva_buf.is_null() {
            if !vieja_buf.is_null() { free(vieja_buf as *mut c_void); }
            if !nueva_buf.is_null() { free(nueva_buf as *mut c_void); }
            return;
        }
        memcpy(vieja_buf as *mut c_void, vieja_ptr as *const c_void, vieja_len);
        *vieja_buf.add(vieja_len) = 0;
        memcpy(nueva_buf as *mut c_void, nueva_ptr as *const c_void, nueva_len);
        *nueva_buf.add(nueva_len) = 0;

        rename(vieja_buf, nueva_buf);
        free(vieja_buf as *mut c_void);
        free(nueva_buf as *mut c_void);
    }

    #[no_mangle]
    pub unsafe extern "C" fn falcato_archivo_escribir_bytes(ruta_desc: i64, datos_ptr: i64, n: i32) {
        if ruta_desc == 0 || datos_ptr == 0 || n <= 0 { return; }
        let ruta_ptr = leer_campo(ruta_desc, OFFSET_PTR) as *const u8;
        let ruta_len = leer_campo(ruta_desc, OFFSET_LEN) as usize;

        let ruta_buf = malloc(ruta_len + 1) as *mut u8;
        if ruta_buf.is_null() { return; }
        memcpy(ruta_buf as *mut c_void, ruta_ptr as *const c_void, ruta_len);
        *ruta_buf.add(ruta_len) = 0;

        let fd = open(ruta_buf, O_WRONLY | O_CREAT | O_TRUNC, 0o644);
        free(ruta_buf as *mut c_void);

        if fd < 0 { return; }
        write(fd, datos_ptr as *const c_void, n as usize);
        close(fd);
    }

    #[no_mangle]
    pub unsafe extern "C" fn falcato_entorno_obtener(nombre_desc: i64, desc_out: i64) {
        if nombre_desc == 0 || desc_out == 0 { return; }
        let nombre_ptr = leer_campo(nombre_desc, OFFSET_PTR) as *const u8;
        let nombre_len = leer_campo(nombre_desc, OFFSET_LEN) as usize;

        let nombre_buf = malloc(nombre_len + 1) as *mut u8;
        if nombre_buf.is_null() { return; }
        memcpy(nombre_buf as *mut c_void, nombre_ptr as *const c_void, nombre_len);
        *nombre_buf.add(nombre_len) = 0;

        let val = getenv(nombre_buf);
        free(nombre_buf as *mut c_void);

        if val.is_null() { return; }

        let val_len = strlen(val);
        let buf = malloc(val_len + 1) as *mut u8;
        if buf.is_null() { return; }
        memcpy(buf as *mut c_void, val as *const c_void, val_len);
        *buf.add(val_len) = 0;

        escribir_campo(desc_out, OFFSET_PTR, buf as i64);
        escribir_campo(desc_out, OFFSET_LEN, val_len as i64);
        escribir_campo(desc_out, OFFSET_CAP, (val_len + 1) as i64);
    }

    #[no_mangle]
    pub unsafe extern "C" fn falcato_directorio_actual(desc_out: i64) {
        if desc_out == 0 { return; }

        let buf = malloc(4096) as *mut u8;
        if buf.is_null() { return; }

        let ret = getcwd(buf, 4096);
        if ret.is_null() {
            free(buf as *mut c_void);
            return;
        }

        let len = strlen(buf);
        escribir_campo(desc_out, OFFSET_PTR, buf as i64);
        escribir_campo(desc_out, OFFSET_LEN, len as i64);
        escribir_campo(desc_out, OFFSET_CAP, 4096i64);
    }

    #[no_mangle]
    pub unsafe extern "C" fn falcato_aleatorio() -> i64 {
        rand() as i64
    }

    #[no_mangle]
    pub unsafe extern "C" fn falcato_archivo_listar(dir_desc: i64, desc_out: i64) {
        if dir_desc == 0 || desc_out == 0 { return; }
        let dir_ptr = leer_campo(dir_desc, OFFSET_PTR) as *const u8;
        let dir_len = leer_campo(dir_desc, OFFSET_LEN) as usize;

        let dir_buf = malloc(dir_len + 1) as *mut u8;
        if dir_buf.is_null() { return; }
        memcpy(dir_buf as *mut c_void, dir_ptr as *const c_void, dir_len);
        *dir_buf.add(dir_len) = 0;

        let d = opendir(dir_buf);
        free(dir_buf as *mut c_void);
        if d.is_null() { return; }

        let mut nombres: Vec<(*mut u8, usize)> = Vec::new();
        loop {
            let entry = readdir(d);
            if entry.is_null() { break; }

            let dirent = &*(entry as *const Dirent);
            let name_ptr = dirent.d_name.as_ptr();
            let name_len = strlen(name_ptr);

            // Saltar "." y ".."
            if name_len == 1 && *name_ptr == b'.' { continue; }
            if name_len == 2 && *name_ptr == b'.' && *name_ptr.add(1) == b'.' { continue; }

            let buf = malloc(name_len + 1) as *mut u8;
            if buf.is_null() { continue; }
            memcpy(buf as *mut c_void, name_ptr as *const c_void, name_len);
            *buf.add(name_len) = 0;
            nombres.push((buf, name_len));
        }
        closedir(d);

          // Construir array de PUNTEROS a descriptores Texto en heap
          // Convención: Vector<Texto> contiene punteros a descriptores individuales
          let count = nombres.len();
          let tam_puntero = 8usize;
          let array_size = count * tam_puntero;
          let array_ptr = malloc(array_size) as *mut u8;
          if array_ptr.is_null() {
              for (ptr, _) in nombres { free(ptr as *mut c_void); }
              return;
          }

          // Crear un descriptor individual para cada nombre y guardar puntero en el array
          for (i, (nombre_ptr, nombre_len)) in nombres.iter().enumerate() {
              let desc_ptr = malloc(24) as *mut u8;
              if desc_ptr.is_null() {
                  free(*nombre_ptr as *mut c_void);
                  continue;
              }
              let cap = *nombre_len + 1;
              *(desc_ptr as *mut i64) = *nombre_ptr as i64;
              *(desc_ptr.offset(8) as *mut i64) = *nombre_len as i64;
              *(desc_ptr.offset(16) as *mut i64) = cap as i64;
              *(array_ptr.offset((i * tam_puntero) as isize) as *mut i64) = desc_ptr as i64;
          }

        escribir_campo(desc_out, OFFSET_PTR, array_ptr as i64);
        escribir_campo(desc_out, OFFSET_LEN, count as i64);
        escribir_campo(desc_out, OFFSET_CAP, count as i64);
    }

    /// Obtiene el tamaño de un archivo en bytes.
    /// Retorna -1 si hay error.
    #[cfg(target_os = "windows")]
    #[no_mangle]
    pub unsafe extern "C" fn falcato_archivo_tamano(ruta: i64) -> i64 {
        use std::ffi::CStr;
        let ptr = leer_campo(ruta, OFFSET_PTR) as *const i8;
        if ptr.is_null() { return -1; }
        let cstr = CStr::from_ptr(ptr);
        let ruta_str = match cstr.to_str() {
            Ok(s) => s,
            Err(_) => return -1,
        };
        match std::fs::metadata(ruta_str) {
            Ok(m) => m.len() as i64,
            Err(_) => -1,
        }
    }

    #[cfg(not(target_os = "windows"))]
    #[no_mangle]
    pub unsafe extern "C" fn falcato_archivo_tamano(ruta: i64) -> i64 {
        use std::ffi::CStr;
        let ptr = leer_campo(ruta, OFFSET_PTR) as *const i8;
        if ptr.is_null() { return -1; }
        let cstr = CStr::from_ptr(ptr);
        let ruta_str = match cstr.to_str() {
            Ok(s) => s,
            Err(_) => return -1,
        };
        match std::fs::metadata(ruta_str) {
            Ok(m) => m.len() as i64,
            Err(_) => -1,
        }
    }
}

/// Funciones de fecha/time — componentes de timestamp Unix
pub mod tiempo {
    use super::*;

    /// Retorna el año de un timestamp Unix.
    #[no_mangle]
    pub unsafe extern "C" fn falcato_fecha_anio(unix: i64) -> i32 {
        // Conversión simple: días desde 1970-01-01
        let dias = unix / 86400;
        let mut year = 1970i32;
        let mut remaining = dias;

        loop {
            let days_in_year = if es_bisiesto(year) { 366 } else { 365 };
            if remaining < days_in_year as i64 {
                break;
            }
            remaining -= days_in_year as i64;
            year += 1;
        }

        // Encontrar mes y día
        let mes_dia = dias_a_mes_dia(remaining as i32, year);
        mes_dia.0 // retorno: mes (1-12)
    }

    /// Retorna el mes (1-12) de un timestamp Unix.
    #[no_mangle]
    pub unsafe extern "C" fn falcato_fecha_mes(unix: i64) -> i32 {
        let dias = unix / 86400;
        let mut year = 1970i32;
        let mut remaining = dias;

        loop {
            let days_in_year = if es_bisiesto(year) { 366 } else { 365 };
            if remaining < days_in_year as i64 {
                break;
            }
            remaining -= days_in_year as i64;
            year += 1;
        }

        let mes_dia = dias_a_mes_dia(remaining as i32, year);
        mes_dia.0
    }

    /// Retorna el día del mes (1-31) de un timestamp Unix.
    #[no_mangle]
    pub unsafe extern "C" fn falcato_fecha_dia(unix: i64) -> i32 {
        let dias = unix / 86400;
        let mut year = 1970i32;
        let mut remaining = dias;

        loop {
            let days_in_year = if es_bisiesto(year) { 366 } else { 365 };
            if remaining < days_in_year as i64 {
                break;
            }
            remaining -= days_in_year as i64;
            year += 1;
        }

        let mes_dia = dias_a_mes_dia(remaining as i32, year);
        mes_dia.1
    }

    fn es_bisiesto(year: i32) -> bool {
        (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
    }

    fn dias_en_mes(mes: i32, year: i32) -> i32 {
        match mes {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => if es_bisiesto(year) { 29 } else { 28 },
            _ => 0,
        }
    }

    /// Convierte días del año a (mes, día_del_mes).
    fn dias_a_mes_dia(dias_restantes: i32, year: i32) -> (i32, i32) {
        let mut remaining = dias_restantes;
        for mes in 1..=12 {
            let dias_mes = dias_en_mes(mes, year);
            if remaining < dias_mes {
                return (mes, remaining + 1);
            }
            remaining -= dias_mes;
        }
        (12, 31) // fallback
    }
}

pub use imp::*;
pub use tiempo::*;
