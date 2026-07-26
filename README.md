![Falcato Banner](assets/images/falcato_banner.png)

**Lenguaje de sistemas iberohablante.** Forjado sobre Cranelift. Compila a binarios nativos x86_64.

```
.fc â†’ analizador léxico â†’ Parser â†’ Concordancia LingÃ¼Ã­stica â†’ Codegen (Cranelift) â†’ .o â†’ enlazador â†’ .exe
```

[![CI](https://github.com/CerebroCanibalus/falcato/actions/workflows/ci.yml/badge.svg)](https://github.com/CerebroCanibalus/falcato)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)
[![Cranelift](https://img.shields.io/badge/motor-Cranelift%200.112-orange)](https://github.com/bytecodealliance/cranelift)
[![Target](https://img.shields.io/badge/target-x86_64%20Windows-lightgrey)](https://github.com/CerebroCanibalus/falcato)

---

## ðŸš€ Inicio rÃ¡pido (3 pasos)

### 1. Descargar
Ve a [Releases](https://github.com/CerebroCanibalus/falcato/releases) y descarga `falcato-v0.2.0-x86_64-windows.zip` (o la versiÃ³n mÃ¡s reciente).

### 2. Instalar
Extrae el ZIP y ejecuta `install.ps1` (PowerShell):
```powershell
falcato-v0.2.0-x86_64-windows.zip  â†’  Extraer aquÃ­
.\install.ps1
```
MenÃº interactivo: eliges quÃ© instalar (PATH obligatorio, VS Code, OpenCode, Claude Code, Cursor).
Instala `falcato.exe` en `%USERPROFILE%\.falcato\bin` y lo agrega al PATH de usuario.

### 3. Probar
Abre una **terminal nueva** y escribe:
```cmd
falcato version
# â†’ Falcato v0.2.0

falcato run ejemplos\hola_mundo.fc
# â†’ Â¡Hola, mundo!
```

> **Â¿Prefieres compilar desde fuente?** Ver [INSTALL.md](INSTALL.md#opciÃ³n-2-compilar-desde-cÃ³digo-fuente)

---

## Â¿QuÃ© es Falcato?

Falcato es un **lenguaje de programaciÃ³n de sistemas** construido desde cero donde la gramÃ¡tica espaÃ±ola no es azÃºcar sintÃ¡ctico â€” **es el sistema de tipos y el modelo de ejecuciÃ³n**.

No traduce keywords de Rust al espaÃ±ol. No interpreta pseudocÃ³digo. No es un wrapper sobre otro compilador.

Falcato tiene su propio **analizador léxico** (logos), **parser** (descendente manual con Pratt), **anÃ¡lisis semÃ¡ntico** (Concordancia LingÃ¼Ã­stica), y **codegen** (Cranelift â†’ .o â†’ .exe). El resultado son binarios nativos x86_64 con ABI de C, sin ejecución oculta, sin recolector de basura.

```falcato
fn principal() -> Entero32 {
    el mensaje: Palabra = "Falcato compila. Punto.";
    imprimir(mensaje);
    retornar 0;
}
```

---

## Â¿Por quÃ© Falcato existe?

Hay **~600 millones de hispanohablantes** en el mundo (nativos + L2, Instituto Cervantes 2024). Menos del 5% programa. La barrera no es la lÃ³gica â€” es el lenguaje de la documentaciÃ³n, los errores, y la sintaxis.

Falcato responde a tres preguntas:

| Pregunta | Respuesta |
|----------|-----------|
| **Â¿Y si el espaÃ±ol pudiera expresar garantÃ­as de compilaciÃ³n?** | Los artÃ­culos (`el`/`la`/`un`) codifican posesión. Los tiempos verbales codifican modos de ejecuciÃ³n. El subjuntivo codifica caminos frÃ­os. |
| **Â¿Y si un LLM pudiera generar cÃ³digo que compila en Nivel 0?** | Nivel 0 (permisivo) siempre compila. El compilador sugiere, no rechaza. Un LLM genera â†’ compiler sugiere â†’ LLM refina â†’ <3 iteraciones a Nivel 2. |
| **Â¿Y si la ingenierÃ­a de lenguajes pudiera explorar una dimensiÃ³n lingÃ¼Ã­stica distinta?** | 500+ aÃ±os de evoluciÃ³n del espaÃ±ol ofrecen dimensiones que el inglÃ©s no tiene: gÃ©nero, ser/estar, subjuntivo, prefijos productivos, voz activa/pasiva. Falcato las convierte en garantÃ­as de compilaciÃ³n. |

---

## Los 5 Pilares

| # | Pilar | QuÃ© significa | Estado |
|---|-------|---------------|--------|
| I | **GÃ©nero = posesión** | `el` = dueño mutable, `la` = prestado immutable, `un` = opcional | âœ… Implementado |
| II | **Ser/Estar = Const/Mut** | `es` = identidad permanente, `estÃ¡` = estado temporal | âœ… Implementado |
| III | **Tiempos = Modos ejecuciÃ³n** | Presente = sync, Futuro = async, Subjuntivo = fallible | âœ… Implementado |
| IV | **C ABI por defecto** | disposición C, calling C, sin distorsión de nombres | âœ… Implementado |
| V | **Prefijos semÃ¡nticos** | `re-` = retry, `des-` = free, `pre-` = comptime | âœ… Documentados |

---

## ðŸ¤” Â¿Pero por quÃ© espaÃ±ol DE VERDAD?

Esta es la pregunta que mÃ¡s nos hacen, y merece una respuesta clara:

**Falcato no usa espaÃ±ol porque "hay que traducir keywords para que los latinos aprendan".**
Falcato usa espaÃ±ol porque **el espaÃ±ol tiene herramientas gramaticales que el inglÃ©s no tiene**,
y esas herramientas permiten construir **sistemas de verificaciÃ³n de compilaciÃ³n mÃ¡s expresivos**.

No es inclusiÃ³n. Es **ingenierÃ­a**.

### ðŸ§  Las 3 razones de fondo

#### 1. El espaÃ±ol tiene mÃ¡s dimensiones semÃ¡nticas que el inglÃ©s

El inglÃ©s es un lenguaje analÃ­tico y minimalista. El espaÃ±ol es **flexivo y sintÃ©tico** â€”
transmite mucha mÃ¡s informaciÃ³n en cada palabra mediante desinencias, gÃ©nero, nÃºmero,
tiempo, modo y aspecto. En programaciÃ³n, **mÃ¡s dimensiones gramaticales = mÃ¡s ejes de verificaciÃ³n**.

| DimensiÃ³n | En inglÃ©s | En espaÃ±ol | QuÃ© permite en Falcato |
|-----------|-----------|------------|----------------------|
| **GÃ©nero** | No existe para objetos | Masculino/femenino para **todo** | posesión: `el` (dueño) vs `la` (prestado) |
| **Ser/Estar** | Traduce ambos como "to be" | Dos verbos de existencia | Const (`es`) vs Mut (`estÃ¡`) |
| **Subjuntivo** | Casi extinto ("If I were...") | Vivo y productivo | Cold paths, incertidumbre, fallo esperado |
| **Prefijos** | Limitados (re-, un-, pre-) | Productivos: re-, des-, pre-, entre-, contra- | SemÃ¡ntica de sistema: retry, free, comptime |
| **ArtÃ­culos** | the/a/an (3) | el/la/un/una/los/las/unos/unas (8) | 5+ niveles de posesión y visibilidad |

#### 2. La brecha semÃ¡ntica LLM â†’ cÃ³digo se reduce drÃ¡sticamente

Un LLM genera texto en lenguaje natural. Cuando el lenguaje de programaciÃ³n **es** lenguaje
natural (estructurado), la distancia entre lo que el LLM "piensa" y lo que escribe se acorta.

```falcato
// Lo que un LLM "piensa" en espaÃ±ol:
// "Guarda este texto en una variable. El texto es mutable (el).
// Si estÃ¡ vacÃ­o, retorna error."

// Lo que genera en Falcato:
el contenido: Texto = texto_desde("datos");
si contenido.tam() estÃ¡ 0 { retornar Resultado.Error(-1); }

// En Rust tendrÃ­a que "traducir" su pensamiento al inglÃ©s:
// "Store this text in a variable. The text is mutable (let mut).
// If it's empty, return an error."
let mut contents: String = String::from("data");
if contents.len() == 0 { return Err(-1); }
```

Esa **fricciÃ³n de traducciÃ³n** no es anecdÃ³tica. Es el motivo principal por el que la
programaciÃ³n tiene una barrera de entrada artificial para 600M de hispanohablantes.
Y es tambiÃ©n el motivo por el que los LLM generan cÃ³digo con mÃ¡s errores semÃ¡nticos
en lenguajes inglÃ©s-nativos: el modelo tiene que traducir dos veces
(idea â†’ lenguaje natural â†’ cÃ³digo) en vez de una (idea â†’ cÃ³digo en su idioma).

#### 3. No es "keywords en espaÃ±ol" â€” es el sistema de TYPES en espaÃ±ol

La diferencia crucial entre Falcato y todos los demÃ¡s lenguajes en espaÃ±ol:

| Proyecto | QuÃ© hace en espaÃ±ol | QuÃ© NO puede hacer |
|----------|-------------------|-------------------|
| **Latino, EsJS, SÃ­, Ãguila** | Traducir keywords (`if` â†’ `si`, `function` â†’ `funcion`) | Nada semÃ¡nticamente nuevo. El motor (JS, Python, Node) no cambia. |
| **WN++** | Keywords + identidad cultural chilena | IntÃ©rprete educativo. Tipado dinÃ¡mico. Sin verificaciÃ³n en compilaciÃ³n. |
| **Falcato** | **El espaÃ±ol es el sistema de tipos** | `el`/`la`/`un` = affine types. `es`/`estÃ¡` = const/mut. `fuese` = cold path. Concordancia = type checking. |

En Falcato, cambiar el artÃ­culo cambia **las garantÃ­as de compilaciÃ³n**:

```falcato
la x: Entero32 = 10;    // Prestado, inmutable â€” no se puede modificar
el x: Entero32 = 10;    // dueño, mutable â€” se puede modificar
x = 20;                  // âœ… si es 'el', âŒ si es 'la'
```

Eso no es decoraciÃ³n. Es **el sistema de affine types integrado en la gramÃ¡tica**.

En WN++, `pega` en vez de `fn` es un cambio lÃ©xico. El intÃ©rprete trata `pega` exactamente
como cualquier otro lenguaje trata `function` o `def`. En Falcato, `el` vs `la` no es lÃ©xico â€”
es semÃ¡ntico. El compilador **razona** sobre esa diferencia.

### ðŸŽ¯ La tesis, clara

> **Falcato existe porque el espaÃ±ol tiene recursos gramaticales que permiten construir
> un lenguaje de sistemas mÃ¡s expresivo, mÃ¡s verificable y mÃ¡s cercano al pensamiento humano
> que cualquier lenguaje diseÃ±ado exclusivamente en inglÃ©s.**

No estamos "traduciendo Rust al espaÃ±ol". Estamos explorando una pregunta que nadie
en la industria del software se ha tomado en serio:

**Â¿Y si 500 aÃ±os de evoluciÃ³n lingÃ¼Ã­stica pudieran informar el diseÃ±o de lenguajes
de programaciÃ³n, en vez de ignorarse porque "el inglÃ©s es el estÃ¡ndar"?**

---

## Lo que nos hace verdaderamente Ãºnicos

### ðŸ§¬ El espaÃ±ol ES el sistema de tipos

En Falcato, la concordancia gramatical es verificaciÃ³n de tipos. Un adjetivo que no concuerda con su sustantivo es un error de compilaciÃ³n â€” igual que en espaÃ±ol real.

```
[T001] test.fc:4:8: Disconcordancia de tipo: 'a' es 'Entero32' pero se declarÃ³ como 'Booleano'
       â”‚ sugerencia: Cambia el tipo a 'Entero32' o el valor
```

### ðŸ”’ posesión sin aprenderlo â€” ya lo sabes

Si hablas espaÃ±ol, ya entiendes la diferencia entre *"el libro"* (lo tengo yo, puedo cambiarlo) y *"la casa"* (me la prestaron, solo la uso). Falcato convierte esa intuiciÃ³n en garantÃ­as de compilaciÃ³n.

| ArtÃ­culo | SemÃ¡ntica | Equivalente Rust |
|----------|-----------|------------------|
| `el` | dueño, mutable | `let mut` |
| `la` | prestado, inmutable | `let` / `&T` |
| `un` | Opcional | `Option<T>` |
| `los` | Posesión compartida (ref-counted) | `Arc<T>` |
| `las` | Prestado compartido | `&[T]` |

### â±ï¸ Los verbos son modos de ejecuciÃ³n

| Tiempo verbal | Modo de ejecuciÃ³n | Equivalente |
|---------------|-------------------|-------------|
| Presente | SÃ­ncrono, bloqueante | `fn` |
| Futuro | AsÃ­ncrono | `fut fn` |
| Subjuntivo | Fallible, cold path | `si x fuese ...` |
| Imperativo | Inseguro (FFI) | `inseguro fn` |

### ðŸ›¡ï¸ control de préstamos gradual â€” no todo o nada

| Nivel | Permisividad | Para quiÃ©n |
|-------|-------------|------------|
| **0** (default) | Permisivo, como C | Principiantes, LLMs |
| **1** (`verificado`) | Use-after-move detection | Intermedios |
| **2** (`estricto`) | control de préstamos completo | Kernels, sistemas |

### ðŸ§© Regiones + Self-referential structs

`regiÃ³n nombre { ... }` â€” arena asignación determinÃ­stica. `&yo T` â€” self-referential structs sin workarounds. Dos cosas que Rust no puede hacer de forma sound.

### ðŸ“¡ Async real con hilos del SO

`lanzar expr` â†’ CreateThread real. `canal_nuevo` â†’ mutex + semaphore + ring buffer. `con_executor(N)` â†’ grupo de hilos con cancelaciÃ³n estructurada. Todo verificado integralmente.

---

## Â¿QuÃ© NO es Falcato?

| âŒ No es... | âœ… SÃ­ es... |
|-------------|------------|
| PseudocÃ³digo | Compilador real â†’ binarios nativos |
| TraducciÃ³n de Rust al espaÃ±ol | Lenguaje nuevo donde la gramÃ¡tica espaÃ±ola IS el sistema de tipos |
| Wrapper sobre LLVM | motor propio sobre Cranelift (contribuciÃ³n activa al ecosistema) |
| Lenguaje interpretado | AOT compilation â†’ .exe sin ejecución |
| Proyecto de traducciÃ³n de keywords | IngenierÃ­a de lenguajes con dimensiones semÃ¡nticas Ãºnicas |
| Solo para aprender espaÃ±ol | Lenguaje de sistemas productivo para kernels, drivers, herramientas |

---

## Â¿En quÃ© se diferencia de otros lenguajes?

| | Falcato | Rust | C |
|---|---------|------|---|
| **Compila a** | Binario nativo x86_64 | Binario nativo | Binario nativo |
| **motor** | Cranelift (propio) | LLVM | GCC/Clang |
| **Sistema de tipos** | GramÃ¡tica espaÃ±ola + affine types | Tipos algebraicos | DÃ©bil |
| **posesión** | ArtÃ­culos (`el`/`la`/`un`) | control de préstamos | Manual (malloc/free) |
| **Errores** | EspaÃ±ol con intervalo + sugerencia | InglÃ©s tÃ©cnico | Cripticos |
| **ABI** | C por defecto | Rust (propia) | C |
| **Async** | hilos reales + canales | async/await (futures) | No nativo |
| **Curva de aprendizaje** | Gradual (Nivel 0â†’2) | Empinada | Baja pero insegura |
| **IA-friendly** | Nivel 0 siempre compila | Nivel 2 rechaza mucho | Sin verificaciÃ³n |

---

### ðŸ” Â¿Y quÃ© hay de los "otros lenguajes en espaÃ±ol"?

De vez en cuando alguien compara Falcato con **Latino**, **PSeInt**, **EsJS** o proyectos similares.
La comparaciÃ³n es natural â€” todos usan espaÃ±ol. Pero tÃ©cnicamente no pertenecen ni a la misma
**categorÃ­a** de lenguaje. Veamos:

#### ðŸ‡ªðŸ‡¸ El ecosistema de lenguajes en espaÃ±ol (investigado a fondo)

| Lenguaje | AÃ±o | CategorÃ­a real | ImplementaciÃ³n | Â¿Compila a nativo? | Â¿posesión? | Â¿Sistemas? |
|----------|-----|----------------|----------------|--------------------|-------------|---|
| **PSeInt** | 2003 | PseudocÃ³digo educativo | IntÃ©rprete en C++ | âŒ Interpreta pseudocÃ³digo | âŒ | âŒ |
| **Latino** | 2015 | Scripting dinÃ¡mico | IntÃ©rprete en C (bytecode VM) | âŒ Interpreta bytecode | âŒ | âŒ |
| **Ãguila** | 2025 | Scripting dinÃ¡mico | Node.js (npm), nÃºcleo privado | âŒ Transpila/interpreta | âŒ | âŒ |
| **EsJS** | 2023 | Transpilador | JS â†’ JS (reescritura de tokens) | âŒ Transpila a JavaScript | âŒ | âŒ |
| **SÃ­** | 2023 | Preprocesador | Python â†’ C++/Python (cambia keywords) | âŒ Traduce a C++ | âŒ | âŒ |
| **WN++** | 2025 | IntÃ©rprete educativo | Rust (tree-walking, bytecode VM en ruta) | âŒ Interpreta AST/bytecode | âŒ | âŒ |
| **Falcato** | 2025 | Lenguaje de sistemas | Compilador Rust â†’ Cranelift â†’ .o | âœ… Binario nativo x86_64 | âœ… ArtÃ­culos + affine | âœ… C ABI + FFI |

#### ðŸ§© Â¿Por quÃ© no tiene sentido compararlos?

**PSeInt** â€” Es una **herramienta educativa** que ejecuta pseudocÃ³digo paso a paso. No produce
binarios. No tiene tipos reales. No tiene memoria dinÃ¡mica. No puede llamar al sistema operativo.
No estÃ¡ diseÃ±ado para producir software â€” estÃ¡ diseÃ±ado para **enseÃ±ar lÃ³gica** a principiantes.

```pseudocodigo
// PSeInt â€” pseudocÃ³digo educativo, no ejecutable fuera del intÃ©rprete
Escribir "Hola mundo"
Leer nombre
```

**Latino** â€” Es un **lenguaje interpretado** con bytecode VM, como Lua o Python pero en espaÃ±ol.
Sus tipos son dinÃ¡micos. No tiene compilaciÃ³n a nativo. No tiene control de memoria. Es
perfectamente vÃ¡lido como lenguaje de scripting educativo, pero estÃ¡ **en las antÃ­podas**
de un lenguaje de sistemas que corre sobre el metal.

```latino
// Latino â€” scripting dinÃ¡mico, interpretado, sin tipos estÃ¡ticos
escribir("Hola mundo")
```

**EsJS** â€” Es un **transpilador** que reemplaza keywords de JavaScript por sus equivalentes
en espaÃ±ol (`si` â†’ `if`, `mientras` â†’ `while`). No tiene su propio parser, no tiene su propio
sistema de tipos, no tiene su propio motor. Es JavaScript con un **diccionario de sinÃ³nimos**.

```esjs
// EsJS â€” transpila 1:1 a JavaScript. Sigue siendo JS.
si (verdadero) {
    consola.escribir("Hola")
}
```

**SÃ­** â€” Es un **preprocesador** que traduce keywords al espaÃ±ol y genera cÃ³digo en C++ o Python.
No tiene implementaciÃ³n propia. No aÃ±ade semÃ¡ntica nueva. Es un `sed` con esteroides.

```sÃ­
// SÃ­ â€” preprocesador que genera C++. No aporta semÃ¡ntica nueva.
imprimir("Hola")
```

**Ãguila** â€” Se presenta como "lenguaje profesional compilado de alto rendimiento", pero se instala
vÃ­a `npm install -g aguila-lang` y su nÃºcleo es privado (no hay compilador real que auditar).
Es un lenguaje de **scripting dinÃ¡mico** sobre Node.js con keywords y mÃ©todos nativos en espaÃ±ol.
Tiene 54 estrellas en GitHub, un gestor de paquetes, y funcionalidades de ciencia de datos.
Su mÃ©rito no estÃ¡ en el motor â€” es esencialmente Node.js con sintaxis en espaÃ±ol.

```aguila
# Ãguila â€” scripting dinÃ¡mico sobre Node.js
funcion saludar(nombre) {
    retornar a"Hola, {nombre}!"
}
imprime(saludar("Mundo"))
```

**WN++** â€” Es un **intÃ©rprete tree-walking** escrito en Rust con identidad **chilena** (`pega` para
fn, `cachai` para if, `lorea` para print). Es explÃ­citamente educativo: su propÃ³sito es que alguien
pueda leer el cÃ³digo fuente y entender cÃ³mo funciona un intÃ©rprete por dentro. Tiene 53 estrellas,
es cÃ³digo abierto real, y es honesto sobre no ser un lenguaje de producciÃ³n (todavÃ­a).

```wn
// WN++ â€” intÃ©rprete educativo chileno, tipado dinÃ¡mico
pega fibonacci(n) {
  cachai (n <= 1) { n }
  si no { fibonacci(n - 1) + fibonacci(n - 2) }
}
lorea(fibonacci(10))  // 55
```

#### ðŸ—ï¸ Ahora, Falcato

```falcato
// Falcato â€” compilador propio, motor Cranelift, tipos reales, posesión, C ABI
el mensaje: Texto = texto_desde("Hola mundo");
imprimir_linea(mensaje);
mensaje.liberar();

inseguro funciÃ³n MessageBoxA(hwnd: Entero64, texto: Palabra,
    titulo: Palabra, tipo: Entero32) -> Entero32;

funciÃ³n principal() -> Entero32 {
    MessageBoxA(0, "Falcato compila a binario nativo", "Falcato", 0);
    retornar 0;
}
```

**La diferencia no es de grado â€” es de categorÃ­a:**

| DimensiÃ³n | Latino / PSeInt / EsJS / SÃ­ / Ãguila / WN++ | Falcato |
|-----------|----------------------------------------------|---------|
| **motor propio** | âŒ (usan C, JS, C++) | âœ… **Cranelift** (Bytecode Alliance) |
| **CompilaciÃ³n a nativo** | âŒ | âœ… **.exe sin ejecución** |
| **Sistema de tipos estÃ¡tico** | âŒ (dinÃ¡mico o pseudotipos) | âœ… **Concordancia LingÃ¼Ã­stica** |
| **posesión en tiempo de compilaciÃ³n** | âŒ | âœ… **ArtÃ­culos + affine types** |
| **ABI de C** | âŒ | âœ… **Llamada directa a Win32/C** |
| **Async real con hilos del SO** | âŒ | âœ… **CreateThread + canales + grupo de hilos** |
| **FFI a C sin glue code** | âŒ | âœ… **`inseguro fn` directo** |
| **Manejo de errores con `Resultado<T,E>` + `?`** | âŒ | âœ… |
| **GenÃ©ricos con monomorfizaciÃ³n** | âŒ | âœ… |
| **Rasgos/Traits** | âŒ | âœ… |
| **LSP con hover, goto-def, find-refs** | âŒ | âœ… |
| **Bitfields para hardware** | âŒ | âœ… |
| **Self-referential structs** | âŒ | âœ… |

> **Falcato no compite con Latino, PSeInt, EsJS, Ãguila, WN++ o SÃ­.** Compite con **Rust, C, Go y Zig**.
> Los proyectos en espaÃ±ol existentes son herramientas educativas o transpiladores ligeros â€”
> perfectamente vÃ¡lidos en su nicho, pero conceptualmente ortogonales a Falcato.
>
> SerÃ­a como comparar **Python** con **C**: ambos son lenguajes de programaciÃ³n, pero estÃ¡n
> diseÃ±ados para problemas fundamentalmente distintos.

---

## Â¿Para quiÃ©n es Falcato?

### ðŸŽ¯ Programadores hispanohablantes
Si piensas en espaÃ±ol cuando programas, Falcato elimina la fricciÃ³n mental de traducir conceptos al inglÃ©s. La posesión, los tipos, los errores â€” todo en tu idioma.

### ðŸ¤– Generadores de cÃ³digo por IA
Nivel 0 siempre compila. El compilador sugiere con cÃ³digos + intervalo + corrección concreto. Un LLM genera â†’ compiler sugiere â†’ LLM refina â†’ compila. Menos iteraciones, mÃ¡s confianza.

### ðŸ”§ Programadores de sistemas
C ABI por defecto. Cranelift para compilaciÃ³n rÃ¡pida. Bitfields para hardware. Regiones para asignación de arena. Sin GC, sin ejecución oculta.

### ðŸ“š Educadores
La concordancia lingÃ¼Ã­stica hace que los errores sean intuitivos. Un estudiante entiende `[T001]` sin necesidad de leer documentaciÃ³n tÃ©cnica.

### ðŸ—ï¸ Proyectos de IA + sistemas
Falcato + Cranelift + WASM = toolchain nativa para cÃ³digo generado por IA. CompilaciÃ³n ultra-rÃ¡pida, sandbox WASM para ejecuciÃ³n segura, binarios nativos para rendimiento.

---

## Funcionalidades implementadas

### Core del lenguaje
- Variables con tipos explÃ­citos (`el x: Entero32 = 10`)
- Operaciones aritmÃ©ticas con precedencia (`+`, `-`, `*`, `/`, `%`)
- Operaciones de comparaciÃ³n (`==`, `!=`, `<`, `>`, `<=`, `>=`)
- Operadores lÃ³gicos (`&&`, `||`, `!`)
- AsignaciÃ³n a identificadores y elementos de array
- Retorno (`retornar valor`)

### Control de flujo
- Condicionales `si` / `sino` con ser/estar y subjuntivo
- Bucles `mientras` y `para` sobre arrays
- Pattern matching con `coincidir`
- Select pattern para canales (`seleccionar`)

### posesión (Pilar I)
- 5 artÃ­culos con semÃ¡ntica de posesión
- `mover x` â€” transferencia explÃ­cita de posesión
- `copiar x` â€” clone explÃ­cito
- Use-after-move detection (Nivel 1)
- control de préstamos gradual (Nivel 0â†’2)
- Referencias `&T`, `&mut T`, dereferencia `*ref`
- vidas lÃ©xicos: `&nombre T`
- Field-level préstamo (`&mut punto.x` vs `&mut punto.y`)
- Branch-aware liveness (borrows mueren por rama del CFG)
- ArtÃ­culos extendidos: `los` = Posesión compartida, `las` = Prestado compartido

### Estructuras de datos
- **Arrays**: `[T; N]`, literales, `todos expr`, acceso, asignaciÃ³n
- **Structs**: `estructural Punto { ... }`, disposición C, acceso a campos
- **Enums**: tag+union, variantes con datos, pattern matching
- **Texto**: texto en montón con `texto_nuevo()`, `texto_agregar()`, `texto_liberar()`
- **Vector<T>**: vector en montón genÃ©rico con `vector_nuevo()`, `vector_agregar()`, etc.
- **Resultado<T,E>**: `Exito(valor)` / `Error(codigo)` con operador `?`
- **Diccionario/K/V** y **Conjunto** (Fase R4)

### Generics
- Const generics: `fn longitud<N: Entero32>(nums: [Entero32; N]) -> Entero32`
- Type generics con bounds: `fn mÃ¡ximo<T que Comparable>(a: T, b: T) -> T`
- MonomorfizaciÃ³n automÃ¡tica por tipo concreto

### Traits / Rasgos
- DeclaraciÃ³n: `rasgo Nombre { fn metodo(...); ... }`
- ImplementaciÃ³n: `implementar Rasgo para Tipo { fn metodo(...) { ... } }`
- VerificaciÃ³n semÃ¡ntica de mÃ©todos requeridos

### Bitwise + I/O + InterpolaciÃ³n
- Operadores bitwise type-safe: `& | ^ << >> ~ >>>`
- Built-ins I/O: `imprimir`, `imprimir_linea` â€” polimÃ³rficos (Texto, Entero, Bool, Flotante)
- String interpolation: `imprimir_linea("x = {x}, y = {y}")`
- `tamaÃ±o_de::<T>()` â€” sizeof comptime
- MÃ©todos en enteros: `x.poner_bit(3)`, `x.unos()`, `x.ceros_izquierda()`

### FFI + ejecución de C
- `inseguro fn` para funciones sin cuerpo
- Built-ins C: `puts`, `malloc`, `free`, `printf`
- `archivo_leer()`, `archivo_escribir()`, `archivo_existe()`
- `abs()`, `max()`, `min()`, `raiz()`, `potencia()`

### Async / Concurrencia (Fase 18)
- `fut fn` â€” funciones async
- `esperar expr` â€” await
- `lanzar expr` â€” spawn hilo real (CreateThread)
- `dormir(ms)` â€” Sleep de kernel32
- Canales mpsc: `canal_nuevo`, `canal_enviar`, `canal_recibir`, `canal_intentar`
- `con_executor(N)` â€” grupo de hilos real con cancelaciÃ³n estructurada
- `seleccionar { }` â€” select pattern sobre canales
- Stackless futures (state machine desugaring)

### Tooling
- CLI: `falcato build`, `falcato run`, `falcato check`, `falcato lsp`, `falcato version`
- LSP completo: diagnÃ³sticos, autocompletado, hover, go-to-definition, find-references
- Script `build.ps1` automÃ¡tico (auto-detecta Visual Studio)
- 40 tests unitarios pasando
- 50+ ejemplos funcionando

---

## ðŸ“¦ InstalaciÃ³n alternativa: Compilar desde fuente

Si quieres contribuir o necesitas la Ãºltima versiÃ³n:

### Requisitos
- [Rust](https://rustup.rs/) (stable)
- [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022) â†’ "Desktop development with C++"

### Compilar
```powershell
git clone https://github.com/CerebroCanibalus/falcato.git
cd falcato
cargo build --release
# falcato.exe estÃ¡ en target/release/
```

### Probar
```powershell
.\target\\release\\falcato.exe version
```

---

## ðŸŽ¨ VS Code Extension

Resaltado de sintaxis, LSP integrado y tema **"Falcato Dorado"**:

1. Descarga el `.vsix` desde [Releases](https://github.com/CerebroCanibalus/falcato/releases)
2. `Ctrl+Shift+P` â†’ "Extensions: Install from VSIX..."
3. Selecciona el archivo `.vsix`
4. Abre un `.fc` â†’ sintaxis + diagnÃ³sticos en tiempo real
5. `Ctrl+K Ctrl+T` â†’ busca "Falcato Dorado" para el tema

---

## Estado actual

| Aspecto | Estado |
|---------|--------|
| Pipeline integralmente | âœ… Operativo |
| motor Cranelift | âœ… Generando binarios nativos |
| Tests unitarios | âœ… 40/40 pasando |
| Ejemplos funcionando | âœ… 50+ |
| LSP | âœ… Completo |
| Async (hilos + TCP + canales + grupo de hilos) | âœ… Fase 18A-18D |
| Stackless futures | âœ… MVP |
| Diccionario + Conjunto | âœ… Fase R4 |
| DocumentaciÃ³n completa | âœ… GUIA.md + 15 capÃ­tulos + REFERENCIA.md + ERRORES.md |
| VS Code Extension | âœ… Syntax + LSP + tema Falcato Dorado |
| CI GitHub Actions | âœ… Build + test |
| DistribuciÃ³n | âš ï¸ Pre-lanzamiento v0.1.0 |

---

## Proyecto

| Recurso | UbicaciÃ³n |
|---------|-----------|
| Repositorio | [github.com/CerebroCanibalus/falcato](https://github.com/CerebroCanibalus/falcato) |
| DocumentaciÃ³n | `GUIA.md` + carpeta `GUIA/` (15 capÃ­tulos) |
| Referencia de built-ins | `REFERENCIA.md` |
| CÃ³digos de error | `ERRORES.md` |
| InstalaciÃ³n | `INSTALL.md` |
| Ejemplos | `ejemplos/` (50+ archivos `.fc`) |
| Skill para LLMs | `falcato-language` (OpenCode) |
| Para contribuidores | `AGENTS.md` |

---

## Stack tÃ©cnico

| Componente | TecnologÃ­a |
|------------|-----------|
| CLI | `clap` 4.5 (Rust) |
| analizador léxico | `logos` 0.14 |
| Parser | Manual descendente + Pratt |
| AST | Propio con intervalo obligatorio |
| SemÃ¡ntica | Concordancia LingÃ¼Ã­stica |
| Codegen | `cranelift-codegen` 0.112 |
| LSP | `tower-lsp` 0.20 |
| Target | x86_64 Windows (msvc) |
| ABI | C por defecto |
| Testing | 40 tests unitarios |

---

## Licencia

MIT OR Apache-2.0 â€” elige la que prefieras.

---

> *Falcato no es una traducciÃ³n de Rust al espaÃ±ol.*
> *Es un lenguaje de sistemas donde el espaÃ±ol es el sistema de tipos.*
> *Donde la concordancia gramatical es verificaciÃ³n de compilaciÃ³n.*
> *Donde los tiempos verbales son modos de ejecuciÃ³n.*
> *Donde 500 aÃ±os de evoluciÃ³n lingÃ¼Ã­stica se convierten en garantÃ­as de cÃ³digo.*

```
  â €â €â €â €â €â €â €"å¤šè¬åž‚æ³¨"
  â €â €â €â£â¡± â£â¡‰ â£â¡± â¡‡ â£Žâ£±   â¡·â¢¾ â¢‡â¡¸
  â €â €â €â §â œ â §â ¤ â ‡â ± â ‡ â ‡â ¸   â ‡â ¸ â ‡â ¸
  â €https://ko-fi.com/general_beria
```
