# 05 — Decisiones: si, es, está, fuese

← [04: Operaciones](04-operaciones.md) | [Indice](../GUIA.md) | [Siguiente: Bucles →](06-bucles.md)

---

Tu programa necesita decidir. Falcato tiene dos formas: `si` (el clásico) y `coincidir` (el elegante). Y dentro de `si`, tres modos: `es`, `está`, `fuese`. No es capricho — cada uno le dice al compilador algo distinto.

## si / sino — el básico

```falcato
el temperatura: Entero32 = 30;

si temperatura > 25 {
    decir("Hace calor");
} sino {
    decir("No hace tanto calor");
}
```

Se lee solo: "si pasa esto, haz esto; sino, haz esto otro".

### Varios caminos con sino si

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

Puedes encadenar tantos `sino si` como quieras. El primero que se cumple gana.

### Condiciones dentro de condiciones

```falcato
// Validación de formulario real
el edad: Entero32 = 25;
el tiene_licencia: Booleano = verdadero;
el es_conductor: Booleano = falso;

si edad >= 18 {
    // Entra aquí solo si edad >= 18
    si tiene_licencia {
        si es_conductor {
            decir("Puede alquilar un coche");
        } sino {
            decir("Puede sacar el carnet");
        }
    } sino {
        decir("Necesita sacar teórica primero");
    }
} sino {
    decir("Demasiado joven");
}
```

Cada `si` abre una caja. Las cajas pueden tener más `si` dentro. El compilador las ordena solo.

## es vs está — la joya de Falcato

En español decimos "**es** de noche" y "**está** nublado". Falcato entiende esa diferencia.

Ambos comparan con `==`... pero el **significado** es distinto. Y el compilador lo usa para entender tu intención.

### `es` — identidad, permanente

Usa `es` cuando compares algo que **es** así por naturaleza.

```falcato
si animal es "perro" { ladrar(); }    // es un perro, siempre lo será
si hoy es sabado { decir("Finde"); }  // el día es sábado
si pais es "japon" { decir("Saluda"); } // su identidad
si usuario es "admin" { panel(); }    // su rol
```

### `está` — estado transitorio

Usa `está` para cosas que **cambian** — lecturas de sensores, estados temporales.

```falcato
si sensor está 25 {
    decir("Temperatura normal");  // ahora está en 25, pero puede subir
}
si bateria está baja { cargar(); }   // ahora está baja, luego no
si modo está noche { oscurecer(); }  // modo noche se puede desactivar
```

### `está` desnudo — truthiness (único en Falcato)

```falcato
el x: Entero32 = 42;
si está {         // sin comparar con nada
    decir("x NO es cero");
}

el y: Entero32 = 0;
si y está {
    decir("Esto NO se ejecuta");  // 0 es "falso"
}
```

¿Qué hace? **Evalúa si el valor "existe" o es "verdadero"**:

| Tipo | Es "verdad" si... | Ejemplo |
|------|--------------------|---------|
| `Entero32` | ≠ 0 | `0` → falso, `42` → verdad |
| `Booleano` | es `verdadero` | `verdadero` → verdad, `falso` → falso |
| Puntero | ≠ null | puntero válido → verdad, nulo → falso |
| `Flotante64` | ❌ **No funciona** | error del compilador |

```falcato
fn comprobar_ptr(la ptr: &Entero32) {
    si ptr está {
        decir("El puntero apunta a {*ptr}");
    } sino {
        decir("El puntero es nulo");
    }
}

el conectado: Booleano = verdadero;
mientras está {   // mientras esté conectado
    procesar();
}
```

> **¿Por qué no funciona con floats?** Porque con decimales, "es cero" es ambiguo. ¿0.0001 es cero o no? Falcato no adivina — te obliga a comparar.

### Tabla rápida: es vs está vs fuese

```falcato
si x es 10      // identidad: "eres ese valor"
si x está 10    // estado: "ahora vales 10"
si x está       // truthiness: "existes y no eres cero"
si x fuese es 10 // improbable: "si acaso fueras 10"
```

## fuese — modo subjuntivo

```falcato
si x fuese es 1000 {
    decir("Esto casi nunca se ejecuta");
}
```

`fuese` marca un camino como **improbable**. El compilador lo mueve a una zona fría de memoria. El resto del código (el caso normal) corre más rápido porque tiene todo junto.

Es como decirle al compilador: "esto rara vez pasa, no molestes al procesador con esto".

