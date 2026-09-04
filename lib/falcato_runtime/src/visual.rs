//! # Visual — ventanas, lienzo, imagen, sonido
//!
//! Builtins para GUI nativa en Windows (Win32 + GDI+ + WaveOut).
//! En POSIX/Linux se implementará con X11/Cairo/PulseAudio (F9).

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

// ============================================================
// Windows-only: Win32 API bindings
// ============================================================
#[cfg(target_os = "windows")]
mod win32 {
    use std::ffi::c_void;

    pub type HWND = *mut c_void;
    pub type HDC = *mut c_void;
    pub type HBRUSH = *mut c_void;
    pub type HFONT = *mut c_void;
    pub type HBITMAP = *mut c_void;
    pub type HGDIOBJ = *mut c_void;
    pub type UINT = u32;
    pub type DWORD = u32;
    pub type LONG = i32;
    pub type WPARAM = usize;
    pub type LPARAM = isize;
    pub type LRESULT = isize;

    pub const WS_OVERLAPPEDWINDOW: DWORD = 0x00CF0000;
    pub const WS_VISIBLE: DWORD = 0x10000000;
    pub const CW_USEDEFAULT: i32 = 0x80000000u32 as i32;
    pub const SW_SHOW: i32 = 5;
    pub const WM_DESTROY: UINT = 0x0002;
    pub const WM_CLOSE: UINT = 0x0010;
    pub const WM_PAINT: UINT = 0x000F;
    pub const WM_LBUTTONDOWN: UINT = 0x0201;
    pub const WM_KEYDOWN: UINT = 0x0100;
    pub const COLOR_WINDOW: UINT = 5;
    pub const DT_CENTER: UINT = 0x00000001;
    pub const DT_VCENTER: UINT = 0x00000004;
    pub const DT_SINGLELINE: UINT = 0x00000020;
    pub const SRCCOPY: DWORD = 0x00CC0020;

    #[repr(C)]
    pub struct WNDCLASSA {
        pub style: UINT,
        pub lpfnWndProc: *const c_void,
        pub cbClsExtra: i32,
        pub cbWndExtra: i32,
        pub hInstance: *mut c_void,
        pub hIcon: *mut c_void,
        pub hCursor: *mut c_void,
        pub hbrBackground: HBRUSH,
        pub lpszMenuName: *const u8,
        pub lpszClassName: *const u8,
    }

    #[repr(C)]
    pub struct MSG {
        pub hwnd: HWND,
        pub message: UINT,
        pub wParam: WPARAM,
        pub lParam: LPARAM,
        pub time: DWORD,
        pub pt: POINT,
    }

    #[repr(C)]
    pub struct POINT {
        pub x: LONG,
        pub y: LONG,
    }

    #[repr(C)]
    pub struct RECT {
        pub left: LONG,
        pub top: LONG,
        pub right: LONG,
        pub bottom: LONG,
    }

    #[repr(C)]
    pub struct PAINTSTRUCT {
        pub hdc: HDC,
        pub fErase: i32,
        pub rcPaint: RECT,
        pub fRestore: i32,
        pub fIncUpdate: i32,
        pub rgbReserved: [u8; 32],
    }

    #[repr(C)]
    pub struct BITMAPINFOHEADER {
        pub biSize: DWORD,
        pub biWidth: LONG,
        pub biHeight: LONG,
        pub biPlanes: u16,
        pub biBitCount: u16,
        pub biCompression: DWORD,
        pub biSizeImage: DWORD,
        pub biXPelsPerMeter: LONG,
        pub biYPelsPerMeter: LONG,
        pub biClrUsed: DWORD,
        pub biClrImportant: DWORD,
    }

    #[repr(C)]
    pub struct BITMAPINFO {
        pub bmiHeader: BITMAPINFOHEADER,
        pub bmiColors: [DWORD; 1],
    }

