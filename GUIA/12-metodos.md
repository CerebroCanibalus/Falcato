# 12 — Métodos: sintaxis .nombre()

← [11: Errores](11-errores.md) | [Indice](../GUIA.md) | [Siguiente: Async →](13-async.md)

---

```falcato
// Antes (funciona):
texto_agregar(t, "hola");

// Ahora (mas natural):
t.agregar("hola");
```

## Métodos disponibles

### Texto

| Código | Equivale a |
|--------|------------|
| `t.agregar("hola")` | `texto_agregar(t, "hola")` |
| `t.tam()` | `texto_longitud(t)` |
| `t.liberar()` | `texto_liberar(t)` |
| `t.obtener(i)` | `texto_obtener_byte(t, i)` |
| `t.concatenar(b)` | `texto_concatenar(t, b)` |
| `t.subtexto(i, f)` | `texto_subtexto(t, i, f)` |

### Vector<T>

| Código | Equivale a |
|--------|------------|
| `v.agregar(x)` | `vector_agregar(v, x)` |
| `v.tam()` | `vector_longitud(v)` |
| `v.obtener(i)` | `vector_obtener(v, i)` |
| `v.liberar()` | `vector_liberar(v)` |

### Enteros (bitwise)

| Código | Efecto |
|--------|--------|
| `x.poner_bit(3)` | Pone el bit 3 en 1 |
| `x.quitar_bit(3)` | Pone el bit 3 en 0 |
| `x.alternar_bit(3)` | Cambia el bit 3 |
| `x.unos()` | Cuenta bits en 1 |

## Operadores

| Operación | Significado |
|-----------|-------------|
| `a + b` con Texto | Concatena |
| `t[0]` | Byte 0 del Texto |
| `t[0..5]` | Rebanada de Texto |
| `v[0]` | Elemento 0 del Vector |

## Ejemplo completo

```falcato
función principal() -> Entero32 {
    el t: Texto = texto_desde("Hola");
    t.agregar(", mundo");

    decir(t);                  // "Hola, mundo"
    decir("Tamaño: {t.tam()}"); // 11

    el byte: Entero8 = t[0];   // 'H'
    decir("Primer byte: {byte}");

    t.liberar();
    retornar 0;
}
```

---

← [11: Errores](11-errores.md) | [Indice](../GUIA.md) | [Siguiente: Async →](13-async.md)
