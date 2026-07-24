# Funciones

Las funciones son, en Falcato, el modo principal de organizar el
pensamiento computacional. Cada función recibe unos parámetros
—con su artículo y tipo, que esto es España— y retorna un valor,
o no, según su declaración.

## Declaración

Así se declara una función, con toda la pompa necesaria:

```falcato
función nombre(param1: Tipo, param2: Tipo) -> TipoRetorno {
    // cuerpo de la función
    retornar valor;
}
```

### Sin retorno

Si la función no retorna nada —que no toda acción requiere recompensa—,
se omite la flecha:

```falcato
función solo_efectos() {
    // cuerpo, sin retorno, que el placer está en ejecutar
}
```

## Parámetros con artículos

Cada parámetro lleva su artículo, porque Falcato es puntilloso
con el régimen de propiedad:

```falcato
función suma(el a: Entero32, el b: Entero32) -> Entero32 {
    retornar a + b;
}
```

## Llamadas

Llamar a una función es cosa sencilla:

```falcato
función principal() -> Entero32 {
    el resultado = suma(10, 20);
    retornar resultado;
}
```

### Verificación de tipos en llamadas

Mas ¡ay del que yerra! El compilador verifica que los argumentos
concuerden en número y tipo:

```falcato
función suma(el a: Entero32, el b: Entero32) -> Entero32 { ... }

suma(10)            // Error: espera 2 argumentos, se pasó 1
suma(10, verdadero) // Error: argumento 2 espera Entero32, encontrado Booleano
```

## FFI (Funciones externas)

Cuando Falcato necesita hablar con C —que es lengua común en
la república de los sistemas— se usa `inseguro` y se declara
la función sin cuerpo:

```falcato
inseguro función puts(el mensaje: Palabra);

función principal() {
    puts("¡Hola, Falcato!");
    retornar 0;
}

Características principales:
- **C ABI por defecto**: no hay que pedir `extern "C"`, es la costumbre
- **Name mangling desactivado**: los nombres se ven tal cual
- **Linkeo con `link.exe`** en Windows, o `gcc` en Linux

## Alias de `función`

Para comodidad de los que vienen de otras tierras, estas tres formas
son equivalentes:

```falcato
función foo() { }
funcion foo() { }   // sin tilde, que no pasa nada
fn foo() { }        // para los que extrañan el Rust
```
