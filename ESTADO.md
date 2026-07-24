# Falcato - Estado Actual del Proyecto

## Fecha: Junio 2026
## Fase: FASE 2 (Semántica Básica) - COMPLETADA

---

## Pipeline Funcional End-to-End

```
archivo.fc → Lexer → Parser → Análisis Semántico → Codegen (Cranelift) → .o → Linker → .exe
```

---

## Componentes Implementados

### 1. Lexer (`src/lexer.rs`)
- **Librería**: `logos` 0.14
- **Tokens soportados**:
  - Keywords: `función`, `retornar`, `si`, `sino`, `mientras`, `para`, `en`, `inseguro`, etc.
  - Artículos (ownership): `el`, `la`, `un`, `los`, `las`
  - Tipos: `Entero8/16/32/64`, `Natural8/16/32/64`, `Flotante32/64`, `Booleano`, `Caracter`, `Palabra`, `Vacío`
  - Literales: enteros, flotantes, strings, caracteres, booleanos
  - Operadores: `+`, `-`, `*`, `/`, `%`, `==`, `!=`, `<`, `>`, `<=`, `>=`, `&&`, `||`
  - Símbolos: `()`, `{}`, `[]`, `,`, `;`, `:`, `=`, `->`
- **Features**:
  - Skip de whitespace y comentarios `//`
  - Span en cada token (Posición: línea, columna, offset)
  - Soporte para caracteres Unicode (ñ, tildes)

### 2. Parser (`src/parser.rs`)
- **Tipo**: Descendente manual (reemplazó chumsky por tipos opacos)
- **Precedencia de operadores** (Pratt parser):
  ```
  1: ||
  2: &&
  3: ==, !=
  4: <, >, <=, >=
  5: +, -
  6: *, /, %
  7: unario: -, !
  ```
- **Estructuras parseadas**:
  - Declaraciones de función (con/sin `inseguro`)
  - Parámetros con artículos
  - Tipos de retorno (`-> Tipo`)
  - Declaraciones de variables (`el/la/un nombre: Tipo = valor`)
  - Expresiones: literales, identificadores, llamadas, binarias, unarias, parentizadas
  - Sentencias: expresión, declaración variable, retorno

### 3. AST (`src/ast.rs`)
- **Span en cada nodo** (no negociable desde Day-0)
- **Nodos**:
  - `Programa`, `Declaracion` (Funcion, etc.)
  - `FuncionDecl`: nombre, params, retorno, cuerpo, es_insegura
  - `Parametro`: articulo, nombre, tipo
  - `Bloque`: lista de sentencias
  - `Sentencia`: Expresion, DeclaracionVariable, Retornar
  - `Expresion`: Literal, Identificador, Llamada, Binaria, Unaria
  - `Literal`: Entero, Flotante, Palabra, Caracter, Booleano
  - `OperadorBinario`: Suma, Resta, Mul, Div, Mod, Igual, Distinto, Menor, Mayor, MenorIgual, MayorIgual, Y, O
  - `OperadorUnario`: Negacion, NegacionLogica, Referencia, Desreferencia
  - `Tipo`: Entero8-64, Natural8-64, Flotante32/64, Booleano, Caracter, Palabra, Vacio, Puntero, Referencia, Nombre
  - `Articulo`: El, La, Un, Los, Las

### 4. Análisis Semántico (`src/semantic.rs`)

#### Innovación: Concordancia Lingüística
Aprovechamos que en español los adjetivos **concuerdan** en género/número con el sustantivo. En Falcato, los valores deben "concordar" en tipo, ownership y estado.

#### Verificaciones implementadas:
- **Disconcordancia de tipo** [T001]: Variable declarada con tipo A pero valor es tipo B
  ```
  [T001] test.fc:4:8: Disconcordancia de tipo: 'a' es 'Entero32' pero se declaró como 'Booleano'
         │ sugerencia: Cambia el tipo a 'Entero32' o el valor
  ```
- **Disconcordancia en retorno** [T002]: Función declara retorno A pero retorna B
- **Retorno faltante** [T003]: Función con retorno declarado pero sentencia `retornar` sin valor
- **Identificador no declarado** [T004]: Variable usada sin declarar previa
  ```
  [T004] test.fc:5:12: Identificador 'x' no declarado. ¿Olvidaste declararlo con 'el', 'la' o 'un'?
  ```
- **Operación inválida** [T006-T010]: Aritmética en no-numéricos, comparación en no-comparables, lógica en no-booleanos

#### Características:
- Entornos anidados (scope) con padre
- Inferencia de tipos de literales
- Verificación de tipos en operaciones binarias y unarias
- Parámetros registrados en entorno de función

### 5. Codegen (`src/codegen.rs`)
- **Backend**: Cranelift 0.112 (puro Rust, sin dependencias del sistema)
- **ABI**: C por defecto (no negociable)
- **Generación**:
  - Firma de funciones con parámetros y retornos
  - Stack slots para variables locales
  - Parámetros almacenados en stack
  - Operaciones: iadd, isub, imul, sdiv, srem, icmp, band, bor, bxor
  - Negación: isub(0, val) y bxor(val, 1)
  - Retorno de valores
  - Strings como data globals (punteros)
  - Llamadas a funciones (incluyendo FFI)