    extern "system" {
        pub fn RegisterClassA(lpWndClass: *const WNDCLASSA) -> u16;
        pub fn CreateWindowExA(
            dwExStyle: DWORD, lpClassName: *const u8, lpWindowName: *const u8,
            dwStyle: DWORD, x: i32, y: i32, nWidth: i32, nHeight: i32,
            hWndParent: HWND, hMenu: *mut c_void, hInstance: *mut c_void, lpParam: *mut c_void,
        ) -> HWND;
        pub fn ShowWindow(hWnd: HWND, nCmdShow: i32) -> i32;
        pub fn UpdateWindow(hWnd: HWND) -> i32;
        pub fn DestroyWindow(hWnd: HWND) -> i32;
        pub fn DefWindowProcA(hWnd: HWND, msg: UINT, wParam: WPARAM, lParam: LPARAM) -> LRESULT;
        pub fn GetMessageA(lpMsg: *mut MSG, hWnd: HWND, wMsgFilterMin: UINT, wMsgFilterMax: UINT) -> i32;
        pub fn TranslateMessage(lpMsg: *const MSG) -> i32;
        pub fn DispatchMessageA(lpMsg: *const MSG) -> LRESULT;
        pub fn PostQuitMessage(nExitCode: i32);
        pub fn GetWindowTextA(hWnd: HWND, lpString: *mut u8, nMaxCount: i32) -> i32;
        pub fn SetWindowTextA(hWnd: HWND, lpString: *const u8) -> i32;
        pub fn GetWindowRect(hWnd: HWND, lpRect: *mut RECT) -> i32;
        pub fn GetClientRect(hWnd: HWND, lpRect: *mut RECT) -> i32;
        pub fn BeginPaint(hWnd: HWND, lpPaint: *mut PAINTSTRUCT) -> HDC;
        pub fn EndPaint(hWnd: HWND, lpPaint: *const PAINTSTRUCT) -> i32;
        pub fn GetDC(hWnd: HWND) -> HDC;
        pub fn ReleaseDC(hWnd: HWND, hDC: HDC) -> i32;
        pub fn CreateCompatibleDC(hdc: HDC) -> HDC;
        pub fn CreateCompatibleBitmap(hdc: HDC, nWidth: i32, nHeight: i32) -> HBITMAP;
        pub fn SelectObject(hdc: HDC, hgdiobj: HGDIOBJ) -> HGDIOBJ;
        pub fn DeleteObject(hObject: HGDIOBJ) -> i32;
        pub fn DeleteDC(hdc: HDC) -> i32;
        pub fn BitBlt(hdc: HDC, x: i32, y: i32, cx: i32, cy: i32, hdcSrc: HDC, x1: i32, y1: i32, rop: DWORD) -> i32;
        pub fn TextOutA(hdc: HDC, x: i32, y: i32, lpString: *const u8, c: i32) -> i32;
        pub fn DrawTextA(hdc: HDC, lpchText: *const u8, cchText: i32, lprc: *const RECT, format: UINT) -> i32;
        pub fn CreateSolidBrush(color: DWORD) -> HBRUSH;
        pub fn FillRect(hdc: HDC, lprc: *const RECT, hbr: HBRUSH) -> i32;
        pub fn Rectangle(hdc: HDC, left: i32, top: i32, right: i32, bottom: i32) -> i32;
        pub fn Ellipse(hdc: HDC, left: i32, top: i32, right: i32, bottom: i32) -> i32;
        pub fn MoveToEx(hdc: HDC, x: i32, y: i32, lppt: *mut POINT) -> i32;
        pub fn LineTo(hdc: HDC, x: i32, y: i32) -> i32;
        pub fn SetPixel(hdc: HDC, x: i32, y: i32, color: DWORD) -> DWORD;
        pub fn CreateFontA(nHeight: i32, nWidth: i32, nEscapement: i32, nOrientation: i32,
            fnWeight: i32, fdwItalic: DWORD, fdwUnderline: DWORD, fdwStrikeOut: DWORD,
            fdwCharSet: DWORD, fdwOutputPrecision: DWORD, fdwClipPrecision: DWORD,
            fdwQuality: DWORD, fdwPitchAndFamily: DWORD, lpszFace: *const u8) -> HFONT;
        pub fn GetModuleHandleA(lpModuleName: *const u8) -> *mut c_void;
    }
}

