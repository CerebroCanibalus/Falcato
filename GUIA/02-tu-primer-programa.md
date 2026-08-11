# 02 — Tu primer programa

← [01: Que es Falcato?](01-que-es-falcato.md) | [Indice](../GUIA.md) | [Siguiente: Variables →](03-variables.md)

---

Crea un archivo `hola.fc`:

```falcato
función principal() -> Entero32 {
    decir("¡Hola, mundo!");
    retornar 0;
}
```

Ejecuta:

```bash
falcato corre hola.fc
```

Verás:

```
¡Hola, mundo!
```

## ¿Qué acaba de pasar?

| Código | Significado |
|--------|-------------|
| `función principal() -> Entero32` | Punto de entrada del programa. Devuelve un número entero. |
| `decir("...")` | Imprime en pantalla con salto de línea |
| `retornar 0` | Devuelve 0 al sistema. 0 = todo bien. |

## ¿No funciona?

- **"falcato no se reconoce"** → [INSTALL.md](../INSTALL.md)
- **Error `[S001]`** → falta un `;` en algún lado
- **Error `[T001]`** → los tipos no coinciden

---

← [01: Que es Falcato?](01-que-es-falcato.md) | [Indice](../GUIA.md) | [Siguiente: Variables →](03-variables.md)