```falcato
// Ejemplo real: archivo de configuración
// El 99% de las veces el archivo existe
el datos: Texto;
si archivo_existe("config.cfg") fuese {
    // Solo si NO existe — caso raro
    datos = texto_desde("valores por defecto");
} sino {
    datos = archivo_leer("config.cfg");
}
// El camino "archivo existe" está caliente en caché
```

**¿Cuándo usar `fuese`?** Para errores, valores extremos, casos bordes. No para ramas 50/50.

## Mapa de decisiones — diagrama de flujo

```
          ┌─ ¿Qué decisión tomas?
          │
    ┌─────┴─────┐
    │            │
  es/está     coincidir
    │            │
    │       muchas opciones
    │       con el mismo valor?
  ┌─┴─┐          │
  │   │      ┌───┴───┐
 es  está    sí      no
  │    │     │       │
  │    │  coincidir  si/sino
  │    │             │
  │  ┌─┴──┐       ¿solo
  │  │    │       dos
  │ está desnudo  caminos?
  │  (truthiness)    │
  │    │         ┌───┴───┐
  │    │         sí      no
  │    │         │       │
  │    │       si/sino  si/sino si
  │    │                │
  │    │          ┌─────┴─────┐
  │    │          │           │
  │    │       ¿hay un     si no,
  │    │       caso muy    usa si/sino
  │    │       improbable?  normal
  │    │          │
  │    │       ┌──┴──┐
  │    │       sí    no
  │    │       │     │
  │    │   fuese  normal
```

## Emparejar (coincidir / match)

Cuando tienes **muchas opciones** sobre el **mismo valor**, `coincidir` es más limpio que un montón de `si`:

```falcato
coincidir x {
    0 => { decir("cero"); }
    1 => { decir("uno"); }
    2 => { decir("dos"); }
    _ => { decir("otro"); }
}
```

Cada `=>` es una ruta. `_` es el **comodín** — atrapa cualquier valor que no haya coincidido antes.

### Match con enums (el pan de cada día)

```falcato
enumeración ResultadoHttp {
    Ok(datos: Texto),
    NoEncontrado,
    ErrorServidor(codigo: Entero32),
}

fn manejar_respuesta(la res: ResultadoHttp) {
    coincidir res {
        ResultadoHttp.Ok como datos => {
            procesar(datos);
        }
        ResultadoHttp.NoEncontrado => {
            decir("404 — no existe");
        }
        ResultadoHttp.ErrorServidor como cod => {
            decir("Error {cod} del servidor");
        }
    }
}
```

### Match con binding directo: es...como

```falcato
// Cuando solo te interesa UNA variante
si res es ResultadoHttp.Ok como datos {
    procesar(datos);  // 'datos' ya es el Texto de dentro
}
// Si res era ErrorServidor, no entra
```

### Match con rangos

```falcato
coincidir nota {
    0..=49 => { decir("Suspenso"); }
    50..=69 => { decir("Aprobado"); }
    70..=89 => { decir("Notable"); }
    90..=100 => { decir("Sobresaliente"); }
    _ => { decir("Nota inválida"); }
}
```

## Errores típicos

```falcato
// Error: comparar booleanos con ==
si activo es verdadero { }      // funciona pero repetitivo
si activo está { }              // mejor: truthiness directo

// Error: olvidar el comodín
coincidir x {
    1 => { ... }
    2 => { ... }
} // ¿qué pasa si x es 3? Error de compilación

// Error: comparar floats con ==
el a: Flotante64 = 0.1 + 0.2;   // 0.30000000000000004
si a es 0.3 { }                 // ¡FALSO! Precisión finita
// Usa: si abs(a - 0.3) < 0.0001 { }

// Error: confundir = con ==
si x = 5 { }  // Esto es asignación, no comparación
si x es 5 { } // Correcto: comparación
```

## ¿Cuándo usar qué? — tabla expandida

| Situación | Usa | Ejemplo |
|-----------|-----|---------|
| Dos caminos | `si / sino` | `si llueve { ... } sino { ... }` |
| Varios caminos, misma variable | `coincidir` | `coincidir color { ... }` |
| Comparar identidad | `es` | `si animal es "perro"` |
| Estado transitorio | `está` | `si sensor está 25` |
| Saber si "es verdad" | `está` desnudo | `si conectado está { }` |
| Caso raro / improbable | `fuese` | `si error fuese { }` |
| Extraer dato de enum | `es ... como` | `si res es Exito como v` |
| Una de muchas opciones exactas | `coincidir` | menús, estados, comandos |
| Condiciones diferentes | `si / sino si` | `si x > 5 && y < 10` |

---

← [04: Operaciones](04-operaciones.md) | [Indice](../GUIA.md) | [Siguiente: Bucles →](06-bucles.md)
