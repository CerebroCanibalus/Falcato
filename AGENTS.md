# Falcato — AGENTS.md

## Filosofía
Lenguaje de bajo nivel *construido desde cero* sobre **Cranelift** (apuesta estratégica, no temporal). NO es traducción de Rust. Es **gramatipado** (la gramática española es el sistema de tipos) y **morfosemántico** (la morfología porta significado de máquina): género, tiempos verbales, ser/estar, subjuntivo.

**Visión:** Falcato + Cranelift + WASM = toolchain nativa para código generado por IA. **Velocidad de compilación > velocidad de ejecución optimizada.**

## Los 5 Pilares
| # | Pilar | Esencia | Estado |
|---|-------|---------|--------|
| I | Género = Ownership | `el`=owned mut, `la`=borrowed inmut, `un`=option | ✅ |
| II | Ser/Estar = Const/Mut | `es`=permanente, `está`=temporal | ✅ |
| III | Tiempos = Modos | Presente=sync, Futuro=async, Subjuntivo=fallible | ✅ |
| IV | C ABI por defecto | Layout C, calling C, mangling off | ✅ |
| V | ~~Prefijos semánticos~~ | ~~`re-`=retry~~ | ⛔ Retirado 2026-08-03 |

## Day-0 (no negociable)
- **🚨 TODO EN ESPAÑOL**: lenguaje, errores, CLI, docs. Excepciones: términos técnicos sin traducción (Cranelift, CLIF, JSON, LSP, WASM).
- **C ABI por defecto**: layout C, SystemV, mangling off, salida `.o`
- **Span en cada nodo AST** — sin span no hay LSP
- **Errores en español con códigos** `[T001] archivo.fc:7:12: mensaje` — S/T/O/C/M/I/W
- **Documentar al agente**: cambios grandes → `falcato.md` + skill `falcato-language` en la misma tanda
- **🚨 SEGURIDAD CRÍTICA**: red/sistema/entrada externa → revisión minuciosa antes de mergear
- **NINGUNA ETIQUETA CAMBIA SEMÁNTICA** — etiqueta solo decide CÓMO se produce el binario
- **`--destino` es la ÚNICA etiqueta de plataforma** — el `.fc` nunca sabe dónde corre
- **Código portable o no compila**: builtin sin impl para target = error
- **Impls juntas**: Windows + POSIX en la misma tanda
- **VERSIONADO**: `MAYOR.menor.parche` — Bump en `Cargo.toml` + tag `vMAYOR.menor.parche`
- **RELEASES EN ESPAÑOL** y **NOVEDADES POR EFECTO** (➕/🔧/🔁, no por fase)

## Problemas abiertos de diseño de lenguaje

**Visión guía:** potencia de Rust · facilidad de Go · morfología española como superpoder para LLMs.

Falcato no traduce Rust ni clona Go. Es **gramatipado y morfosemántico**: la gramática española ES el sistema de tipos, y la morfología verbal codifica modos de cómputo (presente=sync, futuro=async, subjuntivo=fallible). Eso da a los LLMs una propiedad única — el código se lee como español natural.

Estamos en **"pasos de bebé"**: todavía hay tensiones de diseño que debemos resolver ANTES de comprometernos. Cada decisión acá es **casi irreversible** (cambia el "lenguaje sentido" durante años). Mejor resolver lento y bien que rápido y mal.

### Tensiones vivas

| # | Tensión | Estado | Próximo paso |
|---|---------|--------|--------------|
| **P-001** | Stdlib: por tipos (Go: `texto`, `archivo`, `red`) vs por intención (verbos: `hacer.archivo.leer`) | ✅ **RESUELTO 2026-08-28**: tipos fragmentados + verbos consistentes + namespaces explícitos (`::`) + conjugación como azúcar | Implementar en 0.8.0 |
| **P-002** | Sintaxis namespace: `.` (colisión con métodos/campos) vs `::` (Rust-like) vs `snake_case` | ✅ **RESUELTO 2026-08-28**: `::` (evidencia LLMs; coma descartada) | Implementar en 0.8.0 |
| **P-003** | Auto-import: prelude pequeño (Rust) vs todo-std auto (Python) | 🟡 Vinculado a P-001 | Diferir a 1.0 |
| **P-004** | Doble API método/función: `t.contiene(sub)` vs `texto.contiene(t, sub)` | 🟡 Diseño inestable | Formalizar regla antes de 1.0 |
| **P-005** | Builtins inflados: 30+ en Capa 1 (FFI) vs reducir a ~15 | 🟢 Activo | Migración gradual 0.8.x |
| **P-006** | Default `Entero` = 32 (rompe ABI) vs 64 | 🟡 Diferido | RFC 0.8.0 con aliases |
| **P-007** | Keyword renames: `apodo`/`alias`, `rasgo`/`protocolo`, `retornar`/`devolver` | 🟡 Diferido | Requiere `MAYOR` (1.0) |
| **P-008** | Mensajes de error: `T001 disconcordancia` vs `no coincide` | 🟡 Diferido | Decisión de estilo 0.8.0 |
| **P-009** | Artículos opacos `el`/`la`/`un` (curva de aprendizaje) | 🟢 Activo | Tabla + sugerencias clippy |
| **P-010** | Inconsistencia API: `archivo_escribir(ruta=Palabra)` vs `archivo_agregar(ruta=Texto)` | 🔴 Bug | F-019, fix 0.7.6 |
| **P-011** | Concurrencia con keyword `paralelo { }` + integración con modos verbales (`fut`/`fuese`/`lanzar`) | 🟡 RFC abierta | Diseñar antes de 1.0; feature signature |
| **P-012** | Tipos con rangos `T entre 0 y 10` | 🟡 Ya en roadmap (R7.7 F3) | Ejecutar; validar sintaxis propuesta |
| **P-013** | Descubrimiento de API (`t.` → suggestions por tipo) | 🟢 Tooling, no lenguaje | Mejorar LSP en 0.8.0 |
| **P-014** | Warning de `Resultado` no manejado (`must_use` español) | 🟢 Lint, no lenguaje | W-code en 0.8.0 |
| **P-015** | Meta: no delayar features críticos (anti-Go: generics, ownership, errores desde el inicio) | 🟢 Filosofía | Documentar como principio Day-0 |
| **P-016** | **Autofree determinístico** (el artículo ES el ownership → el compiler inserta `liberar()` con certeza). Feature crítica: completar la promesa morfosemántica. | 🟢 Activo | Diseñar RFC + escape hatch; validar contra V (que falla por adivinar ownership) |
| **P-017** | **Complejidad accidental** (API inconsistente, comportamientos silenciosos, empaquetado manual). Epidemia de inconsistencias que penalizan a LLMs. | 🔴 Activo | Regla unificadora de API; fix por familia; ver tabla de detalle |

