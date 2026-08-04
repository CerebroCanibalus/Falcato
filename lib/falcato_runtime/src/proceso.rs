//! # Proceso — creación y captura de salida de procesos del OS
//!
//! Abstracción portable sobre CreateProcess (Win32) y fork/exec (POSIX).
//!
//! API expuesta (C ABI):
//! - `falcato_proceso_crear(comando) -> Handle`       — lanza el proceso, captura stdout+stderr
//! - `falcato_proceso_esperar(handle) -> código`      — bloquea hasta terminar, devuelve exit code
//! - `falcato_proceso_leer_salida(handle) -> Texto*`  — devuelve la salida capturada (heap, caller libera)
//! - `falcato_proceso_cerrar(handle)`                 — libera el handle
//!
//! La salida se captura con un pipe (CreatePipe / pipe()) y un hilo lector
//! que evita el deadlock cuando el proceso llena el buffer del pipe.

use std::ffi::c_void;

// ============================================================
// Windows
// ============================================================
#[cfg(target_os = "windows")]
mod imp {
    use super::*;
    use std::ffi::c_char;
    use std::ptr;

    const HANDLE_FLAG_INHERIT: u32 = 0x00000001;
    const STARTF_USESTDHANDLES: u32 = 0x00000100;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    const INFINITE: u32 = u32::MAX;

    #[repr(C)]
    struct SECURITY_ATTRIBUTES {
        n_length: u32,
        lp_security_descriptor: *mut c_void,
        b_inherit_handle: i32,
    }

    #[repr(C)]
    struct STARTUPINFO {
        cb: u32,
        lp_reserved: *mut u16,
        lp_desktop: *mut u16,
        lp_title: *mut u16,
        dw_x: u32,
        dw_y: u32,
        dw_x_size: u32,
        dw_y_size: u32,
        dw_x_count_chars: u32,
        dw_y_count_chars: u32,
        dw_fill_attribute: u32,
        dw_flags: u32,
        w_show_window: u16,
        cb_reserved2: u16,
        lp_reserved2: *mut u8,
        h_std_input: *mut c_void,
        h_std_output: *mut c_void,
        h_std_error: *mut c_void,
    }

    #[repr(C)]
    struct PROCESS_INFORMATION {
        h_process: *mut c_void,
        h_thread: *mut c_void,
        dw_process_id: u32,
        dw_thread_id: u32,
    }

    extern "system" {
        fn CreatePipe(
            h_read_pipe: *mut *mut c_void,
            h_write_pipe: *mut *mut c_void,
            lp_pipe_attributes: *mut SECURITY_ATTRIBUTES,
            n_size: u32,
        ) -> i32;
        fn SetHandleInformation(h_object: *mut c_void, dw_mask: u32, dw_flags: u32) -> i32;
        fn CreateProcessW(
            lp_application_name: *const u16,
            lp_command_line: *mut u16,
            lp_process_attributes: *mut c_void,
            lp_thread_attributes: *mut c_void,
            b_inherit_handles: i32,
            dw_creation_flags: u32,
            lp_environment: *mut c_void,
            lp_current_directory: *const u16,
            lp_startup_info: *mut STARTUPINFO,
            lp_process_information: *mut PROCESS_INFORMATION,
        ) -> i32;
        fn WaitForSingleObject(h_handle: *mut c_void, dw_milliseconds: u32) -> i32;
        fn GetExitCodeProcess(h_process: *mut c_void, lp_exit_code: *mut u32) -> i32;
        fn CloseHandle(h_object: *mut c_void) -> i32;
        fn ReadFile(
            h_file: *mut c_void,
            lp_buffer: *mut u8,
            n_number_of_bytes_to_read: u32,
            lp_number_of_bytes_read: *mut u32,
            lp_overlapped: *mut c_void,
        ) -> i32;
    }

    extern "C" {
        fn malloc(size: usize) -> *mut c_void;
        fn free(ptr: *mut c_void);
    }

    pub struct Proceso {
        handle: *mut c_void,
        salida: Vec<u8>,
        lector: Option<std::thread::JoinHandle<Vec<u8>>>,
    }

    /// Wrapper Send para mover handles Win32 entre threads.
    struct HandleSend(*mut c_void);
    unsafe impl Send for HandleSend {}

    /// Convierte un comando UTF-8 a UTF-16 (para CreateProcessW), precedido de cmd.exe /C
    fn comando_a_utf16(comando: &[u8]) -> Vec<u16> {
        let comando_str = String::from_utf8_lossy(comando);
        // cmd.exe /C "<comando>"
        let linea = format!("cmd.exe /C \"{}\"", comando_str);
        let mut utf16: Vec<u16> = linea.encode_utf16().collect();
        utf16.push(0); // null terminator
        utf16
    }

