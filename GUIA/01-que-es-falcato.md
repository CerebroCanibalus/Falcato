# 01 — ¿Qué es Falcato?

![Falcato Title](../../assets/images/falcato_title.png)

← [Índice](../GUIA.md) | [Siguiente: Tu primer programa →](02-tu-primer-programa.md)

---

Falcato es un **lenguaje de programación de bajo nivel** —como C o Rust— pero con una idea diferente: es un lenguaje **gramatipado y morfosemántico**. La gramática del español (artículos, tiempos verbales, ser/estar) no es un adorno: **es el sistema de tipos**.

## ¿Para qué sirve?

- Programas rápidos (juegos, motores, servidores)
- Sistemas (kernels, drivers, firmware)
- Donde usarías C o Rust pero quieres algo más legible
- Código generado por inteligencia artificial

## ¿Qué lo hace diferente?

### Gramatipado y Morfosemántico

Son dos conceptos distintos que se complementan. Uno describe el diseño del lenguaje; el otro, el fenómeno lingüístico que lo hace posible.

#### Gramatipado — el diseño

**Gramatipado** = gramática + tipado. Es la idea central de Falcato: el sistema de tipos no es un conjunto de reglas abstractas con una sintaxis encima — **la gramática del español ES el sistema de tipos**.

- El artículo decide la propiedad de la memoria: `el` = mío (owned), `la` = prestado (borrowed), `un` = incierto (option)
- El tiempo verbal decide el modo de ejecución: presente = síncrono, futuro = asíncrono, subjuntivo = fallible
- Ser/estar decide la mutabilidad: `es` = permanente (const), `está` = temporal (mut)

En una frase: Falcato usa la gramática española como sistema de tipos.

#### Morfosemántico — el fundamento

**Morfosemántico** viene de la **morfosemántica**, una rama de la lingüística que estudia cómo la *morfología* (los artículos, las desinencias verbales, los afijos) porta significado. En español, "el" vs "la" no es decoración: distingue posesión y certeza. "Es" vs "está" distingue permanencia de estado. Los tiempos verbales distinguen realidad de hipótesis.

Falcato aprovecha ese significado que el idioma **ya tiene**: la morfología del español porta semántica de máquina sin que el programador tenga que aprender reglas nuevas.

Morfosemántico explica **"¿por qué funciona?"**: la morfología del español ya es semántica — Falcato solo la conecta a la máquina.

#### Cómo se complementan

Piensa en una cerradura y su llave. El español ya trae la cerradura puesta: su morfología distingue posesión (`el`/`la`), certeza (`un`), permanencia (`es`/`está`) y modo (indicativo/subjuntivo). Eso es la morfosemántica, y funciona desde hace siglos. Falcato fabrica la llave: el gramatipado, que conecta esa cerradura a la máquina.

Sin la cerradura, la llave no tendría dónde girar: un lenguaje "en español" sin morfosemántica sería solo keywords traducidas. Sin la llave, la cerradura quedaría ignorada, como en todos los demás lenguajes. Falcato junta ambas.

#### En comparación con otros lenguajes

El mismo programa —guardar un texto, prestarlo y comprobar si está vacío— en Rust y en Falcato:

```rust
// Rust: el ownership es un sistema de tipos que hay que memorizar
let mut contenido = String::from("datos");  // String = owned, mutable
let prestado: &str = &contenido;            // &str = borrowed, inmutable
let quizas: Option<String> = None;          // Option = quizás existe
if contenido.len() == 0 { return Err(-1); }
```

```falcato
// Falcato: la gramática lo dice todo
el contenido: Texto = texto_desde("datos");  // "el" = mío, lo cambio
la prestado: &Texto = &contenido;            // "la" = prestado, solo leo
un quizas: Option<Texto>;                    // "un" = quizás existe
si contenido.tam() está 0 { retornar Resultado.Error(-1); }
```

En Rust, cada garantía es un **tipo distinto que hay que conocer** (`String`, `&str`, `Option<String>`) y un **modificador** (`let` vs `let mut`). El programador traduce su intención a ese vocabulario.

En Falcato, la garantía es **gramática que ya sabes**: "el" = mío, "la" = prestado, "un" = quizás, "está" = estado temporal. No hay que traducir nada: es la misma distinción que haces al hablar.

Y cuando la operación puede fallar, la gramática también lo dice:

```rust
// Rust: checked_add es una API aparte que hay que recordar
let r = a.checked_add(b);  // → Option<i32>
```

```falcato
// Falcato: el subjuntivo ES la hipótesis
el r = a + b fuese;  // → Resultado<Entero32, Entero32>
un x = a + b;        // → Option — "un" = incierto
```

En Rust, la aritmética segura es un **método** que hay que conocer (`checked_add`). En Falcato, es el **modo verbal**: "si la suma se desbordara..." — el subjuntivo, la misma gramática que usas para hablar de hipótesis.

Ningún otro lenguaje es gramatipado ni morfosemántico. Falcato es el primero en ambos.

### Español, no inglés traducido

Todos los lenguajes están en inglés. Tu cerebro hace: **idea → inglés → código**. Con Falcato es: **idea → español → código**. Un paso menos.

### Artículos (el, la, un)

En español decimos "**el** carro" y "**la** casa". Falcato usa la misma idea:

```falcato
el x: Entero32 = 5;    // este es mío, puedo cambiarlo
la y: Entero32 = 10;   // este es prestado, solo lectura
```

Un vistazo y sabes quién controla qué.

### Ser y Estar

"**Es** de noche" es permanente. "**Está** nublado" es temporal. Falcato entiende eso:

```falcato
si x es 5 { }     // "x es 5" — identidad
si x esta 5 { }   // "x esta en 5" — estado pasajero
```

### Compilación instantánea

Falcato usa **Cranelift**, un compilador que traduce código a máquina en **milisegundos**, no minutos.

## ¿Para quién es?

- **Si sabes C o Rust** — te sentirás en casa, pero con sintaxis más natural
- **Si sabes Python o JavaScript** — aprenderás conceptos de bajo nivel sin la barrera del inglés
- **Si programas con IA** — Falcato está diseñado para que una IA genere código correcto sin alucinar

## ¿Qué no es Falcato?

- No es una traducción de Rust al español
- No tiene recolector de basura (tú controlas la memoria)
- No es para páginas web (es para sistemas)

---

← [Índice](../GUIA.md) | [Siguiente: Tu primer programa →](02-tu-primer-programa.md)
