# Guía de Contribución

Si queréis contribuir a Falcato —y es noble deseo—, haced lo siguiente.

## Primeros pasos

1. Clonad el repositorio (si no lo habéis hecho ya)
2. Aseguraos de tener Rust instalado (rustup + toolchain stable)
3. Compilad: `cargo build` o `.\build.bat`
4. Ejecutad los tests: `cargo test`
5. Probad con los ejemplos: `cargo run -- check ejemplos/hola_mundo.fc`

## Áreas de contribución

No todas las contribuciones requieren ser un héroe de la programación.
He aquí lo que podéis hacer según vuestra osadía:

### Fáciles

- Añadir tests (unitarios o de integración)
- Mejorar los mensajes de error con sugerencias más útiles
- Escribir ejemplos en `ejemplos/`

### Medias

- Nuevas prestaciones del lenguaje (asignación compuesta `+=`,
  operadores que falten, etc.)
- Mejoras al LSP (autocompletado contextual, snippets)
- Optimizaciones en el codegen

### Avanzadas

- Implementar el borrow checker con lifetimes de verdad
- Sistema de traits (que no bounds hardcoded)
- Async (cuando los astros se alineen)
- Biblioteca estándar

## Estándares

Todo código que enviéis ha de cumplir estas normas:

### Código Rust

- `cargo fmt` antes de mandar nada
- `cargo clippy` sin avisos
- Tests para todo código nuevo
- Span en cada nodo nuevo del AST (que si no, no hay error con ubicación)

### Código Falcato

- Español, con snake_case
- Sin comentarios de documentación (los docs van en la wiki, no en el código)
- Preferir `función` sobre `fn` (somos españoles, coño)

## Pull Requests

1. Una feature por PR, que no es bueno mezclar churras con merinas
2. Tests incluidos, si no, no vale
3. Documentación wiki actualizada si cambiáis sintaxis o semántica
4. Sin cambios no solicitados a código adyacente (dejad lo que funciona como está)

## Recursos

- `AGENTS.md` — Dónde estamos y para dónde vamos
- `docs/wiki/` — La documentación viva del proyecto (ésta que leéis)
- `docs/` — Papeles de diseño, especulativos pero inspiradores
- `ESTADO.md` — Estado del proyecto, según quién lo mire