### Detalle de problemas clave

#### P-001 — Organización de stdlib

**Tensión:** ¿cómo organizamos las ~50 funciones que pasarán de builtins a stdlib?

- **Opción A — Por tipos (Go/Rust):** `std::texto::contiene`, `std::archivo::leer`, `std::red::get`. Familiar para devs con background en lenguajes establecidos.
- **Opción B — Por intención (verbos):** `std::hacer::archivo::leer`, `std::saber::tiempo::ahora`, `std::decir::consola::imprimir`. Coherente con Pilar III.
- **Opción C — Híbrido:** verbos para nivel alto (`hacer::red::get`), sustantivos para operaciones sobre datos (`texto::contiene`).

**Análisis (2026-08-27):** la propuesta B (verbos puros) tiene tres fallos: (1) solapamiento entre `hacer.archivo.leer` y `leer.archivo`, (2) `.` colisiona con method-call syntax, (3) mezclar métodos y funciones viola "una sola forma de hacer cada cosa". **La idea de创新的 es integrar morfología verbal en signatures, no reorganizar namespaces.**

**Decisión (2026-08-28) — DISEÑO APROBADO: "tipos fragmentados + verbos consistentes + namespaces explícitos".** Basado en evidencia de estudios sobre LLMs (namespaces se alinean con la arquitectura de atención; la sobrecarga/ADL es problemática incluso para humanos; los formatos explícitos benefician a modelos débiles). El diseño:
1. **Tipos fragmentados por dominio** (sustantivos): `Archivo`, `Conexion`, `HttpCliente`, `Socket`, `Directorio` — cada uno con sus campos y estado.
2. **Namespaces explícitos** (para LLMs): `archivo::abrir`, `red::abrir` — el contexto de dominio incrustado en el nombre.
3. **Verbos consistentes** (uniformidad semántica): `abrir`, `leer`, `escribir`, `cerrar`, `conectar`, `enviar`, `recibir` son los MISMOS en todos los dominios — reduce la memorización.
4. **Conjugación como azúcar OPCIONAL**: `abrir(archivo)` es azúcar para `archivo::abrir(archivo)` — el compilador lo expande, pero el LLM siempre puede usar la forma explícita.

```falcato
// Forma canónica (explícita, para LLMs)
el archivo: archivo::Archivo = archivo::abrir("ruta.txt");
el contenido: Texto = archivo::leer(archivo);
archivo::cerrar(archivo);

// Azúcar (conjugación, para humanos)
el conexion: red::Conexion = abrir(conexion, host, puerto);
el datos: Texto = leer(conexion);
cerrar(conexion);
```

**Próximo paso:** implementar en 0.8.0. Los verbos canónicos se definen en un diccionario consultable (LSP + `falcato doc`).

> **📋 Borrador de diseño completo:** `docs/diseno_libest.md` (2026-08-28) — árbol completo de la libEst con 12 dominios (núcleo, colecciones, archivo, red, tiempo, proceso, sistema, matemáticas, visual, compat). Incluye capa visual/gráfica (ventana, control, layout, evento, color, geometria, lienzo, imagen, fuente, animacion, **sonido** para DAW R9.3, terminal_ui). Sintaxis verificada contra el compiler real (v0.7.5): `Option<T>` (no `Opcion`), `Resultado<T, E>` con 2 params, genéricos explícitos `<T>`, `Palabra`/`Texto` en español.

#### P-002 — Sintaxis de namespace

**Tensión:** `.` ya significa method-call Y field-access en Falcato. Usar `.` para namespace requiere parser lookahead y reglas contextuales.

| Sintaxis | Pros | Contras |
|----------|------|---------|
| `hacer.archivo.leer(...)` | "español natural" | Colisión con `.` (método/campo), parser hack |
| `hacer::archivo::leer(...)` | Familiar Rust, parser simple | Menos "natural" |
| `hacer_archivo_leer(...)` | Status quo | Pierde la propuesta de innovación |

**Recomendación actual:** **Opción B (`::`)**. Coherente con `Resultado.Exito` (que ya se usa en pattern matching). Familiar para quien venga de Rust. Permite detectar en parse que es namespace vs method-call sin ambigüedad.

**Decisión (2026-08-28) — CONFIRMADA:** `::` es el símbolo de namespace. La coma `,` queda descartada (ya es el separador universal: parámetros, campos, arrays, enums). La evidencia de estudios sobre LLMs refuerza `::`: los namespaces jerárquicos (`dominio::accion`) se alinean con la atención de los LLMs, reducen la ambigüedad, y son el patrón dominante en training data. Ver P-001 para el diseño completo.

**Estado:** ✅ Resuelto en 0.8.0 con P-001.

#### P-003 — Auto-import scope

**Tensión:** ¿qué está disponible sin `usar`?

- **Opción A — Prelude pequeño (Rust):** auto-import solo `imprimir`, `imprimir_linea`, `salir`. Resto requiere `usar std::*`.
- **Opción B — Todo std auto (Python):** cero imports. Bueno para scripts, malo para sistemas grandes (shadowing, debugging difícil).
- **Opción C — Híbrido:** prelude + std::núcleo auto, std::experimental explícito.

**Recomendación actual:** **Opción A**. El control explícito es coherente con la Day-0 "ninguna etiqueta cambia semántica" — un import explícito es una declaración de intención, no un detalle.

#### P-004 — Métodos vs funciones

**Tensión:** la misma operación puede ser método (`t.contiene(sub)`) o función (`texto.contiene(t, sub)`). ¿Cuál es canónica?

**Regla propuesta (a validar en 0.8.x):**
- **Método** cuando `t` es el sujeto natural y la operación ES sobre `t`.
- **Función en std** cuando hay DOS operandos del mismo tipo, o cuando `t` no es "sujeto".

```falcato
t.longitud()                  // ✅ método — unaria, t es sujeto
t.contiene("hola")            // ✅ método — t es el receptor natural
texto.unir(a, b)              // ✅ función — dos operandos del mismo tipo
vector.agregar(v, x)          // ⚠️ ambigua: ¿método sobre v o función sobre v+x?
```

**Decisión pendiente:** la regla debe quedar escrita y ANTES de reorganizar stdlib, para no crear ambas formas accidentalmente.

