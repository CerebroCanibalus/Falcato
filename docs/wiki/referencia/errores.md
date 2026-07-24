# Códigos de Error

Catálogo de los errores que el compilador puede escupir, con su
número y significación. Cuando veáis uno, venid aquí a buscar
consuelo.

## Sintaxis [S001-S099]

| Código | Significado |
|--------|-------------|
| S001 | Token inesperado: no esperaba encontrarme esto |
| S002 | Fin de archivo inesperado: se acabó antes de tiempo |
| S003 | Se esperaba otro token, no éste |
| S004 | Se esperaba un nombre (identificador) |
| S005 | Se esperaba una expresión, pero no hay |
| S006 | Se esperaba un tipo (Entero32, Booleano, etc.) |
| S007 | Artículo esperado (el, la, un, los, las) |
| S008 | Carácter que no debiera estar ahí |

## Tipo [T001-T099]

| Código | Significado |
|--------|-------------|
| T001 | Disconcordancia de tipo: valor no casa con declaración |
| T002 | Disconcordancia en retorno: lo que retorna no es lo prometido |
| T003 | Retorno faltante: función con retorno declarado pero sin valor |
| T004 | Variable no declarada: no está, no existe |
| T005 | Tipos distintos en operación binaria |
| T006 | Aritmética en tipo no numérico |
| T007 | Comparación en tipo no comparable |
| T008 | Operación lógica en no booleano |
| T009 | Negación de un no número |
| T010 | Negación lógica en no booleano |
| T011 | Condición del `si` no es Booleano |
| T012 | Condición del `mientras` no es Booleano |
| T013 | Asignación con tipo incompatible |

## Ownership [O001-O099]

| Código | Significado |
|--------|-------------|
| O001 | Asignación a variable inmutable (declarada con `la`) |

## FFI [C001-C099]

| Código | Significado |
|--------|-------------|
| C001 | Función FFI no encontrada |

## Interno [I001-I099]

Éstos no debierais verlos, pero si aparecen, algo raro pasó dentro
del compilador:

| Código | Significado |
|--------|-------------|
| I005 | Literal no soportado en el codegen |
| I006 | Variable que el codegen no encuentra |
| I010 | Error definiendo función |
| I015 | Variable no hallada para asignación |
| I020 | Acceso a array en tipo que no es array |
| I021 | Expresión no válida para inicializar un array |
| I022 | `todos` usado fuera de una inicialización |
| I030 | Struct no registrado en el codegen |
| I031 | Campo no encontrado en el layout del struct |
| I032 | Acceso a campo en tipo que no es struct |
| I040 | Bucle `para` sin array en el codegen |
| I050 | Enum no registrado en el codegen |
