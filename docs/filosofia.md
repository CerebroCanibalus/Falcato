# Filosofía de Falcato

## Los 5 pilares

Falcato se define por 5 pilares que le dan ventaja concreta sobre
Rust, Zig, C y Carbon. Ver `docs/hoja_de_ruta.md` para la definición
completa de cada pilar.

### ⚔️ Poder (trasversal a los 5 pilares)

Falcato da **control total del hardware**. Esto significa:

- Sin runtime oculto. Sin recolector de basura. Sin VM.
- Gestión de memoria explícita pero *asistida semánticamente* por el idioma.
- Zero-cost abstractions: toda feature del lenguaje se pliega en tiempo de
  compilación. Si no se usa, no se paga.
- Acceso directo a memoria, registros y ensamblador cuando sea necesario.
- Sistema de tipos que previene errores *en compilación*, no en ejecución.

### ⚡ Eficiencia

El código generado debe competir con C, Rust y Zig en rendimiento:

- LLVM IR como backend — las mismas optimizaciones que Rust/Clang.
- Layout de memoria controlable (C ABI compatible, repr especificable).
- Sin branching oculto (alloc implícito, panic handlers, etc.).
- El programador siempre sabe lo que paga en tiempo y espacio.

### 🛡️ Iberofonía

El español no es una capa de traducción sobre conceptos ingleses. Es la
**fuente de diseño del sistema de tipos y ejecución**:

| Rasgo español | Traducción al sistema de tipos |
|--------------|--------------------------------|
| Género (el/la) | Ownership / préstamo de memoria |
| Tiempo verbal | Modo de ejecución (sync/async/deferred/fallible) |
| Ser / Estar | Permanencia (const) vs transitoriedad (mut) |
| Subjuntivo | Incertidumbre en tiempo de compilación (Result/Option) |
| Prefijos (re-, des-, pre-, co-) | Semántica de operaciones |
| Voz activa / pasiva | Dirección del flujo de datos |
| Diminutivos | Scope/intencionalidad de asignación |
| Compuestos aglutinantes | APIs auto-documentadas |

## Principios de diseño

> **"El mayor poder a la mayor rapidez y siendo baratos."**

- **Barato** no significa fácil. Significa que cada línea de código hace
  lo que dice, sin magia oculta.
- **Rápido** no significa apresurado. Significa que el compilador optimiza
  sin preguntar, y el programador solo paga por lo que usa explícitamente.
- **Poderoso** no significa complejo. Significa que las herramientas del
  lenguaje crecen con la madurez del programador, no al revés.

## Lo que Falcato NO es

- ❌ No es Rust con keywords en español.
- ❌ No es un lenguaje educativo / para niños.
- ❌ No es un reemplazo de C para embedded.
- ❌ No es un lenguaje con runtime pesado.

## Lo que Falcato QUIERE ser

- ✅ Un lenguaje de sistemas donde el español *aporta* nuevas formas de
  expresar restricciones de memoria y concurrencia.
- ✅ Un experimento serio de ingeniería de lenguajes con 500+ años de
  evolución lingüística como materia prima.
- ✅ Un caballo de batalla para sistemas donde el control total importa.