#### P-005 — Reducir builtins (migración gradual)

**Problema:** demasiadas operaciones viven en Capa 1 (FFI C / Cranelift intrinsic) cuando deberían vivir en stdlib (Falcato puro). Builtins son costosas: 3 backends × N builtins = multiplicación de mantenimiento.

**Plan:**
- **Quedan en builtins** (irreducibles): constructores de heap (`texto_nuevo`, `vector_nuevo`), I/O primitives (`imprimir`, `imprimir_linea`), sizeof/alignment, FFI primitives.
- **Mueven a stdlib:** todas las operaciones que se puedan expresar en Falcato puro sobre los tipos anteriores.

| Builtin actual | Reemplazo en std | Versión |
|----------------|------------------|---------|
| `texto_agregar_texto` | `std::texto::agregar` | 0.8.0 |
| `archivo_leer` | `std::archivo::leer` | 0.8.0 |
| `tcp_conectar` | `std::red::tcp::conectar` | 0.8.1 |
| `tls_conectar` | `std::red::tls::conectar` | 0.8.1 |
| `proceso_crear` | `std::proceso::crear` | 0.8.2 |
| `http_get` | `std::red::http::get` | 0.9.0 |
| `json_serializar` | `std::json::serializar` | 0.9.0 |

**Sin romper compatibilidad:** los nombres actuales se mantienen como aliases en `std::compat` durante 2-3 releases.

#### P-011 — Concurrencia con keyword `paralelo { }`

**Tensión:** no hay forma clara hoy de "corre esto en paralelo y sincroniza al final". `lanzar` es fire-and-forget; `fut` es async pero secuencial.

**Propuesta (analizada 2026-08-27):**

```falcato
paralelo {
    el a = computar(x);        // sync dentro del thread A
    el b = computar(y) fuese;  // sync + puede fallar, thread B
    fut c = obtener_async(z);  // async dentro del thread C
}
// sincronización implícita al salir del bloque (espera todos los hilos)
```

**Innovación real ≠ nombre bonito, es la integración coherente:** los modos verbales existentes (`fut`/`fuese`/`lanzar`) funcionan dentro de `paralelo { }` sin cambios. La gramática española se extiende sin forzarla.

**Nota lingüística:** la propuesta inicial usaba "presente progresivo" como etiqueta, pero en español "progresivo" es un *aspecto* (gerundio), no un *modo verbal*. Los modos son indicativo/subjuntivo/imperativo. Forzar morfología verbal al bloque (`computando { }`) sería gramaticalmente forzado. **Recomendación: keyword dedicada (`paralelo`, `en_paralelo`), morfología aplicada a los verbos internos.**

**Costo real:** ~150-200 LOC en parser + ~80 LOC en codegen (no ~100 como se estimó originalmente). El runtime ya tiene threads via `lanzar`.

**Estado:** 🟡 RFC abierta. Diseñar antes de 1.0. **Feature signature de Falcato** si se implementa bien.

#### P-012 — Tipos con rangos

**Tensión:** bounds checks son verbosos, runtime, y propensos a olvidar.

**Propuesta:**
```falcato
el x: Entero32 entre 0 y 10 = 5;          // subtipo de Entero32
función dividir(la a: Entero32, la b: Entero32 entre 1 y 100) -> Entero32 {
    retornar a / b;  // b ∈ [1, 100] garantizado
}
el i: Entero32 entre 0 y vector_longitud(v) - 1 = 0;  // válido para indexar v
```

**Estado:** 🟡 **Ya en roadmap como R7.7 F3** ("Rangos: verificación compile-time + runtime checks"). No duplicar — confirmar que la propuesta coincide y ejecutar.

**Costo real:** ~500-600 LOC (mayor que la estimación inicial de 200). Cada operación aritmética debe chequear bounds; inferencia de subtipos se complica (`entre 0 y 10` es un tipo, no un literal).

**Precursores:** Ada (subtypes), Zig (sentinel values), Rust (const generics limitado). Falcato puede hacerlo más legible con sintaxis natural.

#### P-013 — Descubrimiento de API

**Tensión:** usuarios (humanos y LLMs) tienen que memorizar o buscar la API de cada tipo.

**Propuesta:**
```falcato
el t: Texto = texto_desde("hola");
t.  // ← el compiler/LSP muestra métodos disponibles con firmas
// t.contiene(otro: Texto) -> Booleano
// t.empieza_con(prefijo: Texto) -> Booleano
// t.longitud() -> Entero32
// ...
```

**Estado:** 🟢 **Tooling, no lenguaje.** Ya existe el LSP; solo falta indexar métodos por tipo y exponerlos en completion. ~100 LOC en semántica + mejoras en LSP.

**No es innovación** (VS Code lo tiene desde 2015 para TypeScript), pero es **necesario** para DX. Programar para 0.8.0 junto con P-005 (reducir builtins).

#### P-014 — Warning de `Resultado` no manejado

**Tensión:** un usuario puede aceptar un `Resultado` sin `fuese` y olvidarse de manejar el error.

**Propuesta:**
```falcato
el r: Resultado<Texto> = procesar(dato, config);
// [W001] Resultado no manejado. Usa `si r es Exito { ... }` o `r?`

si r es Exito {
    decir.consola.imprimir(r.valor);
} sino {
    decir.consola.imprimir("error");
}
```

**Estado:** 🟢 **Lint, no lenguaje.** Patrón conocido (`#[must_use]` en Rust). ~80 LOC para análisis de flujo + emisión.

**Nota:** `fuese` ya obliga a manejar (compile-time error). El warning es para el caso donde el usuario explícitamente acepta `Resultado` sin `fuese` — raro pero posible. W-code en 0.8.0, baja prioridad.

#### P-015 — No delayar features críticos (anti-Go)

**Principio:** generics, ownership, errores — todos desde el inicio. No esperar al "después" como Go.

**Estado:** 🟢 Filosofía. Falcato YA cumple:
- Generics: desde 0.6.x.
- Ownership: desde el primer release.
- Errores: `Resultado` + `?` desde el inicio.
- Async: desde R7.

**Por qué importa:** Go tardó 8 años en añadir generics (2009 → 2017 → estable 2021). Esa deuda costó: paquetes enteros reescritos, fragmentación del ecosistema, herramientas con APIs inconsistentes. **Falcato no puede permitirse esa hipoteca.**

**Regla operativa:** si una feature es "necesaria para hacer el lenguaje usable", entra en 0.x. Si es "nice to have", va a roadmap. La línea está en si Cid podría escribir un programa no-trivial sin esa feature.

