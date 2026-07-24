# Desarrollo

Guías para quien quiera meter mano en el compilador Falcato:
ya sea para corregir un error, añadir una función, o simplemente
entender cómo funciona este artilugio.

## Contenido

| Documento | Descripción |
|-----------|-------------|
| [Guía de Contribución](contribuir.md) | Cómo empezar, qué estándares seguir, cómo mandar parches |
| [Build](build.md) | Cómo compilar el compilador desde el código fuente |
| [Testing](testing.md) | Cómo ejecutar y escribir pruebas |

## Stack técnico

Esto es lo que mueve el tinglado:

| Componente | Tecnología |
|------------|------------|
| Lenguaje del compilador | Rust (edition 2021) |
| Lexer | `logos` 0.14 |
| Parser | Manual descendente + Pratt |
| Codegen | Cranelift 0.112 |
| CLI | `clap` 4.5 (derive) |
| LSP | `tower-lsp` 0.20 |
| Async runtime | `tokio` 1.x |
| Target | x86_64 Windows (msvc) / Linux |