### 6. CLI (`src/main.rs`)
- **Comandos**:
  - `falcato build <archivo>`: Compila a binario
  - `falcato run <archivo>`: Compila y ejecuta
  - `falcato check <archivo>`: Solo análisis (lexer + parser + semántica)
  - `falcato version`: Muestra versión
- **Linker automático**: Busca `link.exe` de Visual Studio en ubicaciones comunes
- **Flags**: `--output`, `--target`, `--release`

### 7. Sistema de Errores (`src/error.rs`)
- Categorías: Sintaxis (S), Tipo (T), Ownership (O), FFI (C), Módulos (M), Interno (I), Warning (W)
- Formato: `[Código] archivo:linea:columna: mensaje`
- Sugerencias opcionales

---

## Ejemplos Funcionales

### 1. Hola Mundo (retorna 42)
```falcato
función principal() -> Entero32 {
    retornar 42;
}
```

### 2. Operaciones Aritméticas
```falcato
función principal() -> Entero32 {
    el a: Entero32 = 10;
    el b: Entero32 = 20;
    el c: Entero32 = a + b * 2;  // Precedencia: 50
    retornar c;
}
```

### 3. Condicional
```falcato
función principal() -> Entero32 {
    el x: Entero32 = 10;
    si x > 5 {
        retornar 100;
    } sino {
        retornar 0;
    }
}
```

### 4. Error Semántico (tipo mismatch)
```falcato
función principal() -> Entero32 {
    el a: Booleano = 10;  // [T001] Disconcordancia de tipo
    retornar 0;
}
```

---

## Tests
- **Total**: 11 tests
- **Lexer**: 4 tests (hola_mundo, string, aritmetica, articulos)
- **Parser**: 3 tests (funcion_simple, expresion_aritmetica, ffi_puts)
- **Semántica**: 4 tests (correcta, tipo_mismatch, variable_no_declarada, retorno_incorrecto)

---

## Innovaciones Documentadas (Pendientes de Implementar)

### 1. Condicionales Modales (FASE 3-4)
Comentado en `src/ast.rs` y `src/parser.rs`:
- **Indicativo** (`si`): Branch normal (hot path)
- **Subjuntivo** (`si fuese`): Branch improbable (cold path, `[[unlikely]]`)
- **Imperativo** (`sea`): Assertion/contract (`assume`)
- **Ser/Estar** en condiciones: identidad vs estado

### 2. Concordancia Lingüística (FASE 2 - IMPLEMENTADA)
En `src/semantic.rs`:
- Errores de tipo como "disconcordancia"
- Sugerencias que usan artículos (`el`, `la`, `un`)
- Mensajes intuitivos para hispanohablantes

---

## Stack Técnico
- **Lexer**: logos 0.14
- **Parser**: Manual descendente
- **AST**: Propio con Span
- **Semántica**: Propio (concordancia lingüística)
- **Codegen**: Cranelift 0.112
- **CLI**: clap 4.5
- **Testing**: Tests unitarios integrados

## Dependencias Cargo
```toml
[dependencies]
clap = { version = "4.5", features = ["derive"] }
logos = "0.14"
cranelift-codegen = "0.112"
cranelift-frontend = "0.112"
cranelift-module = "0.112"
cranelift-object = "0.112"
cranelift-native = "0.112"
target-lexicon = "0.12"
```

---

## Componentes Implementados (Actualizado)

### 8. Condicionales (`si` / `sino`)
- **Parser**: Estructura `si condicion { bloque } [sino { bloque }]`
- **AST**: `Sentencia::Condicional` con condición, bloque_entonces, bloque_sino
- **Semántica**: Verifica que la condición sea `Booleano` [T011]
- **Codegen**: Generación de bloques Cranelift con `brif`, manejo de terminación (retorno vs jump)

#### Ejemplo funcional:
```falcato
función principal() -> Entero32 {
    el x: Entero32 = 10;
    si x > 5 {
        retornar 100;
    } sino {
        retornar 0;
    }
}
```

## Próximos Pasos (FASE 3)
1. **Bucles** `mientras` / `para`
2. **Ownership** con artículos (género = owned/borrowed)
3. **Funciones con parámetros** (verificación de tipos en llamadas)
4. **Condicionales Modales** (subjuntivo como optimización - documentado)

---

## Notas Técnicas
- **Windows**: Requiere `$env:LIB` apuntando a librerías de Visual Studio para linking
- **C ABI**: Layout C por defecto, calling convention C, mangling desactivado
- **Span**: Cada nodo AST tiene Span (inicio, fin, archivo) para errores con ubicación
- **Errores**: En español con códigos alfanuméricos (T001, S042, etc.)
