# 04 — Operaciones

← [03: Variables](03-variables.md) | [Indice](INDICE.md) | [Siguiente: Decisiones →](05-decisiones.md)

---

## Aritmeticas

```
10 + 5    // suma           → 15
10 - 5    // resta          → 5
10 * 5    // multiplicación → 50
10 / 3    // división entera → 3 (¡no 3.333!)
10 % 3    // resto          → 1
```

> `10 / 3` da `3`, no `3.333`. Para decimales usa `Flotante64`: `10.0 / 3.0`.

## Comparaciones

Devuelven `Booleano`:

```
10 == 10   // igual          → verdadero
10 != 5    // distinto       → verdadero
10 < 20    // menor          → verdadero
10 > 5     // mayor          → verdadero
10 <= 10   // menor o igual  → verdadero
10 >= 5    // mayor o igual  → verdadero
```

## Logicas

```
verdadero && falso   // y (las dos)       → falso
verdadero || falso    // o (al menos una) → verdadero
!verdadero            // no (lo contrario) → falso
```

## Bit a bit

```
a & b    // AND
a | b    // OR
a ^ b    // XOR
~a       // NOT (invertir bits)
a << 3   // desplazar izquierda (×8)
a >> 2   // desplazar derecha (÷4)
a >>> 2  // desplazar lógico (ceros siempre)
```

## Precendencia

Como en matemáticas: primero `*` `/` `%`, luego `+` `-`.

```
2 + 3 * 4   // 14 (primero 3*4)
(2 + 3) * 4 // 20 (paréntesis primero)
```

Si dudas, usa paréntesis.

---

← [03: Variables](03-variables.md) | [Indice](INDICE.md) | [Siguiente: Decisiones →](05-decisiones.md)
