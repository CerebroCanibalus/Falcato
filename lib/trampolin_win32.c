// trampolin_win32.c — Funciones helper en C para Falcato GUI
// Compila: cl /c /Fo:trampolin_win32.obj trampolin_win32.c
// Linkea: falcato build incluye el .obj automáticamente

#include <windows.h>

// WNDPROC para la ventana de prueba
LRESULT CALLBACK fc_WndProc(HWND hwnd, UINT msg, WPARAM wParam, LPARAM lParam) {
    switch (msg) {
    case WM_DESTROY:
        PostQuitMessage(0);
        return 0;
    }
    return DefWindowProcA(hwnd, msg, wParam, lParam);
}

// Crea y muestra una ventana simple, retorna HWND o NULL si falla
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
void __stdcall fc_BucleMensajes(void) {
    MSG msg;
    while (GetMessageA(&msg, NULL, 0, 0)) {
        TranslateMessage(&msg);
        DispatchMessageA(&msg);
    }
}
