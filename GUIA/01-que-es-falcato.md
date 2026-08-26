# 01 — ¿Qué es Falcato?

Falcato es un lenguaje de programación construido desde cero, sobre Cranelift como backend. No es una traducción de Rust ni de ningún otro lenguaje. Su idea central es que la gramática del español no es un adorno: es el sistema de tipos. Lo que en otros lenguajes son reglas abstractas con una sintaxis encima, en Falcato es la propia estructura del idioma lo que dicta cómo funciona la memoria, la propiedad y la ejecución.

## ¿Para qué sirve?

- Programas de bajo nivel: scripts de sistema, herramientas, automatizaciones.
- Donde usarías C o Rust, pero quieres una sintaxis más natural y menos inglesa.
- Código generado por inteligencia artificial: Falcato está diseñado para que el compilador sea el juez, no la memoria del modelo.

## ¿Qué lo hace diferente?

### Gramatipado

**Gramatipado** = gramática + tipado. La idea central de Falcato: el sistema de tipos no es un conjunto de reglas arbitrarias con una sintaxis encima. **La gramática del español ES el sistema de tipos**.

- El artículo (`el`, `la`, `un`) decide la propiedad de la memoria: `el` = dueño mutable, `la` = prestado inmutable, `un` = incierto/opcional.
- El tiempo verbal (`es`, `está`, `fuese`) decide el modo de ejecución: presente = sincrónico, futuro = asíncrono, subjuntivo = fallible.
- Ser y estar (`es` vs `está`) decide la mutabilidad: `es` = permanente (const), `está` = temporal (mut).

En una frase: Falcato usa la gramática española como sistema de tipos. No hay que traducir una regla nueva: la distinción ya la haces al hablar.

### Morfosemántico

**Morfosemántico** viene de la lingüística: estudia cómo la morfología (artículos, desinencias verbales) porta significado. En español, "el" vs "la" no es decoración: distingue posesión y certeza. "Es" vs "está" distingue permanencia de estado. Los tiempos verbales distinguen realidad de hipótesis.

Falcato aprovecha ese significado que el idioma ya tiene: la morfología del español porta semántica de máquina sin que el programador tenga que aprender reglas nuevas. La cerradura ya viene puesta en el idioma; Falcato fabrica la llave.

### En comparación con otros lenguajes

Mismo programa, guardando un texto, prestándolo y comprobando si está vacío, en Rust y en Falcato:

```rust
// Rust: cada garantía es un tipo distinto que hay que memorizar
let mut contenido = String::from("datos");
let prestado: &str = &contenido;
let quizas: Option<String> = None;
if contenido.len() == 0 { return Err(-1); }
```

```falcato
// Falcato: la garantía es gramática que ya sabes
el contenido: Texto = texto_desde("datos");  // "el" = mío, lo cambio
la prestado: &Texto = &contenido;            // "la" = prestado, solo leo
un quizas: Option<Texto>;                    // "un" = quizás existe
si contenido.tam() está 0 { retornar Resultado.Error(-1); }
```

En Rust, cada garantía es un tipo distinto que hay que conocer y un modificador aparte. En Falcato, la garantía es gramática que ya sabes: "el" = mío, "la" = prestado, "un" = quizás. No hay que traducir nada: es la misma distinción que haces al hablar.

Y cuando la operación puede fallar, la gramática también lo dice:

```rust
// Rust: checked_add es una API aparte
let r = a.checked_add(b);  // → Option<i32>
```

```falcato
// Falcato: el subjuntivo ES la hipótesis
el r = a + b fuese;  // → Resultado<Entero32, Entero32>
un x = a + b;        // → Option — "un" = incierto
```

Ningún otro lenguaje es gramatipado ni morfosemántico. Falcato es el primero en ambos.

### Español, no inglés traducido

Todos los lenguajes están en inglés. Tu cerebro hace: idea → inglés → código. Con Falcato es: idea → español → código. Un paso menos.

### Artículos (el, la, un)

En español decimos "el carro" y "la casa". Falcato usa la misma idea:

```falcato
el x: Entero32 = 5;    // este es mío, puedo cambiarlo
la y: Entero32 = 10;   // este es prestado, solo lectura
```

Un vistazo y sabes quién controla qué.

### Ser y Estar

"Es" de noche es permanente. "Está" nublado es temporal. Falcato entiende eso:

```falcato
si x es 5 { }     // "x es 5" — identidad
si x esta 5 { }   // "x está en 5" — estado pasajero
```

### Compilación instantánea

Falcato usa **Cranelift**, un compilador que traduce código a máquina en **milisegundos**, no minutos.

## ¿Para quién es?

- **Si sabes C o Rust** — te sentirás en casa, pero con sintaxis más natural.
- **Si sabes Python o JavaScript** — aprenderás conceptos de bajo nivel sin la barrera del inglés.
- **Si programas con IA** — Falcato está diseñado para que una IA genere código correcto sin alucinar.

## ¿Qué no es Falcato?

- No es una traducción de Rust al español.
- No tiene recolector de basura (tú controlas la memoria).
- No es para páginas web (es para sistemas).

---

← [Índice](../GUIA.md) | [Siguiente: Tu primer programa →](02-tu-primer-programa.md)