#### P-016 — Autofree determinístico (feature crítica)

**Problema:** hoy Falcato es Rust con artículos españoles. La morfosemántica es un empaque brillante, pero el motor de memoria (borrow checker con `el`/`la`/`un`/`los`/`las`) es Rust con otro disfraz. El programador (o el LLM) tiene que escribir `liberar()` a mano — la promesa morfosemántica queda a medias: el artículo declara ownership, pero el compilador no lo USA para gestionar la memoria.

**La oportunidad:** Falcato YA tiene lo que a V le falta: un modelo de ownership explícito en cada variable. V's autofree falla porque **adivina** el ownership por análisis de escape (double-free, use-after-free, no puede auto-compilarse, solo cubre ~90%). Falcato **declara** el ownership con el artículo → el compilador puede insertar `liberar()` con **certeza**, no con heurística.

**Propuesta (autofree determinístico por artículos):**
- `el` = dueño → compiler inserta `liberar()` al salir del scope (caso común, sin `liberar()` manual).
- `la`/`las` = prestado → nunca libera (el compiler sabe que no es dueño).
- `un` = opcional → libera si está presente.
- `los` = ref-count → libera cuando el contador llega a 0 (ya es ARC).
- **Escape hatch:** `inseguro`/`manual` para quien quiera control fino (como `[manualfree]` de V, pero seguro porque el default es determinístico).

**Por qué es innovación real (no re-empaque):**
- **No es Rust** (Rust no libera automáticamente — obliga a RAII/drop).
- **No es V** (V adivina; Falcato declara).
- **No es GC** (estático, determinístico, zero-cost).
- **Completa la promesa morfosemántica:** la gramática no solo describe el sistema de tipos, lo EJECUTA. El LLM genera `el`/`la` correctamente (gramática natural) y el compilador hace el resto — no tiene que recordar `liberar()`.

**Riesgos (honestos):**
- Requiere análisis de escape real en el compiler (hoy no existe). Trabajo no trivial.
- Riesgo de regresión: bugs en autofree → double-free donde antes había manual. V es la advertencia.
- Pérdida de control para quien QUIERE free manual.
- **Recomendación:** default conservador — autofree como opt-in primero (como V), pero con la ventaja de ownership declarado. Validar contra el caso de Cid.

**Estado:** 🟢 Activo. **Feature crítica** — completar la promesa morfosemántica. Diseñar RFC + escape hatch antes de implementar.

#### P-017 — Complejidad accidental (epidemia de inconsistencias)

**Problema:** Falcato creció por acumulación de features sin una regla unificadora de API. Cada builtin se añadió cuando se necesitó, con el tipo que parecía natural en ese momento, sin preguntarse "¿cómo se comportan sus hermanos?". Resultado: una epidemia de complejidad accidental que penaliza a los LLMs (que no pueden memorizar excepciones) y a los humanos.

**Distinción clave:** complejidad **esencial** (aportar valor: ownership, generics, errores) vs **accidental** (solo dificulta sin beneficio). P-017 es sobre la accidental.

**Regla unificadora propuesta:** *"Toda API debe ser consistente con su familia. Si hay dos formas de hacer lo mismo, es un bug de diseño, no una feature."*

### Catálogo de complejidad accidental (por familia)

**Familia 1 — Misma operación, tipos de argumento inconsistentes** (la peor)
| Builtin | Espera | Hermano | Espera | Inconsistencia |
|---------|--------|---------|--------|----------------|
| `archivo_escribir` | `Palabra` | `archivo_agregar` | `Texto` | Misma familia, tipos distintos |
| `tcp_conectar` | `Palabra` | `tls_conectar` | `Texto` | Misma red, tipos distintos |
| `entero_a_texto` | `Entero64` | `flotante_a_texto` | `Flotante64` | Conversión, tipos arbitrarios |
| `archivo_listar` | **crash** con `Palabra`, ok con `Texto` | — | — | Comportamiento según cómo pasas el arg |

**Regla:** TODOS los builtins aceptan `Texto` y `Palabra` por igual (coerción uniforme). No hay razón para que `archivo_escribir` sea distinto de `archivo_agregar`.

**Familia 2 — Comportamientos silenciosos incorrectos** (los más traicioneros)
| Problema | Comportamiento | Riesgo |
|----------|---------------|--------|
| **F-020** | `"{f(x)}"` interpolación con llamada → imprime vacío | Silencio total, parece que funciona |
| **F-021** | `a > 0` infiere `Entero32` no `Booleano` | Tipo equivocado, lógica rota |
| **F-013** | campo `Entero64` trunca literal grande a i32 | Pérdida de datos silenciosa |
| `archivo_listar` | literal `Palabra` → descriptor basura | Crash en runtime |

**Regla:** *si no puedes hacerlo bien, da error.* Un `[S080]` en compile-time es infinitamente mejor que un vacío silencioso.

**Familia 3 — API inflada y ambigua** (P-005 + P-004)
- 30+ builtins en Capa 1 que el usuario tiene que memorizar; muchos expresables en Falcato puro.
- Doble API: `t.contiene(sub)` Y `texto.contiene(t, sub)` — ¿cuál es canónica?
- `texto_agregar` solo acepta literales `Palabra`, pero `texto_agregar_texto` acepta `Texto` — dos funciones para "agregar texto" con reglas distintas.

**Familia 4 — Empaquetado manual de datos** (debería ser automático)
- `terminal_dimensiones` devuelve ancho/alto empaquetados en un `Entero64` — el usuario hace bit-shifting a mano (`como_entero32(dims >> 32)`). Debería devolver un struct o dos valores.

**Familia 5 — Inconsistencias sintácticas**
- `retornar Struct { ... }` no parsea, pero asignar a variable sí.
- `&buf[0]` vs `&buf` — el compiler convierte, pero el usuario no lo sabe.
- `todos 0` infiere `Entero32` → falla con `[Entero8; N]`; el usuario tiene que usar `[Entero32; N]` para buffers crudos.

**Familia 6 — Múltiples nombres para lo mismo** (P-007)
`apodo`/`alias`, `rasgo`/`protocolo`, `retornar`/`devolver`, `función`/`funcion`/`fn`, `coincidir`/`emparejar`. Cada alias es una decisión extra.

### Priorización

