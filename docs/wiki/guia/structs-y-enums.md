# Structs y Enums

Llegamos a las formas compuestas: los structs, que agrupan campos
de diversa índole, y los enums, que ofrecen alternativas bajo un
mismo nombre. Cosas son éstas que todo lenguaje que se precie ha
de tener, y Falcato las posee con sus propias peculiaridades.

## Structs

### Declaración

Un struct se declara con la palabra `estructural`, que es larga
pero muy clara en su cometido:

```falcato
estructural Punto {
    x: Entero32,
    y: Entero32,
}
```

El layout en memoria sigue el convenio de C, que es el de la
comunidad internacional: cada campo se alinea según su tamaño,
con el relleno —padding, que dicen los ingleses— necesario.

### Inicialización

```falcato
el p: Punto = Punto { x: 10, y: 20 };
```

Se nombra cada campo y se le asigna su valor. No hay confusión
posible: cada cosa en su lugar.

### Acceso a campos

```falcato
el px: Entero32 = p.x;
p.x = 30;  // si es mutable, se puede cambiar
```

### Verificación semántica

El compilador es puntilloso con los structs:

- **Campos inexistentes**: error. No vale inventar.
- **Tipos incorrectos**: error. Cada campo tiene su tipo.
- **Campos faltantes**: error. No se dejan cosas sin declarar.

```falcato
Punto { x: 10 }         // Error: falta el campo 'y'
Punto { x: 10, z: 0 }  // Error: el struct no tiene campo 'z'
```

## Enums

Los enums —o enumeraciones— son tipos que pueden ser una cosa
u otra, pero no ambas a la vez. Como el gato de Schrödinger,
pero con tag explícito.

### Declaración

```falcato
enumeración Estado {
    Activo,
    Inactivo,
    Pausado,
}
```

### Variantes con datos

Y si las variantes llevan datos, mejor que mejor:

```falcato
enumeración Resultado {
    Exito(valor: Entero32),
    Error(codigo: Entero32),
}
```

### Constructores

Para crear un valor del enum, se usa la sintaxis de punto,
que es como invocar a la variante por su nombre:

```falcato
el estado: Estado = Estado.Activo;
el resultado: Resultado = Resultado.Exito(42);
```

### Pattern matching

Y para saber qué variante tenemos entre manos, el `si` con `es`
hace las veces de emparejamiento:

```falcato
si estado es Estado.Activo {
    // branch para el estado activo
}
```

### Verificación semántica

El compilador vigila que no haya desmanes:

- **Variante inexistente**: error. El enum no tiene eso.
- **Tipo incorrecto en argumentos**: error. Lo que se pasa ha de concordar.
- **Enum no declarado**: error. No se invocan cosas que no existen.

## Layout en memoria

Para los curiosos que quieran saber cómo yacen estas cosas en
la memoria:

- **Structs**: layout C con alineación natural. Cada campo se coloca
  a partir de un offset múltiplo de su tamaño. Entre campos puede
  haber relleno.

- **Enums**: un tag (entero de 32 bits, 4 bytes) seguido de una
  unión cuyo tamaño es el máximo entre todas las variantes. Así,
  el tag dice qué variante es, y la unión guarda sus datos.