// ============================================================
// Ventana — wrappers Win32
// ============================================================

/// Ventana como handle (HWND en Windows, X11 Window en Linux)
#[cfg(target_os = "windows")]
#[no_mangle]
pub unsafe extern "C" fn falcato_ventana_nueva(
    desc_titulo: i64,
    ancho: i32,
    alto: i32,
) -> i64 {
    use win32::*;

    static mut CLASS_REGISTERED: bool = false;
    static mut HINSTANCE: *mut c_void = std::ptr::null_mut();

    if HINSTANCE.is_null() {
        // Obtener HINSTANCE del proceso actual
        HINSTANCE = GetModuleHandleA(std::ptr::null());
    }

    if !CLASS_REGISTERED {
        let class_name = b"FalcatoWindow\0";
        let wnd_class = WNDCLASSA {
            style: 0,
            lpfnWndProc: DefWindowProcA as *const c_void,
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: HINSTANCE,
            hIcon: std::ptr::null_mut(),
            hCursor: std::ptr::null_mut(),
            hbrBackground: CreateSolidBrush(COLOR_WINDOW as DWORD),
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
        };
        RegisterClassA(&wnd_class);
        CLASS_REGISTERED = true;
    }

    let titulo_ptr = if desc_titulo != 0 {
        leer_campo(desc_titulo, OFFSET_PTR) as *const u8
    } else {
        b"Falcato\0".as_ptr()
    };

    let hwnd = CreateWindowExA(
        0,
        b"FalcatoWindow\0".as_ptr(),
        titulo_ptr,
        WS_OVERLAPPEDWINDOW | WS_VISIBLE,
        CW_USEDEFAULT, CW_USEDEFAULT,
        ancho, alto,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        HINSTANCE,
        std::ptr::null_mut(),
    );

    if hwnd.is_null() {
        0
    } else {
        hwnd as i64
    }
}

#[cfg(not(target_os = "windows"))]
#[no_mangle]
pub unsafe extern "C" fn falcato_ventana_nueva(
    _desc_titulo: i64,
    _ancho: i32,
    _alto: i32,
) -> i64 {
    // TODO: X11/Cocoa implementation
    0
}

/// Muestra la ventana
#[cfg(target_os = "windows")]
#[no_mangle]
pub unsafe extern "C" fn falcato_ventana_mostrar(hwnd: i64) {
    #[cfg(target_os = "windows")]
    {
        win32::ShowWindow(hwnd as win32::HWND, win32::SW_SHOW);
        win32::UpdateWindow(hwnd as win32::HWND);
    }
}

#[cfg(not(target_os = "windows"))]
#[no_mangle]
pub unsafe extern "C" fn falcato_ventana_mostrar(_hwnd: i64) {}

/// Cierra la ventana
#[cfg(target_os = "windows")]
#[no_mangle]
pub unsafe extern "C" fn falcato_ventana_cerrar(hwnd: i64) {
    #[cfg(target_os = "windows")]
    {
        win32::DestroyWindow(hwnd as win32::HWND);
    }
}

#[cfg(not(target_os = "windows"))]
#[no_mangle]
pub unsafe extern "C" fn falcato_ventana_cerrar(_hwnd: i64) {}

