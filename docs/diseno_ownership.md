# Falcato — Diseño de Ownership y Borrow Checking
## "Concordancia de Posesión" — Superando a Rust con gramática española

> **Tesis central**: El borrow checker de Rust es *overly restrictive*, causa frustración
> desproporcionada, y rechaza programas perfectamente válidos. Falcato puede hacerlo mejor
> aprovechando: (1) tipos lineales/afines como base teórica, (2) gramática española como
> sintaxis de ownership, (3) análisis gradual que no pelea con el programador.

---

## 1. Problemas de Rust que resolvemos

| # | Problema de Rust | Solución Falcato |
|---|-----------------|------------------|
| 1 | Lifetimes crípticos (`'a`, `'b`, `'static`) | Lifetimes léxicos: nombres de variables |
| 2 | False positive: campos distintos del mismo struct | Field-level borrowing |
| 3 | No razona entre branches | Branch-aware analysis |
| 4 | Self-referential structs imposibles | Lifetimes anclados a `self` |
| 5 | Refactoring cascading | Borrow checker gradual (opt-in) |
| 6 | Workarounds vergonzosos (Arc, RefCell, índices) | Artículos extendidos (los/las = shared) |
| 7 | Learning curve brutal | Default permisivo + feedback educativo |
| 8 | No razona entre funciones | Anotaciones de efecto (`puro`, `muta`) |

---

## 2. Base teórica: Tipos Afines (no Lineales)

**Linear types** (Girard, 1987): cada recurso se usa *exactamente* una vez.
**Affine types** (relajación): cada recurso se usa *a lo más* una vez (permite drop sin usar).

Falcato usa **tipos afines** como base:
- Owned values (`el x`) son afines: pueden usarse 0 o 1 veces
- Borrowed values (`la x`) son no-lineales: pueden usarse N veces
- Shared values (`los x`) son reference-counted: uso libre

**Ventaja sobre Rust**: Rust implementa ownership como *reglas ad-hoc* sobre tipos normales.
Falcato lo implementa como *propiedad del sistema de tipos*, lo que permite:
- Inferencia más precisa
- Errores más claros
- Extensibilidad (nuevos artículos = nuevos modos de ownership)

---

## 3. Sintaxis: Artículos como Ownership

### 3.1 Artículos existentes (Fase 1-3)

```falcato
el x: Entero32 = 5;       // owned, mutable (affine: usar 0 o 1 veces)
la x: Entero32 = 5;       // borrowed, inmutable (no-lineal: usar N veces)
un x: Entero32 = 5;       // optional (puede ser nulo)
```

### 3.2 Artículos nuevos (Fase 12)

```falcato
los x: Texto = ...;       // shared ownership (reference-counted, como Arc)
las x: &Texto = ...;      // shared borrowed (referencia a shared)
```

### 3.3 Referencias explícitas

```falcato
el dato: Texto = texto_nuevo();
la ref: &dato Texto = &dato;    // Borrow inmutable, lifetime = scope de 'dato'
el ref_mut: &mut dato Texto = &mut dato;  // Borrow mutable, exclusivo
```

**Innovación clave**: `&dato` no es `&'a T` de Rust. Es `&nombre_variable T`,
donde el lifetime se infiere del scope de la variable nombrada.

### 3.4 Transferencia de ownership explícita

```falcato
el x = crear_dato();
mover x a procesar;     // Transferencia explícita (no implícita como Rust)
// x ya no es válido

el y = crear_dato();
copiar y;               // Clone explícito
procesar(y);            // y sigue siendo válido

la z = &x;
prestar z a leer;       // Borrow explícito (x sigue siendo owner)
```

**Ventaja sobre Rust**: los moves son *explícitos*, no implícitos.
El programador siempre sabe cuándo pierde ownership.

---

## 4. Lifetimes léxicos (Innovación principal)

### 4.1 Problema de Rust

```rust
// Rust: ¿qué significa 'a?
fn longest<'a>(x: &'a str, y: &'a str) -> &'a str { ... }
```

### 4.2 Solución Falcato

```falcato
// Falcato: el lifetime ES el nombre de la variable
función más_larga(la x: &x Texto, la y: &y Texto) -> &x Texto {
    // '&x' = "referencia que vive mientras 'x' exista"
    // El retorno vive mientras 'x' viva
    si texto_longitud(x) > texto_longitud(y) {
        retornar x;
    }
    retornar y;  // ERROR: 'y' puede vivir menos que 'x'
}
```

### 4.3 Reglas de inferencia

