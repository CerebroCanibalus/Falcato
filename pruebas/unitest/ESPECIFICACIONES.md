# ESPECIFICACIONES — Unitest R7.6

Registro de decisiones de spec por caso. Cada caso termina en una decisión
escrita, nunca en "así es la vida". Clasificación:
- 🟢 **Confirmado** — correcto por diseño, documentado como spec
- 🔴 **Bug** — se arregla el compiler
- 🟡 **Por especificar** — requiere investigación y documentación a fondo;
  cada caso se abre como tarea y termina en decisión escrita

Fecha de la pasada de descubrimiento: 2026-08-11 (compiler v0.6.1 debug)

---

## 1. Mutabilidad y ownership

| Caso | Resultado | Clasificación | Decisión |
|------|-----------|---------------|----------|
| Use-after-move en N0 | Compila y ejecuta (imprime 5) | 🟢 | N0 permisivo: "todo compila, sugiere". Test: `unitest_compilan/mover_en_n0.fc` |
| Use-after-move en N2 | `[O001]` con sugerencia A/B/C | 🟢 | N2 estricto: detectado con mensaje educativo. Test: `unitest_negativos/mover_en_n2.fc` |
| Mutar `la` (inmutable) | `[O001]` **en TODOS los niveles** | 🟢 | El artículo es semántica estática (Pilar I), no regla de nivel. `la` = inmutable siempre. Test: `unitest_negativos/mutar_la_n0.fc` |
| Borrow mut + inmut simultáneo | `[O002]` (no O001) | 🟢 | Código propio para borrow conflict. Test: `unitest_negativos/borrow_mut_inmut.fc` |
| Múltiples borrows inmutables | Compila | 🟢 | 1 mut XOR N inmut. Test: `unitest_ownership.fc` |
| Escritura vía `*ref_mut = x` | `[S003]` — NO soportada | 🟡 | `&mut` solo lectura/paso. ¿Feature futura o limitación documentada? **TAREA**: decidir si N2 debe soportar escritura por dereferencia |
| Dereferencia en comparación | `(*ref) == 42` requiere paréntesis | 🟢 | `*ref == 42` se parsea como `*(ref == 42)` → T030. Documentar en GUIA.md |

## 2. Edge cases numéricos

| Caso | Resultado | Clasificación | Decisión |
|------|-----------|---------------|----------|
| Overflow Entero32 (MAX+1) | Wrap silencioso: `-2147483648` | 🟢 | **Wrap módulo 2ⁿ, estilo Go** — definido y documentado. Test: `unitest_numeros.fc` |
| Overflow Natural32 (MAX+1) | Wrap: `0` | 🟢 | Idem. Test: `unitest_numeros.fc` |
| Literales límite (MAX/MIN) | Compilan e imprimen bien | 🟢 | `2147483647` / `-2147483648` válidos. Test: `unitest_numeros.fc` |
| Cast `como_entero32(2^32+1)` | Trunca: `1` (mod 2^32) | 🟢 | Conversión trunca módulo 2ⁿ. Test: `unitest_numeros.fc` |
| Aritmética mixta E32+E64 | `[T005]` — NO compila | 🟢 | **Exige operandos del mismo tipo** (no promoción automática). AGENTS.md decía "operando mayor manda" — INCORRECTO: fecha.fc usa `como_entero32()` explícito. Test: `unitest_negativos/aritmetica_mixta.fc` |
| División por cero | Crash `0xC0000095` (INTEGER_DIVIDE_BY_ZERO) | 🟡 | UB estilo C hoy. **TAREA**: ¿crash controlado con mensaje en N2 (Zig-style) o UB documentado? |
| División por cero flotante | No probado | 🟡 | Pendiente de probar (IEEE 754 → inf, probablemente OK) |

## 3. Texto y colecciones

| Caso | Resultado | Clasificación | Decisión |
|------|-----------|---------------|----------|
| `texto_nuevo()` vacío | Imprime vacío (no "(null)") | 🟢 | Fix 2026-08-08 verificado. Test: `unitest_texto.fc` |
| Escapes `\n \t \\` | Funcionan | 🟢 | Test: `unitest_texto.fc` |
| Concatenación `a + b` | Funciona (len correcto) | 🟢 | Test: `unitest_texto.fc` |
| `vector_agregar` + realloc (100 items) | Funciona | 🟢 | Fix cap=1 verificado. Test: `unitest_vector.fc` |
| `vector_obtener` fuera de rango | **UB INESTABLE** — corridas alternas 1/0, lee memoria basura | 🔴→🟢 | **FIX 2026-08-11**: bounds check → devuelve 0 definido (5/5 corridas estables). Test: `unitest_vector.fc` |
| `Diccionario` con tipos simples | **PANIC del compiler**: `block2 is not sealed` (exit 101) | 🔴→🟢 | **FIX 2026-08-11**: (a) `body_block` sin sellar en `compilar_buscar_en_diccionario`; (b) SSA dominance: `stride_i64`/`val_offset_val` definidos en found_block pero usados en not_found_block → movidos al bloque dominante; (c) `diccionario_nuevo` con cap=0 → `hash % 0` crash → cap inicial 16 buckets + resize realloc 2×; (d) **internado de strings**: dos literales "clave" eran punteros distintos → comparación por puntero fallaba → cache `strings_internados`. Tests: `unitest_diccionario.fc` (básico, Palabra, resize 30, overwrite) |
| Doble free | Crash `0xC0000005` (ACCESS_VIOLATION) | 🟡 | Sin detección. **TAREA**: ¿detección en N2 o UB documentado? (R6 drop automático podría ayudar) |

## 4. Toolchain (contrato de tests)

| Caso | Resultado | Clasificación | Decisión |
|------|-----------|---------------|----------|
| `verifica --json` con error | `{"ok":false,...}` pero **exit 0** | 🔴→🟢 | **FIX 2026-08-11**: retorna `Err(JSON_YA_IMPRESO)` (marcador) → main hace exit(1) sin duplicar el mensaje. Contrato: JSON limpio + exit 1 con error, exit 0 sin error |
| `verifica` sin `--json` con error | exit 1 | 🟢 | Contrato OK para Fase 3 |
| Sintaxis `función estricto principal()` | `[S004]` — INCORRECTA | 🟢 | Formato correcto: `función principal() -> T estricto` (como `borrow_ok.fc`). Documentar en GUIA.md |
| Turbofish en colecciones | `vector_nuevo<Entero32>()` obligatorio | 🟢 | Sin inferencia en builtins genéricos. Documentar en GUIA.md |
| `falcato prueba -` (stdin) | No soportado | 🟢 | Solo archivos. OK |

## 5. Bugs arreglados (🔴→🟢) — 2026-08-11

1. **`verifica --json` exit code** — ahora exit 1 con `ok:false` (marcador `JSON_YA_IMPRESO`)
2. **Diccionario panic** — 4 causas raíz: body_block sin sellar, SSA dominance en insertar, cap=0 → div por cero, literales sin internar. Todo arreglado + resize automático
3. **`vector_obtener` sin bounds check** — ahora devuelve 0 definido (estable 5/5)

## 6. Tareas abiertas (🟡)

- [ ] Escritura vía `*ref_mut` — ¿soportar en N2 o documentar limitación?
- [ ] División por cero — ¿crash controlado (Zig) o UB documentado (C)?
- [ ] Doble free — ¿detección en N2 o UB documentado?
- [ ] División por cero flotante — probar IEEE 754