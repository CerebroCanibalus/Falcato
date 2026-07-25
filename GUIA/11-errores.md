# 11 — Errores: cuando las cosas salen mal

← [10: Datos compuestos](10-datos.md) | [Indice](../GUIA.md) | [Siguiente: Métodos →](12-metodos.md)

---

Los programas fallan. Archivos que no existen, redes que se caen, divisiones por cero. Falcato maneja estos casos con `Resultado<T, E>`.

## Resultado — éxito o error

`Resultado` es un enum con dos variantes:

- `Resultado.Exito(valor)` — todo bien, aquí está el dato
- `Resultado.Error(codigo)` — algo falló, aquí está la razón

```falcato
function dividir(el a: Entero32, el b: Entero32) -> Resultado<Entero32, Entero32> {
    si b es 0 {
        retornar Resultado.Error(-1);  // no se puede dividir por cero
    }
    retornar Resultado.Exito(a / b);
}
```

**`Resultado<Entero32, Entero32>`** significa: "éxito da un Entero32, error da otro Entero32".

## Usar el resultado

```falcato
el res = dividir(10, 2);

coincidir res {
    Resultado.Exito como valor => {
        decir("Funcionó: {valor}");  // 5
    }
    Resultado.Error como cod => {
        decir("Falló: {cod}");
    }
}
```

O con `si ... como` si solo te importa una rama:

```falcato
si res es Resultado.Exito como valor {
    decir("Todo bien: {valor}");
}
```

## Error personalizado con enum

Puedes usar tu propio enum para los errores, mucho más descriptivo que un número:

```falcato
enumeración ErrorMatematico {
    DivisionPorCero,
    RaizNegativa,
    Desbordamiento,
}

fn raiz_cuadrada(x: Entero32) -> Resultado<Entero32, ErrorMatematico> {
    si x < 0 {
        retornar Resultado.Error(ErrorMatematico.RaizNegativa);
    }
    // calcular raíz entera aproximada...
    retornar Resultado.Exito(aprox);
}

fn manejar() {
    el res = raiz_cuadrada(-4);

    coincidir res {
        Resultado.Exito como v => { decir("Raíz: {v}"); }
        Resultado.Error como e => {
            coincidir e {
                ErrorMatematico.RaizNegativa => { decir("No hay raíz de negativo"); }
                ErrorMatematico.DivisionPorCero => { decir("No dividas por cero"); }
                ErrorMatematico.Desbordamiento => { decir("Número muy grande"); }
            }
        }
    }
}
```

## El operador `?` — "y si falla, que vuele"

```falcato
fn procesar() -> Resultado<Entero32, Entero32> {
    el valor = dividir(10, 0)?;  // si falla, retorna el error automáticamente
    retornar Resultado.Exito(valor * 2);
}
```

`?` hace dos cosas:
- Si es `Exito(v)` → extrae `v` y sigue
- Si es `Error(e)` → **retorna inmediatamente** con ese error

Es como decir "intenta esto, y si falla, nos vamos". Ahorra un montón de `coincidir`.

### Cadena de `?` — varias operaciones que pueden fallar

```falcato
fn proceso_complejo() -> Resultado<Entero32, Entero32> {
    el a = dividir(100, 2)?;   // 50
    el b = dividir(a, 5)?;      // 10
    el c = dividir(b, 2)?;      // 5
    retornar Resultado.Exito(c);
}
```

Si cualquiera falla, toda la función se corta y devuelve el error. Como una tubería: si un tramo se rompe, todo el tubo se vacía.

## Programa completo con errores

```falcato
enumeración ErrorArchivo {
    NoExiste,
    PermisoDenegado,
    Vacio,
}

fn leer_numero(la ruta: Palabra) -> Resultado<Entero32, ErrorArchivo> {
    // Falcato no tiene archivo_leer implementado con Resultado,
    // pero ilustra el patrón conceptual
    si !archivo_existe(ruta) {
        retornar Resultado.Error(ErrorArchivo.NoExiste);
    }

    el contenido: Texto = archivo_leer(ruta);

    si contenido.tam() es 0 {
        contenido.liberar();
        retornar Resultado.Error(ErrorArchivo.Vacio);
    }

    // ... parsear contenido a número ...
    // Suponiendo que encontramos el número:
    contenido.liberar();
    retornar Resultado.Exito(42);
}

fn main() -> Entero32 {
    el res = leer_numero("config.txt");

    coincidir res {
        Resultado.Exito como n => {
            decir("Configuración: {n}");
        }
        Resultado.Error como e => {
            coincidir e {
                ErrorArchivo.NoExiste => { decir("Archivo no encontrado"); }
                ErrorArchivo.PermisoDenegado => { decir("Sin permisos"); }
                ErrorArchivo.Vacio => { decir("Archivo vacío"); }
            }
        }
    }

    retornar 0;
}
```

## Errores del compilador

Además de manejar errores en tu código, el compilador también se queja:

```
[T001] archivo.fc:7:12: Disconcordancia de tipo
       │ sugerencia: Cambia el tipo o el valor
```

Formato: **[Código] archivo:línea:columna: mensaje**

| Código | Categoría | Significado |
|--------|-----------|-------------|
| `[S001]` | Sintaxis | Algo mal escrito |
| `[T001]` | Tipo | Los tipos no concuerdan |
| `[O001]` | Ownership | Usaste algo que ya no es tuyo |
| `[M001]` | Módulo | Importaste algo que no existe |