/// Bucle de mensajes — retorna 0 al cerrar
#[cfg(target_os = "windows")]
#[no_mangle]
pub unsafe extern "C" fn falcato_ventana_bucle_mensajes(hwnd: i64) -> i32 {
    #[cfg(target_os = "windows")]
    {
        let mut msg: win32::MSG = std::mem::zeroed();
        loop {
            let result = win32::GetMessageA(&mut msg, hwnd as win32::HWND, 0, 0);
            if result == 0 || result == -1 {
                break;
            }
            win32::TranslateMessage(&msg);
            win32::DispatchMessageA(&msg);
        }
        msg.wParam as i32
    }
    #[cfg(not(target_os = "windows"))]
    { 0 }
}

#[cfg(not(target_os = "windows"))]
#[no_mangle]
pub unsafe extern "C" fn falcato_ventana_bucle_mensajes(_hwnd: i64) -> i32 { 0 }

/// Obtiene el título de la ventana
#[cfg(target_os = "windows")]
#[no_mangle]
pub unsafe extern "C" fn falcato_ventana_titulo(hwnd: i64, desc_out: i64) {
    #[cfg(target_os = "windows")]
    {
        let mut buf = [0u8; 256];
        let len = win32::GetWindowTextA(hwnd as win32::HWND, buf.as_mut_ptr(), 256);
        if len > 0 {
            texto_desde_buffer(&buf[..len as usize], desc_out);
        } else {
            texto_desde_buffer(b"", desc_out);
        }
    }
}

#[cfg(not(target_os = "windows"))]
#[no_mangle]
pub unsafe extern "C" fn falcato_ventana_titulo(_hwnd: i64, desc_out: i64) {
    texto_desde_buffer(b"", desc_out);
}

/// Establece el título de la ventana
#[cfg(target_os = "windows")]
#[no_mangle]
pub unsafe extern "C" fn falcato_ventana_establecer_titulo(hwnd: i64, desc_titulo: i64) {
    #[cfg(target_os = "windows")]
    {
        let ptr = leer_campo(desc_titulo, OFFSET_PTR) as *const u8;
        win32::SetWindowTextA(hwnd as win32::HWND, ptr);
    }
}

#[cfg(not(target_os = "windows"))]
#[no_mangle]
pub unsafe extern "C" fn falcato_ventana_establecer_titulo(_hwnd: i64, _desc_titulo: i64) {}

/// Obtiene posición de la ventana (x, y)
#[cfg(target_os = "windows")]
#[no_mangle]
pub unsafe extern "C" fn falcato_ventana_posicion(hwnd: i64, desc_out: i64) {
    #[cfg(target_os = "windows")]
    {
        let mut rect: win32::RECT = std::mem::zeroed();
        win32::GetWindowRect(hwnd as win32::HWND, &mut rect);
        // Escribir como struct { x: Entero32, y: Entero32 }
        *(desc_out as *mut i32) = rect.left;
        *((desc_out + 4) as *mut i32) = rect.top;
    }
}

#[cfg(not(target_os = "windows"))]
#[no_mangle]
pub unsafe extern "C" fn falcato_ventana_posicion(_hwnd: i64, desc_out: i64) {
    *(desc_out as *mut i32) = 0;
    *((desc_out + 4) as *mut i32) = 0;
}

/// Obtiene tamaño de la ventana (ancho, alto)
#[cfg(target_os = "windows")]
#[no_mangle]
pub unsafe extern "C" fn falcato_ventana_tamano(hwnd: i64, desc_out: i64) {
    #[cfg(target_os = "windows")]
    {
        let mut rect: win32::RECT = std::mem::zeroed();
        win32::GetClientRect(hwnd as win32::HWND, &mut rect);
        *(desc_out as *mut i32) = rect.right - rect.left;
        *((desc_out + 4) as *mut i32) = rect.bottom - rect.top;
    }
}

#[cfg(not(target_os = "windows"))]
#[no_mangle]
pub unsafe extern "C" fn falcato_ventana_tamano(_hwnd: i64, desc_out: i64) {
    *(desc_out as *mut i32) = 0;
    *((desc_out + 4) as *mut i32) = 0;
}

// ============================================================
// Lienzo (Canvas 2D) — wrappers GDI
// ============================================================

