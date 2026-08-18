//! # Terminal — modo raw y lectura de teclas (TUI)
//!
//! Abstracción portable sobre consola (Win32) y termios (POSIX).
//!
//! API expuesta (C ABI):
//! - `falcato_terminal_modo_raw(activo) -> i32` — activa/desactiva modo raw
//! - `falcato_terminal_leer_tecla() -> i32`     — bloquea hasta una tecla, devuelve código
//!
//! Códigos devueltos:
//! - 0..=127: carácter ASCII/UTF-8 byte
//! - 0x100 + n: teclas especiales (flechas, Enter, Tab, Esc, Backspace)
//!
//! En Windows también activa ENABLE_VIRTUAL_TERMINAL_PROCESSING para
//! que los códigos ANSI de color/cursor funcionen.

use std::ffi::c_void;

// ============================================================
// Windows
// ============================================================
#[cfg(target_os = "windows")]
mod imp {
    use std::ffi::c_void;
    use std::ptr;

    const STD_INPUT_HANDLE: i32 = -10;
    const STD_OUTPUT_HANDLE: i32 = -11;

    // Modo raw de entrada: sin echo, sin line-buffering (solo VT input)
    const ENABLE_VIRTUAL_TERMINAL_INPUT: u32 = 0x0200;

    // Modos de salida
    const ENABLE_PROCESSED_OUTPUT: u32 = 0x0001;
    const ENABLE_VIRTUAL_TERMINAL_PROCESSING: u32 = 0x0004;

    // Tipos de evento
    const KEY_EVENT: u16 = 0x0001;

    // Códigos virtuales (mapas a 0x100+ para distinguir de ASCII)
    const VK_UP: i32 = 0x26;
    const VK_DOWN: i32 = 0x28;
    const VK_LEFT: i32 = 0x25;
    const VK_RIGHT: i32 = 0x27;
    const VK_RETURN: i32 = 0x0D;
    const VK_BACK: i32 = 0x08;
    const VK_TAB: i32 = 0x09;
    const VK_ESCAPE: i32 = 0x1B;
    const VK_DELETE: i32 = 0x2E;
    const VK_HOME: i32 = 0x24;
    const VK_END: i32 = 0x23;
    const VK_PRIOR: i32 = 0x21; // PageUp
    const VK_NEXT: i32 = 0x22;  // PageDown
    const VK_INSERT: i32 = 0x2D;

    #[repr(C)]
    struct InputRecord {
        event_type: u16,
        _pad: u16,
        // KEY_EVENT_RECORD (los otros eventos tienen el mismo tamaño de union)
        key_down: i32,
        repeat_count: u16,
        virtual_key_code: u16,
        virtual_scan_code: u16,
        unicode_char: u16,
        control_key_state: u32,
    }

    #[repr(C)]
    struct Coord {
        x: i16,
        y: i16,
    }

    #[repr(C)]
    struct SmallRect {
        left: i16,
        top: i16,
        right: i16,
        bottom: i16,
    }

    #[repr(C)]
    struct CONSOLE_SCREEN_BUFFER_INFO {
        dw_size: Coord,
        dw_cursor_position: Coord,
        w_attributes: u16,
        sr_window: SmallRect,
        dw_maximum_window_size: Coord,
    }

    extern "system" {
        fn GetStdHandle(n_std_handle: i32) -> *mut c_void;
        fn GetConsoleMode(h_console_handle: *mut c_void, lp_mode: *mut u32) -> i32;
        fn SetConsoleMode(h_console_handle: *mut c_void, dw_mode: u32) -> i32;
        fn ReadConsoleInputW(
            h_console_input: *mut c_void,
            lp_buffer: *mut InputRecord,
            n_length: u32,
            lp_number_of_events_read: *mut u32,
        ) -> i32;
        fn GetConsoleScreenBufferInfo(
            h_console_output: *mut c_void,
            lp_console_screen_buffer_info: *mut CONSOLE_SCREEN_BUFFER_INFO,
        ) -> i32;
    }

    static mut MODO_ORIGINAL_INPUT: u32 = 0;
    static mut MODO_ORIGINAL_OUTPUT: u32 = 0;
    static mut MODO_GUARDADO: bool = false;

    fn tecla_especial(vk: u16) -> bool {
        matches!(vk as i32,
            VK_UP | VK_DOWN | VK_LEFT | VK_RIGHT | VK_RETURN | VK_BACK | VK_TAB |
            VK_ESCAPE | VK_DELETE | VK_HOME | VK_END | VK_PRIOR | VK_NEXT | VK_INSERT)
    }

