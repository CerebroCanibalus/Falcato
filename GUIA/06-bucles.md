# 06 — Bucles: mientras, para

← [05: Decisiones](05-decisiones.md) | [Indice](INDICE.md) | [Siguiente: Funciones →](07-funciones.md)

---

## mientras

```falcato
el i: Entero32 = 0;
mientras i < 5 {
    decir("Vuelta {i}");
    i = i + 1;
}
// Vuelta 0, Vuelta 1, ..., Vuelta 4
```

## para

```falcato
// Sobre un rango
para i en 0..5 {
    decir("Numero {i}");  // 0, 1, 2, 3, 4
}

// Sobre un array
los valores: [Entero32; 3] = [10, 20, 30];
para v en valores {
    decir("Valor: {v}");
}
```

## Rangos

```
0..5     // 0, 1, 2, 3, 4  (exclusivo: el 5 no entra)
0..=5    // 0, 1, 2, 3, 4, 5 (inclusivo: el 5 si entra)
```

## Error tipico

```falcato
// Bucle infinito: olvidaste incrementar
mientras i < 10 {
    decir("nunca termina");
    // falta i = i + 1
}
```

---

← [05: Decisiones](05-decisiones.md) | [Indice](INDICE.md) | [Siguiente: Funciones →](07-funciones.md)