1. **Sin anotación**: lifetime = scope léxico de la variable
2. **Con `&nombre`**: lifetime = scope de `nombre`
3. **Con `&yo`**: lifetime = scope del struct (para self-referential)
4. **Con `&'estático`**: lifetime = duración del programa (como `'static`)

### 4.4 Elisión (como Rust pero más intuitivo)

```falcato
// Estas dos son equivalentes:
función primero(la x: &x Texto) -> &x Texto { retornar x; }
función primero(la x: &Texto) -> &Texto { retornar x; }
// Sin anotación: lifetime se infiere del scope
```

---

## 5. Borrow Checker Gradual (Innovación #2)

### 5.1 Filosofía

> "No peleamos con el programador. Le damos seguridad cuando la pide."

### 5.2 Niveles de verificación

```falcato
// NIVEL 0 (default): Sin verificación de ownership
// Como C: rápido, flexible, pero inseguro
función rápido() {
    el x = crear_dato();
    procesar(x);
    usar(x);  // Posible use-after-move, pero no se verifica
}

// NIVEL 1 (opt-in por función): Verificación de moves
función seguro verificado() {
    el x = crear_dato();
    procesar(x);  // Move explícito
    usar(x);  // ERROR [O001]: 'x' fue movido a 'procesar'
}

// NIVEL 2 (opt-in por módulo): Verificación completa
módulo núcleo verificado {
    // Todas las funciones en este módulo tienen borrow checking completo
    // Incluyendo: moves, borrows, lifetimes, field-level
}
```

### 5.3 Keywords de verificación

| Keyword | Nivel | Verifica |
|---------|-------|----------|
| (ninguno) | 0 | Nada (como C) |
| `verificado` | 1 | Moves + use-after-move |
| `estricto` | 2 | Moves + borrows + lifetimes |
| `inseguro` | -1 | Desactiva verificación en bloque |

### 5.4 Ventaja para IA

Los LLMs pueden generar código en **Nivel 0** (siempre compila),
y luego el compilador sugiere elevar a Nivel 1/2 con fixes específicos.

```
[W001] ejemplo.fc:5:1: Función 'procesar' no tiene verificación de ownership
       │ sugerencia: Agrega 'verificado' para detectar use-after-move
       │            función procesar verificado() { ... }
```

---

## 6. Field-Level Borrowing (Innovación #3)

### 6.1 Problema de Rust

```rust
// Rust RECHAZA esto (false positive):
let x_ref = point.x_mut();
let y_ref = point.y_mut();  // ERROR: two mutable borrows
```

### 6.2 Solución Falcato

```falcato
// Falcato ACEPTA esto (field-level analysis):
estructural Punto { x: Flotante64, y: Flotante64 }

función principal() verificado {
    el punto = Punto { x: 1.0, y: 2.0 };
    el ref_x: &mut punto.x Flotante64 = &mut punto.x;
    el ref_y: &mut punto.y Flotante64 = &mut punto.y;
    // OK: campos distintos, no hay aliasing
    ref_x = ref_x * 2.0;
    ref_y = ref_y * 2.0;
}
```

### 6.3 Implementación

- El borrow checker trackea **paths de acceso** (no solo variables)
- `punto.x` y `punto.y` son paths distintos → no hay conflicto
- `punto.x` y `punto` sí conflictúan (borrow parcial vs total)

---

## 7. Branch-Aware Analysis (Innovación #4)

### 7.1 Problema de Rust

```rust
// Rust RECHAZA esto (no razona entre branches):
match map.get_mut(&key) {
    Some(value) => value,
    None => {
        map.insert(key, V::default());  // ERROR: second mutable borrow
        map.get_mut(&key).unwrap()
    }
}
```

### 7.2 Solución Falcato

```falcato
// Falcato ACEPTA esto (branch-aware):
función obtener_o_crear(el mapa: &mut mapa Diccionario, la clave: Texto) verificado {
    coincidir mapa.obtener_mut(clave) {
        Alguno(valor) => retornar valor,
        Ninguno => {
            // El borrow de 'valor' murió en la otra rama
            mapa.insertar(clave, valor_por_defecto());
            retornar mapa.obtener_mut(clave).desenvolver();
        }
    }
}
```

### 7.3 Implementación

- Análisis de **liveness por branch** (como NLL de Rust, pero más preciso)
- Un borrow muere al final de su branch si no escapa
- Integración con el CFG de Cranelift (ya tenemos bloques)

---

## 8. Self-Referential Structs (Innovación #5)

