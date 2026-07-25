# 04 — Operaciones

← [03: Variables](03-variables.md) | [Indice](../GUIA.md) | [Siguiente: Decisiones →](05-decisiones.md)

---

## Aritméticas

```falcato
10 + 5    // suma           → 15
10 - 5    // resta          → 5
10 * 5    // multiplicación → 50
10 / 3    // división entera → 3 (¡no 3.333!)
10 % 3    // resto          → 1
```

> `10 / 3` da `3`, no `3.333`. Falcato divide enteros como en C — el resultado trunca el decimal. Para divisiones exactas usa `Flotante64`: `10.0 / 3.0`.

### División entera en la vida real

```falcato
// Repartir objetos entre personas
el galletas: Entero32 = 10;
el personas: Entero32 = 3;
el cada_una = galletas / personas;  // 3 (sobran)
el sobran = galletas % personas;    // 1

// Saber si un número es par
si numero % 2 es 0 {
    decir("Es par");
}

// Extraer dígitos de un número
el numero: Entero32 = 1234;
el unidades = numero % 10;        // 4
el decenas = (numero / 10) % 10;  // 3
```

## Comparaciones

Devuelven `Booleano` (`verdadero` o `falso`):

```falcato
10 == 10   // igual          → verdadero
10 != 5    // distinto       → verdadero
10 < 20    // menor          → verdadero
10 > 5     // mayor          → verdadero
10 <= 10   // menor o igual  → verdadero
10 >= 5    // mayor o igual  → verdadero
```

## Lógicas

```falcato
verdadero && falso   // y (las dos)       → falso
verdadero || falso    // o (al menos una) → verdadero
!verdadero            // no (lo contrario) → falso

// Ejemplo real
si edad >= 18 && tiene_licencia {
    decir("Puede conducir");
}
```

## Bit a bit

Trabajan **bit por bit** sobre el número. Solo con enteros.

```falcato
// Suponiendo a = 6 (110 en binario), b = 3 (011)
a & b    // AND:  110 & 011 = 010 → 2
a | b    // OR:   110 | 011 = 111 → 7
a ^ b    // XOR:  110 ^ 011 = 101 → 5
~a       // NOT:  ~110 = ...001 → -7 (en complemento a 2)
a << 1   // shift izq: 110 << 1 = 1100 → 12 (multiplica por 2)
a >> 1   // shift der: 110 >> 1 = 11 → 3 (divide por 2)
a >>> 1  // shift lógico: 110 >>> 1 = 011 → 3 (ceros a la izquierda)
```

### Bitwise en la vida real (flags y máscaras)

```falcato
// Permisos de archivo (como Linux)
el PERMISO_LECTURA:   Entero32 = 1 << 0;  // 001 = 1
el PERMISO_ESCRITURA: Entero32 = 1 << 1;  // 010 = 2
el PERMISO_EJECUTAR:  Entero32 = 1 << 2;  // 100 = 4

el permisos: Entero32 = 0;
permisos = permisos | PERMISO_LECTURA;    // activar lectura
permisos = permisos | PERMISO_EJECUTAR;   // activar ejecución
// permisos = 101 = 5

si permisos & PERMISO_LECTURA != 0 {
    decir("Tiene permiso de lectura");
}

// Limpiar un flag
permisos = permisos & ~PERMISO_EJECUTAR;  // quitar ejecución
// permisos = 001 = 1
```

### Extraer bits

```falcato
// Métodos built-in (más legibles que operadores)
el x: Entero32 = 0b1101;

x.poner_bit(1);           // activa bit 1: 0b1111
x.quitar_bit(0);          // desactiva bit 0: 0b1110
x.alternar_bit(2);        // invierte bit 2: 0b1010
x.unos();                 // cuenta bits en 1 → 3
x.ceros_izquierda();      // cuenta ceros a la izquierda
x.extraer_bits(2, 3);     // extrae bits 2-4 como número
```

## Precedencia

Como en matemáticas: `* / %` antes que `+ -`. Como en C para bitwise.

```
2 + 3 * 4     // 14  (primero 3*4)
(2 + 3) * 4   // 20  (paréntesis primero)
1 << 2 + 3    // 32  (2+3=5, luego 1<<5) — ¡cuidado!
1 << (2 + 3)  // 32  (más claro con paréntesis)
```

Precedencia completa (de mayor a menor):
```
*  /  %
+  -
<<  >>
&
^
|
&&
||
```

Si dudas, **usa paréntesis**. Siempre es más legible.

---

← [03: Variables](03-variables.md) | [Indice](../GUIA.md) | [Siguiente: Decisiones →](05-decisiones.md)
