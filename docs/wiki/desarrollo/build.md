# Compilar el Compilador

Instrucciones para levantar el tinglado.

## Requisitos

- **Rust**: toolchain stable (rustup + cargo, que es el pan nuestro
  de cada día)
- **Visual Studio Build Tools** (Windows): para el linker
  `link.exe`. Descargad de:
  https://visualstudio.microsoft.com/es/downloads/#build-tools-for-visual-studio-2022
  - Componente necesario: "Herramientas de compilación de VC++"
  - O instalad Visual Studio 2022 Community con la carga
    "Desarrollo para el escritorio con C++"

## Build

```bash
# Debug (para desarrollar)
cargo build

# Release (con LTO, para repartir)
cargo build --release

# Scripts
.\build.bat
.\build.ps1     # detecta Visual Studio solito
.\build_release.bat
```

## Verificación

Para comprobar que todo funciona como debe:

```bash
# Compilar y ejecutar el hola mundo
cargo run -- run ejemplos/hola_mundo.fc

# Sólo verificar sintaxis y tipos
cargo run -- check ejemplos/hola_mundo.fc

# Compilar a binario
cargo run -- build ejemplos/hola_mundo.fc -o hola.exe
./hola.exe
```

## Dependencias

Éstas son las bibliotecas que el compilador necesita. Se descargan
solas con `cargo build`, no hay que ir a buscarlas:

```toml
clap = "4.5"                   # CLI
logos = "0.14"                 # Lexer
cranelift-codegen = "0.112"    # Codegen
cranelift-frontend = "0.112"
cranelift-module = "0.112"
cranelift-object = "0.112"
cranelift-native = "0.112"
target-lexicon = "0.12"
tower-lsp = "0.20"             # LSP
tokio = "1"                    # Async para el LSP
```

## Posibles problemas

### «No se encontró link.exe»
Tenéis que instalar Visual Studio Build Tools o añadir MSVC al
PATH. El script `build.ps1` busca las ubicaciones comunes por sí
solo, que para eso es listo.

### Error de enlace: símbolos sin resolver
Aseguraos de que la función `principal` existe en vuestro programa.
Es el punto de entrada, y sin ella el linker no sabe por dónde
empezar.
