# 09 — Colecciones: arrays y vectores

← [08: Texto y Palabra](08-texto.md) | [Indice](../GUIA.md) | [Siguiente: Datos compuestos →](10-datos.md)

---

Falcato tiene dos formas de guardar listas de cosas: **arrays** (tamaño fijo, rápidos) y **vectores** (tamaño variable, flexibles).

## Array `[T; N]` — tamaño fijo

Los arrays viven en la pila (stack). Son **rápidos** pero su tamaño **no puede cambiar**.

```falcato
// Crear
los numeros: [Entero32; 5] = [10, 20, 30, 40, 50];

// Acceder
el primero = numeros[0];  // 10

// Modificar (si el array es 'el')
numeros[1] = 25;

// Recorrer
para n en numeros {
    decir("Numero: {n}");
}
```

### Inicializar con `todos`

```falcato
los ceros: [Entero32; 100] = todos 0;   // 100 ceros
los unos: [Entero32; 50] = todos -1;    // 50 unos
```

### Matrices 2D (arrays anidados)

```falcato
los matriz: [[Entero32; 3]; 3] = [
    [1, 2, 3],
    [4, 5, 6],
    [7, 8, 9],
];

el centro = matriz[1][1];  // 5
```

### ¿Cuándo usar arrays en la vida real?

```falcato
// Días de la semana — siempre son 7
los DIAS: [Palabra; 7] = ["Lu", "Ma", "Mi", "Ju", "Vi", "Sa", "Do"];

// Buffer fijo para datos de sensor
el buffer: [Entero8; 1024];  // 1 KB de buffer
// Leer exactamente 1024 bytes del hardware

// Paleta de colores fija
los COLORES: [[Entero8; 3]; 4] = [
    [255, 0, 0],      // rojo
    [0, 255, 0],      // verde
    [0, 0, 255],      // azul
    [255, 255, 0],    // amarillo
];
```

## Vector `<T>` — tamaño variable

Los vectores viven en el heap. Usan memoria dinámica: **pueden crecer** (con `agregar`) pero hay que **liberarlos**.

```falcato
el v: Vector<Entero32> = vector_nuevo();
v.agregar(10);
v.agregar(20);
v.agregar(30);

el primero = v[0];    // 10
el cantidad = v.tam(); // 3

para val en v {
    decir("Valor: {val}");
}

v.liberar();  // ← ¡importante! Sin esto, pierdes memoria
```

### ¿Cuándo usar vectores?

```falcato
// No sabes cuántos datos van a llegar
el usuarios: Vector<Entero32> = vector_nuevo();

fn conectar_usuario(id: Entero32) {
    usuarios.agregar(id);  // cada vez que alguien se conecta
}

fn desconectar_todos() {
    usuarios.liberar();  // limpiar al cerrar
}

// Procesar líneas de un archivo
el lineas: Vector<Texto> = vector_nuevo();
el contenido: Texto = archivo_leer("datos.txt");
// ... procesar cada línea y agregarla a 'lineas'
lineas.liberar();
contenido.liberar();
```

## ¿Array o Vector?

| Situación | Array | Vector |
|-----------|-------|--------|
| Sabes cuántos datos desde el principio | ✅ | ❌ (pagas overhead innecesario) |
| Los datos llegan de a uno | ❌ | ✅ |
| Es una constante (días de la semana) | ✅ | ❌ |
| Necesitas buffer de tamaño fijo | ✅ | ❌ |
| No sabes cuántos serán | ❌ | ✅ |
| Máximo rendimiento (stack vs heap) | ✅ | ❌ |
| Memoria flexible | ❌ | ✅ |

**Regla práctica:** Si sabes el tamaño exacto → array. Si los datos llegan sobre la marcha → vector.

## Tabla rápida

| | Array `[T; N]` | Vector `Vector<T>` |
|---|-------|--------|
| Crear | `[10, 20, 30]` | `vector_nuevo()` |
| Añadir | No puede | `v.agregar(x)` |
| Acceder | `arr[0]` | `v[0]` |
| Tamaño | Fijo (N) | `v.tam()` |
| Liberar | No necesita | `v.liberar()` |
| Memoria | Stack | Heap |

---

← [08: Texto y Palabra](08-texto.md) | [Indice](../GUIA.md) | [Siguiente: Datos compuestos →](10-datos.md)
