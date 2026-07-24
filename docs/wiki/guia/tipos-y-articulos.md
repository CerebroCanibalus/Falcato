# Tipos y Artículos

Llegamos al corazón de Falcato. Pues habréis de saber que en este
lenguaje los tipos no son meras etiquetas, sino esencia misma de
lo que las cosas son; y los artículos, su régimen de pertenencia
y mutabilidad. Cosas son éstas que harían las delicias de cualquier
gramático.

## Tipos Primitivos

He aquí los tipos fundamentales, que son los ladrillos con que
todo se construye:

| Tipo | Descripción | Tamaño |
|------|-------------|--------|
| `Entero8` | Entero con signo | 8 bits |
| `Entero16` | Entero con signo | 16 bits |
| `Entero32` | Entero con signo (predeterminado) | 32 bits |
| `Entero64` | Entero con signo | 64 bits |
| `Natural8` | Entero sin signo | 8 bits |
| `Natural16` | Entero sin signo | 16 bits |
| `Natural32` | Entero sin signo | 32 bits |
| `Natural64` | Entero sin signo | 64 bits |
| `Flotante32` | IEEE 754, media mantisa | 32 bits |
| `Flotante64` | IEEE 754, mantisa cumplida | 64 bits |
| `Booleano` | `verdadero` / `falso` | 8 bits |
| `Caracter` | Escalar Unicode | 32 bits |
| `Palabra` | Cadena UTF-8 | variable |
| `Vacío` | Tipo unidad (que es como llamar al vacío) | 0 bits |

## Artículos (Ownership)

Y es aquí donde Falcato se distingue, usando los artículos del
español para codificar quién posee qué y con qué poder.

Sabed que los artículos tienen el siguiente régimen:

| Artículo | Semántica | ¿Se puede cambalachear? |
|----------|-----------|--------------------------|
| `el` | Owned, mutable | Sí, como posesión plena |
| `la` | Borrowed, inmutable | No, es prestado y hay que cuidarlo |
| `un` | Optional (quizá existe, quizá no) | Sí, si está presente |
| `los` | Colección owned | Sí |
| `las` | Colección prestada | No |

### Declaración de variables

Dígase con ejemplos, que valen más que mil preceptos:

```falcato
el x: Entero32 = 10;      // owned, mutable: cosa de uno
la y: Entero32 = 20;      // prestada, inmutable: devuélvase tal cual
un z: Booleano = verdadero; // optional: quién sabe si está
```

### Error de ownership

Mas ¡ay! si intentáis mudar lo que es inmutable:

```falcato
la x: Entero32 = 10;
x = 20;  // [O001] Error: 'x' se declaró con 'la' (inmutable/prestada)
         // sugerencia: Usa 'el x' para hacerlo mutable (owned)
```

El compilador, cual severo preceptor, os recordará vuestro lugar.

## Inferencia de tipos

No es menester declarar siempre el tipo, que el compilador es astuto
y lo adivina por sí solo:

```falcato
el x: Entero32 = 10;   // explícito: no hay duda
el y = 10;             // inferido como Entero32: el compilador no es lerdo
```

El valor numérico sin adornos se entiende como `Entero32`. El flotante,
como `Flotante64`. El string, como `Palabra`. Así de sencillo.

## Literales

Finalmente, los valores literales que podéis escribir:

```falcato
42                        // Entero32
3.14                      // Flotante64
"Hola mundo"              // Palabra (string)
'a'                       // Caracter
verdadero                 // Booleano
falso                     // Booleano
```

Y con esto, buena pieza, ya sabéis cómo declarar las variables
y sus posesiones.
