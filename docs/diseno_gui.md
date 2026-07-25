# 🖼️ Ventana — Sistema Gráfico Nativo de Falcato

> **Fase:** GUI-1  
> **Estado:** Diseño  
> **Dependencias:** FFI a Win32 (user32, gdi32, dwmapi, d2d1)  
> **Filosofía:** Cero abstracciones gratuitas. Ownership-driven. LLM-friendly.

---

## 1. Filosofía de diseño

Ventana no es "otro binding a Win32". Explota las dimensiones únicas de Falcato:

| Dimensión Falcato | Aplicación en GUI |
|-------------------|-------------------|
| **`el`/`la`/`los`/`las`** | Recursos GDI con lifetime automático. `el` = owned (DeleteObject al drop), `la` = borrowed, `los` = refcounted |
| **Ser/Estar** | `es` = atributo permanente (clase ventana), `está` = estado temporal (visible, maximizada) |
| **Subjuntivo** | `fuese` en WM_PAINT → cold-block optimization para paint complejo |
| **Bitfields** | Estilos de ventana y controles como `bits { }` con verificación compile-time |
| **Regiones** | Arena allocation por fotograma (GetDC/ReleaseDC automático) |
| **Rasgos** | `Renderizable`, `Clickable`, `Redimensionable` como traits gráficos |
| **C ABI default** | 1:1 con Win32 API — sin overhead de marshalling ni P/Invoke |

### Meta principal

**Que un LLM pueda generar interfaces gráficas funcionales sin alucinar APIs.**

Cada error tiene código, span y sugerencia concreta. Los nombres de funciones Win32
se envuelven en español semántico (`dc.rellenar_rect()` en vez de `FillRect()`).

---

## 2. Arquitectura en capas

```
┌─────────────────────────────────────────────┐
│  APLICACIÓN                                  │
│  aplicación.fc — bucle de mensajes, timer    │
├─────────────────────────────────────────────┤
│  DISEÑO                                      │
│  contenedor.fc — layout, anchors            │
│  evento.fc — dispatch, binding              │
├─────────────────────────────────────────────┤
│  CONTROLES                                   │
│  botón.fc, caja.fc, etiqueta.fc             │
│  lista.fc, barra.fc, área_texto.fc          │
│  (Wrapper sobre Common Controls + owner-draw)│
├─────────────────────────────────────────────┤
│  GRÁFICO                                     │
│  dc.fc, color.fc, lápiz.fc, brocha.fc       │
│  fuente.fc, bitmap.fc, ruta.fc              │
│  (GDI → GDI+ → Direct2D progresivo)         │
├─────────────────────────────────────────────┤
│  NÚCLEO                                      │
│  núcleo.fc — HWND, WNDCLASS, RegisterClass   │
│  mensaje.fc — WM_*, WPARAM, LPARAM          │
│  punto.fc, rect.fc — geometría básica        │
└─────────────────────────────────────────────┘
```

### 2.1 Núcleo — Bindings directos a Win32

```falcato
// núcleo.fc — Tipos fundamentales
estructural Punto { x: Entero32, y: Entero32 }
estructural Rect { izquierda: Entero32, superior: Entero32, derecha: Entero32, inferior: Entero32 }

// HWND como puntero opaco
estructural Ventana { hwnd: *mut Entero32 }  // HANDLE

// WNDCLASS para registro
estructural ClaseVentana {
    estilo: Entero32,
    proc_ventana: *mut Entero32,  // WNDPROC
    fondo: *mut Entero32,         // HBRUSH
    cursor: *mut Entero32,        // HCURSOR
    icono: *mut Entero32,         // HICON
    nombre: Palabra,
}

// Mensajes — enumeración completa
enumeración Mensaje {
    // Ventana
    Crear = 1,         // WM_CREATE
    Destruir = 2,      // WM_DESTROY
    Cerrar = 16,       // WM_CLOSE
    Pintar = 15,       // WM_PAINT
    Tamaño = 5,        // WM_SIZE
    // Ratón
    ClickIzquierdo = 513,  // WM_LBUTTONDOWN
    SoltarIzquierdo = 514, // WM_LBUTTONUP
    Mover = 512,           // WM_MOUSEMOVE
    // Teclado
    TeclaAbajo = 256,  // WM_KEYDOWN
    TeclaArriba = 257,  // WM_KEYUP
    // Comando
    Comando = 273,     // WM_COMMAND
}
```

