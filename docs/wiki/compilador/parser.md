# Parser

Si el lexer trocea, el parser ordena y da sentido. Es quien
construye el Árbol de Sintaxis Abstracta (AST) a partir del
río de tokens que el lexer le entrega.

## Tecnología

Parser descendente manual, repartido en cinco archivos que son
como los capítulos de una novela:

| Archivo | Propósito |
|---------|-----------|
| `src/parser/mod.rs` | `ParserCursor`, `ParserFalcato::parse()`, y los tests |
| `src/parser/errores.rs` | `ErrorSintaxis` con códigos [S###] |
| `src/parser/tipos.rs` | `parse_articulo()`, `parse_tipo()` |
| `src/parser/expresiones.rs` | Pratt parser + postfix |
| `src/parser/sentencias.rs` | Variables, asignación, condicionales, bucles |
| `src/parser/declaraciones.rs` | Funciones, structs, enums, genéricos |

## ParserCursor

Es el ojo que mira hacia adelante —un wrapping del stream de tokens
con capacidad de asomarse sin consumir:

```rust
pub struct ParserCursor {
    tokens: Vec<TokenConSpan>,
    posicion: usize,
    pub genericos: Vec<String>,  // type params activos
}
```

Métodos clave:
- `actual()` / `peek(offset)` — mira sin tocar (lookahead)
- `esperar(token)` — consume o alza la voz (error)
- `sincronizar(tokens)` — se recupera de un error saltando hasta
  la siguiente declaración (como quien cambia de tema en una
  conversación incómoda)
- `span_desde(inicio)` — fabrica un span combinado desde donde
  se guardó la posición

## Pratt Parser (Expresiones)

Las expresiones se analizan con un Pratt parser, que es un método
elegante para manejar precedencia de operadores sin volverse loco:

| Nivel | Operadores | Asociatividad |
|-------|------------|---------------|
| 1 | `\|\|` | Izquierda |
| 2 | `&&` | Izquierda |
| 3 | `==`, `!=` | Izquierda |
| 4 | `<`, `>`, `<=`, `>=` | Izquierda |
| 5 | `+`, `-` | Izquierda |
| 6 | `*`, `/`, `%` | Izquierda |
| 7 | `-` unario, `!` | Derecha (unario) |

Postfix (aún mayor precedencia):
- `expr[índice]` — acceso a array
- `expr.campo` — acceso a campo de struct

## Recovery de errores

Cuando el parser encuentra algo inesperado, no se rinde. Hace esto:

1. Reporta el error con su span y una sugerencia amigable
2. Sincroniza: salta tokens hasta dar con una keyword de declaración
   (`función`, `inseguro`, `estructural`, `enumeración`, `módulo`, `usar`)
3. Continúa desde ahí como si nada

Así se evita que un solo carácter maldito provoque una cascada
de errores sin sentido.

## Spans reales

Cada nodo del AST conoce su origen. Las expresiones binarias
combinan los spans de sus operandos, cubriendo desde el primer
símbolo hasta el último:

```rust
Expresion::Binaria(izq, op, der, span)
// span = combinar(izq.span(), der.span())
```

## Tests

13 tests vigilan al parser en `src/parser/mod.rs`, cubriendo
funciones, expresiones, condicionales, bucles, FFI, asignación,
enums, const generics y bounds.