    pub unsafe fn terminal_modo_raw(activo: i32) -> i32 {
        let input = GetStdHandle(STD_INPUT_HANDLE);
        let output = GetStdHandle(STD_OUTPUT_HANDLE);

        // Comprobar si los handles son de consola real (GetConsoleMode falla si
        // la salida/entrada está redirigida a pipe/archivo)
        let mut in_mode: u32 = 0;
        let mut out_mode: u32 = 0;
        let input_es_consola = GetConsoleMode(input, &mut in_mode) != 0;
        let output_es_consola = GetConsoleMode(output, &mut out_mode) != 0;

        if activo != 0 {
            // Guardar modos originales solo una vez (y solo si son consola)
            if !MODO_GUARDADO {
                if input_es_consola {
                    MODO_ORIGINAL_INPUT = in_mode;
                }
                if output_es_consola {
                    MODO_ORIGINAL_OUTPUT = out_mode;
                }
                MODO_GUARDADO = true;
            }

            // Modo raw de entrada: sin echo, sin line-buffering (solo VT input)
            if input_es_consola {
                if SetConsoleMode(input, ENABLE_VIRTUAL_TERMINAL_INPUT) == 0 {
                    // No es fatal: continuar con output
                }
            }

            // Activar ANSI en salida (si es consola)
            if output_es_consola {
                let ansi_output = ENABLE_PROCESSED_OUTPUT | ENABLE_VIRTUAL_TERMINAL_PROCESSING;
                if SetConsoleMode(output, ansi_output) == 0 {
                    return 0;
                }
            }
            1
        } else {
            // Restaurar modos originales (solo si se guardaron)
            if MODO_GUARDADO {
                if input_es_consola {
                    SetConsoleMode(input, MODO_ORIGINAL_INPUT);
                }
                if output_es_consola {
                    SetConsoleMode(output, MODO_ORIGINAL_OUTPUT);
                }
                MODO_GUARDADO = false;
            }
            1
        }
    }

    pub unsafe fn terminal_leer_tecla() -> i32 {
        let input = GetStdHandle(STD_INPUT_HANDLE);
        if GetConsoleMode(input, &mut 0) == 0 {
            // Entrada no es consola (redirigida): leer byte de stdin como fallback
            let mut buf = [0u8; 1];
            // ReadFile sobre el handle de input
            extern "system" {
                fn ReadFile(
                    h_file: *mut c_void,
                    lp_buffer: *mut u8,
                    n_number_of_bytes_to_read: u32,
                    lp_number_of_bytes_read: *mut u32,
                    lp_overlapped: *mut c_void,
                ) -> i32;
            }
            let mut leidos: u32 = 0;
            if ReadFile(input, buf.as_mut_ptr(), 1, &mut leidos, ptr::null_mut()) == 0 || leidos == 0 {
                return -1;
            }
            return buf[0] as i32;
        }

        loop {
            let mut record: InputRecord = std::mem::zeroed();
            let mut leidos: u32 = 0;

            let ok = ReadConsoleInputW(input, &mut record, 1, &mut leidos);
            if ok == 0 || leidos == 0 {
                return -1; // error / EOF
            }

            // Solo teclas presionadas (key_down != 0)
            if record.event_type == KEY_EVENT && record.key_down != 0 {
                let vk = record.virtual_key_code as i32;

                // Caracteres imprimibles: usar UnicodeChar
                let ch = record.unicode_char;
                if ch != 0 && !tecla_especial(ch as u16) {
                    // Devolver el carácter (limitado a BMP por simplicidad)
                    return ch as i32;
                }

                // Teclas especiales
                match vk {
                    VK_UP => return 0x100 + 0,
                    VK_DOWN => return 0x100 + 1,
                    VK_LEFT => return 0x100 + 2,
                    VK_RIGHT => return 0x100 + 3,
                    VK_RETURN => return 0x100 + 4,
                    VK_BACK => return 0x100 + 5,
                    VK_TAB => return 0x100 + 6,
                    VK_ESCAPE => return 0x100 + 7,
                    VK_DELETE => return 0x100 + 8,
                    VK_HOME => return 0x100 + 9,
                    VK_END => return 0x100 + 10,
                    VK_PRIOR => return 0x100 + 11,
                    VK_NEXT => return 0x100 + 12,
                    VK_INSERT => return 0x100 + 13,
                    _ => {
                        // Si hay char repetido (ej: Ctrl+C = 3), devolverlo
                        if ch != 0 {
                            return ch as i32;
                        }
                        // Ignorar otras teclas (Shift, Ctrl, Alt solos)
                        continue;
                    }
                }
            }
        }
    }

    /// Obtiene dimensiones de la terminal (ancho, alto).
    /// Retorna Entero64 empaquetado: ancho en low 32, alto en high 32.
    #[no_mangle]
    pub unsafe extern "C" fn falcato_terminal_dimensiones() -> i64 {
        let h = GetStdHandle(STD_OUTPUT_HANDLE);
        let mut info: CONSOLE_SCREEN_BUFFER_INFO = std::mem::zeroed();
        if GetConsoleScreenBufferInfo(h, &mut info) == 0 {
            return 0;
        }
        let ancho = info.dw_size.x as i64;
        let alto = info.dw_size.y as i64;
        (alto << 32) | ancho
    }
}

// ============================================================
// POSIX (Linux/macOS)
// ============================================================
#[cfg(not(target_os = "windows"))]
mod imp {
    use std::ptr;