### 2.2 Gráfico — Device Context + GDI

```falcato
// dc.fc — Device Context wrapper con ownership
estructural DC {
    hdc: *mut Entero32,
    ventana: &Ventana,
    propio: Booleano,  // ¿necesita ReleaseDC al drop?
}

el función DC.nueva_ventana(la v: &Ventana) -> DC  // GetDC
el función DC.nueva_pintura(la v: &Ventana) -> DC   // BeginPaint
la función DC.liberar(el self: DC)                    // ReleaseDC / EndPaint

// Métodos de dibujo (GDI)
función rellenar_rect(el self: &DC, la rect: &Rect, color: Entero32)  // FillRect
función dibujar_texto(el self: &DC, texto: Palabra, rect: &Rect)      // DrawText
función marco_rect(el self: &DC, la rect: &Rect, el lapiz: &Lapiz)    // FrameRect
función linea(el self: &DC, x1: Entero32, y1: Entero32, x2: Entero32, y2: Entero32)  // MoveToEx + LineTo
```

### 2.3 Controles (owner-draw + Common Controls)

```falcato
estructural Boton {
    hwnd: *mut Entero32,
    texto: Texto,
    rect: Rect,
    habilitado: Booleano,
    // Callbacks
    al_click: *mut Entero32,  // closure/callback ptr
}

el función Boton.nueva(la ventana: &Ventana, texto: Palabra, rect: Rect) -> Boton
función Boton.al_click(el self: &mut Boton, cb: *mut Entero32)
función Boton.texto(el self: &Boton) -> Palabra  // GetWindowText
```

### 2.4 Diseño — Layout declarativo

```falcato
// anchors — constraint simple
estructural Anclaje {
    izquierda: Entero32, derecha: Entero32,
    superior: Entero32, inferior: Entero32,
}

// Contenedor con layout automático
estructural Contenedor {
    elementos: Vector<ControlBox>,
    tipo: TipoLayout,  // Vertical, Horizontal, Cuadricula
}

// Responde a WM_SIZE recalculando posiciones
función Contenedor.recalcular(el self: &mut Contenedor, la rect: &Rect)
```

### 2.5 Aplicación — Ciclo de vida

```falcato
función principal() -> Entero32 {
    el clase = ClaseVentana.nueva("MiApp", al_proc_ventana)
    clase.registrar()  // RegisterClassEx
    
    el ventana = Ventana.nueva(clase, "Mi App", 800, 600)
    ventana.mostrar()  // ShowWindow + UpdateWindow
    
    ventana.bucle_mensajes()  // GetMessage + DispatchMessage
    retornar 0
}

función al_proc_ventana(la ventana: Ventana, msg: Mensaje, w: Entero64, l: Entero64) -> Entero64 {
    coincidir msg {
        Mensaje.Destruir => { ventana.salir(); retornar 0 }
        Mensaje.Pintar => {
            el dc = ventana.dc_pintura()
            dc.rellenar_rect(ventana.rect_cliente(), 0xFFFFFF)
            dc.liberar()
            retornar 0
        }
        _ => { retornar ventana.proc_defecto(msg, w, l) }
    }
}
```

---

## 3. Innovaciones únicas de Falcato en GUI

### 3.1 Ownership de recursos GDI

El problema #1 en Win32: olvidar `DeleteObject` en brushes, pens, bitmaps.

