# 11 — Errores: cuando las cosas salen mal

← [10: Datos compuestos](10-datos.md) | [Indice](INDICE.md) | [Siguiente: Métodos →](12-metodos.md)

---

## Resultado <T, E> — éxito o error

```falcato
función dividir(el a: Entero32, el b: Entero32) -> Resultado<Entero32, Entero32> {
    si b es 0 {
        retornar Resultado.Error(-1);
    }
    retornar Resultado.Exito(a / b);
}
```

## Usar el resultado

```falcato
el res = dividir(10, 2);

coincidir res {
    Resultado.Exito como valor => {
        decir("Funciono: {valor}");
    }
    Resultado.Error como cod => {
        decir("Fallo: {cod}");
    }
}
```

## El operador ? — "y si falla?"

```falcato
función procesar() -> Resultado<Entero32, Entero32> {
    el valor = dividir(10, 0)?;  // si falla, retorna el error
    retornar Resultado.Exito(valor * 2);
}
```

`?` es como decir "**¿**estoy seguro?**¿**" — si hay error, lo devuelves.

## Errores del compilador

```
[T001] archivo.fc:7:12: Disconcordancia de tipo
       | sugerencia: Cambia el tipo o el valor
```

| Código | Categoría |
|--------|-----------|
| `[S###]` | Sintaxis (algo mal escrito) |
| `[T###]` | Tipo (no concuerdan) |
| `[O###]` | Ownership (propiedad) |
| `[M###]` | Modulos (importación) |

Para la lista completa: [ERRORES.md](../ERRORES.md)

---

← [10: Datos compuestos](10-datos.md) | [Indice](INDICE.md) | [Siguiente: Métodos →](12-metodos.md)