/// Lienzo = HDC double-buffered (compatible DC + bitmap)
#[repr(C)]
struct LienzoInterno {
    hwnd: i64,      // Ventana asociada (0 = offscreen)
    hdc: i64,       // Device context
    hbitmap: i64,   // Bitmap offscreen
    ancho: i32,
    alto: i32,
}

/// Crea un lienzo offscreen de las dimensiones dadas
#[cfg(target_os = "windows")]
#[no_mangle]
pub unsafe extern "C" fn falcato_lienzo_nuevo(ancho: i32, alto: i32) -> i64 {
    #[cfg(target_os = "windows")]
    {
        use win32::*;

        let hdc_screen = GetDC(std::ptr::null_mut());
        let hdc_mem = CreateCompatibleDC(hdc_screen);
        let hbitmap = CreateCompatibleBitmap(hdc_screen, ancho, alto);
        SelectObject(hdc_mem, hbitmap as HGDIOBJ);
        ReleaseDC(std::ptr::null_mut(), hdc_screen);

        // Allocar LienzoInterno
        let lienzo = malloc(std::mem::size_of::<LienzoInterno>());
        if lienzo.is_null() { return 0; }

        let li = lienzo as *mut LienzoInterno;
        (*li).hwnd = 0;
        (*li).hdc = hdc_mem as i64;
        (*li).hbitmap = hbitmap as i64;
        (*li).ancho = ancho;
        (*li).alto = alto;

        lienzo as i64
    }
}

#[cfg(not(target_os = "windows"))]
#[no_mangle]
pub unsafe extern "C" fn falcato_lienzo_nuevo(_ancho: i32, _alto: i32) -> i64 { 0 }

/// Limpia el lienzo con un color
#[cfg(target_os = "windows")]
#[no_mangle]
pub unsafe extern "C" fn falcato_lienzo_limpiar(desc_lienzo: i64, color: i32) {
    #[cfg(target_os = "windows")]
    {
        use win32::*;

        let li = desc_lienzo as *mut LienzoInterno;
        let hdc = (*li).hdc as HDC;
        let ancho = (*li).ancho;
        let alto = (*li).alto;

        let brush = CreateSolidBrush(color as DWORD);
        let mut rect = RECT { left: 0, top: 0, right: ancho, bottom: alto };
        FillRect(hdc, &mut rect, brush);
        DeleteObject(brush as HGDIOBJ);
    }
}

#[cfg(not(target_os = "windows"))]
#[no_mangle]
pub unsafe extern "C" fn falcato_lienzo_limpiar(_desc_lienzo: i64, _color: i32) {}

/// Dibuja una línea
#[cfg(target_os = "windows")]
#[no_mangle]
pub unsafe extern "C" fn falcato_lienzo_linea(
    desc_lienzo: i64, x1: i32, y1: i32, x2: i32, y2: i32
) {
    #[cfg(target_os = "windows")]
    {
        use win32::*;

        let li = desc_lienzo as *mut LienzoInterno;
        let hdc = (*li).hdc as HDC;

        let brush = CreateSolidBrush(0x00000000); // negro
        SelectObject(hdc, brush as HGDIOBJ);
        let mut pt: POINT = std::mem::zeroed();
        MoveToEx(hdc, x1, y1, &mut pt);
        LineTo(hdc, x2, y2);
        DeleteObject(brush as HGDIOBJ);
    }
}

#[cfg(not(target_os = "windows"))]
#[no_mangle]
pub unsafe extern "C" fn falcato_lienzo_linea(
    _desc_lienzo: i64, _x1: i32, _y1: i32, _x2: i32, _y2: i32
) {}