```falcato
región dibujo {
    el lapiz_rojo = Lapiz.nuevo(PS_SOLID, 2, Color.ROJO)    // CreatePen → owned
    el brocha_azul = Brocha.nuevo(Color.AZUL)                 // CreateSolidBrush → owned
    dc.seleccionar(lapiz_rojo)                                 // SelectObject (borrowed)
    dc.seleccionar(brocha_azul)
    dc.rectangulo(10, 10, 100, 100)
    // Al salir de región:
    //   brocha_azul → DeleteObject (por ser `el`)
    //   lapiz_rojo  → DeleteObject (por ser `el`)
    //   dc NO se libera (era prestado)
}
```

**Reglas de ownership:**
- `el` recurso → se crea con CreateXXX, se destruye con DeleteObject al salir de scope
- `la` recurso → referencia prestada (ej: SelectObject devuelve el anterior)
- `los` recurso → CreateXXX + refcount; DeleteObject cuando último ref se libera

### 3.2 Subjuntivo = paint cold path

```falcato
función al_pintar(el dc: &DC, el self: &MiControl) {
    // Hot path — siempre se ejecuta
    dc.rellenar_rect(self.rect, self.color_fondo)
    dc.dibujar_texto(self.texto, self.rect, DT_CENTER)
    
    // Cold path — el compilador reordena fuera de la línea principal
    si self.tiene_sombra fuese {
        el sombra_color = Color.NEGRO.con_alfa(128)
        dc.dibujar_sombra(self.rect, sombra_color, 4)
    }
    si self.estilo fuese BORDE_REDONDEADO {
        dc.ruta_redondeada(self.rect, 8, 8)
    }
}
```

El compilador detecta `fuese` y reordena los bloques cold al final de la función,
dejando el hot path en línea. Esto es especialmente valioso en WM_PAINT donde
el render loop principal debe ser rápido.

### 3.3 Bitfields para estilos

```falcato
// Estilos de ventana como bits — verificación compile-time
estructural EstiloVentana { bits {
    redimensionable: Natural1,   // WS_SIZEBOX  = 0x00040000L
    minimizable: Natural1,        // WS_MINIMIZEBOX
    maximizable: Natural1,        // WS_MAXIMIZEBOX
    cerrar: Natural1,             // WS_SYSMENU (incluye botón cerrar)
    titulo: Natural1,             // WS_CAPTION
    borde_delgado: Natural1,      // WS_BORDER (no usado con WS_CAPTION)
    borde_grueso: Natural1,       // WS_DLGFRAME? No, esto es simplificado
}}

el estilo = EstiloVentana {
    redimensionable: 1,
    minimizable: 1,
    maximizable: 1,
    cerrar: 1,
    titulo: 1,
    borde_delgado: 0,
    borde_grueso: 0,
}
// Compiler genera: WS_SIZEBOX | WS_MINIMIZEBOX | WS_MAXIMIZEBOX | WS_SYSMENU | WS_CAPTION
// = 0x00CF0000L
```

### 3.4 Layout declarativo con `=`

```falcato
// El operador `=` en contexto de layout NO es asignación — es enlace de constraint
el boton = Boton.nuevo("Click")
boton.izquierda = formulario.izquierda + 20   // constraint: boton.left = form.left + 20
boton.superior = etiqueta.inferior + 8         // constraint: boton.top = label.bottom + 8
boton.ancho = 120                              // constraint: boton.width = 120
// El sistema resuelve constraints en WM_SIZE
```

### 3.5 Region-based frame rendering

```falcato
función al_pintar(el self: &MiApp, el dc: &DC) {
    región fotograma {
        // Todos los recursos creados aquí se liberan al final del fotograma
        el lapiz = Lapiz.nuevo(PS_SOLID, 1, self.color_linea)
        el brocha = Brocha.nuevo(self.color_relleno)
        el fuente = Fuente.nueva("Arial", 14, FW_NORMAL)
        
        dc.seleccionar(lapiz)
        dc.seleccionar(brocha)
        dc.seleccionar(fuente)
        
        dc.rectangulo(10, 10, 200, 100)
        dc.dibujar_texto("Hola", Rect{10, 10, 200, 100}, DT_CENTER)
        
        // lapiz, brocha, fuente se liberan aquí (DeleteObject)
    }
}
```

