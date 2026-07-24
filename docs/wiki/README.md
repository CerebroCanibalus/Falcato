# Falcato Wiki

En un lugar de la Mancha —o más bien, en esta memoria de ordenador—
habremos de encontrar la documentación viva del lenguaje Falcato.
Sepa vuesa merced que lo que aquí se describe no es sueño ni
promesa de futuro, sino el **estado real** del proyecto: el fiero
compilador, el lenguaje que parla, y las herramientas que lo asisten.

> Los papeles que hallaréis en `docs/` raíz son borradores de diseño,
> especulaciones y cartas de navegación de lo que algún día podría ser.
> Esta wiki, en cambio, es la verdad de la milicia: lo que el código
> realmente hace, sin embelecos ni vanas fantasías.

## Secciones

| Sección | Contenido |
|---------|-----------|
| [Guía del Lenguaje](guia/) | Sintaxis, tipos, artículos, control de flujo, funciones, arrays, structs, enums, genéricos |
| [Compilador](compilador/) | Pipeline, lexer, parser, AST, semántica, codegen, LSP, sistema de errores |
| [Referencia](referencia/) | CLI, códigos de error, FFI |
| [Desarrollo](desarrollo/) | Cómo contribuir, build, testing, roadmap |

## Estado Actual (Fase 8C)

Digo, pues, que el camino está andado hasta la Fase 8C, y el
compilador hace lo que debe:

- Pipeline end-to-end: `.fc` → Lexer → Parser → AST → Semántica → Cranelift → `.o` → `link.exe` → `.exe`
- **31 tests** que pasan sin deshonra
- Backend: Cranelift 0.112 (puro Rust, sin ataduras del sistema)
- CLI con cinco mandatos: `build`, `run`, `check`, `lsp`, `version`
- LSP con diagnósticos en vivo, hover, go-to-definition y autocompletado

## Convenciones

- Código fuente: `.fc`
- Documentación: en español, que es lengua de imperios
- Nombres de funciones y tipos: español, con la mesura del `snake_case`
- Código Rust del compilador: inglés (que en eso se entiende mejor con las herramientas)
- Versión: SemVer hasta que llegue el 1.0
