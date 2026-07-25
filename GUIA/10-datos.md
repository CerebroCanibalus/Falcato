# 10 — Datos compuestos: structs y enums

← [09: Colecciones](09-colecciones.md) | [Indice](INDICE.md) | [Siguiente: Errores →](11-errores.md)

---

## Structs

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
p.edad = 31;  // se puede cambiar
```

## Enums

```falcato
enumeración Estado {
    Activo,
    Inactivo,
}

el estado: Estado = Estado.Activo;
```

### Con datos

```falcato
enumeración Resultado {
    Exito(valor: Entero32),
    Error(codigo: Entero32),
}
```

### Coincidir

```falcato
coincidir estado {
    Estado.Activo => { decir("Funcionando"); }
    Estado.Inactivo => { decir("Detenido"); }
    _ => { decir("Ni idea"); }
}
```

`_` es el **comodín**: atrapa cualquier caso no cubierto.

### Binding con "como"

```falcato
si res es Resultado.Exito como valor {
    decir("Ganamos: {valor}");
}
```

---

← [09: Colecciones](09-colecciones.md) | [Indice](INDICE.md) | [Siguiente: Errores →](11-errores.md)