### 3.6 Mensajes de error en español (códigos V###)

```
[V001] ventana.fc:12: Clase de ventana no registrada
       │ sugerencia: Llama a ClaseVentana.registrar() antes de Ventana.nueva()

[V002] ventana.fc:20: Recurso GDI no liberado: 'brocha_azul'
       │ sugerencia: Sal del scope o llama a brocha.liberar()

[V003] ventana.fc:45: Control 'boton1' no pertenece a esta ventana
       │ sugerencia: El HWND padre debe pasarse en Boton.nuevo(padre, ...)

[V004] ventana.fc:67: WNDPROC nulo en clase 'MiClase'
       │ sugerencia: Asigna un procedimiento de ventana con ClaseVentana.proc(...)

[V005] ventana.fc:80: No se puede dibujar fuera de WM_PAINT
       │ sugerencia: Usa dc.nueva_ventana() en vez de dc.nueva_pintura()
```

---

## 4. Plan de implementación por fases

### Fase GUI-1: Núcleo + MessageBox ✅ (ESTA SESIÓN)

| Tarea | Archivos | Estado |
|-------|----------|--------|
| Agregar `user32.lib` + `gdi32.lib` al linker | `src/main.rs` | ✅ |
| Diseño de sistema (`diseno_gui.md`) | `docs/diseno_gui.md` | ✅ |
| MessageBox FFI | `ejemplos/messagebox.fc` | ✅ |
| Ventana básica + message loop | `ejemplos/ventana_simple.fc` | ✅ |

### Fase GUI-2: GDI básico (siguiente sesión)

| Tarea | Archivos | Depende de |
|-------|----------|------------|
| Tipos base: `Punto.fc`, `Rect.fc`, `Color.fc` | `stdlib/ventana/` | GUI-1 |
| `DC` wrapper con ownership | `stdlib/ventana/dc.fc` | GUI-1 |
| `Lapiz` + `Brocha` + `Fuente` | `stdlib/ventana/graf.fc` | GUI-1 |
| Pintar con GDI (FillRect, TextOut, LineTo) | `ejemplos/dibujo_simple.fc` | DC, Lapiz, Brocha |
| Verificación semántica de recursos no liberados | `src/semantic.rs` | — |

### Fase GUI-3: Controles (siguiente sesión)

| Tarea | Archivos | Depende de |
|-------|----------|------------|
| `Boton` wrapper con al_click | `stdlib/ventana/boton.fc` | GUI-2 |
| `Etiqueta` wrapper | `stdlib/ventana/etiqueta.fc` | GUI-2 |
| `CajaTexto` (edit control) | `stdlib/ventana/entrada.fc` | GUI-2 |
| Dispatch de WM_COMMAND a callbacks | `stdlib/ventana/evento.fc` | GUI-2 |
| Ejemplo formulario con controles | `ejemplos/formulario.fc` | Boton, Etiqueta, CajaTexto |

### Fase GUI-4: Layout + estilos (siguiente sesión)

| Tarea | Archivos | Depende de |
|-------|----------|------------|
| Contenedor con layout vertical/horizontal | `stdlib/ventana/diseno.fc` | GUI-3 |
| Constraints simples (`boton.izquierda = ...`) | `src/semantic.rs` | GUI-3 |
| Bitfield window styles | `stdlib/ventana/estilo.fc` | GUI-3 |
| Temas de color | `stdlib/ventana/tema.fc` | GUI-3 |

### Fase GUI-5: GDI+ / Direct2D para motores (futuro)

