# Testing

Para que el compilador no se rompa cada vez que alguien toca algo,
hay tests. Muchos tests. Treinta y uno, para ser exactos.

## Ejecutar tests

```bash
# Todos los tests (31, que pasan todos)
cargo test

# Tests de un módulo en concreto
cargo test lexer
cargo test parser
cargo test semantica

# Tests con nombre específico
cargo test test_parse_funcion_simple
```

## Tests existentes (31 en total)

### Lexer (`src/lexer.rs`) — 5 tests
- `test_lexer_hola_mundo`: palabras clave básicas
- `test_lexer_aritmetica`: operadores y números
- `test_lexer_string`: cadenas de texto
- `test_lexer_articulos`: el/la/un/los/las
- `test_lexer_funcion_alias`: función/funcion/fn

### Parser (`src/parser/mod.rs`) — 13 tests
- Funciones simples y con retorno
- Expresiones aritméticas con precedencia
- Condicionales con y sin sino
- Bucles mientras
- FFI (declaración insegura)
- Asignación
- Enums (simples y con datos)
- Const generics
- Bounds declarativos (que Comparable)
- Recuperación de errores (token inesperado)

### Semántica (`src/semantic.rs`) — 13 tests
- Código correcto (todo bien)
- Type mismatch (T001)
- Variable no declarada (T004)
- Retorno incorrecto (T002)
- Condicional correcto y con tipo inválido (T011)
- Ownership mutable (el) e inmutable (la) con error (O001)
- Bucle mientras correcto
- Enum correcto (varios casos)
- Llamadas con argumentos
- Struct correcto

## Escribir tests

### Para features nuevas

Si añadís una prestación nueva, su test correspondiente ha de
seguir este patrón:

```rust
#[test]
fn test_feature_nueva() {
    let fuente = r#"código falcato de prueba"#;
    let lexer = LexerFalcato::nuevo(fuente, "test.fc");
    let tokens = lexer.tokenizar();
    let programa = ParserFalcato::parse(tokens).unwrap();

    let mut semantica = AnalizadorSemantico::nuevo();
    assert!(semantica.analizar(&programa).is_ok());
}
```

### Para errores esperados

Y si lo que probáis es que un error aparezca cuando debe:

```rust
#[test]
fn test_error_esperado() {
    let fuente = r#"código con error"#;
    let lexer = LexerFalcato::nuevo(fuente, "test.fc");
    let tokens = lexer.tokenizar();
    let programa = ParserFalcato::parse(tokens).unwrap();

    let mut semantica = AnalizadorSemantico::nuevo();
    let resultado = semantica.analizar(&programa);
    assert!(resultado.is_err());

    let errores = resultado.unwrap_err();
    assert!(errores.errores.iter().any(|e| e.codigo == CODIGO_ESPERADO));
}
```

## Ejemplos funcionales

En la raíz del proyecto hay unos cuantos `.exe` que en su día
sirvieron para demostrar que el compilador funcionaba. Están
probablemente desactualizados, pero ahí quedan como testimonio:

| Archivo | Feature |
|---------|---------|
| `aritmetica.exe` | Operaciones aritméticas |
| `condicional.exe` | Condicionales |
| `condicional_else.exe` | Condicional con sino |
| `mientras.exe` | Bucles mientras |
| `ownership.exe` | Artículos de posesión |
| `hola_mundo.exe` | El clásico de los clásicos |
