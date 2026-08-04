# 15 — Glosario rápido

← [14: Ownership](14-ownership.md) | [Indice](../GUIA.md)

---

| Término | Significado | Capítulo |
|---------|-------------|----------|
| **Articulo** | `el`, `la`, `un`, `los`, `las` — indican quien es dueño de una variable | [03](03-variables.md) |
| **Apodo** | `apodo Nombre = Tipo;` — dar un nombre con significado a un tipo | [03](03-variables.md) |
| **Binding** | Asignar un valor a un nombre | |
| **Borrow (prestamo)** | Usar un valor sin ser dueño, solo lectura | [14](14-ownership.md) |
| **Compilación** | Convertir .fc a .exe | [02](02-tu-primer-programa.md) |
| **Comodín** | `_` — atrapa cualquier valor no cubierto | [10](10-datos.md) |
| **Copiar** | `copiar x` — clonar un valor explícitamente | [14](14-ownership.md) |
| **Dueño (owned)** | Variable que controla la memoria del dato | [03](03-variables.md) |
| **Field-level borrowing** | Prestar campos distintos de un struct sin conflicto | [14](14-ownership.md) |
| **Generico** | Funciona con cualquier tipo (`<T>`) | [07](07-funciones.md) |
| **Heap** | Memoria dinámica (malloc/free) | |
| **Inferencia** | El compilador deduce el tipo automáticamente | |
| **Interpolación** | Meter variables en strings con `{var}` | [08](08-texto.md) |
| **Lógico** | Tipo booleano natural (`verdadero`/`falso`) — igual que `Booleano` | [03](03-variables.md) |
| **Lifetime léxico** | `&nombre T` — referencia ligada a la vida de `nombre` | [14](14-ownership.md) |
| **Mover** | Transferir propiedad: `mover x a fn` | [14](14-ownership.md) |
| **Ownership** | Sistema de propiedad de datos (quién es dueño, quién presta) | [03](03-variables.md), [14](14-ownership.md) |
| **Palabra** | Texto fijo en el programa (&str) | [08](08-texto.md) |
| **Pila (stack)** | Memoria rápida, automática, por ámbito | |
| **Prestado (borrowed)** | Préstamo temporal, no se modifica ni libera | [14](14-ownership.md) |
| **Rango** | `0..5` (exclusivo) o `0..=5` (inclusivo) | [06](06-bucles.md) |
| **Real** | Tipo flotante natural (Flotante64) — `RealCorto` = Flotante32 | [03](03-variables.md) |
| **Referencia mutable** | `&mut T` — préstamo con permiso de escritura | [14](14-ownership.md) |
| **Región** | `región { }` — bloque donde todo se libera junto | [14](14-ownership.md) |
| **Self-referential** | `&yo T` — struct que se referencia a sí mismo | [14](14-ownership.md) |
| **Subjuntivo** | `fuese` para casos improbables (cold path) | [05](05-decisiones.md) |
| **Texto** | Texto dinámico heap-allocado (hay que liberarlo) | [08](08-texto.md) |

---

← [14: Ownership](14-ownership.md) | [Indice](../GUIA.md)
