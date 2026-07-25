# 09 — Colecciones

← [08: Texto y Palabra](08-texto.md) | [Indice](INDICE.md) | [Siguiente: Datos compuestos →](10-datos.md)

---

## Arrays fijos [T; N]

Tamaño fijo, en la pila (rápido).

```falcato
los nums: [Entero32; 3] = [10, 20, 30];

el primero = nums[0];  // 10
nums[1] = 25;           // modificar

para n en nums {
    decir("Numero: {n}");
}
```

## Vector<T> — el array que crece

```falcato
el v: Vector<Entero32> = vector_nuevo();
v.agregar(10);
v.agregar(20);
v.agregar(30);

el primero = v[0];  // 10
el tam = v.tam();   // 3

v.liberar();  // importante
```

## ¿Cuál usar?

| | Array | Vector |
|---|-------|--------|
| Crear | `[10, 20, 30]` | `vector_nuevo()` |
| Añadir | No puede | `v.agregar(x)` |
| Acceder | `arr[0]` | `v[0]` |
| Liberar | No necesita | `v.liberar()` |

Sabes cuantos datos? → Array. Llegan sobre la marcha? → Vector.

---

← [08: Texto y Palabra](08-texto.md) | [Indice](INDICE.md) | [Siguiente: Datos compuestos →](10-datos.md)