| Prioridad | Problema | Esfuerzo | Impacto en LLMs |
|-----------|----------|----------|-----------------|
| 🔴 Alta | Coerción uniforme `Texto`/`Palabra` en todos los builtins | Medio | Elimina la peor clase de errores |
| 🔴 Alta | Comportamientos silenciosos → errores compile-time | Bajo | El LLM ve el fallo y lo arregla |
| 🟡 Media | `terminal_dimensiones` → struct o tupla | Bajo | Elimina bit-shifting manual |
| 🟡 Media | Unificar doble API método/función (P-004) | Alto | Reduce ambigüedad |
| 🟢 Baja | Renombres (P-007) | Requiere MAYOR | Menos decisiones |

**Estado:** 🔴 Activo. **Regla unificadora de API como Day-0** — toda API nueva debe ser consistente con su familia. Fix por familia, no por builtin aislado.

### Próximas decisiones a tomar

| Plazo | Acciones |
|-------|----------|
| **🎯 META 0.8.0 — La gran actualización** | **Rediseño y mejora brutal del lenguaje.** Agrupa: P-002 (namespace), P-005 (reducir builtins), P-009 (artículos), P-013 (LSP), P-014 (warning Resultado), P-017 (regla unificadora API), `falcato formatea` (gofmt), stdlib HTTP+JSON (desbloquea Cid), modernizers (desbloquea P-007 renombres). Ver sección "R0.8.0 — La gran actualización". |
| **Inmediato (0.7.6)** | P-010: fix F-019, F-020, F-021. **No romper nada.** |
| **Corto plazo (0.8.x)** | P-005 (reducir builtins), P-009 (documentar artículos), P-013 (LSP completitud), P-014 (warning `Resultado`). **Decidir P-002 (sintaxis namespace).** **P-017: aplicar regla unificadora de API por familia (coerción `Texto`/`Palabra`, errores en vez de silencio).** |
| **Mediano plazo (0.8.x-0.9.x)** | P-011 (diseñar `paralelo { }` con feature signature), P-012 ejecutar R7.7 F3 (rangos). **P-016 (autofree determinístico): diseñar RFC + escape hatch, validar contra V.** Validar con Cid. |
| **Largo plazo (1.0)** | P-001 + P-003 + P-004 juntos con migration plan completo. Requiere RFC pública con casos de uso reales de Cid. P-015 seguir como principio rector. |
| **Validación empírica** | Antes de reorganizar stdlib: esperar a que Cid genere 3 proyectos reales y observar qué patrones emergen. **No reorganizar antes de tener datos.** |

### Principio guía para todas estas decisiones

> **La morfología española ES el sistema de tipos.** Cualquier decisión de stdlib debe reforzar esto, no contradecirlo.
>
> Ejemplo: si los verbos del namespace (`hacer`, `saber`, `decir`) coinciden con los modos verbales del lenguaje (presente, subjuntivo, futuro), el código es autodocumentado:
> ```falcato
> hacer::red::get(url)              // presente → síncrono
> fut hacer::red::get(url)          // futuro → asíncrono
> hacer::red::get(url) fuese        // subjuntivo → fallible (devuelve Resultado)
> ```
> Esta integración namespace-morfología **sí es创新** y diferencia Falcato de todo lo demás. Pero requiere P-002 resuelto primero.

> **Anti-Go: no delayar features críticos.** Generics, ownership, errores, async — todo desde el inicio. La línea divisoria: si Cid podría escribir un programa no-trivial sin esa feature, es 0.x. Si es nice-to-have, va a roadmap (ver P-015). Go tardó 8 años en añadir generics; **esa hipoteca es lo que Falcato no puede permitirse.**

## Lecciones de Go (benchmark de plataforma, 2026-08-27)

Go es hoy el benchmark explícito de "lenguaje ideal para AI-assisted software engineering" (Google Developers Blog, 2026-08). Compite por el MISMO público que Falcato (LLMs generando código), y en varias dimensiones de **plataforma** lo hace mejor que nosotros. La lección central: **Falcato no pierde contra Go en el lenguaje — pierde en la plataforma.** La gramática morfosemántica es genuinamente innovadora, pero Go tiene 16 años de plataforma pulida.

### El gap #1: NO tenemos formateador automático (crítico para LLMs)

Go tiene `gofmt` — un solo formato, enforzado por el toolchain. Todo el código Go se ve igual, sin importar quién (o qué LLM) lo escribió. Esto es crítico para LLMs porque:
- **Datos de entrenamiento uniformes** → modelos generan mejor código en menos intentos (efecto de red).
- **Detección de errores más rápida** → con formato predecible, un humano/LLM detecta una API alucinada o un fallo de lógica más rápido.
- **Elimina el debate de estilo** → cero discusiones de "cómo se escribe esto".
- **Reduce complejidad accidental (P-017)** → un solo formato canónico = menos decisiones = menos errores.

**Falcato NO tiene formateador.** Cada LLM genera código con su propio estilo, y el humano tiene que descifrar la intención. Para un lenguaje cuyo público objetivo son LLMs, esto es casi un requisito de existencia.

**Propuesta:** `falcato formatea` (alias `fmt`) como Day-0. Un solo estilo, enforzado. Prioridad máxima.

### Los gaps de plataforma (tabla)

| Herramienta Go | Falcato | Gap | Prioridad |
|----------------|---------|-----|-----------|
| `gofmt` | ❌ no existe | Formateo automático | 🔴 Crítica para LLMs |
| `net/http` + `encoding/json` | `http_get` R9.0, JSON R9.1 (pendientes) | Stdlib HTTP/JSON — **bloquea Cid** | 🔴 Crítica |
| Cross-compilation (`GOOS`/`GOARCH`) | `--destino` R8 (pendiente) | Compilar para múltiples targets | 🔴 Crítica para agentes |
| Goroutines (M:N scheduler) | threads del OS via `lanzar` | Concurrencia masiva barata | 🟡 Alta |
| Binario estático único | depende de runtime C (UCRT/VCRuntime) | `FROM scratch` sin dependencias | 🟡 Alta |
| `go vet` / race detector | `verifica` (básico) | Análisis estático avanzado | 🟡 Media |
| `go test -cover` | `prueba` | Cobertura de tests | 🟡 Media |
| `go doc` | ❌ no existe | Documentación generada | 🟢 Media |
| `go fix` + modernizers | ❌ no existe | Refactorización determinística | 🟢 Media |
| Promesa de compatibilidad Go 1.0 | aliases, sin promesa formal | Garantía "código de hace 15 años compila hoy" | 🟢 Media |
| `defer` | ❌ no existe | Limpieza idiomática de recursos | 🟢 (P-016 lo reduce) |
| `context.Context` | ❌ no existe | Cancelación/deadlines estándar | 🟡 Alta para agentes |
| `slog` (logging estructurado) | ❌ no existe | Logs estructurados en stdlib | 🟢 Media |
| `range-over-func` | `para` sobre arrays/vectores/rangos | Iteradores arbitrarios | 🟢 Media |
| `maps`/`slices` packages | Diccionario/Vector | Utilidades estándar anti-bugs | 🟢 Media |

