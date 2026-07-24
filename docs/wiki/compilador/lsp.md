# Servidor LSP

**Archivo:** `src/lsp.rs`

## Tecnología

Usamos `tower-lsp` 0.20, que es el marco de trabajo —framework,
que dicen— más asentado para construir servidores LSP en Rust.
La comunicación va por stdio, como corresponde a una herramienta
que no necesita aspavientos.

## Capacidades

### Diagnósticos en vivo

En cuanto abrís o cambiáis un documento, el LSP ejecuta:

1. **Lexer**: trocea el texto
2. **Parser**: construye el AST (y reporta errores de sintaxis)
3. **Índice semántico**: averigua qué variables y funciones hay
4. **Análisis semántico**: busca disconcordancias de tipo,
   ownership, llamadas, etc.

Los diagnósticos se publican al editor sin que tengáis que pedirlos.

### Hover

Pasando el cursor sobre un identificador, el LSP os susurra:

- **Variables**: `el nombre: Tipo` con explicación del artículo
- **Funciones**: `fn nombre(params) -> Tipo`

### Go to Definition

Saltad a la declaración de cualquier cosa con un clic:

- Variables (a su declaración en el código)
- Funciones (a su definición)

## Autocompletado

Ofrece una lista de sugerencias —modesta pero útil— con:

- **Keywords**: `función`, `retornar`, `si`, `sino`, `mientras`, `para`, etc.
- **Artículos**: `el`, `la`, `un`, `los`, `las` con descripción
- **Tipos primitivos**: `Entero8`–`Entero64`, `Flotante32`/`64`, etc.
- **Booleanos**: `verdadero`, `falso`

## Estructura

```rust
pub struct Backend {
    client: Client,
    documentos: Arc<RwLock<HashMap<Url, DocumentoLsp>>>,
}

pub struct DocumentoLsp {
    pub contenido: String,
    pub indice: IndiceSemantico,
    pub ast: Option<Programa>,
}
```

### IndiceSemantico

```rust
pub struct IndiceSemantico {
    pub variables: HashMap<String, InfoVariableLsp>,
    pub funciones: HashMap<String, InfoFuncionLsp>,
}
```

## Uso

```bash
falcato lsp
```

El servidor escucha en stdio. Funciona con VS Code, Vim (con
un plugin LSP), Emacs (eglot o lsp-mode), y Neovim (LSP
integrado). Cada cual con sus armas.