### 8.1 Problema de Rust

```rust
// Rust NO PUEDE expresar esto:
struct Node {
    value: i32,
    next: &Node,  // ERROR: missing lifetime specifier
    // ¿Cuánto vive la referencia? ¿Quién es el owner?
}
```

### 8.2 Solución Falcato

```falcato
// Falcato SÍ PUEDE (lifetime anclado a self):
estructural Nodo {
    valor: Entero32,
    siguiente: &yo Nodo,  // '&yo' = "vive mientras este struct viva"
}

función principal() verificado {
    región lista {
        el nodo2 = Nodo { valor: 2, siguiente: nulo };
        el nodo1 = Nodo { valor: 1, siguiente: &nodo2 };
        // OK: nodo2 vive en la misma región que nodo1
    }
    // Ambos se liberan juntos al final de la región
}
```

### 8.3 Regiones de memoria

```falcato
región transacción {
    el cliente = obtener_cliente();
    el pedido = crear_pedido(&cliente);
    el pago = procesar_pago(&pedido);
    // Todas las variables se liberan juntas (LIFO)
    // No hay leaks posibles: la región garantiza cleanup
}
```

**Ventaja para kernels**: regiones = arena allocation determinístico.
Sin malloc/free individual, sin GC, sin fragmentation.

---

## 9. Anotaciones de Efecto (Innovación #6)

### 9.1 Problema de Rust

```rust
// Rust no puede ver que increment_counter() no muta self.items:
fn count_items(&mut self) {
    for _ in &self.items {
        self.increment_counter();  // ERROR: can't borrow self mutably
    }
}
```

### 9.2 Solución Falcato

```falcato
estructural Colección {
    contador: Entero32,
    elementos: Vector<Entero32>
}

// 'puro' = no muta nada fuera de su scope
función incrementar_contador(el self: &mut self Colección) puro {
    self.contador = self.contador + 1;
}

función contar_elementos(el self: &mut self Colección) verificado {
    para _ en self.elementos {
        self.incrementar_contador();
        // OK: 'puro' garantiza que no muta 'elementos'
    }
}
```

### 9.3 Keywords de efecto

| Keyword | Significado |
|---------|-------------|
| `puro` | No muta nada fuera de su scope (como `const` en C++) |
| `muta(campo)` | Solo muta el campo especificado |
| `lee(campo)` | Solo lee el campo especificado |
| (ninguno) | Puede mutar cualquier cosa (conservador) |

---

## 10. Feedback Educativo (Innovación #7)

### 10.1 Formato de errores de ownership

```
[O001] ejemplo.fc:10:5: 'x' fue movido a 'procesar' en línea 8
       │
       │  8 │     procesar(x);      ← x se mueve aquí
       │    │     ...
       │ 10 │     usar(x);          ← ERROR: x ya no es válido
       │
       │ sugerencia: Si necesitas usar 'x' después:
       │   opción A: copiar x antes de pasar
       │     8 │     procesar(copiar x);
       │   opción B: pasar por referencia
       │     8 │     procesar(&x);
       │   opción C: reordenar para usar x antes del move
       │     7 │     usar(x);
       │     8 │     procesar(x);
```

### 10.2 Diagnósticos progresivos

- **Nivel 0**: solo warnings (no errores)
- **Nivel 1**: errores de moves
- **Nivel 2**: errores de moves + borrows + lifetimes
- Cada error incluye **múltiples opciones de fix**

---

## 11. Para Kernels y Sistemas

### 11.1 Sin heap obligatorio

```falcato
// Stack-only: sin malloc, sin free, sin GC
función kernel_init() verificado {
    el buffer: [Entero8; 4096] = todos 0;  // Stack allocation
    // 'buffer' se libera automáticamente al salir del scope
}
```

### 11.2 Regiones como arena allocation

```falcato
// Arena: allocation en bloque, liberación en bloque
región frame {
    el entidades: [Entidad; 1024] = todos Entidad::vacío();
    el render_data = preparar_render(&entidades);
    // Todo se libera al final del frame (determinístico)
}
```

### 11.3 `inseguro` para hardware

```falcato
inseguro función escribir_registro(el puerto: Natural16, el valor: Natural8) {
    // Sin verificación de ownership (acceso directo a hardware)
    // El programador asume responsabilidad
    asm("out dx, al");
}
```

### 11.4 `pre-` para comptime (Pilar V)

```falcato
// Evaluación en tiempo de compilación
pre-función tamaño_buffer() -> Natural32 {
    retornar 4096 * 16;  // Calculado en compilación
}

el buffer: [Entero8; tamaño_buffer()] = todos 0;
```