### El patrón común

Falcato se ha centrado en el **lenguaje** (gramática, tipos, ownership) pero no en la **plataforma** (tooling, stdlib, runtime). Go ganó porque es "no solo un lenguaje, es una plataforma" — y para LLMs, la plataforma importa tanto como el lenguaje.

### Lo que Go hace bien que DEBERÍAMOS copiar (sin vergüenza)

| Gap | Prioridad | Esfuerzo |
|-----|-----------|----------|
| `falcato formatea` (gofmt) | 🔴 Crítica para LLMs | Bajo |
| Stdlib HTTP + JSON (desbloquea Cid) | 🔴 Crítica | Medio |
| Cross-compilation (`--destino`) | 🔴 Crítica para agentes | R8 |
| Scheduler de goroutines (M:N) | 🟡 Alta | Alto |
| Binario estático | 🟡 Alta | Medio |
| `go vet` / race detector | 🟡 Media | Medio |
| `go doc` | 🟢 Media | Bajo |
| Promesa de compatibilidad formal | 🟢 Media | Bajo (política) |

### Principios de diseño del estudio de Google (2026-08-11)

El estudio "Why Go is an Ideal Language for AI-Assisted Software Engineering" (Google Developers Blog) revela principios accionables para Falcato:

- **De escribir a revisar:** el bottleneck de la ingeniería con AI pasa de la generación a la **verificación**. Todo el toolchain debe optimizar revisar/verificar/mantener, no escribir.
- **Ingeniería ≠ programación:** programar es resolver un problema; ingeniería es construir un sistema durable que un equipo (humano+AI) mantiene años. Falcato debe optimizar para lo segundo.
- **AI y humanos tienen necesidades similares:** lo claro para humanos es claro para AI (y viceversa). No diseñar "para AI" por separado.
- **Loop de auto-corrección:** "un primer paso 95% correcto, pasos compuestos degradan precisión y contaminan el contexto". Compilador rápido + formateador + verificación = loop barato de genera→verifica→arregla.
- **Readability over writability:** legibilidad > escritura. Para AI = predictabilidad, explicitismo, estructura rígida. Conecta con P-017 (una forma de hacer cada cosa).
- **Batteries-included = seguridad:** los LLMs sugieren dependencias stale/maliciosas de su training data. Una stdlib completa guía a usar paquetes oficiales → reduce superficie de supply chain. Refuerza R9.1 (JSON) y R9.0 (HTTP) como prioridades de **seguridad**, no solo conveniencia.
- **Compatibilidad = requisito de seguridad/operación:** no lujo. Permite que código generado hoy funcione mañana.
- **El lenguaje es MÁS importante que nunca con AI:** contraintuitivo pero cierto — un lenguaje claro+predecible+con toolchain vale más cuando AI escribe la mayoría del código.

### Features de plataforma a adoptar (del estudio)

| Feature Go | Falcato | Esfuerzo |
|------------|---------|----------|
| `go fix` + **modernizers** (migra patrones viejos determinísticamente) | ❌ | Desbloquea P-007 (renombres sin miedo) |
| **Fuzzing nativo** en test framework | ❌ | Medio |
| `govulncheck` (solo CVEs que tu código llama) | ❌ | Medio |
| **PGO** (profile-guided optimization) en `--lanzar` | ❌ | Medio |
| Profiling + execution tracing integrados | `perfil_*` básico | Medio |
| **go-skills** (ecosistema de skills para agentes) | skill `falcato-language` única | Bajo |

### Veredicto honesto

Falcato no pierde contra Go en el lenguaje — pierde en la plataforma. El mayor error estratégico: **no tener formateador automático** (casi requisito de existencia para un lenguaje de LLMs) y **no tener HTTP/JSON** (bloquea a Cid, el primer dogfooding real).

## Stack técnico
- **CLI:** `clap` 4.5 | **Lexer:** `logos` 0.14 | **Parser:** descendente manual + Pratt | **AST:** propio con span
- **Semántica:** Concordancia Lingüística | **Codegen:** `cranelift-codegen` 0.112 | **LSP:** `tower-lsp` 0.20
- **Target:** x86_64 Windows (msvc) | **Build:** `build.ps1` | **Estilo:** Rust inglés en compiler, español snake_case en lenguaje

### Patrones Cranelift (críticos)
1. Loop header: NUNCA sellar antes del back-edge. Sellar DESPUÉS del `jump`.
2. Cadena if/else con 1 predecesor: sellar inmediato es seguro.
3. `compilar_sentencia` crea sub-bloques: padre sellado ANTES de llamarla.
4. SSA dominance: valores de bloque A no se usan en bloque no-dominado.
5. `iconst` 2º arg: siempre `i64`.
6. `create_sized_stack_slot` (no `create_stack_slot` en 0.112).
7. `FunctionBuilderContext`: desde `cranelift_frontend`.
8. `define_function(func_id, &mut ctx)` — 2 args.
9. Doble sellado = panic.
10. `Linkage::Local` para funciones con cuerpo, `Import` solo para FFI.

### Platform Runtime Layer (3 capas)
```
Capa A — C Runtime (lib/falcato_runtime/): ops multi-paso → staticlib
Capa B — PlatformRuntime trait (src/platform/): primitivas sync → windows.rs/linux.rs/macos.rs
Capa C — BuiltinRegistry (src/platform/registry.rs): remapeo nombre→función C
```
**Regla de oro:** codegen NUNCA hace `#[cfg(target_os)]`.

## Pipeline
```
.fc → Lexer → Parser → Analisis Semantico → Codegen (Cranelift) → .o → Linker → .exe
```

## Estructura del proyecto
```
src/
├── main.rs, span.rs, error.rs, lexer.rs, parser/, ast.rs
├── semantic/ (mod, tipos, ownership, funciones, sentencias)
├── codegen/ (mod, funciones, sentencias, expresiones, generics, tipos) + builtins/
├── platform/ (mod, registry, traits, linker, windows, linux, macos)
├── futuros.rs, resolver.rs, backend.rs
└── lsp/ (mod, indice, completar, diagnostico, hover, referencias)
lib/falcato_runtime/   # Canal, Executor, Thread — staticlink
```

