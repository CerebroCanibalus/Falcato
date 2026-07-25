# 01 — ¿Qué es Falcato?

← [Índice](../GUIA.md) | [Siguiente: Tu primer programa →](02-tu-primer-programa.md)

---

Falcato es un **lenguaje de programación de bajo nivel** —como C o Rust— pero con una idea diferente: está diseñado **en español**, aprovechando cosas del idioma que el inglés no tiene.

## ¿Para qué sirve?

- Programas rápidos (juegos, motores, servidores)
- Sistemas (kernels, drivers, firmware)
- Donde usarías C o Rust pero quieres algo más legible
- Código generado por inteligencia artificial

## ¿Qué lo hace diferente?

### Español, no inglés traducido

Todos los lenguajes están en inglés. Tu cerebro hace: **idea → inglés → código**. Con Falcato es: **idea → español → código**. Un paso menos.

### Artículos (el, la, un)

En español decimos "**el** carro" y "**la** casa". Falcato usa la misma idea:

```falcato
el x: Entero32 = 5;    // este es mio, puedo cambiarlo
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