---

## 12. Para IA (Toolchain de código generado)

### 12.1 Verificación progresiva

```
LLM genera código → Nivel 0 (siempre compila)
                  → Compiler sugiere: "agrega 'verificado'"
                  → LLM refina → Nivel 1 (moves verificados)
                  → Compiler sugiere: "agrega 'estricto'"
                  → LLM refina → Nivel 2 (borrow checking completo)
```

### 12.2 Errores accionables para LLMs

Cada error tiene:
- **Código único** (parseable): `[O001]`
- **Span exacto**: línea:columna
- **Sugerencia concreta**: código de fix
- **Múltiples opciones**: A, B, C

### 12.3 WASM sandbox

```
Código generado por IA → Compilar a WASM → Ejecutar en sandbox
                                            → Si viola ownership: trap
                                            → Si pasa: seguro
```

---

## 13. Plan de implementación (Fases)

### Fase 12A: Ownership básico (Moves + Drop)
- Tracking de moves en semántico
- Use-after-move detection (Nivel 1: `verificado`)
- Drop automático al salir de scope (codegen: insertar `free`)
- Keyword `mover`, `copiar`
- **Duración estimada**: 2-3 semanas

### Fase 12B: Referencias y Borrowing
- `&variable` y `&mut variable` en parser/AST
- Borrow checker básico (Nivel 2: `estricto`)
- Reglas de exclusividad (1 mutable XOR N inmutables)
- Field-level borrowing
- **Duración estimada**: 3-4 semanas

### Fase 12C: Lifetimes léxicos
- `&nombre_variable Tipo` en parser
- Inferencia de lifetimes desde scopes
- Verificación de consistencia (retorno no vive más que parámetro)
- Elisión (sin anotación = inferido)
- **Duración estimada**: 2-3 semanas

### Fase 12D: Regiones y Self-referential
- Keyword `región` en parser
- Arena allocation en codegen
- `&yo` para self-referential structs
- Drop checker (orden LIFO)
- **Duración estimada**: 3-4 semanas

### Fase 12E: Efectos y Branch-aware
- Keywords `puro`, `muta(campo)`, `lee(campo)`
- Branch-aware liveness analysis
- Integración con CFG de Cranelift
- **Duración estimada**: 3-4 semanas

### Fase 12F: Artículos extendidos + Feedback
- `los`/`las` (shared ownership, reference-counted)
- Feedback educativo en errores
- Diagnósticos progresivos
- **Duración estimada**: 2 semanas

---

## 14. Comparación final: Falcato vs Rust

| Aspecto | Rust | Falcato |
|---------|------|---------|
| Sintaxis de ownership | `&`, `&mut`, `Box`, `Arc`, `Rc` | `el`, `la`, `los`, `las`, `un` |
| Lifetimes | `'a`, `'b`, `'static` | `&nombre_variable`, `&yo`, `&'estático` |
| Default | Verificación completa (frustrante) | Permisivo (opt-in a seguridad) |
| Moves | Implícitos (sorpresas) | Explícitos (`mover x a f`) |
| Self-referential | Imposible (workarounds) | `&yo` + regiones |
| Field borrowing | No (false positives) | Sí (path-based) |
| Branch analysis | Limitado (NLL) | Completo (CFG-aware) |
| Efectos | No | `puro`, `muta(campo)` |
| Kernels | Posible pero verboso | Regiones + stack-only + comptime |
| IA | No diseñado para ello | Gradual + errores accionables + WASM |
| Curva de aprendizaje | Brutal | Gradual (Nivel 0 → 2) |

---

## 15. Riesgos y mitigaciones

| Riesgo | Mitigación |
|--------|-----------|
| Complejidad de implementación | Fases pequeñas, cada una funcional |
| Performance del análisis | Cranelift es rápido; análisis incremental |
| Compatibilidad con C ABI | `inseguro` blocks, layout C por defecto |
| Adopción | Nivel 0 = siempre compila; migración gradual |
| Comparación con Rust | No competimos en features, sino en ergonomía |

---

## 16. Criterio de éxito

Falcato supera a Rust cuando:
1. Un programador hispanohablante escribe un linked list **sin fight con el compiler**
2. Un LLM genera código que compila en Nivel 0 y pasa a Nivel 2 con <3 iteraciones
3. Un kernel module se escribe en Falcato con **menos líneas** que en Rust equivalente
4. Los errores de ownership se entienden **sin leer documentación**