Para la lista completa: [ERRORES.md](../ERRORES.md)

## Errores típicos con Resultado

```falcato
// Error: olvidar manejar el resultado
el res = dividir(10, 2);
// olvidaste comprobar si es Exito o Error
// Si es Error, 'valor' no existe y crash

// Error: usar ? fuera de función que devuelve Resultado
fn main() -> Entero32 {
    el v = dividir(10, 2)?;  // Error: main devuelve Entero32, no Resultado
}

// Error: tipos de error distintos en cadena de ?
fn mezclar() -> Resultado<Entero32, ErrorMatematico> {
    el v = dividir(10, 2)?;
    // 'dividir' devuelve Resultado<Entero32, Entero32>
    // pero 'mezclar' espera Resultado<Entero32, ErrorMatematico>
    // Los códigos de error no coinciden
}
```

## Programa completo: Calculadora

Este programa une todo lo visto: `Resultado<T,E>`, operador `?`, `coincidir`, arrays, bucles `para`, pattern matching con `es...como`, e interpolación de strings.

```falcato
// calculadora.fc — Calculadora de 4 operaciones con manejo de errores

enumeración ErrorMatematico {
    DivisionPorCero,
    OperacionInvalida,
}

fn dividir(la a: Entero32, la b: Entero32) -> Resultado<Entero32, ErrorMatematico> {
    si b es 0 {
        retornar Resultado.Error(ErrorMatematico.DivisionPorCero);
    }
    retornar Resultado.Exito(a / b);
}

fn operar(la op: Entero32, la a: Entero32, la b: Entero32) -> Resultado<Entero32, ErrorMatematico> {
    coincidir op {
        1 => { retornar Resultado.Exito(a + b); }
        2 => { retornar Resultado.Exito(a - b); }
        3 => { retornar Resultado.Exito(a * b); }
        4 => { retornar dividir(a, b)?; }  // propaga error si b=0
        _ => { retornar Resultado.Error(ErrorMatematico.OperacionInvalida); }
    }
}

fn main() -> Entero32 {
    imprimir_linea("=== CALCULADORA FALCATO ===");
    imprimir_linea("1) Sumar");
    imprimir_linea("2) Restar");
    imprimir_linea("3) Multiplicar");
    imprimir_linea("4) Dividir");
    imprimir_linea("0) Salir");
    imprimir_linea("");

    // Array de opciones para el menú
    los opciones: [Entero32; 4] = [1, 2, 3, 4];

    mientras verdadero {
        imprimir("Elige opcion: ");
        // En Falcato real usarías entrada de usuario; aquí simulamos:
        el opcion: Entero32 = 4;  // Cambia esto para probar

        si opcion es 0 {
            imprimir_linea("Hasta luego!");
            retornar 0;
        }

        // Validar que la opción existe en el array
        el valida: Booleano = falso;
        para op en opciones {
            si op es opcion {
                valida = verdadero;
            }
        }

        si !valida {
            imprimir_linea("Opcion invalida. Intenta de nuevo.");
            continuar;
        }

        imprimir("Primer numero: ");
        el a: Entero32 = 10;  // Simulado
        imprimir("Segundo numero: ");
        el b: Entero32 = 5;   // Simulado

        // Llamada que devuelve Resultado — usamos coincidir para manejar ambos casos
        coincidir operar(opcion, a, b) {
            Resultado.Exito como res => {
                imprimir_linea("Resultado: {res}");
            }
            Resultado.Error como err => {
                coincidir err {
                    ErrorMatematico.DivisionPorCero => {
                        imprimir_linea("Error: No se puede dividir por cero");
                    }
                    ErrorMatematico.OperacionInvalida => {
                        imprimir_linea("Error: Operacion no reconocida");
                    }
                }
            }
        }

        imprimir_linea("");
    }

    retornar 0;
}
```

**Puntos clave del ejemplo:**

| Concepto | Dónde se ve |
|----------|-------------|
| Enum con datos | `ErrorMatematico { DivisionPorCero, OperacionInvalida }` |
| Función que devuelve `Resultado` | `dividir`, `operar` |
| Propagar error con `?` | `dividir(a, b)?` en `operar` |
| `coincidir` exhaustivo | En `operar` (por `op`) y en `main` (por resultado) |
| `es...como` para extraer | `Resultado.Exito como res`, `Resultado.Error como err` |
| Array stack-allocated | `los opciones: [Entero32; 4]` |
| Bucle `para` sobre array | `para op en opciones` |
| Interpolación | `imprimir_linea("Resultado: {res}")` |

**Compilar y ejecutar:**
```bash
falcato run calculadora.fc
```

---

## Tabla rápida

| Patrón | Sintaxis | Cuándo usarlo |
|--------|----------|---------------|
| Función que puede fallar | `-> Resultado<T, E>` | Cuando hay un camino correcto y otro erróneo |
| Propagar error | `expr?` | Dentro de función con Resultado |
| Extraer valor | `si res es Exito como v` | Cuando solo interesa el éxito |
| Manejar ambas ramas | `coincidir res { Exito..., Error... }` | Cuando necesitas manejar ambos casos |

---

← [10: Datos compuestos](10-datos.md) | [Indice](../GUIA.md) | [Siguiente: Métodos →](12-metodos.md)
