# AST (Árbol de Sintaxis Abstracta)

**Archivo:** `src/ast.rs`

El AST es la representación del programa en forma de árbol.
Cada nodo —cada rama, cada hoja— lleva consigo un `Span` que
indica dónde en el código fuente original fue concebido.

## Jerarquía

```
Programa
  └── Vec<Declaracion>
        ├── Funcion(FuncionDecl)
        ├── Estructural(EstructuralDecl)
        ├── Enumeracion(EnumeracionDecl)
        ├── Modulo(ModuloDecl)
        └── Usar(UsarDecl)
```

### FuncionDecl

```rust
pub struct FuncionDecl {
    pub nombre: String,
    pub parametros_genericos: Vec<ParametroGenerico>,
    pub parametros: Vec<Parametro>,
    pub retorno: Option<Tipo>,
    pub cuerpo: Bloque,
    pub es_insegura: bool,
    pub span: Span,
}
```

### Sentencias

Lo que se ejecuta dentro de las funciones:

```rust
pub enum Sentencia {
    Expresion(Expresion),
    DeclaracionVariable(DeclaracionVariable),
    Asignacion(Asignacion),
    Retornar(Option<Expresion>, Span),
    Condicional(Condicional),
    BucleMientras(BucleMientras),
    BuclePara(BuclePara),
}
```

### Expresiones

Los ladrillos que producen valores:

```rust
pub enum Expresion {
    Literal(Literal),
    Identificador(String, Span),
    Llamada(Llamada),
    Binaria(Box<Expresion>, OperadorBinario, Box<Expresion>, Span),
    Unaria(OperadorUnario, Box<Expresion>, Span),
    AccesoArray(Box<Expresion>, Box<Expresion>, Span),
    LiteralArray(Vec<Expresion>, Span),
    ArrayRelleno(Box<Expresion>, usize, Span),
    InicializacionStruct(String, Vec<(String, Expresion)>, Span),
    AccesoCampo(Box<Expresion>, String, Span),
    ConstructorEnum(String, String, Vec<Expresion>, Span),
    EsVariante(Box<Expresion>, String, String, Span),
}
```

### Tipos

```rust
pub enum Tipo {
    Entero8, Entero16, Entero32, Entero64,
    Natural8, Natural16, Natural32, Natural64,
    Flotante32, Flotante64,
    Booleano, Caracter, Palabra, Vacio,
    Puntero(Box<Tipo>),
    Referencia(Box<Tipo>),
    Array(Box<Tipo>, usize),         // [T; N]
    ArrayGenerico(Box<Tipo>, String), // [T; N] con N genérico
    Generico(String),                // T
    Nombre(String),                  // nombre de struct/enum
    NombreGenerico(String, Vec<Tipo>), // Nombre<T1, T2>
}
```

### Artículo

```rust
pub enum Articulo {
    El,   // owned, mutable: cosa de uno
    La,   // borrowed, inmutable: prestado
    Un,   // optional: quizá, quizá no
    Los,  // colección owned
    Las,  // colección borrowed
}
```

### Span

```rust
pub struct Span {
    pub inicio: Posicion,
    pub fin: Posicion,
    pub archivo: Arc<str>,
}
```
