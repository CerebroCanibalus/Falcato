# Arrays

Los arrays son colecciones ordenadas de elementos del mismo tipo,
dispuestos en la memoria como soldados en formación. Se declaran
con corchetes, que no hay mejor símbolo para encerrar lo que es
de igual naturaleza.

## Declaración

```falcato
los nums: [Entero32; 5];  // array de cinco enteros
```

El tipo se escribe así: `[Tipo; Longitud]`. Nótese el punto y coma
entre el tipo y la longitud, que no es adorno: es separar la esencia
de la cantidad.

## Literales

```falcato
los nums: [Entero32; 3] = [1, 2, 3];
```

Todos los elementos han de ser del mismo tipo, so pena de disconformidad
—que el compilador es severo en estas lides.

## Inicialización con `todos`

Cuando se desea llenar todo el array con un mismo valor, la palabra
`todos` acude al quite:

```falcato
los nums: [Entero32; 5] = todos 0;  // [0, 0, 0, 0, 0]
```

Útil es para inicializar sin tener que escribir cada elemento uno
por uno, como quien barre toda la casa de una vez.

## Acceso por índice

```falcato
el primero: Entero32 = nums[0];
el i: Entero32 = 2;
el valor: Entero32 = nums[i];  // acceso dinámico: cuando no se sabe
                               // de antemano qué casilla toca
```

El índice ha de ser `Entero32` o `Entero64`. No valen otros números.

## Asignación a elementos

```falcato
nums[2] = 30;    // modifica el tercer elemento
nums[i] = 99;    // con índice variable
```

## Asignación completa

Para reemplazar todos los elementos de golpe:

```falcato
los otros: [Entero32; 3] = [10, 20, 30];
```

## Bucle `para` sobre arrays

El bucle `para` permite recorrer los elementos sin tener que
lidiar con índices:

```falcato
para num en nums {
    // num toma el valor de cada elemento, uno tras otro
}
```
