# Pipeline de Compilación

Dígase ahora el camino completo, paso a paso, con las líneas
de código donde ocurre cada maravilla.

## Flujo completo

### 1. Lectura (`src/main.rs:129`)
El compilador lee el archivo entero en una cuerda —string, que
dicen—:

```rust
let fuente = fs::read_to_string(archivo)?;
```

### 2. Lexer (`src/main.rs:133-134`)
Se invoca al lexer para que trocee el texto:

```rust
let lexer = LexerFalcato::nuevo(&fuente, archivo);
let tokens = lexer.tokenizar();
```

### 3. Parser (`src/main.rs:138-143`)
Los tokens pasan al parser, que construye el árbol:

```rust
let programa = ParserFalcato::parse(tokens)?;
```

### 4. Análisis Semántico (`src/main.rs:148-151`)
El analizador verifica que todo tenga sentido:

```rust
let mut semantica = semantic::AnalizadorSemantico::nuevo();
semantica.analizar(&programa)?;
```

### 5. Codegen (`src/main.rs:154-157`)
Cranelift entra en escena para generar código máquina:

```rust
let mut codegen = Codegen::nuevo("main")?;
codegen.compilar_programa(&programa)?;
```

### 6. Objeto (`src/main.rs:161-162`)
Se escribe el archivo objeto:

```rust
codegen.escribir_objeto(&obj_ruta)?;
```

### 7. Linkeo (`src/main.rs:175-178`)
Y finalmente se linkea para obtener el ejecutable:

```rust
link_objeto(&obj_ruta, &binario, target, release)?;
```

## Linker

### Windows
En el reino de los ventanas, se busca `link.exe` de MSVC
en las sendas habituales:

```
link.exe archivo.o /OUT:salida.exe /SUBSYSTEM:CONSOLE /ENTRY:principal libcmt.lib
```

### Linux/macOS
En las tierras del pingüino y la manzana:

```bash
gcc archivo.o -o salida
```

## Funciones genéricas

El codegen maneja los genéricos en tres pasadas, como quien
prepara una obra de teatro:

1. **Registro**: se anotan structs y enums primero (los decorados)
2. **Declaración**: las funciones no genéricas declaran su firma
3. **Compilación**: los cuerpos de las funciones no genéricas

Las funciones genéricas se guardan para monomorfizarse en el
punto de llamada, que es donde se sabe con qué tipo concreto
trabajan.
