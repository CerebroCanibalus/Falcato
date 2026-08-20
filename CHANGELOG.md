# Changelog de Falcato

## [0.7.2] - 2026-08-19

### 🔧 ARREGLOS

- **F-006 — Análisis semántico en dos pasadas (BLOQUEANTE)**: el analizador
  semántico ahora recolecta firmas en una primera pasada y analiza cuerpos
  en una segunda. Esto resuelve falsos positivos de `[T001]` y `[T011]` cuando
  una función llama a otra declarada más abajo (forward reference). Antes
  del fix, la inferencia de tipos caía en el default `Entero32` y producía
  errores espurios como "balance_ok es Entero32 pero se declaró Booleano".

  Ejemplo que antes fallaba y ahora compila:
  ```falcato
  función usa_validar(la t: Texto) -> Booleano {
      el r: Booleano = _validar(t);  // _validar declarada abajo
      retornar r;
  }
  función _validar(la t: Texto) -> Booleano { retornar verdadero; }
  ```

  Cubre también forward references de **structs, enums, rasgos, apodos y
  métodos de impl** (no solo funciones). Patrón estándar de compiladores
  modernos (Rust, Go, Zig).

- **Detección de shadowing (warnings T031-T035)**: declaraciones duplicadas
  de funciones, structs, enums, rasgos o apodos ahora emiten un warning con
  span, en vez de sobrescribirse silenciosamente. Antes, `json_reparador.fc`
  tenía `_jr_copiar` y `_jr_es_igual` duplicadas (líneas 163/119 y 253/123)
  y la última definición ganaba sin avisar — bomba de tiempo silenciosa.

  Los warnings no bloquean la compilación. Códigos:
  - `T031` Función duplicada
  - `T032` Struct duplicado
  - `T033` Enum duplicado
  - `T034` Rasgo duplicado
  - `T035` Apodo duplicado

  Salida `--json` ahora incluye el campo `warnings: [...]`.

### 🐛 BUGS DESCUBIERTOS (no resueltos en este release)

- **F-007 — `Vacio` vs `Vacío` en archivos de Cid**: el lexer solo reconoce
  `Vacío` (con tilde) como tipo `Tipo::Vacio`. Varios archivos de Cid
  (`json_reparador.fc`, `http.fc`) escriben `Vacio` (sin tilde), que el
  parser acepta como `Tipo::Nombre("Vacio")` pero el codegen panicea con
  "No se puede compilar tipo Nombre 'Vacio' sin resolver". Es bug del
  archivo, no del compiler. Fix recomendado: en Cid, sed `s/-> Vacio/-> Vacío/g`.

### ✅ VERIFICACIÓN

- `cargo test`: 54/54 verde
- `pruebas/unitest/`: 18 archivos (incluido nuevo `unitest_forward_refs.fc`)
- Ejemplos: 76/83 compilan (7 intencionales)
- Caso del reporte (Cid `json_reparador.fc`): ✅ verifica sin errores
  (2 warnings `T031` por duplicados que el código silenciaba — fix recomendado)

## [0.7.1] - 2026-08-19

### 🔧 ARREGLOS

- **Reorganización de módulos**: división de archivos monolíticos en submódulos manejables para reducir deuda técnica.
  - `semantic.rs` (4316 LOC) → `semantic/` con 5 submódulos (`mod.rs`, `tipos.rs`, `sentencias.rs`, `funciones.rs`, `ownership.rs`).
  - `codegen/builtins.rs` (3684 LOC) → `codegen/builtins/` con 12 submódulos (`io.rs`, `tcp.rs`, `canal.rs`, `texto.rs`, `conversion.rs`, `archivo.rs`, `math.rs`, `vector.rs`, `diccionario.rs`, `proceso.rs`, `sistema.rs`, `tls.rs`).
- **Keywords eliminadas**: `tipo` y `entonces` removidas del lexer y LSP (nunca usadas en parser/semantic).

## [0.7.0] - 2026-08-18 — "¡Fuese!"

### ➕ ADICIONES

