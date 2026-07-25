// trampolin_win32.c — Funciones helper en C para Falcato GUI
//
// ================================================================
// ARQUITECTURA: Patrón Trampolín C
// ================================================================
// Falcato compila a Cranelift IR -> código máquina x86_64 con C ABI.
// Llamar a funciones Win32 complejas (que requieren structs como
// WNDCLASSEXA, MSG, WNDPROC callbacks) desde Cranelift IR es frágil
// porque los structs grandes (>32 bytes) requieren manipulación
// byte a byte en IR, propensa a errores de layout y alineación.
//
// SOLUCIÓN: Envolver la lógica Win32 compleja en funciones C
// simples que Falcato llama via FFI directo (inseguro fn).
// El .obj se linkea automáticamente desde src/main.rs.
//
// Separación de responsabilidades:
//   Trampolín (C)      → RegisterClassEx, CreateWindowEx, message loop
//   Falcato (FFI)      → MessageBoxA, LoadCursorA, GetModuleHandleA,
//                         SetLastError/GetLastError, lógica de app
// ================================================================
//
// Compilar (una vez):
//   cl /c /Fo:lib\trampolin_win32.obj lib\trampolin_win32.c
//
// Auto-linkeado por falcato build — no requiere flags manuales.

#include <windows.h>

// WNDPROC para la ventana de prueba
// Maneja WM_DESTROY -> PostQuitMessage para salir del message loop
LRESULT CALLBACK fc_WndProc(HWND hwnd, UINT msg, WPARAM wParam, LPARAM lParam) {
    switch (msg) {
    case WM_DESTROY:
        PostQuitMessage(0);
        return 0;
    }
    return DefWindowProcA(hwnd, msg, wParam, lParam);
}

// Crea y muestra una ventana Windows nativa.
// 1. Obtiene HINSTANCE (módulo actual)
// 2. Registra clase "FalcatoVentana" con WNDPROC fc_WndProc
// 3. Crea ventana WS_OVERLAPPEDWINDOW, 800x600
// 4. Muestra con ShowWindow + UpdateWindow
// Retorna: HWND (Entero64) o NULL si falla
HWND __stdcall fc_CrearVentana(void) {
    HINSTANCE hInst = GetModuleHandleA(NULL);

    WNDCLASSEXA wc = {0};
    wc.cbSize = sizeof(WNDCLASSEXA);
    wc.style = CS_HREDRAW | CS_VREDRAW;
    wc.lpfnWndProc = fc_WndProc;
    wc.hInstance = hInst;
    wc.hCursor = LoadCursorA(NULL, IDC_ARROW);
    wc.hbrBackground = (HBRUSH)(COLOR_WINDOW + 1);
    wc.lpszClassName = "FalcatoVentana";

    if (!RegisterClassExA(&wc)) {
        return NULL;
    }

    HWND hwnd = CreateWindowExA(
        0, "FalcatoVentana", "Falcato - Ventana Nativa",
        WS_OVERLAPPEDWINDOW,
        CW_USEDEFAULT, CW_USEDEFAULT, 800, 600,
        NULL, NULL, hInst, NULL
    );

    if (!hwnd) return NULL;

    ShowWindow(hwnd, SW_SHOW);
    UpdateWindow(hwnd);

    return hwnd;
}

// Bucle de mensajes simple — bloquea hasta WM_QUIT
// Procesa: GetMessage -> TranslateMessage -> DispatchMessage
void __stdcall fc_BucleMensajes(void) {
    MSG msg;
    while (GetMessageA(&msg, NULL, 0, 0)) {
        TranslateMessage(&msg);
        DispatchMessageA(&msg);
    }
}
