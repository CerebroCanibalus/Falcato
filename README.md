![Falcato Banner](assets/images/falcato_banner.png)

**Lenguaje de sistemas iberohablante.** Forjado sobre Cranelift. Compila a binarios nativos x86_64.

```
.fnc → Lexer → Parser → Concordancia Lingüística → Codegen (Cranelift) → .o → Linker → .exe
```

[![CI](https://github.com/CerebroCanibalus/falcato/actions/workflows/ci.yml/badge.svg)](https://github.com/CerebroCanibalus/falcato)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)
[![Cranelift](https://img.shields.io/badge/backend-Cranelift%200.112-orange)](https://github.com/bytecodealliance/cranelift)
[![Target](https://img.shields.io/badge/target-x86_64%20Windows-lightgrey)](https://github.com/CerebroCanibalus/falcato)

---

## 🚀 Inicio rápido (3 pasos)

### 1. Descargar
Ve a [Releases](https://github.com/CerebroCanibalus/falcato/releases) y descarga `falcato-v0.2.0-x86_64-windows.zip` (o la versión más reciente).

### 2. Instalar
Extrae el ZIP y ejecuta `install.ps1` (PowerShell):
```powershell
falcato-v0.2.0-x86_64-windows.zip  →  Extraer aquí
.\install.ps1
```
Menú interactivo: eliges qué instalar (PATH obligatorio, VS Code, OpenCode, Claude Code, Cursor).
Instala `falcato.exe` en `%USERPROFILE%\.falcato\bin` y lo agrega al PATH de usuario.

### 3. Probar
Abre una **terminal nueva** y escribe:
```cmd
falcato version
# → Falcato v0.2.0

falcato run ejemplos\hola_mundo.fc
# → ¡Hola, mundo!
```

> **¿Prefieres compilar desde fuente?** Ver [INSTALL.md](INSTALL.md#opción-2-compilar-desde-código-fuente)

---

## ¿Qué es Falcato?

Falcato es un **lenguaje de programación de sistemas** construido desde cero donde la gramática española no es azúcar sintáctico — **es el sistema de tipos y el modelo de ejecución**.

No traduce keywords de Rust al español. No interpreta pseudocódigo. No es un wrapper sobre otro compilador.

Falcato tiene su propio **lexer** (logos), **parser** (descendente manual con Pratt), **análisis semántico** (Concordancia Lingüística), y **codegen** (Cranelift → .o → .exe). El resultado son binarios nativos x86_64 con ABI de C, sin runtime oculto, sin garbage collector.

```falcato
fn principal() -> Entero32 {
    el mensaje: Palabra = "Falcato compila. Punto.";
    imprimir(mensaje);
    retornar 0;
}
```

---

## ¿Por qué Falcato existe?

Hay **~600 millones de hispanohablantes** en el mundo (nativos + L2, Instituto Cervantes 2024). Menos del 5% programa. La barrera no es la lógica — es el lenguaje de la documentación, los errores, y la sintaxis.

Falcato responde a tres preguntas:

| Pregunta | Respuesta |
|----------|-----------|
| **¿Y si el español pudiera expresar garantías de compilación?** | Los artículos (`el`/`la`/`un`) codifican ownership. Los tiempos verbales codifican modos de ejecución. El subjuntivo codifica caminos fríos. |
| **¿Y si un LLM pudiera generar código que compila en Nivel 0?** | Nivel 0 (permisivo) siempre compila. El compiler sugiere, no rechaza. Un LLM genera → compiler sugiere → LLM refina → <3 iteraciones a Nivel 2. |
| **¿Y si la ingeniería de lenguajes pudiera explorar una dimensión lingüística distinta?** | 500+ años de evolución del español ofrecen dimensiones que el inglés no tiene: género, ser/estar, subjuntivo, prefijos productivos, voz activa/pasiva. Falcato las convierte en garantías de compilación. |

---

## Los 5 Pilares

| # | Pilar | Qué significa | Estado |
|---|-------|---------------|--------|
| I | **Género = Ownership** | `el` = owned mutable, `la` = borrowed immutable, `un` = opcional | ✅ Implementado |
| II | **Ser/Estar = Const/Mut** | `es` = identidad permanente, `está` = estado temporal | ✅ Implementado |
| III | **Tiempos = Modos ejecución** | Presente = sync, Futuro = async, Subjuntivo = fallible | ✅ Implementado |
| IV | **C ABI por defecto** | Layout C, calling C, sin name mangling | ✅ Implementado |
| V | **Prefijos semánticos** | `re-` = retry, `des-` = free, `pre-` = comptime | ✅ Documentados |

---

## Lo que nos hace verdaderamente únicos

### 🧬 El español ES el sistema de tipos

En Falcato, la concordancia gramatical es verificación de tipos. Un adjetivo que no concuerda con su sustantivo es un error de compilación — igual que en español real.

```
[T001] test.fc:4:8: Disconcordancia de tipo: 'a' es 'Entero32' pero se declaró como 'Booleano'
       │ sugerencia: Cambia el tipo a 'Entero32' o el valor
```

### 🔒 Ownership sin aprenderlo — ya lo sabes

Si hablas español, ya entiendes la diferencia entre *"el libro"* (lo tengo yo, puedo cambiarlo) y *"la casa"* (me la prestaron, solo la uso). Falcato convierte esa intuición en garantías de compilación.

| Artículo | Semántica | Equivalente Rust |
|----------|-----------|------------------|
| `el` | Owned, mutable | `let mut` |
| `la` | Borrowed, inmutable | `let` / `&T` |
| `un` | Opcional | `Option<T>` |
| `los` | Shared ownership (ref-counted) | `Arc<T>` |
| `las` | Shared borrowed | `&[T]` |

### ⏱️ Los verbos son modos de ejecución

| Tiempo verbal | Modo de ejecución | Equivalente |
|---------------|-------------------|-------------|
| Presente | Síncrono, bloqueante | `fn` |
| Futuro | Asíncrono | `fut fn` |
| Subjuntivo | Fallible, cold path | `si x fuese ...` |
| Imperativo | Inseguro (FFI) | `inseguro fn` |

### 🛡️ Borrow checker gradual — no todo o nada

| Nivel | Permisividad | Para quién |
|-------|-------------|------------|
| **0** (default) | Permisivo, como C | Principiantes, LLMs |
| **1** (`verificado`) | Use-after-move detection | Intermedios |
| **2** (`estricto`) | Borrow checker completo | Kernels, sistemas |

### 🧩 Regiones + Self-referential structs

`región nombre { ... }` — arena allocation determinística. `&yo T` — self-referential structs sin workarounds. Dos cosas que Rust no puede hacer de forma sound.

### 📡 Async real con threads del SO

`lanzar expr` → CreateThread real. `canal_nuevo` → mutex + semaphore + ring buffer. `con_executor(N)` → thread pool con cancelación estructurada. Todo verificado end-to-end.

---

## ¿Qué NO es Falcato?

| ❌ No es... | ✅ Sí es... |
|-------------|------------|
| Pseudocódigo | Compilador real → binarios nativos |
| Traducción de Rust al español | Lenguaje nuevo donde la gramática española IS el sistema de tipos |
| Wrapper sobre LLVM | Backend propio sobre Cranelift (contribución activa al ecosistema) |
| Lenguaje interpretado | AOT compilation → .exe sin runtime |
| Proyecto de traducción de keywords | Ingeniería de lenguajes con dimensiones semánticas únicas |
| Solo para aprender español | Lenguaje de sistemas productivo para kernels, drivers, herramientas |

---

## ¿En qué se diferencia de otros lenguajes?

| | Falcato | Rust | C |
|---|---------|------|---|
| **Compila a** | Binario nativo x86_64 | Binario nativo | Binario nativo |
| **Backend** | Cranelift (propio) | LLVM | GCC/Clang |
| **Sistema de tipos** | Gramática española + affine types | Algebraic types | Débil |
| **Ownership** | Artículos (`el`/`la`/`un`) | Borrow checker | Manual (malloc/free) |
| **Errores** | Español con span + sugerencia | Inglés técnico | Cripticos |
| **ABI** | C por defecto | Rust (propia) | C |
| **Async** | Threads reales + canales | async/await (futures) | No nativo |
| **Curva de aprendizaje** | Gradual (Nivel 0→2) | Empinada | Baja pero insegura |
| **IA-friendly** | Nivel 0 siempre compila | Nivel 2 rechaza mucho | Sin verificación |

---

### 🔍 ¿Y qué hay de los "otros lenguajes en español"?

De vez en cuando alguien compara Falcato con **Latino**, **PSeInt**, **EsJS** o proyectos similares.
La comparación es natural — todos usan español. Pero técnicamente no pertenecen ni a la misma
**categoría** de lenguaje. Veamos:

#### 🇪🇸 El ecosistema de lenguajes en español (investigado a fondo)

| Lenguaje | Año | Categoría real | Implementación | ¿Compila a nativo? | ¿Ownership? | ¿Sistemas? |
|----------|-----|----------------|----------------|--------------------|-------------|---|
| **PSeInt** | 2003 | Pseudocódigo educativo | Intérprete en C++ | ❌ Interpreta pseudocódigo | ❌ | ❌ |
| **Latino** | 2015 | Scripting dinámico | Intérprete en C (bytecode VM) | ❌ Interpreta bytecode | ❌ | ❌ |
| **EsJS** | 2023 | Transpilador | JS → JS (reescritura de tokens) | ❌ Transpila a JavaScript | ❌ | ❌ |
| **Sí** | 2023 | Preprocesador | Python → C++/Python (cambia keywords) | ❌ Traduce a C++ | ❌ | ❌ |
| **Falcato** | 2025 | Lenguaje de sistemas | Compilador Rust → Cranelift → .o | ✅ Binario nativo x86_64 | ✅ Artículos + affine | ✅ C ABI + FFI |

#### 🧩 ¿Por qué no tiene sentido compararlos?

**PSeInt** — Es una **herramienta educativa** que ejecuta pseudocódigo paso a paso. No produce
binarios. No tiene tipos reales. No tiene memoria dinámica. No puede llamar al sistema operativo.
No está diseñado para producir software — está diseñado para **enseñar lógica** a principiantes.

```pseudocodigo
// PSeInt — pseudocódigo educativo, no ejecutable fuera del intérprete
Escribir "Hola mundo"
Leer nombre
```

**Latino** — Es un **lenguaje interpretado** con bytecode VM, como Lua o Python pero en español.
Sus tipos son dinámicos. No tiene compilación a nativo. No tiene control de memoria. Es
perfectamente válido como lenguaje de scripting educativo, pero está **en las antípodas**
de un lenguaje de sistemas que corre sobre el metal.

```latino
// Latino — scripting dinámico, interpretado, sin tipos estáticos
escribir("Hola mundo")
```

**EsJS** — Es un **transpilador** que reemplaza keywords de JavaScript por sus equivalentes
en español (`si` → `if`, `mientras` → `while`). No tiene su propio parser, no tiene su propio
sistema de tipos, no tiene su propio backend. Es JavaScript con un **diccionario de sinónimos**.

```esjs
// EsJS — transpila 1:1 a JavaScript. Sigue siendo JS.
si (verdadero) {
    consola.escribir("Hola")
}
```

**Sí** — Es un **preprocesador** que traduce keywords al español y genera código en C++ o Python.
No tiene implementación propia. No añade semántica nueva. Es un `sed` con esteroides.

```sí
// Sí — preprocesador que genera C++. No aporta semántica nueva.
imprimir("Hola")
```

#### 🏗️ Ahora, Falcato

```falcato
// Falcato — compilador propio, backend Cranelift, tipos reales, ownership, C ABI
el mensaje: Texto = texto_desde("Hola mundo");
imprimir_linea(mensaje);
mensaje.liberar();

inseguro función MessageBoxA(hwnd: Entero64, texto: Palabra,
    titulo: Palabra, tipo: Entero32) -> Entero32;

función principal() -> Entero32 {
    MessageBoxA(0, "Falcato compila a binario nativo", "Falcato", 0);
    retornar 0;
}
```

**La diferencia no es de grado — es de categoría:**

| Dimensión | Latino / PSeInt / EsJS / Sí | Falcato |
|-----------|---------------------------|---------|
| **Backend propio** | ❌ (usan C, JS, C++) | ✅ **Cranelift** (Bytecode Alliance) |
| **Compilación a nativo** | ❌ | ✅ **.exe sin runtime** |
| **Sistema de tipos estático** | ❌ (dinámico o pseudotipos) | ✅ **Concordancia Lingüística** |
| **Ownership en tiempo de compilación** | ❌ | ✅ **Artículos + affine types** |
| **ABI de C** | ❌ | ✅ **Llamada directa a Win32/C** |
| **Async real con threads del SO** | ❌ | ✅ **CreateThread + canales + thread pool** |
| **FFI a C sin glue code** | ❌ | ✅ **`inseguro fn` directo** |
| **Manejo de errores con `Resultado<T,E>` + `?`** | ❌ | ✅ |
| **Genéricos con monomorfización** | ❌ | ✅ |
| **Rasgos/Traits** | ❌ | ✅ |
| **LSP con hover, goto-def, find-refs** | ❌ | ✅ |
| **Bitfields para hardware** | ❌ | ✅ |
| **Self-referential structs** | ❌ | ✅ |

> **Falcato no compite con Latino, PSeInt, EsJS o Sí.** Compite con **Rust, C, Go y Zig**.
> Los proyectos en español existentes son herramientas educativas o transpiladores ligeros —
> perfectamente válidos en su nicho, pero conceptualmente ortogonales a Falcato.
>
> Sería como comparar **Python** con **C**: ambos son lenguajes de programación, pero están
> diseñados para problemas fundamentalmente distintos.

---

## ¿Para quién es Falcato?

### 🎯 Programadores hispanohablantes
Si piensas en español cuando programas, Falcato elimina la fricción mental de traducir conceptos al inglés. La ownership, los tipos, los errores — todo en tu idioma.

### 🤖 Generadores de código por IA
Nivel 0 siempre compila. El compiler sugiere con códigos + span + fix concreto. Un LLM genera → compiler sugiere → LLM refina → compila. Menos iteraciones, más confianza.

### 🔧 Programadores de sistemas
C ABI por defecto. Cranelift para compilación rápida. Bitfields para hardware. Regiones para arena allocation. Sin GC, sin runtime oculto.

### 📚 Educadores
La concordancia lingüística hace que los errores sean intuitivos. Un estudiante entiende `[T001]` sin necesidad de leer documentación técnica.

### 🏗️ Proyectos de IA + sistemas
Falcato + Cranelift + WASM = toolchain nativa para código generado por IA. Compilación ultra-rápida, sandbox WASM para ejecución segura, binarios nativos para rendimiento.

---

## Features implementadas

### Core del lenguaje
- Variables con tipos explícitos (`el x: Entero32 = 10`)
- Operaciones aritméticas con precedencia (`+`, `-`, `*`, `/`, `%`)
- Operaciones de comparación (`==`, `!=`, `<`, `>`, `<=`, `>=`)
- Operadores lógicos (`&&`, `||`, `!`)
- Asignación a identificadores y elementos de array
- Retorno (`retornar valor`)

### Control de flujo
- Condicionales `si` / `sino` con ser/estar y subjuntivo
- Bucles `mientras` y `para` sobre arrays
- Pattern matching con `coincidir`
- Select pattern para canales (`seleccionar`)

### Ownership (Pilar I)
- 5 artículos con semántica de ownership
- `mover x` — transferencia explícita de ownership
- `copiar x` — clone explícito
- Use-after-move detection (Nivel 1)
- Borrow checker gradual (Nivel 0→2)
- Referencias `&T`, `&mut T`, dereferencia `*ref`
- Lifetimes léxicos: `&nombre T`
- Field-level borrowing (`&mut punto.x` vs `&mut punto.y`)
- Branch-aware liveness (borrows mueren por rama del CFG)
- Artículos extendidos: `los` = shared ownership, `las` = shared borrowed

### Estructuras de datos
- **Arrays**: `[T; N]`, literales, `todos expr`, acceso, asignación
- **Structs**: `estructural Punto { ... }`, layout C, acceso a campos
- **Enums**: tag+union, variantes con datos, pattern matching
- **Texto**: heap string con `texto_nuevo()`, `texto_agregar()`, `texto_liberar()`
- **Vector<T>**: heap vector genérico con `vector_nuevo()`, `vector_agregar()`, etc.
- **Resultado<T,E>**: `Exito(valor)` / `Error(codigo)` con operador `?`
- **Diccionario/K/V** y **Conjunto** (Fase R4)

### Generics
- Const generics: `fn longitud<N: Entero32>(nums: [Entero32; N]) -> Entero32`
- Type generics con bounds: `fn máximo<T que Comparable>(a: T, b: T) -> T`
- Monomorfización automática por tipo concreto

### Traits / Rasgos
- Declaración: `rasgo Nombre { fn metodo(...); ... }`
- Implementación: `implementar Rasgo para Tipo { fn metodo(...) { ... } }`
- Verificación semántica de métodos requeridos

### Bitwise + I/O + Interpolación
- Operadores bitwise type-safe: `& | ^ << >> ~ >>>`
- Built-ins I/O: `imprimir`, `imprimir_linea` — polimórficos (Texto, Entero, Bool, Flotante)
- String interpolation: `imprimir_linea("x = {x}, y = {y}")`
- `tamaño_de::<T>()` — sizeof comptime
- Métodos en enteros: `x.poner_bit(3)`, `x.unos()`, `x.ceros_izquierda()`

### FFI + C runtime
- `inseguro fn` para funciones sin cuerpo
- Built-ins C: `puts`, `malloc`, `free`, `printf`
- `archivo_leer()`, `archivo_escribir()`, `archivo_existe()`
- `abs()`, `max()`, `min()`, `raiz()`, `potencia()`

### Async / Concurrencia (Fase 18)
- `fut fn` — funciones async
- `esperar expr` — await
- `lanzar expr` — spawn thread real (CreateThread)
- `dormir(ms)` — Sleep de kernel32
- Canales mpsc: `canal_nuevo`, `canal_enviar`, `canal_recibir`, `canal_intentar`
- `con_executor(N)` — thread pool real con cancelación estructurada
- `seleccionar { }` — select pattern sobre canales
- Stackless futures (state machine desugaring)

### Tooling
- CLI: `falcato build`, `falcato run`, `falcato check`, `falcato lsp`, `falcato version`
- LSP completo: diagnósticos, autocompletado, hover, go-to-definition, find-references
- Script `build.ps1` automático (auto-detecta Visual Studio)
- 40 tests unitarios pasando
- 50+ ejemplos funcionando

---

## 📦 Instalación alternativa: Compilar desde fuente

Si quieres contribuir o necesitas la última versión:

### Requisitos
- [Rust](https://rustup.rs/) (stable)
- [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022) → "Desktop development with C++"

### Compilar
```powershell
git clone https://github.com/CerebroCanibalus/falcato.git
cd falcato
cargo build --release
# falcato.exe está en target/release/
```

### Probar
```powershell
.\target\release\falcato.exe version
```

---

## 🎨 VS Code Extension

Resaltado de sintaxis, LSP integrado y tema **"Falcato Dorado"**:

1. Descarga el `.vsix` desde [Releases](https://github.com/CerebroCanibalus/falcato/releases)
2. `Ctrl+Shift+P` → "Extensions: Install from VSIX..."
3. Selecciona el archivo `.vsix`
4. Abre un `.fc` → sintaxis + diagnósticos en tiempo real
5. `Ctrl+K Ctrl+T` → busca "Falcato Dorado" para el tema

---

## Estado actual

| Aspecto | Estado |
|---------|--------|
| Pipeline end-to-end | ✅ Operativo |
| Backend Cranelift | ✅ Generando binarios nativos |
| Tests unitarios | ✅ 40/40 pasando |
| Ejemplos funcionando | ✅ 50+ |
| LSP | ✅ Completo |
| Async (threads + TCP + canales + thread pool) | ✅ Fase 18A-18D |
| Stackless futures | ✅ MVP |
| Diccionario + Conjunto | ✅ Fase R4 |
| Documentación completa | ✅ GUIA.md + 15 capítulos + REFERENCIA.md + ERRORES.md |
| VS Code Extension | ✅ Syntax + LSP + tema Falcato Dorado |
| CI GitHub Actions | ✅ Build + test |
| Distribución | ⚠️ Pre-release v0.1.0 |

---

## Proyecto

| Recurso | Ubicación |
|---------|-----------|
| Repositorio | [github.com/CerebroCanibalus/falcato](https://github.com/CerebroCanibalus/falcato) |
| Documentación | `GUIA.md` + carpeta `GUIA/` (15 capítulos) |
| Referencia de built-ins | `REFERENCIA.md` |
| Códigos de error | `ERRORES.md` |
| Instalación | `INSTALL.md` |
| Ejemplos | `ejemplos/` (50+ archivos `.fc`) |
| Skill para LLMs | `falcato-language` (OpenCode) |
| Para contribuidores | `AGENTS.md` |

---

## Stack técnico

| Componente | Tecnología |
|------------|-----------|
| CLI | `clap` 4.5 (Rust) |
| Lexer | `logos` 0.14 |
| Parser | Manual descendente + Pratt |
| AST | Propio con Span obligatorio |
| Semántica | Concordancia Lingüística |
| Codegen | `cranelift-codegen` 0.112 |
| LSP | `tower-lsp` 0.20 |
| Target | x86_64 Windows (msvc) |
| ABI | C por defecto |
| Testing | 40 tests unitarios |

---

## Licencia

MIT OR Apache-2.0 — elige la que prefieras.

---

> *Falcato no es una traducción de Rust al español.*
> *Es un lenguaje de sistemas donde el español es el sistema de tipos.*
> *Donde la concordancia gramatical es verificación de compilación.*
> *Donde los tiempos verbales son modos de ejecución.*
> *Donde 500 años de evolución lingüística se convierten en garantías de código.*

```
  ⠀⠀⠀⠀⠀⠀⠀"多謝垂注"
  ⠀⠀⠀⣏⡱ ⣏⡉ ⣏⡱ ⡇ ⣎⣱   ⡷⢾ ⢇⡸
  ⠀⠀⠀⠧⠜ ⠧⠤ ⠇⠱ ⠇ ⠇⠸   ⠇⠸ ⠇⠸
  ⠀https://ko-fi.com/general_beria
```