    fn leer_pipe(pipe: *mut c_void) -> Vec<u8> {
        let mut salida: Vec<u8> = Vec::new();
        let mut buf = [0u8; 4096];
        loop {
            let mut leidos: u32 = 0;
            let ok = unsafe {
                ReadFile(
                    pipe,
                    buf.as_mut_ptr(),
                    buf.len() as u32,
                    &mut leidos,
                    ptr::null_mut(),
                )
            };
            if ok == 0 || leidos == 0 {
                break;
            }
            salida.extend_from_slice(&buf[..leidos as usize]);
        }
        unsafe { CloseHandle(pipe); }
        salida
    }

    pub unsafe fn proceso_crear(comando: *const c_char) -> *mut c_void {
        let comando_bytes = std::ffi::CStr::from_ptr(comando).to_bytes();

        // 1. Crear pipes para stdout y stderr (heredables por el hijo)
        let mut read_out: *mut c_void = ptr::null_mut();
        let mut write_out: *mut c_void = ptr::null_mut();
        let mut read_err: *mut c_void = ptr::null_mut();
        let mut write_err: *mut c_void = ptr::null_mut();

        let mut sa = SECURITY_ATTRIBUTES {
            n_length: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lp_security_descriptor: ptr::null_mut(),
            b_inherit_handle: 1,
        };

        if CreatePipe(&mut read_out, &mut write_out, &mut sa, 0) == 0 {
            return ptr::null_mut();
        }
        if CreatePipe(&mut read_err, &mut write_err, &mut sa, 0) == 0 {
            CloseHandle(read_out);
            CloseHandle(write_out);
            return ptr::null_mut();
        }

        // 2. No heredar los lados de lectura (solo escritura)
        SetHandleInformation(read_out, HANDLE_FLAG_INHERIT, 0);
        SetHandleInformation(read_err, HANDLE_FLAG_INHERIT, 0);

        // 3. Configurar STARTUPINFO con stdout/stderr → pipes
        let mut si: STARTUPINFO = std::mem::zeroed();
        si.cb = std::mem::size_of::<STARTUPINFO>() as u32;
        si.dw_flags = STARTF_USESTDHANDLES;
        si.h_std_output = write_out;
        si.h_std_error = write_err;
        si.h_std_input = ptr::null_mut();

        let mut pi: PROCESS_INFORMATION = std::mem::zeroed();
        let mut cmd_utf16 = comando_a_utf16(comando_bytes);

        let ok = CreateProcessW(
            ptr::null(),
            cmd_utf16.as_mut_ptr(),
            ptr::null_mut(),
            ptr::null_mut(),
            1, // bInheritHandles
            CREATE_NO_WINDOW,
            ptr::null_mut(),
            ptr::null(),
            &mut si,
            &mut pi,
        );

        // Cerrar los write ends en el padre (el hijo tiene copias heredadas)
        CloseHandle(write_out);
        CloseHandle(write_err);

        if ok == 0 {
            CloseHandle(read_out);
            CloseHandle(read_err);
            return ptr::null_mut();
        }

        // 4. Hilo lector para stdout y stderr (evita deadlock)
        let out_send = HandleSend(read_out);
        let err_send = HandleSend(read_err);
        let hilo_out = std::thread::spawn(move || { let h = out_send; leer_pipe(h.0) });
        let hilo_err = std::thread::spawn(move || { let h = err_send; leer_pipe(h.0) });

        let proc = Box::new(Proceso {
            handle: pi.h_process,
            salida: Vec::new(),
            lector: Some(std::thread::spawn(move || {
                let out = hilo_out.join().unwrap_or_default();
                let err = hilo_err.join().unwrap_or_default();
                let mut total = out;
                total.extend_from_slice(&err);
                total
            })),
        });

        Box::into_raw(proc) as *mut c_void
    }

    pub unsafe fn proceso_esperar(handle: *mut c_void) -> i32 {
        if handle.is_null() {
            return -1;
        }
        let proc = &mut *(handle as *mut Proceso);

        WaitForSingleObject(proc.handle, INFINITE);

        let mut exit_code: u32 = 0;
        GetExitCodeProcess(proc.handle, &mut exit_code);

        // Recoger la salida del hilo lector
        if let Some(lector) = proc.lector.take() {
            proc.salida = lector.join().unwrap_or_default();
        }

        CloseHandle(proc.handle);
        proc.handle = std::ptr::null_mut();

        exit_code as i32
    }

    pub unsafe fn proceso_leer_salida(handle: *mut c_void) -> *mut c_char {
        if handle.is_null() {
            return ptr::null_mut();
        }
        let proc = &*(handle as *mut Proceso);

        // Copiar salida + null terminator a heap C (caller libera con free)
        let len = proc.salida.len();
        let buf = malloc(len + 1) as *mut u8;
        if buf.is_null() {
            return ptr::null_mut();
        }
        if len > 0 {
            std::ptr::copy_nonoverlapping(proc.salida.as_ptr(), buf, len);
        }
        *buf.add(len) = 0;
        buf as *mut c_char
    }

