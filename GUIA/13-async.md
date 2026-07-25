# 13 — Async: varias cosas a la vez

← [12: Metodos](12-metodos.md) | [Indice](../GUIA.md) | [Siguiente: Ownership →](14-ownership.md)

---

Falcato puede hacer varias cosas **al mismo tiempo** con threads reales.

## fut funcion

```falcato
fut función trabajador(la id: Entero32) -> Entero32 {
    esperar dormir(1000);   // espera 1 segundo
    retornar id * 2;
}
```

## lanzar

```falcato
función principal() -> Entero32 {
    lanzar trabajador(1);   // otro hilo
    lanzar trabajador(2);   // otro hilo

    esperar dormir(1500);   // espera a que terminen
    retornar 0;
}
```

## Canales

```falcato
la canal: Entero64 = canal_nuevo(16);
canal_enviar(canal, 42);
el valor = canal_recibir(canal);
canal_cerrar(canal);
```

| Funcion | Que hace |
|---------|----------|
| `canal_nuevo(cap)` | Crea canal con capacidad |
| `canal_enviar(c, v)` | Envia valor |
| `canal_recibir(c)` | Recibe (bloquea) |
| `canal_cerrar(c)` | Cierra y libera |

## seleccionar

```falcato
seleccionar {
    canal_a como valor => { decir("Llego del A: {valor}"); }
    canal_b como valor => { decir("Llego del B: {valor}"); }
    _ => { decir("Ninguno listo"); }
}
```

## con_executor

```falcato
con_executor(4) {          // 4 hilos en pool
    lanzar tarea(1);
    lanzar tarea(2);
    esperar dormir(500);
    cancelar();             // cancela lo que falte
}
```

---

← [12: Metodos](12-metodos.md) | [Indice](../GUIA.md) | [Siguiente: Ownership →](14-ownership.md)