| Tarea | Archivos | Depende de |
|-------|----------|------------|
| GDI+ bindings (Graphics, Bitmap, Pen, Brush) | `stdlib/ventana/gdiplus.fc` | GUI-4 |
| Alpha blending y antialiasing | `stdlib/ventana/gdiplus.fc` | GUI-4 |
| Transformaciones 2D | `stdlib/ventana/gdiplus.fc` | GUI-4 |
| Canvas / Framebuffer para motores | `stdlib/ventana/canvas.fc` | GUI-4 |
| Double buffering automático | `stdlib/ventana/buffer.fc` | GUI-4 |
| Ejemplo motor 2D (pong, space invaders) | `ejemplos/motor_2d.fc` | Todo lo anterior |

### Fase GUI-6: Direct3D 11 para 3D (futuro lejano)

| Tarea | Archivos |
|-------|----------|
| D3D11 bindings (CreateDevice, CreateSwapChain) | `stdlib/ventana/d3d11.fc` |
| Pipeline de rendering (VS, PS, rasterizer) | `stdlib/ventana/d3d11.fc` |
| Shaders en HLSL compilados | `ejemplos/shaders/` |
| Ejemplo cubo 3D rotante | `ejemplos/cubo_3d.fc` |

---

## 5. API de alto nivel (visión final)

```falcato
// app.fc — Aplicación completa con GUI
función principal() -> Entero32 {
    el app = App.nueva("Mi App Gráfica")
    
    // Ventana principal
    el ventana = app.ventana(800, 600, "Motor Falcato")
    ventana.estilo = EstiloVentana {
        redimensionable: 1, minimizable: 1,
        maximizable: 1, cerrar: 1, titulo: 1,
    }
    
    // Controles
    el boton_iniciar = Boton.nuevo("Iniciar")
    boton_iniciar.al_click(|| { app.iniciar_motor() })
    ventana.agregar(boton_iniciar)
    
    // Layout declarativo
    ventana.aplicar_layout(|| {
        boton_iniciar.izquierda = 10
        boton_iniciar.superior = 10
        boton_iniciar.ancho = 100
        boton_iniciar.alto = 30
    })
    
    // Canvas para el motor
    el canvas = Canvas.nuevo(ventana, 780, 560)
    canvas.al_pintar(|| {
        canvas.rellenar(Color.NEGRO)
        canvas.dibujar_textura(mi_textura, 100, 100)
        canvas.dibujar_texto("FPS: {fps}", Color.VERDE)
    })
    
    // Loop principal
    app.ejecutar()  // GetMessage + DispatchMessage
    
    retornar 0
}
```

### 5.1 Estructura de módulos stdlib

```
stdlib/
└── ventana/
    ├── núcleo.fc          # HWND, WNDCLASS, RegisterClass, CreateWindow
    ├── mensaje.fc         # WM_* constantes, MSG struct
    ├── punto.fc           # Punto, Rect, Tamaño
    ├── color.fc           # Color (RGB, HSL)
    ├── dc.fc              # Device Context con ownership
    ├── lápiz.fc           # Pen (CreatePen → DeleteObject)
    ├── brocha.fc          # Brush (CreateSolidBrush → DeleteObject)
    ├── fuente.fc          # Font (CreateFont → DeleteObject)
    ├── bitmap.fc          # Bitmap (LoadBitmap, CreateBitmap)
    ├── botón.fc           # Button control
    ├── etiqueta.fc        # Static control
    ├── entrada.fc         # Edit control
    ├── lista.fc           # ListBox control
    ├── barra.fc           # ScrollBar control
    ├── diseno.fc          # Layout containers
    ├── evento.fc          # Event dispatch
    ├── aplicación.fc      # App lifecycle
    └── tema.fc            # Color themes
```

---

## 6. Integración con el compilador

### 6.1 Linker automático

Cuando el compilador detecta funciones `inseguro` que referencian símbolos Win32
(user32, gdi32, dwmapi), añade automáticamente las librerías correspondientes:

