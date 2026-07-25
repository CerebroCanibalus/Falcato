# 09 — Colecciones: arrays y vectores

← [08: Texto y Palabra](08-texto.md) | [Indice](../GUIA.md) | [Siguiente: Datos compuestos →](10-datos.md)

---

Falcato tiene dos formas de guardar listas de cosas: **arrays** (tamaño fijo, rápidos) y **vectores** (tamaño variable, flexibles). Los dos usan `[índice]` para acceder.

## Array `[T; N]` — tamaño fijo

Los arrays viven en la **pila** (stack). Son rapidísimos pero su tamaño se decide al escribirlos y **no puede cambiar**.

```
Cómo se ve en memoria (array de 5 enteros):

   Dirección baja                    Dirección alta
   ┌────┬────┬────┬────┬────┐
   │ 10 │ 20 │ 30 │ 40 │ 50 │
   └─┬──┴─┬──┴─┬──┴─┬──┴─┬──┘
     │    │    │    │    │
   arr[0] [1] [2] [3] [4]

   Todo seguido, en el stack.
   El tamaño (5) se sabe en compilación.
```

```falcato
// Crear
los numeros: [Entero32; 5] = [10, 20, 30, 40, 50];

// Acceder
el primero = numeros[0];  // 10

// Modificar (si es 'el')
numeros[1] = 25;

// Recorrer
para n en numeros {
    decir("Número: {n}");
}

// Recorrer con índice
para i en 0..5 {
    decir("Posición {i}: {numeros[i]}");
}
```

### Inicializar con `todos`

```falcato
los ceros: [Entero32; 100] = todos 0;    // 100 ceros
los unos: [Entero32; 50] = todos -1;     // 50 unos
```

### Copiar un array

```falcato
los original: [Entero32; 3] = [1, 2, 3];
los copia: [Entero32; 3] = copiar original;
// Ahora son independientes. Modificar copia no afecta original.
```

### Arrays en la vida real

```falcato
// Días de la semana — siempre 7
los DIAS: [Palabra; 7] = ["Lu", "Ma", "Mi", "Ju", "Vi", "Sa", "Do"];

// Paleta de colores RGB fija
los PALETA: [[Entero8; 3]; 4] = [
    [255, 0, 0],    // rojo
    [0, 255, 0],    // verde
    [0, 0, 255],    // azul
    [255, 255, 0],  // amarillo
];
// paleta[1][2] → 0 (el verde no tiene azul)

// Buffer de sensor — sabes que llegan exactamente 1024 bytes
el buffer: [Entero8; 1024];
leer_sensor(&buffer);  // llena el buffer

// Historial fijo — siempre los últimos 10 valores
el historial: [Entero32; 10];
para i en 0..9 {
    historial[i] = historial[i + 1];  // desplazar
}
historial[9] = nuevo_valor;
```

### Matrices 2D (arrays de arrays)

```falcato
// Un tablero 3×3
los tablero: [[Entero32; 3]; 3] = [
    [1, 2, 3],
    [4, 5, 6],
    [7, 8, 9],
];

el centro = tablero[1][1];  // 5

// Recorrer filas y columnas
para fila en 0..3 {
    para col en 0..3 {
        imprimir("{tablero[fila][col]} ");
    }
    imprimir_linea("");
}
// → 1 2 3
//   4 5 6
//   7 8 9
```

## Vector `<T>` — tamaño variable

Los vectores viven en el **heap**. Pueden **crecer** con `agregar` pero hay que **liberarlos** con `liberar()`.

```
Cómo se ve en memoria:

     Stack                     Heap
   ┌──────────┐           ┌────┬────┬────┐
   │ ptr ────────────────→│ 10 │ 20 │ 30 │
   ├──────────┤           └────┴────┴────┘
   │ len: 3   │
   ├──────────┤           Capacidad actual: 4
   │ cap: 4   │           (si agregas otro más,
   └──────────┘            se reasigna a 8)

   Son 24 bytes fijos      Los datos están aparte,
   en el stack.            en el heap, y pueden
                            reubicarse al crecer.
```

```falcato
el v: Vector<Entero32> = vector_nuevo();
v.agregar(10);
v.agregar(20);
v.agregar(30);

el primero = v[0];     // 10
el cantidad = v.tam();  // 3

// Recorrer
para val en v {
    decir("Valor: {val}");
}

// Recorrer con índice
para i en 0..v.tam() {
    decir("Posición {i}: {v[i]}");
}

v.liberar();  // ← SIEMPRE
```

## Patrones reales con colecciones

### Acumular (sumar todos)

```falcato
fn sumar_array(nums: [Entero32; 5]) -> Entero32 {
    el total: Entero32 = 0;
    para n en nums {
        total = total + n;
    }
    retornar total;
}

fn sumar_vector(v: &Vector<Entero32>) -> Entero32 {
    el total: Entero32 = 0;
    para i en 0..v.tam() {
        total = total + v[i];
    }
    retornar total;
}
```

### Buscar (encontrar elemento)

