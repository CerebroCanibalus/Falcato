# Codegen (Cranelift)

**Archivo:** `src/codegen.rs`

Aquí es donde las palabras se vuelven carne —o mejor, donde el
AST se vuelve código máquina. Cranelift 0.112 es el caballo de
batalla que genera las instrucciones que el procesador ejecuta.

## Tecnología

Cranelift es un generador de código en Rust puro. No necesita
dependencias del sistema: él mismo produce el binario objeto.
Ocupa el lugar que en otros compiladores tiene LLVM, pero con
la ventaja de ser más ligero y estar escrito en el mismo lenguaje
que el compilador mismo.

## Arquitectura

```rust
pub struct Codegen {
    module: ObjectModule,
    funciones: HashMap<String, FuncId>,
    funciones_genericas: HashMap<String, FuncionDecl>,
    instanciaciones: HashMap<(String, Vec<String>), FuncId>,
    structs: HashMap<String, LayoutStruct>,
    enums: HashMap<String, LayoutEnum>,
    errores: Errores,
    contador_strings: u32,
}
```

## Estrategia de compilación

### 1. Registro
Se recorren todas las declaraciones para registrar structs (con su
layout) y enums (con su tag+union).

### 2. Declaración
Las funciones no genéricas declaran su firma. Las genéricas se
almacenan para más tarde, como quien guarda un as en la manga.

### 3. Compilación de funciones

Cada función se compila en su propio contexto Cranelift:

```
FunctionBuilder → entry_block
  ├── stack slots para parámetros
  ├── stack slots para variables locales
  ├── compilación de sentencias
  │   ├── expresiones → valores IR
  │   ├── condicionales → bloques con brif
  │   ├── bucles → header/body/exit blocks
  │   └── retornos → return_ instruction
  └── finalize → definir en ObjectModule
```

### C ABI por defecto

El convenio de llamada es SystemV, sin necesidad de declaración
externa:

```rust
let mut sig = Signature::new(CallConv::SystemV);
```

## Mapeo de tipos

| Falcato | Cranelift |
|---------|-----------|
| `Entero32` | `types::I32` |
| `Entero64` | `types::I64` |
| `Flotante64` | `types::F64` |
| `Booleano` | `types::I8` |
| `Palabra` | `types::I64` (puntero) |
| Arrays | Stack slot + puntero base |

## Layout de structs

Layout C con alineación natural, que es como el compilador de
C colocaría los campos:

- Cada campo se alinea al múltiplo de su tamaño
- Relleno entre campos si es necesario
- Relleno final al mayor alineamiento presente

## Layout de enums

Tag+union, una estructura que guarda tanto el tipo de variante
como los datos:

- Tag: `I32` (4 bytes) al inicio
- Datos: a partir del byte 4, con tamaño igual a la variante
  más grande

## Strings

Se crean como datos globales con nombre único para que el linker
pueda encontrarlos:

```rust
let data_id = module.declare_data("str_1_13", Linkage::Local, ...);
```

## Manejo de errores

Los errores del codegen —variables que no aparecen, tipos no
registrados— se acumulan y se reportan al final. No hay drama
prematuro.
