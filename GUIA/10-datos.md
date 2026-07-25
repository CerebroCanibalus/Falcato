# 10 — Datos compuestos: structs y enums

← [09: Colecciones](09-colecciones.md) | [Indice](../GUIA.md) | [Siguiente: Errores →](11-errores.md)

---

Cuando los números sueltos no bastan, agrupas datos en **structs** (estructuras) y **enums** (enumeraciones).

## Struct — agrupar datos relacionados

```falcato
estructural Persona {
    nombre: Palabra,
    edad: Entero32,
}

el p: Persona = Persona {
    nombre: "Ana",
    edad: 30,
};

decir("{p.nombre} tiene {p.edad} años");
p.edad = 31;  // los campos de 'el' se pueden cambiar
```

### Structs anidados

```falcato
estructural Direccion {
    calle: Palabra,
    numero: Entero32,
}

estructural Persona {
    nombre: Palabra,
    direccion: Direccion,
}

el p: Persona = Persona {
    nombre: "Ana",
    direccion: Direccion { calle: "Calle Mayor", numero: 42 },
};

decir("{p.nombre} vive en {p.direccion.calle}");
```

### ¿Para qué sirven los structs en la vida real?

```falcato
// Una petición HTTP
estructural Peticion {
    metodo: Palabra,        // "GET", "POST"
    ruta: Palabra,          // "/api/usuarios"
    cuerpo: Texto,          // datos del body
}

fn procesar_peticion(la req: Peticion) -> Entero32 {
    si req.metodo es "GET" {
        return buscar(req.ruta);
    }
    si req.metodo es "POST" {
        return crear(req.ruta, req.cuerpo);
    }
    retornar 404;
}

// Un punto en 2D (útil para juegos, gráficos)
estructural Punto {
    x: Flotante64,
    y: Flotante64,
}

fn distancia(origen: Punto, destino: Punto) -> Flotante64 {
    el dx = destino.x - origen.x;
    el dy = destino.y - origen.y;
    retornar raiz(dx * dx + dy * dy);
}
```

## Enum — un valor entre varias opciones

Un enum es un tipo que **solo puede ser uno de varios valores** posibles.

```falcato
enumeración Estado {
    Activo,
    Inactivo,
}

el estado: Estado = Estado.Activo;
```

### Con datos asociados

Cada variante puede llevar datos diferentes:

```falcato
enumeración Resultado {
    Exito(valor: Entero32),      // trae un número
    Error(codigo: Entero32),     // trae un código
}

el exito: Resultado = Resultado.Exito(200);
el error: Resultado = Resultado.Error(404);
```

### Ejemplos reales de enums

```falcato
// Una máquina de estados
enumeración FaseJuego {
    Menu,
    Jugando(puntuacion: Entero32),
    Pausa,
    GameOver,
}

fn actualizar_juego(la fase: FaseJuego) {
    coincidir fase {
        FaseJuego.Menu => { mostrar_menu(); }
        FaseJuego.Jugando como puntos => {
            actualizar_partida(puntos);
        }
        FaseJuego.Pausa => { mostrar_pausa(); }
        FaseJuego.GameOver => { mostrar_fin(); }
    }
}

// Resultado de una operación (muy común)
enumeración FalloArchivo {
    NoEncontrado,
    PermisoDenegado,
    ErrorLectura(codigo: Entero32),
}
```

### Pattern matching con `coincidir` y `es...como`

```falcato
coincidir res {
    Resultado.Exito como valor => {
        decir("Todo bien: {valor}");
    }
    Resultado.Error como cod => {
        decir("Error: {cod}");
    }
}

// También con 'es' y 'como' directamente
si res es Resultado.Exito como valor {
    decir("Ganamos: {valor}");
}
```

## Bitfields — registros de hardware

Cuando trabajas con hardware, los registros son bits individuales. Falcato permite declararlos como campos:

```falcato
estructural RegistroUART {
    bits {
        habilitado: Natural8,    // bit 0: 1 bit
        modo_tx: Natural8,       // bit 1: 1 bit
        baud_div: Natural16,     // bits 2-3: 2 bits
    }
}

fn configurar_uart(reg: &mut RegistroUART) {
    reg.habilitado = 1;         // el compilador genera shifts+masks
    reg.baud_div = 2;
}
```

El compilador genera automáticamente las máscaras y desplazamientos. El código es más legible que `REG |= (1 << 3)`.

## ¿Struct o Enum?

| Situación | Usa |
|-----------|-----|
| Varios campos que van juntos | `estructural` |
| Un valor que puede ser de varios tipos | `enumeración` |
| Representar una máquina de estados | `enumeración` |
| Agrupar datos de una entidad | `estructural` |
| Resultado de operación (éxito/error) | `enumeración` |

---

← [09: Colecciones](09-colecciones.md) | [Indice](../GUIA.md) | [Siguiente: Errores →](11-errores.md)