## Estado del proyecto (v0.7.5)
Pipeline end-to-end operativo. Turing-completo:
- **Core:** variables, ops, condicionales, bucles, arrays, structs, enums, generics, apodos
- **Ownership:** `el`/`la`/`un`/`los`/`las`, `&T`/`&mut T`, field-level borrowing, lifetimes, `puro`/`muta`/`lee`
- **Async:** threads, TCP, canales mpsc, thread pool, `seleccionar { }`
- **Built-ins:** Texto, Vector, Diccionario, Conjunto, Resultado, Option, bitwise, I/O, file I/O, math, trig, sizeof
- **Aritmética consciente:** `a + b fuese` (checked), `un x = a + b` (Option), `romper`/`continuar`
- **Two-pass:** forward refs + shadowing T031-T035
- **v0.7.5 (2026-08-21):** coerción polimórfica `42`→`Entero64`/`Natural`, azúcar `42 largo`/`natural largo`/`corto`, profiling `reloj_mono_ns`+`perfil_*`, cross-file `verifica a.fc b.fc`, fixes F-008/F-009/F-013/F-014/F-016/F-017
- **54/54 tests + 19 unitest. 76/83 ejemplos compilan** (7 intencionales)

## Tipos naturales
| Categoría | Nombre | Equiv. |
|-----------|--------|--------|
| Entero | `Entero` | `Entero32` |
|  | `EnteroLargo` | `Entero64` |
|  | `EnteroCorto` | `Entero16` |
|  | `EnteroMínimo` | `Entero8` |
| Natural | `Natural` | `Natural32` |
| Real | `Real` | `Flotante64` |
| Lógico | `Lógico` | `Booleano` |
```falcato
apodo ID = Entero64;
el id: ID = 1234567890123;
```

## Comandos CLI
```bash
falcato compila <file.fc> --salida out.exe
falcato corre <file.fc>
falcato verifica <file.fc>        # multi-archivo: verifica a.fc b.fc
falcato prueba <file.fc>
falcato lsp
falcato instala --todo
```

## Plan R9.x — Integración nativa de Forge (`nuevo` / `limpia` / `vigila`)

Herramienta externa ofrecida (`forge`, Python) — la absorbemos como comandos nativos
en Rust, en español e integrados con el CLI actual. Sin dependencias nuevas
(polling con `std` en vez de `watchdog`).

### `falcato nuevo <nombre>` — scaffold de proyecto ejecutable
- Crea carpeta `<nombre>/`.
- **Reusa** `paquete inicia`: genera `falcato.toml` + `falcato.lock` (mismo formato del
  sistema de paquetes, `paquetes::Manifiesto` / `LockFile`).
- Crea `main.fc` con punto de entrada **correcto** de Falcato:
  `función principal() -> Entero32 { retornar 0; }`
  (corrige el bug del forge original que generaba `fn main()`, que Falcato no reconoce).
- Crea `build/` (directorio de salida que luego limpia `limpia`).
- Alias ocultos: `new`, `crear`.

### `falcato limpia [dir] [--binarios]` — borra artefactos
- Default `dir = "."`.
- Elimina `build/` (y lo recrea vacío) + `*.o` en el directorio (no recursivo).
- `--binarios` también elimina `*.exe` generados en el directorio.
- Alias ocultos: `clean`, `limpiar`.

### `falcato vigila [archivo]` — recompilación automática
- Si hay `archivo`, vigila su directorio; si no, busca `main.fc` en el actual.
- Compilación inicial vía `compilar(...)` (reusa el pipeline existente, sin duplicar).
- Polling cada 500 ms sobre `.fc` (ignora `.falcato-cache`, `.git`, `build`): si cambia
  el mtime, recompila.
- `Ctrl+C` sale. Sin nuevas dependencias (evita `notify` / `watchdog`).
- Alias ocultos: `watch`, `observar`.

**Integración:** los tres reusan infraestructura existente (`Manifiesto`, `compilar`,
`falcato.toml`). No tocan semántica de lenguaje ni etiquetas.
**Estado: 📋 plan** (pendiente de implementar en `main.rs`).

## Etiquetas del toolchain
**Regla:** etiqueta solo decide CÓMO se produce el binario, no qué significa.

| Etiqueta | Alias | Para qué | Estado |
|----------|-------|----------|--------|
| `--emitir-clif` | `emit-clif` | CLIF debug | ✅ |
| `--json` | — | Diagnósticos JSON | ✅ |
| `--incremental` | — | Cache verificación <100ms | ✅ |
| `--entrada` | `stdin` | `echo "code" \| verifica -` | ✅ |
| `--destino <triple>` | `target` | Cross-compile — **única etiqueta plataforma** | 🟡 R8 |
| `--lanzar` | `release` | Optimización global | 🔴 R8 |
| `-o, --salida` / `--detallado` / `-g` | `output`/`verbose` | Output/verbosidad | ✅ |

**Prohibidas:** N0/N1/N2, `--windows`, `--cfg(os)`, `--compat-*` (todo es lenguaje o `falcato.toml`)

## Roadmap — Pendiente real

### 🎯 R0.8.0 — La gran actualización (rediseño y mejora brutal del lenguaje)

**Meta:** 0.8.0 es la actualización que rediseña y mejora brutalmente el lenguaje. Agrupa los problemas de diseño abiertos más importantes y las lecciones de Go. Es el "gofmt + stdlib + rediseño" que lleva a Falcato de "Rust con artículos" a "lenguaje de plataforma para LLMs".

**Alcance (agrupa P-002, P-005, P-009, P-013, P-014, P-017 + lecciones de Go):**
- [x] **P-002 — Sintaxis namespace** (`::`): ✅ Parser soporta `mod::func` (resuelto 2026-08-28)
- [ ] **P-005 — Reducir builtins**: mover de Capa 1 (FFI) a Capa 2 (Falcato puro), sin romper compatibilidad (aliases en `std::compat`)
- [ ] **P-009 — Documentar artículos** `el`/`la`/`un`/`los`/`las`: tabla + sugerencias clippy
- [ ] **P-013 — LSP completitud**: indexar métodos por tipo, completion
- [ ] **P-014 — Warning de `Resultado` no manejado** (W-code)
- [ ] **P-017 — Regla unificadora de API**: coerción uniforme `Texto`/`Palabra`, errores en vez de silencio, fix por familia
- [ ] **`falcato formatea`** (gofmt): un solo estilo enforzado — prioridad máxima para LLMs
- [ ] **Stdlib HTTP + JSON**: desbloquea Cid (R9.0/R9.1)
- [ ] **Modernizers** (`falcato arregla`): migra patrones viejos determinísticamente — desbloquea P-007 (renombres sin miedo)
- [ ] **P-007 — Renombres** (con modernizers + aliases): `apodo`/`alias`, `rasgo`/`protocolo`, `retornar`/`devolver`
- [ ] **Promesa de compatibilidad formal**: "código de hoy compila en 10 años"