```rust
// En main.rs — detección automática de libs Win32
let has_user32 = objeto.importa("user32");
let has_gdi32 = objeto.importa("gdi32");
if has_user32 { cmd.arg("user32.lib"); }
if has_gdi32 { cmd.arg("gdi32.lib"); }
```

### 6.2 Funciones `inseguro` reservadas para stdlib

Las funciones de Win32 se declaran como `inseguro` en la stdlib:

```falcato
// núcleo.fc — bindings inseguros
inseguro función RegisterClassExW(la clase: *mut Entero32) -> Entero16
inseguro función CreateWindowExW(ex_estilo: Entero32, clase: Palabra, titulo: Palabra,
    estilo: Entero32, x: Entero32, y: Entero32, ancho: Entero32, alto: Entero32,
    padre: *mut Entero32, menu: *mut Entero32, instancia: *mut Entero32, param: *mut Entero32) -> *mut Entero32
inseguro función DefWindowProcW(hwnd: *mut Entero32, msg: Entero32, w: Entero64, l: Entero64) -> Entero64
inseguro función GetMessageW(la msg: *mut Entero32, hwnd: *mut Entero32, min: Entero32, max: Entero32) -> Booleano
inseguro función DispatchMessageW(la msg: *mut Entero32) -> Entero64
```

### 6.3 Palabras reservadas adicionales

No se añaden keywords nuevos. El sistema GUI usa:
- `inseguro` — ya existe para FFI
- `estructural` — ya existe para structs C-layout
- `enumeración` — ya existe para WM_* como enums
- `implementar` para `rasgo` — ya existe para traits

### 6.4 Spans en errores de recursos

El checker semántico se extiende para tracking de recursos GDI:

```rust
// En semantic.rs — nuevo
struct RecursoGDI {
    nombre: String,
    tipo: TipoRecurso,        // Lapiz, Brocha, Fuente, DC, Bitmap
    creado_en: Span,
    liberado_en: Option<Span>,
    scope: ScopeId,
}
// Verifica que todo recurso creado con `el` tenga DeleteObject al salir de scope
```

---

## 7. Referencia rápida de funciones Win32 envueltas

### 7.1 Ventana (user32)

| Falcato | Win32 | Descripción |
|---------|-------|-------------|
| `ClaseVentana.nueva(nombre, proc)` | `WNDCLASS` init | Crea descripción de clase |
| `ventana.registrar()` | `RegisterClassEx` | Registra la clase |
| `Ventana.nueva(clase, titulo, ancho, alto)` | `CreateWindowEx` | Crea la ventana |
| `ventana.mostrar()` | `ShowWindow` + `UpdateWindow` | Muestra la ventana |
| `ventana.bucle_mensajes()` | `GetMessage` loop | Procesa mensajes |
| `ventana.cerrar()` | `SendMessage(WM_CLOSE)` | Cierre graceful |
| `ventana.salir()` | `PostQuitMessage` | Sale del bucle |

### 7.2 Mensajes (user32)

| Falcato | Win32 | Cuándo ocurre |
|---------|-------|---------------|
| `Mensaje.Crear` | WM_CREATE | Tras crear ventana |
| `Mensaje.Pintar` | WM_PAINT | Necesita repintar |
| `Mensaje.Destruir` | WM_DESTROY | Ventana cerrándose |
| `Mensaje.Cerrar` | WM_CLOSE | Botón X presionado |
| `Mensaje.ClickIzquierdo` | WM_LBUTTONDOWN | Click de ratón |
| `Mensaje.Tamaño` | WM_SIZE | Redimensionar |
| `Mensaje.Comando` | WM_COMMAND | Botón presionado |
| `Mensaje.TeclaAbajo` | WM_KEYDOWN | Tecla presionada |
| `Mensaje.Timer` | WM_TIMER | Timer expirado |