- **Aritmética consciente**: el subjuntivo aritmético llega al lenguaje.
  - `a + b fuese` → checked (overflow → `Resultado.Error`, no crash).
  - `un x = a + b` → Option (overflow → `Nada`, sin desbordamiento → `Algo(valor)`).
  - `romper` / `continuar` en bucles.
  - Notación científica en literales (`6.674e-11`).
  - Impresión flotante con `%.17g` (round-trip exacto).
- **20 primitivas nativas** para Cid (5 fases):
  - Strings dinámicos (`texto_agregar_texto`, `texto_poner_byte`, `texto_puntero`, `texto_desde_bytes`).
  - Conversión número↔texto (`entero_a_texto`, `flotante_a_texto`, `booleano_a_texto`).
  - Archivos avanzados + entorno (`archivo_agregar`, `archivo_borrar`, `archivo_listar`, `entorno_obtener`, etc.).
  - TUI + TLS/HTTPS (`terminal_dimensiones`, `tls_conectar/escribir/leer/cerrar`).
- **Trigonometría completa**: builtins libm (`seno`, `coseno`, `tangente`, `arcseno`, `arctangente`, `exp`, `log`, etc.), sufijos de precisión (`_preciso`, `_rapido`, `_aprox`), `seno_2pi`/`coseno_2pi` con polinomios minimax en Cranelift, efecto `vectorizable`, fase nativa para osciladores.
- **Conversión numérica completa**: `como_entero8/16/32/64` + `como_flotante32/64` — familia unificada.
- **Structs entre archivos**: retorno y parámetros de struct cross-file.
- **Diccionario con tipos compuestos**: `Diccionario<Texto, Vector<Texto>>` funciona.

### 🔧 ARREGLOS