    const STDIN_FILENO: i32 = 0;
    const STDOUT_FILENO: i32 = 1;

    #[repr(C)]
    struct Termios {
        c_iflag: u32,
        c_oflag: u32,
        c_cflag: u32,
        c_lflag: u32,
        c_line: u8,
        c_cc: [u8; 32],
        c_ispeed: u32,
        c_ospeed: u32,
    }

    const ICANON: u32 = 0x0002;
    const ECHO: u32 = 0x0008;
    const ISIG: u32 = 0x0001;
    const VTIME: usize = 5;
    const VMIN: usize = 6;

    extern "C" {
        fn tcgetattr(fd: i32, termios_p: *mut Termios) -> i32;
        fn tcsetattr(fd: i32, optional_actions: i32, termios_p: *const Termios) -> i32;
        fn read(fd: i32, buf: *mut u8, count: usize) -> isize;
        fn write(fd: i32, buf: *const u8, count: usize) -> isize;
    }

    static mut MODO_ORIGINAL: Termios = Termios {
        c_iflag: 0,
        c_oflag: 0,
        c_cflag: 0,
        c_lflag: 0,
        c_line: 0,
        c_cc: [0; 32],
        c_ispeed: 0,
        c_ospeed: 0,
    };
    static mut MODO_GUARDADO: bool = false;

    pub unsafe fn terminal_modo_raw(activo: i32) -> i32 {
        if activo != 0 {
            let mut t: Termios = std::mem::zeroed();
            if tcgetattr(STDIN_FILENO, &mut t) != 0 {
                return 0;
            }
            if !MODO_GUARDADO {
                MODO_ORIGINAL = t;
                MODO_GUARDADO = true;
            }
            // Modo raw: sin canon, sin echo, sin señales
            t.c_lflag &= !(ICANON | ECHO | ISIG);
            t.c_cc[VMIN] = 1;
            t.c_cc[VTIME] = 0;
            if tcsetattr(STDIN_FILENO, 0, &t) != 0 {
                return 0;
            }
            1
        } else {
            if MODO_GUARDADO {
                tcsetattr(STDIN_FILENO, 0, &MODO_ORIGINAL);
                MODO_GUARDADO = false;
            }
            1
        }
    }

    pub unsafe fn terminal_leer_tecla() -> i32 {
        let mut buf = [0u8; 1];
        let n = read(STDIN_FILENO, buf.as_mut_ptr(), 1);
        if n <= 0 {
            return -1;
        }
        let c = buf[0];

        // Secuencia de escape ANSI: ESC [ <byte>
        if c == 0x1B {
            let mut b2 = [0u8; 1];
            let n2 = read(STDIN_FILENO, b2.as_mut_ptr(), 1);
            if n2 <= 0 {
                return 0x100 + 7; // Esc
            }
            if b2[0] == b'[' {
                let mut b3 = [0u8; 1];
                let n3 = read(STDIN_FILENO, b3.as_mut_ptr(), 1);
                if n3 <= 0 {
                    return 0x100 + 7;
                }
                return match b3[0] {
                    b'A' => 0x100 + 0, // Up
                    b'B' => 0x100 + 1, // Down
                    b'C' => 0x100 + 3, // Right
                    b'D' => 0x100 + 2, // Left
                    b'H' => 0x100 + 9, // Home
                    b'F' => 0x100 + 10, // End
                    b'3' => 0x100 + 8, // Delete (ESC [ 3 ~)
                    b'5' => 0x100 + 11, // PageUp
                    b'6' => 0x100 + 12, // PageDown
                    b'2' => 0x100 + 13, // Insert
                    _ => 0x100 + 7,
                };
            }
            return 0x100 + 7;
        }

        match c {
            b'\r' | b'\n' => 0x100 + 4, // Enter
            0x7F => 0x100 + 5,          // Backspace
            b'\t' => 0x100 + 6,         // Tab
            b'\x03' => 3,               // Ctrl+C
            _ => c as i32,
        }
    }

    /// Obtiene dimensiones de la terminal (ancho, alto).
    /// Retorna Entero64 empaquetado: ancho en low 32, alto en high 32.
    #[no_mangle]
    pub unsafe extern "C" fn falcato_terminal_dimensiones() -> i64 {
        #[repr(C)]
        struct Winsize {
            ws_row: u16,
            ws_col: u16,
            ws_xpixel: u16,
            ws_ypixel: u16,
        }
        
        const TIOCGWINSZ: u64 = 0x5413;
        
        extern "C" {
            fn ioctl(fd: i32, request: u64, argp: *mut Winsize) -> i32;
        }
        
        let mut ws: Winsize = std::mem::zeroed();
        if ioctl(STDOUT_FILENO, TIOCGWINSZ, &mut ws) < 0 {
            return 0;
        }
        let ancho = ws.ws_col as i64;
        let alto = ws.ws_row as i64;
        (alto << 32) | ancho
    }
}

pub use imp::*;
