# Compilador Falcato — Arquitectura Interna

Ésta es la crónica del compilador contada desde las entrañas
del código en `src/`. No hay aquí especulación, sino la verdad
de lo que cada engranaje hace.

## Pipeline

El viaje desde el archivo `.fc` hasta el `.exe` sigue este camino:

```
archivo.fc
  │
  ▼
┌─────────────┐
│   Lexer     │ logos 0.14 — trocea el texto en tokens con Span
└─────────────┘
  │ stream de tokens
  ▼
┌─────────────────────┐
│   Parser            │ Descendente manual (en 5 archivos)
│                     │ ParserCursor + Pratt parser
│                     │ Recovery de errores (no se rinde ante el primero)
└─────────────────────┘
  │ AST con Span en cada nodo
  ▼
┌─────────────────────┐
│  Análisis Semántico │ Concordancia Lingüística
│                     │ Entornos anidados, inferencia de tipos
│                     │ Verificación de ownership, llamadas, bounds
└─────────────────────┘
  │ AST verificado
  ▼
┌─────────────────────┐
│   Codegen           │ Cranelift 0.112
│                     │ C ABI (SystemV), stack slots
│                     │ Branching, loops, arrays, structs, enums
└─────────────────────┘
  │ .o (objeto COFF/ELF)
  ▼
┌─────────────────────┐
│   Linker            │ link.exe (MSVC) o gcc
│                     │ Entry point: principal
│                     │ Salida: .exe nativo (¡albricias!)
└─────────────────────┘
```

## Componentes

| Componente | Archivo | Tecnología |
|------------|---------|------------|
| [Lexer](lexer.md) | `src/lexer.rs` | `logos` 0.14 |
| [Parser](parser.md) | `src/parser/` | Manual descendente + Pratt |
| [AST](ast.md) | `src/ast.rs` | Structs Rust con Span |
| [Semántica](semantica.md) | `src/semantic.rs` | Propio (concordancia lingüística) |
| [Codegen](codegen.md) | `src/codegen.rs` | Cranelift 0.112 |
| [LSP](lsp.md) | `src/lsp.rs` | `tower-lsp` 0.20 |
| [Sistema de Errores](sistema-errores.md) | `src/error.rs` | Propio con códigos |

## Diferencias con los papeles de `docs/`

Sabed que los documentos en `docs/` raíz describen una arquitectura
que nunca llegó a ser —con chumsky en el parser e inkwell (LLVM)
en el backend. El compilador real, el que aquí os presentamos,
es otro cantar:

| Aspecto | docs/ (especulativo) | Realidad (wiki) |
|---------|----------------------|------------------|
| Parser | chumsky combinators | Descendente manual |
| Backend | inkwell (LLVM) | Cranelift 0.112 |
| IR intermedio | FAL IR propio | Directo a Cranelift |
| Semántica | `semantic/` en varios módulos | `semantic.rs` único |
| LSP | tower-lsp 0.20 | tower-lsp 0.20 ✅ (éste acertaron) |
