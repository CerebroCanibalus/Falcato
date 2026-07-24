# Sistema de Errores

**Archivo:** `src/error.rs`

El sistema de errores es la voz del compilador. Cuando algo no
marcha bien, no se calla: habla claro, en español, con códigos
que permiten identificar el problema de un vistazo.

## Categorías

Cada error lleva una categoría que dice de qué palo va el asunto:

| Categoría | Prefijo | Rango | Propósito |
|-----------|---------|-------|-----------|
| Sintaxis | `S` | S001-S099 | Errores del parser: lo escrito no se entiende |
| Tipo | `T` | T001-T099 | Disconcordancias de tipo |
| Ownership | `O` | O001-O099 | Tropiezos con los artículos |
| FFI | `C` | C001-C099 | Problemas con C |
| Módulos | `M` | M001-M099 | Módulos e imports |
| Interno | `I` | I001-I099 | Cosas que no debieran pasar |
| Warning | `W` | W001-W099 | Avisos, que no errores |

## Formato

```
[Código] archivo.fc:línea:columna: mensaje
       │ sugerencia: texto opcional
```

## Estructura

```rust
pub struct ErrorCompilador {
    pub categoria: CategoriaError,
    pub codigo: u32,
    pub span: Span,
    pub mensaje: String,
    pub sugerencia: Option<String>,
}
```

## Errores predefinidos (`error::errores`)

```rust
// Sintaxis
token_inesperado(span, token)           // [S001]
fin_archivo_inesperado(span)            // [S002]

// Tipo
tipo_no_encontrado(span, tipo)          // [T001]

// FFI
funcion_ffi_no_encontrada(span, fn)     // [C001]
```

## Errores de sintaxis (`parser::errores.rs`)

```rust
// S001-S008
token_inesperado(span, esperado, encontrado)   // S001
fin_archivo_inesperado(span)                    // S002
esperaba(span, esperado, encontrado)             // S003
identificador_esperado(span, contexto)           // S004
expresion_esperada(span)                         // S005
tipo_esperado(span)                              // S006
articulo_esperado(span)                          // S007
token_invalido(span, texto)                      // S008
```