### 7.3 Dibujo (gdi32)

| Falcato | Win32 | Descripción |
|---------|-------|-------------|
| `dc.nueva_ventana(v)` | `GetDC` | Obtiene DC de ventana |
| `dc.nueva_pintura(v)` | `BeginPaint` | DC para WM_PAINT |
| `dc.liberar()` | `ReleaseDC` / `EndPaint` | Libera DC |
| `dc.rellenar_rect(r, color)` | `FillRect` | Rellena rectángulo |
| `dc.dibujar_texto(t, r, fmt)` | `DrawTextW` | Texto formateado |
| `dc.marco_rect(r, lápiz)` | `FrameRect` | Borde de rectángulo |
| `dc.linea(x1,y1,x2,y2)` | `MoveToEx` + `LineTo` | Línea recta |
| `dc.rectangulo(x1,y1,x2,y2)` | `Rectangle` | Rectángulo relleno |
| `dc.elipse(x1,y1,x2,y2)` | `Ellipse` | Elipse rellena |
| `dc.arco(x1,y1,x2,y2,x3,y3,x4,y4)` | `Arc` | Arco de elipse |
| `dc.texto(x,y,t)` | `TextOutW` | Texto en posición |

### 7.4 Recursos GDI (gdi32)

| Falcato | Win32 | Descripción |
|---------|-------|-------------|
| `Lapiz.nuevo(estilo, ancho, color)` | `CreatePen` | Nuevo lápiz |
| `Brocha.nuevo(color)` | `CreateSolidBrush` | Nueva brocha |
| `Fuente.nueva(nombre, altura, peso)` | `CreateFontW` | Nueva fuente |
| `lapiz.liberar()` | `DeleteObject` | Libera lápiz |
| `dc.seleccionar(recurso)` | `SelectObject` | Activa recurso en DC |
| `dc.restaurar(recurso_viejo)` | `SelectObject(old)` | Restaura recurso anterior |

---

## 8. Anti-patrones y bugs comunes

| # | Anti-patrón | Problema | Alternativa Falcato |
|---|-------------|----------|---------------------|
| 1 | Olvidar `DeleteObject` de brush/pen | Fuga de GDI (10k límite = pantallazo) | Usar `el` (auto-liberación) |
| 2 | Olvidar `ReleaseDC` | Fuga de DC (solo 5 por proceso) | Usar `región fotograma { ... }` |
| 3 | Dibujar fuera de WM_PAINT sin GetDC | Invalidación incorrecta | Forzar con `dc.nueva_ventana()` |
| 4 | Crear ventana sin registrar clase | Crash | Error [V001] con sugerencia |
| 5 | WNDPROC nulo | Crash al primer mensaje | Error [V004] con sugerencia |
| 6 | Usar HWND después de WM_DESTROY | Use-after-free | Ownership: ventana se mueve a Destruir |
| 7 | Layout manual en WM_SIZE | Código repetitivo | Usar `Contenedor` + constraints |
| 8 | No llamar `DefWindowProc` en mensajes no manejados | Comportamiento incorrecto | El trait `ProcVentana` lo exige |
| 9 | Crear GDI object por fotograma sin liberar | Fuga lenta | `región fotograma` libera automático |
| 10 | Thread unsafe: crear/destruir ventanas desde hilo incorrecto | Race conditions | Verificación de thread affinity en Nivel 2 |

---

## 9. Checklist para GUI-1

- [x] `user32.lib` + `gdi32.lib` en linker
- [x] Documento de diseño (`docs/diseno_gui.md`)
- [x] MessageBox FFI funcional (`ejemplos/messagebox.fc`)
- [x] Ventana nativa + message loop (`ejemplos/ventana_simple.fc`)
- [x] Verificado: `falcato check` + `falcato build` + ejecución
- [x] Commiteado y pusheado

---

> "La interfaz gráfica no es azúcar — es la puerta de entrada a motores, herramientas y juegos."