**Criterio de éxito:** Falcato pasa de "lenguaje con gramática innovadora" a "plataforma completa para LLMs" — formateador, stdlib, tooling integrado, compatibilidad.

### 📦 Estado actual de la libEst (2026-08-28)

**Diseño completo:** `docs/diseno_libest.md` (1361 líneas) — 12 dominios, 1300+ firmas.

**Archivos base creados (20 archivos, 10 módulos):**
```
libEst/
├── nucleo/          texto.fc, numeros.fc, opcion.fc
├── colecciones/     vector.fc, diccionario.fc, conjunto.fc
├── archivo/         archivo.fc
├── red/             tcp.fc, http.fc, json.fc
├── tiempo/          tiempo.fc
├── proceso/         proceso.fc
├── sistema/         sistema.fc
├── matematicas/     mate_fc (abs, maximo, minimo, raiz, potencia, seno, coseno, tangente, logaritmo, piso, techo, pi, e)
├── visual/          ventana.fc, color.fc, lienzo.fc, imagen.fc, sonido.fc
└── compat/          compat.fc (aliases builtins viejos)
```

**Compiler: namespace `::` funcional**
- Parser: `mod::func` → `Llamada { funcion: "modulo::funcion" }` ✅
- Semántico: resolución calificada contra `simbolos_publicos_importados` ✅
- Codegen: `aplanar_con_prefijo` registra `modulo::funcion` → `FuncId` ✅
- Lexer: `::` = dos `DosPuntos` consecutivos (no token dedicado) ✅

**Clasificación builtins vs Falcato:** `docs/libest_clasificacion.md` — 120 builtins Rust (83%), 25 Falcato puro (17%). Builtins: FFI, OS, memoria, rendimiento. Falcato: constructores, constantes, algoritmos simples.

**Pendiente:** crear builtins Rust faltantes (texto_contiene, http_get, json_parsear, etc.) — indispensables para Cid.

### R7.7 — Aritmética consciente (pendiente)

### R7.7 — Aritmética consciente (pendiente)
- [ ] **F3 — Rangos** (`entre ... y ...`): verificación compile-time + runtime checks
- [ ] **F4 — Unidades** (`por`/`entre`): dimensiones en tipos
- [ ] **F5 — Decimal nativo**: `0.1 + 0.2 = 0.3` exacto

### R9 — DAW + Cid
- [ ] **R9.1.0 — Librería JSON mínima** (`librerias/json.fc`) — parser para MCP
- [ ] **R9.2.0 — Cid core**: loop agente + contexto + MCP stdio
- [ ] **R9.3.0 — DAW**: WAV, buffers, mezcla, secuenciador, efectos

### R8 — Sistema de paquetes P2P
- Formato `falcato.toml` + `falcato.lock`, `falcato paquete add/publicar/buscar`
- Cliente torrent + DHT (BEP44), 8 capas de seguridad (hash, solo fuente, ed25519, permisos por tipos, WoT, blocklist, transparency log)
- **Pendientes seguridad R8S.1-7** (ver AGENTS previo): SET sin firma, DoS memoria/CPU, buffer slicing, `dht_consultar` len, `proceso_crear` inyección, auth peers — cerrar antes de exponer a red real

### Calidad y deuda
- [ ] Azure Trusted Signing (falso positivo Defender) + winget/Scoop
- [ ] `cargo fix` warnings (100 en release), reducir panics <2/1000 LOC

### 📋 Mejoras 0.7.6 (reporte 2026-08-21)
> **Bugs confirmados para 0.7.6 (no bloqueantes Cid, pero reporte pendiente):**
- **F-019** `archivo_listar("literal")` — coerción Palabra→Texto con descriptor fantasma en rutas largas. Fix: misma `strlen+malloc+memcpy` que otros builtins + free temporal.
- **F-020** `"{f(x)}"` interpolación con llamada → vacío silencioso. Fix: diagnóstico `[S080] interpolación con llamada no soportada, usa variable temporal` (implementación real en 0.8.0 si se pide).
- **F-021** `a > 0` infiere `Entero32` no `Booleano` → `T011`. Fix: `Binaria` con `> < >= <= == !=` siempre `Booleano` en `codegen/tipos.rs` y `semantic/tipos.rs`.

> **Sugerencias de lenguaje (defer a RFC 0.8.0, no a 0.7.6):**
> Renombres con breaking change → necesitan `MAYOR` + aliases. No tocar en 0.7.6.
> - `apodo` ↔ `alias` (RAE vs pureza), `rasgo` ↔ `protocolo`, `vector` ↔ `lista`, `Booleano`/`Lógico` unificar, `Entero` default 32→64 (rompe ABI), `retornar`→`devolver` canónico, `romper`→`salir`.
> - Mensajes: traducir `Borrow`→`préstamo`, `Verifier`→`[C]` español, `T001 disconcordancia`→`no coincide`.
> - Artículos `el/la/un` opacos → documentar tabla + sugerencias clippy.
> - Azúcares `texto_desde→de`, `como_entero64→.entero64` → esperar 3 proyectos reales.

### Gotchas operativos (no bugs)
- **Exe zombie:** `build.bat` falla pero `.exe` viejo sigue. Verificar siempre `target/release/falcato.exe` timestamp.
- **PowerShell .ps1 sin BOM → ANSI corrompe acentos:** usar `write` con UTF-8 + anclas ASCII, nunca `Set-Content` directo en `.fc`.
- **LSP cross-file:** `verifica a.fc b.fc` sí es cross-file, el LSP solo ve 1 archivo → ansiedad normal, no bug.

## Criterio de "listo para usar"
1. Proyecto >500 líneas multi-archivo compilable
2. stdlib suficiente (I/O, strings, colecciones)
3. Errores sin `retornar 1` manual
4. Borrow checker sin GC
5. Docs claras para hispanohablante
6. Registros hardware sin FFI manual (bitfields)

## Auditoría
Ver `ESTADO.md` para criterios McCabe/SonarQube/Sebesta/HumanEval. Semáforo 🟢/🟡/🔴.
