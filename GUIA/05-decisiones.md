# 05 — Decisiones: si, es, está

← [04: Operaciones](04-operaciones.md) | [Indice](INDICE.md) | [Siguiente: Bucles →](06-bucles.md)

---

## si / sino

```falcato
el temperatura: Entero32 = 30;

si temperatura > 25 {
    decir("Hace calor");
} sino {
    decir("No hace tanto calor");
}
```

## es vs está — la joya de Falcato

En español decimos "**es** de noche" (permanente) y "**está** nublado" (temporal).

```falcato
si x es 10 {            // identidad: "x es 10, siempre"
    decir("exactamente 10");
}

si x está 10 {          // estado: "ahora está en 10"
    decir("temporalmente en 10");
}
```

Ambos comparan igual, pero el código queda más expresivo.

## fuese — "casi nunca pasa"

```falcato
si x fuese es 100 {
    decir("Esto casi nunca se ejecuta");
}
```

El compilador mueve ese código a una zona "fría". El camino normal corre más rápido.

## Condiciones compuestas

```falcato
si x > 0 && x < 100 {
    decir("x está entre 0 y 100");
}
```

## Emparejar (coincidir)

Para muchas opciones:

```falcato
coincidir x {
    0 => { decir("cero"); }
    1 => { decir("uno"); }
    _ => { decir("otro"); }  // comodín
}
```

`_` es el **comodín**: atrapa cualquier valor no cubierto.

---

← [04: Operaciones](04-operaciones.md) | [Indice](INDICE.md) | [Siguiente: Bucles →](06-bucles.md)
