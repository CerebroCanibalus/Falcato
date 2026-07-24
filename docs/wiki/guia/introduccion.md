# Introducción

¡Oh, lector! Si has llegado hasta aquí, es porque tu curiosidad
—o tu necesidad— te ha traído al mundo de Falcato. No esperes
nubes de azúcar ni abstracciones vaporosas: aquí todo es fiero,
concreto y bien medido.

## Hola Mundo

Y allévese el primer ejemplo, que en todo lenguaje de programación
es costumbre comenzar con un saludo al mundo:

```falcato
función principal() -> Entero32 {
    retornar 42;
}
```

Mas habréis de notar que no imprimimos «¡Oh, Mundo!» sino que
retornamos cuarenta y dos, que es número de hondo significado
—y además, el código de salida del proceso.

Compilar y ejecutar:

```bash
falcato build ejemplo.fc
./ejemplo.exe
echo $?  # → 42
```

## Estructura de un programa

Un programa en Falcato no es cosa desordenada, sino secuencia
de **declaraciones top-level** que se alinean como soldados:

- **Funciones**: `función nombre(params) -> Tipo { ... }`
- **Structs**: `estructural Nombre { ... }`
- **Enums**: `enumeración Nombre { ... }`

El punto de entrada es la función `principal`, que retorna un entero
—el código de salida del proceso, como ya se ha dicho.

## Comandos CLI

El compilador obedece a estos mandatos:

```bash
falcato build <archivo.fc>   # Compila a binario .exe
falcato run   <archivo.fc>   # Compila y ejecuta en un suspiro
falcato check <archivo.fc>   # Sólo análisis, sin engendrar binario
falcato lsp                   # Servidor LSP (por stdio, como los hidalgos)
falcato version               # Muestra la versión del artefacto
```