- **10 fixes macOS** (Issue #5): linker POSIX, `CLOCK_MONOTONIC`, `PTHREAD_MUTEX_SIZE`, globales Mach-O, `CallConv::AppleAarch64`, `nanosleep`, printf variádica, closures firma, `canal_cerrar`.
- **Aritmética flotante completa**: dispatch por tipo en operaciones binarias.
- **Drop automático**: `free` al final de scope para heap owned (Texto, Vector, Diccionario, Conjunto).
- **Diccionario**: 4 bugs — SSA dominance, cap=0 → crash, strings internados, structs por tipo no por tamaño.
- **`vector_obtener` bounds check**: fuera de rango → 0 definido.
- **`entero_a_texto`** acepta `Entero32` con sextend automático.
- **Unitest de codebase**: suite 12/12 verde.

## [0.6.1] - 2026-08-08

### ➕ ADICIONES
- **Librería `args_avanzados`** (`librerias/args_avanzados.fc`): subcomandos,
  valores por defecto, repetición de etiquetas y argumentos posicionales, sobre
  `argumentos()` — sin tocar el compilador. API: `args_tiene`, `args_obtener`,
  `args_todos`, `args_subcomando`, `args_posicionales`, `args_cuenta`.
- **Conversión de texto a número**: `texto_a_entero`, `texto_a_natural`,
  `texto_a_flotante`, `texto_a_booleano` — parsean un `Texto` al tipo numérico.

### 🔧 ARREGLOS
- `Diccionario<K,V>` / `Conjunto<T>` no resolvían tipos concretos (parser +
  sustitución de genéricos) → ahora sí.
- `texto_nuevo()` imprimía "(null)" (descriptor con puntero NULL) → ahora crea un
  buffer vacío real con terminador nulo.
- `vector_agregar<Texto>` crasheaba por escritura fuera de heap (cap falso en el
  descriptor del vector) → separadas las responsabilidades de descriptor.

### 🛠️ Infraestructura
- `release.ps1`: validación pre-release anti-frágil (mojibake, EOL, versión, árbol).
- `.gitattributes`: fuerza EOL por tipo de archivo (CRLF para wix/bat/ps1).
- Release en español: título/cuerpo generados desde el CHANGELOG.md.

## [0.6.0] - 2026-08-08

### ➕ ADICIONES
- **Etiquetas tipadas**: `función principal(el args: Struct) -> Entero32` — el
  compilador genera el parseo automático de `--etiqueta valor`, validación de tipos
  y `--ayuda` en español. Los artículos de los campos codifican el esquema:
  `el`=requerido, `un`=opcional, `la`=inmutable/validado, `los`=varargs.
- **Builtin `argumentos()`**: `argumentos() -> Vector<Texto>` — argv crudo estilo C,
  con null terminator. Runtime multiplataforma (Windows `CommandLineToArgvW`,
  POSIX `__argc`/`__argv`).
- **Interpolación con acceso a campo**: `{args.nombre}` ahora imprime el campo.

### 🔧 ARREGLOS
- `vector_obtener<Texto>` rompía el verifier de Cranelift → el índice siempre se
  extiende a I64.

### 🛠️ Infraestructura
- Terminología: "etiquetas" en lugar de "flags" en toda la documentación.
- CLI 100 % en español (subcomandos, opciones y ayuda).

## [0.1.0] - Pre-alpha funcional con LSP completo

### Core del lenguaje
- Variables con tipos explícitos (`el x: Entero32 = 10`)
- Operaciones aritméticas con precedencia (`+`, `-`, `*`, `/`, `%`)
- Operaciones de comparación (`==`, `!=`, `<`, `>`, `<=`, `>=`)
- Operadores lógicos (`&&`, `||`, `!`)
- Asignación (`x = expr`)
- Retorno (`retornar valor`)

### Control de flujo
- Condicionales `si` / `sino`
- Bucles `mientras`

### Ownership (Pilar I)
- `el` = mutable (owned)
- `la` = inmutable (borrowed)
- Verificación en tiempo de compilación
- Errores con sugerencias de artículos

### Semántica — Concordancia Lingüística
- Verificación de tipos ("disconcordancia")
- Detección de variables no declaradas
- Verificación de retornos
- Verificación de condiciones Booleanas
- Constantes nombradas para códigos de error (`DISCONCORDANCIA_TIPO`, etc.)
- Mensajes de error en español con metáfora gramatical

### Parser modular
- Arquitectura separada: expresiones, sentencias, declaraciones, tipos
- Recovery de errores: sincronización hasta siguiente declaración
- Errores de sintaxis con códigos [S###] y sugerencias
- Spans reales disponibles en ParserCursor

### Spans reales
- Span en cada nodo AST: expresiones, sentencias, declaraciones, bloques
- Spans combinados: expresiones binarias/unarias cubren todo el operando
- Spans de funciones: desde `función` hasta fin del bloque
- Spans de parámetros: desde artículo hasta tipo

### Lexer mejorado
- Errores léxicos (caracteres inválidos) reportados con span real
- No se silencian con `.ok()?`

### Codegen robusto
- Spans reales en errores
- IDs únicos para strings globales (evita colisión de símbolos)
- Reutilización de func_id existente (no re-declara)

### LSP (Language Server Protocol)
- **Diagnósticos en tiempo real**: lexer + parser + semántica al escribir
- **Spans reales**: errores subrayados con ubicación exacta
- **Autocompletado**: keywords, artículos (el/la/un), tipos primitivos
- **Hover information**: tipo y artículo de variables al pasar el cursor
- **Go to definition**: saltar a la declaración de variables y funciones
- **Índice semántico**: construido desde el AST para navegación rápida
- **Comunicación stdio**: compatible con VS Code, Vim, Emacs

### CLI
- `falcato build` — compila a binario nativo
- `falcato run` — compila y ejecuta
- `falcato check` — análisis estático
- `falcato lsp` — inicia servidor LSP
- `falcato version` — muestra versión

### Arrays (Fase 3.5 — COMPLETADO)
- Tipo `[T; N]` con sintaxis explícita: `los nums: [Entero32; 5]`
- Literal array: `[1, 2, 3]`
- Inicialización replicada: `todos 0` (rellena todo el array con el mismo valor)
- Acceso por índice: `nums[0] = 10`, `nums[i] + nums[j]`
- Asignación a elementos: `nums[2] = 30`
- Stack allocation con `create_sized_stack_slot`
- Índices extendidos a I64 para aritmética de punteros
- Variables de tipo Array se cargan como puntero (dirección base)

### Testing
- 31 tests unitarios pasando
- Tests de lexer, parser, semántica
- Ejemplos verificados: `hola_mundo`, `aritmetica`, `condicional`, `mientras`, `ownership`, `arrays`, `structs`, `enums`, `const_generics`, `que_bounds`

### Tooling
- Script `build.ps1` automático (auto-detecta Visual Studio)
- Agente IA actualizado
- Folder `ejemplos/` limpio (solo `.fc`, sin `.o`/`.exe`)

## [0.2.0] — En desarrollo (Fase 4)

### ✅ Structs (Fase 4 — COMPLETADO)
- Declaración: `estructural Punto { x: Entero32, y: Entero32 }`
- Inicialización: `el p: Punto = Punto { x: 10, y: 20 }`
- Acceso a campos: `p.x`, `p.y`
- Layout C con alineación automática
- Verificación semántica: campos existen, tipos concuerdan, no faltan campos
- Codegen: stack allocation, offsets calculados, load/store por campo

### ✅ Verificación de tipos en llamadas (Fase 5 — COMPLETADO)
- Registro de firmas de funciones en análisis semántico
- Verificación de cantidad de argumentos
- Verificación de concordancia de tipos en cada argumento
- Mensajes de error con nombre de parámetro esperado

### ✅ Ser/Estar en condiciones (Pilar II — COMPLETADO)
- `si x es 5` — comparación de identidad estructural (==)
- `si x está 10` — verificación de estado temporal (== en Fase 5, estado mutable en Fase 6+)
- Semántica diferenciada: `es` = permanente, `está` = temporal

### ✅ Subjuntivo como optimización (Fase 5 — COMPLETADO)
- `si x fuese es 100` — condición improbable, marca cold path
- AST: `ModoVerbal::Indicativo | Subjuntivo`
- Codegen: branch funcional, optimización cold hint en Fase 6+

### ✅ `para` (Fase 6 — COMPLETADO)
- `para num en nums { ... }` — iteración sobre arrays
- Variable de iteración con tipo inferido del elemento
- Codegen: bucle con índice implícito, carga por offset

### ✅ Enums (Fase 7 — COMPLETADO)
- Declaración: `enumeración Estado { Activo, Inactivo }`
- Variantes con datos: `Exito(valor: Entero32)`
- Constructor: `Estado.Activo`, `Resultado.Exito(42)`
- Pattern matching: `si estado es Estado.Activo { ... }`
- Layout tag+union en codegen (I32 tag + datos)
- Verificación semántica: variantes existen, tipos concuerdan

### ✅ Const Generics (Fase 8A — COMPLETADO)
- Declaración: `función longitud<N: Entero32>(los nums: [Entero32; N]) -> Entero32`
- Uso de `N` como valor en el cuerpo de la función
- Monomorfización en el punto de llamada: `longitud(nums)` → `longitud_5`
- Inferencia del valor genérico desde el tipo del argumento array
- AST: `Tipo::ArrayGenerico`, `Tipo::Generico`, `ParametroGenerico`
- Codegen: funciones genéricas almacenadas, instanciaciones cacheadas

### ✅ Type Generics + "que" bounds (Fase 8C — COMPLETADO)
- Declaración: `función máximo<T que Comparable>(el a: T, el b: T) -> T`
- Parseo de bounds como cláusula relativa: `T que Comparable`, `T que Ordenable`
- Verificación semántica: bound `Comparable`/`Ordenable` habilita operaciones de comparación
- Monomorfización por tipo concreto inferido de los argumentos
- Sustitución de `Tipo::Generico` por tipos concretos en codegen
- Ejemplo funcional: `ejemplos/que_bounds.fc`

### En progreso (Post-Fase 8)
1. Find references
2. Refactorings básicos (renombrar variable)
3. Optimización cold block para subjuntivo
4. Genéricos en Enums (`enumeración alguno<T> { ... }`)

## [0.3.0] — Futuro