    pub unsafe fn proceso_cerrar(handle: *mut c_void) {
        if handle.is_null() {
            return;
        }
        let proc = Box::from_raw(handle as *mut Proceso);
        // Si el hilo lector aún existe (no se esperó), join para no colgar
        let _ = proc.lector;
        // CloseHandle no hace falta (proceso_esperar lo hace); si nunca se esperó, cerrar:
        if !proc.handle.is_null() {
            CloseHandle(proc.handle);
        }
        drop(proc);
    }
}

// ============================================================
// POSIX (Linux/macOS)
// ============================================================
#[cfg(not(target_os = "windows"))]
mod imp {
    use super::*;
    use std::ffi::c_char;
    use std::ptr;

    extern "C" {
        fn pipe(fds: *mut i32) -> i32;
        fn fork() -> i32;
        fn execvp(file: *const c_char, argv: *const *const c_char) -> i32;
        fn dup2(oldfd: i32, newfd: i32) -> i32;
        fn close(fd: i32) -> i32;
        fn read(fd: i32, buf: *mut u8, count: usize) -> isize;
        fn waitpid(pid: i32, status: *mut i32, options: i32) -> i32;
        fn malloc(size: usize) -> *mut c_void;
        fn free(ptr: *mut c_void);
    }

    pub struct Proceso {
        pid: i32,
        salida: Vec<u8>,
        lector: Option<std::thread::JoinHandle<Vec<u8>>>,
    }

    pub unsafe fn proceso_crear(comando: *const c_char) -> *mut c_void {
        let comando_str = std::ffi::CStr::from_ptr(comando).to_bytes();
        let comando_owned = String::from_utf8_lossy(comando_str).to_string();

        let mut fds: [i32; 2] = [0; 2];
        if pipe(fds.as_mut_ptr()) != 0 {
            return ptr::null_mut();
        }

        let pid = fork();
        if pid < 0 {
            close(fds[0]);
            close(fds[1]);
            return ptr::null_mut();
        }

        if pid == 0 {
            // Hijo: stdout/stderr → pipe, ejecutar /bin/sh -c comando
            dup2(fds[1], 1);
            dup2(fds[1], 2);
            close(fds[0]);
            close(fds[1]);
            let sh = b"/bin/sh\0".as_ptr() as *const c_char;
            let flag = b"-c\0".as_ptr() as *const c_char;
            let mut cmd_c: Vec<u8> = comando_owned.clone().into_bytes();
            cmd_c.push(0);
            let cmd_ptr = cmd_c.as_ptr() as *const c_char;
            let argv: [*const c_char; 4] = [sh, flag, cmd_ptr, ptr::null()];
            execvp(sh, argv.as_ptr());
            std::process::exit(127);
        }

        // Padre: cerrar write end, hilo lector
        close(fds[1]);
        let read_fd = fds[0];
        let lector = std::thread::spawn(move || {
            let mut salida: Vec<u8> = Vec::new();
            let mut buf = [0u8; 4096];
            loop {
                let n = read(read_fd, buf.as_mut_ptr(), buf.len());
                if n <= 0 {
                    break;
                }
                salida.extend_from_slice(&buf[..n as usize]);
            }
            close(read_fd);
            salida
        });

        let proc = Box::new(Proceso {
            pid,
            salida: Vec::new(),
            lector: Some(lector),
        });
        Box::into_raw(proc) as *mut c_void
    }

    pub unsafe fn proceso_esperar(handle: *mut c_void) -> i32 {
        if handle.is_null() {
            return -1;
        }
        let proc = &mut *(handle as *mut Proceso);
        let mut status: i32 = 0;
        waitpid(proc.pid, &mut status, 0);
        if let Some(lector) = proc.lector.take() {
            proc.salida = lector.join().unwrap_or_default();
        }
        if status & 0x7f == 0 {
            (status >> 8) & 0xff
        } else {
            -1
        }
    }

    pub unsafe fn proceso_leer_salida(handle: *mut c_void) -> *mut c_char {
        if handle.is_null() {
            return ptr::null_mut();
        }
        let proc = &*(handle as *mut Proceso);
        let len = proc.salida.len();
        let buf = malloc(len + 1) as *mut u8;
        if buf.is_null() {
            return ptr::null_mut();
        }
        if len > 0 {
            std::ptr::copy_nonoverlapping(proc.salida.as_ptr(), buf, len);
        }
        *buf.add(len) = 0;
        buf as *mut c_char
    }

    pub unsafe fn proceso_cerrar(handle: *mut c_void) {
        if handle.is_null() {
            return;
        }
        let _ = Box::from_raw(handle as *mut Proceso);
    }
}

pub use imp::*;