```falcato
fn buscar(nums: [Entero32; 5], el buscado: Entero32) -> Entero32 {
    para i en 0..5 {
        si nums[i] es buscado {
            retornar i;  // devuelve la posición
        }
    }
    retornar -1;  // no encontrado
}

fn buscar_vector(v: &Vector<Entero32>, buscado: Entero32) -> Entero32 {
    para i en 0..v.tam() {
        si v[i] es buscado {
            retornar i;
        }
    }
    retornar -1;
}
```

### Filtrar (quedarse con algunos)

```falcato
// Dado un vector de notas, quedarse solo con las aprobadas
fn aprobados(la notas: &Vector<Entero32>) -> Vector<Entero32> {
    el resultado: Vector<Entero32> = vector_nuevo();
    para i en 0..notas.tam() {
        si notas[i] >= 50 {
            resultado.agregar(notas[i]);
        }
    }
    retornar resultado;  // quien llama recibe el nuevo vector
}

fn main() {
    el notas: Vector<Entero32> = vector_nuevo();
    notas.agregar(30);
    notas.agregar(70);
    notas.agregar(45);
    notas.agregar(85);

    el buenas = aprobados(&notas);  // prestamos, no movemos

    para i en 0..buenas.tam() {
        decir("Aprobado: {buenas[i]}");
    }
    // → 70, 85

    buenas.liberar();
    notas.liberar();
}
```

### Programa completo: procesar notas

```falcato
función principal() -> Entero32 {
    // Recolectar datos
    el notas: Vector<Entero32> = vector_nuevo();
    notas.agregar(45);
    notas.agregar(80);
    notas.agregar(60);
    notas.agregar(30);
    notas.agregar(90);

    // Calcular promedio
    el suma: Entero32 = 0;
    para i en 0..notas.tam() {
        suma = suma + notas[i];
    }
    el promedio = suma / notas.tam();
    decir("Promedio: {promedio}");

    // Buscar la máxima
    el maxima: Entero32 = 0;
    para i en 0..notas.tam() {
        si notas[i] > maxima {
            maxima = notas[i];
        }
    }
    decir("Nota máxima: {maxima}");

    // Contar aprobados
    el aprobados: Entero32 = 0;
    para i en 0..notas.tam() {
        si notas[i] >= 50 {
            aprobados = aprobados + 1;
        }
    }
    decir("Aprobados: {aprobados}");

    notas.liberar();
    retornar 0;
}
```

## Errores típicos

```falcato
// Error: índice fuera de rango
los arr: [Entero32; 3] = [1, 2, 3];
arr[5] = 99;  // ¡CRASH! Solo hay posiciones 0,1,2

// Error: confundir array con vector
el datos: [Entero32; 3] = [1, 2, 3];
datos.agregar(4);  // Error: los arrays no tienen agregar

// Error: olvidar liberar vector
fn perder_memoria() {
    el v: Vector<Entero32> = vector_nuevo();
    v.agregar(42);
    // falta v.liberar() → PÉRDIDA DE MEMORIA
}

// Error: usar array para datos dinámicos
fn cuantos_usuarios(cantidad: Entero32) {
    el usuarios: [Entero32; 10] = ???;  // ¿y si son más de 10?
    // Usa Vector<Entero32> mejor
}
```

## ¿Array o Vector? — tabla expandida

| Situación | Array | Vector | ¿Por qué? |
|-----------|-------|--------|-----------|
| Días de la semana | ✅ | ❌ | Siempre 7, fijo |
| Buffer de 1024 bytes | ✅ | ❌ | El hardware espera exactamente eso |
| Lectura de archivo | ❌ | ✅ | No sabes cuántas líneas tiene |
| Lista de usuarios conectados | ❌ | ✅ | Van y vienen |
| Paleta de 4 colores | ✅ | ❌ | Son 4, punto |
| Matriz 3×3 para juegos | ✅ | ❌ | Tamaño conocido |
| Dibujar píxeles en pantalla | ✅ ❌ | Depende | Si es resolución fija → array |
| Cache de resultados | ❌ | ✅ | No sabes cuántos serán |

**Regla de oro:** Si sabes el número antes de ejecutar → array. Si los datos llegan vivos → vector.

## Tabla rápida (referencia)

| | Array `[T; N]` | Vector `Vector<T>` |
|---|-------|--------|
| Crear | `[10, 20, 30]` | `vector_nuevo::<T>()` |
| Añadir | ❌ No puede | `v.agregar(x)` |
| Acceder | `arr[0]` | `v[0]` |
| Recorrer | `para n en arr` | `para n en v` |
| Con índice | `para i en 0..N` | `para i en 0..v.tam()` |
| Tamaño | Fijo en tipo | `v.tam()` |
| Copiar | `copiar arr` | — |
| Liberar | No necesita | `v.liberar()` |
| Memoria | Stack (rápida) | Heap (flexible) |

---

← [08: Texto y Palabra](08-texto.md) | [Indice](../GUIA.md) | [Siguiente: Datos compuestos →](10-datos.md)
