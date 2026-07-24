# Análisis Semántico

**Archivo:** `src/semantic.rs`

Si el lexer separa y el parser ordena, el análisis semántico
es quien juzga si lo ordenado tiene sentido. Aquí es donde
Falcato muestra su vena lingüística.

## Concordancia Lingüística

Ésta es la innovación central del lenguaje: los errores semánticos
se reportan como «disconcordancias», que es término que cualquier
hablante de español entiende. Como cuando un adjetivo no concuerda
con el sustantivo, aquí un tipo no concuerda con otro.

## Estructura

### AnalizadorSemantico

```rust
pub struct AnalizadorSemantico {
    errores: Errores,
    entorno: Entorno,           // scopes anidados
    structs: HashMap<String, InfoStruct>,
    enums: HashMap<String, InfoEnum>,
    funciones: HashMap<String, FirmaFuncion>,
    funcion_actual: Option<FuncionDecl>,
}
```

### Entorno

Ámbitos anidados que recuerdan quién es quién y dónde vive:

```rust
pub struct Entorno {
    variables: HashMap<String, InfoVariable>,
    tipos: HashMap<String, Tipo>,          // type params
    consts: HashMap<String, (Tipo, Option<usize>)>, // const params
    padre: Option<Box<Entorno>>,
}
```

## Verificaciones implementadas

### Tipo [T###]

| Código | Nombre | Qué detecta |
|--------|--------|-------------|
| T001 | DISCONCORDANCIA_TIPO | Variable declarada como A, valor es B |
| T002 | DISCONCORDANCIA_RETORNO | Se retorna algo que no es lo prometido |
| T003 | RETORNO_FALTANTE | Función promete retorno pero no da nada |
| T004 | VARIABLE_NO_DECLARADA | Usar algo que no existe |
| T005 | DISCONCORDANCIA_OPERANDOS | Tipos distintos en misma operación |
| T006 | OPERACION_ARITMETICA_INVALIDA | Aritmética en no numéricos |
| T007 | COMPARACION_INVALIDA | Comparar lo incomparable |
| T008 | OPERACION_LOGICA_INVALIDA | `&&`/`\|\|` en no booleanos |
| T009 | NEGACION_ARITMETICA_INVALIDA | Negar un no número |
| T010 | NEGACION_LOGICA_INVALIDA | `!` en no booleano |
| T011 | CONDICIONAL_NO_BOOLEANO | Condición del `si` no es booleana |
| T012 | BUCLE_NO_BOOLEANO | Condición del `mientras` no es booleana |
| T013 | ASIGNACION_INCOMPATIBLE | Tipo en asignación no casa |

### Ownership [O###]

| Código | Qué detecta |
|--------|-------------|
| O001 | Asignación a variable declarada con `la` (inmutable) |

### Verificación de llamadas

- Cantidad de argumentos
- Tipos de cada argumento
- Bounds declarativos (`que Comparable`, `que Ordenable`)

### Arrays

- Tipos consistentes entre elementos
- Índices han de ser Entero
- `todos` compatible con el tipo del array

### Structs

- Campos que existen en el struct
- Tipos que casan
- Campos que no falten

### Enums

- Variante existe en el enum
- Tipos de argumentos en el constructor
- Pattern matching con tipo correcto

## Inferencia de tipos

El compilador adivina el tipo sin que se lo digan:

- Literal entero → `Entero32`
- Literal flotante → `Flotante64`
- Literal string → `Palabra`
- Literal booleano → `Booleano`
- Genéricos → lo que sea, si cumple los bounds

## Mensajes de error

Con el formato que ya conocéis:

```
[T001] test.fc:4:8: Disconcordancia de tipo: 'a' es 'Entero32' pero se declara 'Booleano'
       │ sugerencia: Cambia el tipo a 'Entero32' o el valor
```