/// Dibuja un rectángulo
#[cfg(target_os = "windows")]
#[no_mangle]
pub unsafe extern "C" fn falcato_lienzo_rectangulo(
    desc_lienzo: i64, x: i32, y: i32, ancho: i32, alto: i32
) {
    #[cfg(target_os = "windows")]
    {
        use win32::*;

        let li = desc_lienzo as *mut LienzoInterno;
        let hdc = (*li).hdc as HDC;

        let brush = CreateSolidBrush(0x00000000);
        SelectObject(hdc, brush as HGDIOBJ);
        Rectangle(hdc, x, y, x + ancho, y + alto);
        DeleteObject(brush as HGDIOBJ);
    }
}

#[cfg(not(target_os = "windows"))]
#[no_mangle]
pub unsafe extern "C" fn falcato_lienzo_rectangulo(
    _desc_lienzo: i64, _x: i32, _y: i32, _ancho: i32, _alto: i32
) {}

/// Dibuja un círculo (elipse inscrita en el rectángulo)
#[cfg(target_os = "windows")]
#[no_mangle]
pub unsafe extern "C" fn falcato_lienzo_circulo(
    desc_lienzo: i64, cx: i32, cy: i32, radio: i32
) {
    #[cfg(target_os = "windows")]
    {
        use win32::*;

        let li = desc_lienzo as *mut LienzoInterno;
        let hdc = (*li).hdc as HDC;

        let brush = CreateSolidBrush(0x00000000);
        SelectObject(hdc, brush as HGDIOBJ);
        Ellipse(hdc, cx - radio, cy - radio, cx + radio, cy + radio);
        DeleteObject(brush as HGDIOBJ);
    }
}

#[cfg(not(target_os = "windows"))]
#[no_mangle]
pub unsafe extern "C" fn falcato_lienzo_circulo(
    _desc_lienzo: i64, _cx: i32, _cy: i32, _radio: i32
) {}

/// Dibuja texto en el lienzo
#[cfg(target_os = "windows")]
#[no_mangle]
pub unsafe extern "C" fn falcato_lienzo_texto(
    desc_lienzo: i64, x: i32, y: i32, desc_texto: i64
) {
    #[cfg(target_os = "windows")]
    {
        use win32::*;

        let li = desc_lienzo as *mut LienzoInterno;
        let hdc = (*li).hdc as HDC;

        let texto_ptr = leer_campo(desc_texto, OFFSET_PTR) as *const u8;
        let texto_len = leer_campo(desc_texto, OFFSET_LEN) as i32;

        TextOutA(hdc, x, y, texto_ptr, texto_len);
    }
}

#[cfg(not(target_os = "windows"))]
#[no_mangle]
pub unsafe extern "C" fn falcato_lienzo_texto(
    _desc_lienzo: i64, _x: i32, _y: i32, _desc_texto: i64
) {}

/// Guarda el lienzo como PNG (placeholder — requiere libpng o stb_image_write)
#[no_mangle]
pub unsafe extern "C" fn falcato_lienzo_guardar_png(
    _desc_lienzo: i64, _desc_ruta: i64
) -> i32 {
    // TODO: implementar con stb_image_write o libpng
    -1
}

/// Libia un lienzo
#[cfg(target_os = "windows")]
#[no_mangle]
pub unsafe extern "C" fn falcato_lienzo_liberar(desc_lienzo: i64) {
    #[cfg(target_os = "windows")]
    {
        use win32::*;

        let li = desc_lienzo as *mut LienzoInterno;
        if !li.is_null() {
            let hdc = (*li).hdc as HDC;
            let hbitmap = (*li).hbitmap as HBITMAP;
            if !hbitmap.is_null() {
                DeleteObject(hbitmap as HGDIOBJ);
            }
            if !hdc.is_null() {
                DeleteDC(hdc);
            }
            free(li as *mut c_void);
        }
    }
}

#[cfg(not(target_os = "windows"))]
#[no_mangle]
pub unsafe extern "C" fn falcato_lienzo_liberar(_desc_lienzo: i64) {}

// ============================================================
// Imagen — wrappers stubs (requiere stb_image)
// ============================================================

