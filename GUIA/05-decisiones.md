# 05 — Decisiones: si, es, está

← [04: Operaciones](04-operaciones.md) | [Indice](../GUIA.md) | [Siguiente: Bucles →](06-bucles.md)

---

## si / sino — el básico

```falcato
el temperatura: Entero32 = 30;

si temperatura > 25 {
    decir("Hace calor");
} sino {
    decir("No hace tanto calor");
}
```

### Varios caminos

```falcato
el nota: Entero32 = 85;

si nota >= 90 {
    decir("Sobresaliente");
} sino si nota >= 70 {
    decir("Notable");
} sino si nota >= 50 {
    decir("Aprobado");
} sino {
    decir("Suspenso");
}
```

## es vs está — la joya de Falcato

En español decimos "**es** de noche" (permanente) y "**está** nublado" (temporal). Falcato entiende esa diferencia.

```falcato
// es — identidad, permanente
si x es 10 {
    decir("x es exactamente 10");
}

si x es y {
    decir("x y y son el mismo valor");
}

// está — estado, temporal
si x está 10 {
    decir("x está en 10 ahora");
}
```

En la práctica ambos comparan con `==`, pero el código comunica **intención**:

```falcato
// es: cosas que son por naturaleza
si animal es "perro" { decir("Es un perro"); }  // su especie
si hoy es sabado  { decir("Finde"); }             // el día es
si pais es "japon" { decir("Saluda"); }           // su identidad

// está: cosas que cambian
si sensor está 25 { decir("Temperatura normal"); }  // lectura actual
si bateria está baja { decir("Cargar"); }            // estado pasajero
```

## fuese — "casi nunca pasa" (subjuntivo)

```falcato
si x fuese es 1000 {
    decir("Esto casi nunca se ejecuta");
    // El compilador mueve este código a una zona "fría"
    // El camino normal (cuando x ≠ 1000) corre más rápido
}
```

Usa `fuese` para **casos raros**: errores, valores extremos, configuraciones extrañas. El compilador optimiza el camino "normal" para que sea más rápido.

```falcato
// Ejemplo real: el caso común es que el archivo exista
el datos: Texto;
si archivo fuese no_existe {
    datos = texto_desde("default");
} sino {
    datos = archivo_leer("config.cfg");
}
// El camino "archivo existe" queda en línea (caliente)
// El camino "archivo no existe" va a zona fría
```

## Condiciones compuestas

```falcato
si x > 0 && x < 100 {
    decir("x está entre 0 y 100");
}

si usuario es "admin" || usuario es "mod" {
    decir("Tienes permisos");
}

si !activo {
    decir("Está desactivado");
}
```

## Emparejar (coincidir / match)

Cuando tienes **muchas opciones**, `coincidir` es más limpio que muchos `si`:

```falcato
coincidir x {
    0 => { decir("cero"); }
    1 => { decir("uno"); }
    2 => { decir("dos"); }
    _ => { decir("otro"); }  // comodín
}
```

`_` es el **comodín**: atrapa cualquier valor no cubierto.

### Match con enums (el caso más útil)

```falcato
enumeración Estado {
    Activo,
    Inactivo,
    Error(codigo: Entero32),
}

el estado: Estado = Estado.Activo;

coincidir estado {
    Estado.Activo => { decir("Funcionando"); }
    Estado.Inactivo => { decir("Detenido"); }
    Estado.Error como cod => {
        decir("Error {cod}");  // extraes el dato asociado
    }
}
```

### Binding con "como"

```falcato
// Extraer el valor de un enum con 'como'
si res es Resultado.Exito como valor {
    decir("Ganamos: {valor}");
    // 'valor' es el Entero32 dentro de Exito
}
```

## ¿Cuándo usar qué?

| Situación | Usa |
|-----------|-----|
| Dos caminos | `si / sino` |
| Muchos caminos con el mismo valor | `coincidir` |
| Comparar identidad | `es` |
| Comparar estado transitorio | `está` |
| La mayoría de veces no se cumple | `fuese` |
| Extraer datos de un enum | `coincidir ... como` o `es ... como` |

---

← [04: Operaciones](04-operaciones.md) | [Indice](../GUIA.md) | [Siguiente: Bucles →](06-bucles.md)
