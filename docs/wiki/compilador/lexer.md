# Lexer

El lexer es el primer centinela. Toma el texto fuente y lo
trocea en tokens —las palabras mínimas que el compilador entiende.

## Tecnología

Usamos `logos` 0.14, que es una biblioteca que genera analizadores
léxicos type-safe. Separa el blanco (whitespace) y los comentarios
de forma automática, sin que tengamos que sudar.

**Archivo:** `src/lexer.rs`

## Estructura

### Token (enum logos)

```rust
#[derive(Logos)]
pub enum Token {
    // Keywords
    #[token("función")]
    Funcion,
    // ... (véase src/lexer.rs para la lista cumplida)

    // Artículos (ownership)
    #[token("el")] ArticuloEl,
    #[token("la")] ArticuloLa,

    // Tipos primitivos
    #[token("Entero32")] Entero32,

    // Literales
    #[regex(r"[0-9]+")]
    EnteroLiteral(Option<i64>),

    // Operadores, símbolos, identificadores...
}
```

### TokenConSpan

Cada token lleva su ubicación, que es como las señas de una casa:

```rust
pub struct TokenConSpan {
    pub token: Token,
    pub span: Span,
}
```

## Características

- **Skip automático**: el blanco `[ \t\r\n]+` y los comentarios
  `//[^\n]*` se saltan sin decir ni pío
- **Spans reales**: cada token conoce su línea, columna y offset
  —no se pierde ni una
- **Unicode**: soporta la ñ y las tildes en identificadores, que
  es lengua de Cervantes
- **Tres alias de función**: `función`, `funcion` (sin tilde, que
  a veces falla el teclado), y `fn` (para los extranjeros)

## Cobertura de tokens

| Categoría | Tokens |
|-----------|--------|
| Keywords | `función`, `retornar`, `si`, `sino`, `mientras`, `para`, `en`, `estructural`, `enumeración`, `inseguro`, `usar`, `módulo`, `todos`, `tipo`, `como`, `es`, `está`, `fuese`, `entonces` |
| Artículos | `el`, `la`, `un`, `los`, `las` |
| Tipos | `Entero{8,16,32,64}`, `Natural{8,16,32,64}`, `Flotante{32,64}`, `Booleano`, `Caracter`, `Palabra`, `Vacío` |
| Operadores | `+`, `-`, `*`, `/`, `%`, `==`, `!=`, `<`, `>`, `<=`, `>=`, `&&`, `\|\|`, `!`, `&`, `.`, `=` |
| Símbolos | `()`, `{}`, `[]`, `,`, `;`, `:`, `->` |
| Literales | Enteros, flotantes, strings `"..."`, caracteres `'x'`, `verdadero`/`falso` |
| Error | Token inválido, reportado con su span —no hay escapatoria |

## Tests

Cinco tests velan por el lexer en `src/lexer.rs`:
- `test_lexer_hola_mundo`
- `test_lexer_aritmetica`
- `test_lexer_string`
- `test_lexer_articulos`
- `test_lexer_funcion_alias`