/// Carga imagen desde archivo (stub — requiere stb_image)
#[no_mangle]
pub unsafe extern "C" fn falcato_imagen_desde_archivo(
    _desc_ruta: i64, desc_out: i64
) -> i32 {
    // TODO: implementar con stb_image
    escribir_campo(desc_out, OFFSET_PTR, 0);
    escribir_campo(desc_out, OFFSET_LEN, 0);
    -1
}

/// Ancho de imagen (stub)
#[no_mangle]
pub unsafe extern "C" fn falcato_imagen_ancho(_desc_img: i64) -> i32 { 0 }

/// Alto de imagen (stub)
#[no_mangle]
pub unsafe extern "C" fn falcato_imagen_alto(_desc_img: i64) -> i32 { 0 }

/// Redimensiona imagen (stub)
#[no_mangle]
pub unsafe extern "C" fn falcato_imagen_redimensionar(
    _desc_img: i64, _ancho: i32, _alto: i32, desc_out: i64
) {
    escribir_campo(desc_out, OFFSET_PTR, 0);
    escribir_campo(desc_out, OFFSET_LEN, 0);
}

/// Guarda imagen como PNG (stub)
#[no_mangle]
pub unsafe extern "C" fn falcato_imagen_guardar_png(
    _desc_img: i64, _desc_ruta: i64
) -> i32 { -1 }

/// Libera imagen (stub)
#[no_mangle]
pub unsafe extern "C" fn falcato_imagen_liberar(_desc_img: i64) {}

// ============================================================
// Sonido — wrappers stubs (requiere WaveOut/PulseAudio)
// ============================================================

/// Crea un buffer de audio vacío
#[no_mangle]
pub unsafe extern "C" fn falcato_audio_nuevo(
    canales: i32, frecuencia: i32, desc_out: i64
) {
    // Audio = { muestras: Vector<Flotante64>, canales: Entero32, frecuencia: Entero32 }
    // Por ahora: stub
    escribir_campo(desc_out, OFFSET_PTR, 0);
    escribir_campo(desc_out, OFFSET_LEN, 0);
}

/// Carga audio desde archivo WAV (stub)
#[no_mangle]
pub unsafe extern "C" fn falcato_audio_desde_archivo(
    _desc_ruta: i64, desc_out: i64
) -> i32 {
    escribir_campo(desc_out, OFFSET_PTR, 0);
    escribir_campo(desc_out, OFFSET_LEN, 0);
    -1
}

/// Genera un tono puro (stub)
#[no_mangle]
pub unsafe extern "C" fn falcato_audio_tono(
    _frecuencia: f64, _duracion_ms: i32, _canales: i32, _frecuencia_muestra: i32,
    desc_out: i64
) {
    escribir_campo(desc_out, OFFSET_PTR, 0);
    escribir_campo(desc_out, OFFSET_LEN, 0);
}

/// Mezcla dos buffers de audio (stub)
#[no_mangle]
pub unsafe extern "C" fn falcato_audio_mezclar(
    _desc_a: i64, _desc_b: i64, desc_out: i64
) {
    escribir_campo(desc_out, OFFSET_PTR, 0);
    escribir_campo(desc_out, OFFSET_LEN, 0);
}

/// Fade in (stub)
#[no_mangle]
pub unsafe extern "C" fn falcato_audio_fade_in(
    _desc_audio: i64, _duracion_ms: i32
) {}

/// Fade out (stub)
#[no_mangle]
pub unsafe extern "C" fn falcato_audio_fade_out(
    _desc_audio: i64, _duracion_ms: i32
) {}

/// Guarda como WAV (stub)
#[no_mangle]
pub unsafe extern "C" fn falcato_audio_guardar_wav(
    _desc_audio: i64, _desc_ruta: i64
) -> i32 { -1 }

/// Reproduce audio (stub)
#[no_mangle]
pub unsafe extern "C" fn falcato_audio_reproducir(
    _desc_audio: i64
) -> i32 { -1 }


