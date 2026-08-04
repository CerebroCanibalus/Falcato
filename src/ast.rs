use crate::span::Span;

/// Programa completo: lista de declaraciones
#[derive(Debug, Clone)]
pub struct Programa {
    pub declaraciones: Vec<Declaracion>,
    pub span: Span,
}

/// Declaración top-level
#[derive(Debug, Clone)]
pub enum Declaracion {
    Funcion(FuncionDecl),
    Estructural(EstructuralDecl),
    Enumeracion(EnumeracionDecl),
    Modulo(ModuloDecl),
    Usar(UsarDecl),
    Rasgo(RasgoDecl),
    Implementacion(ImplDecl),
    Prueba(PruebaDecl),
    Apodo(ApodoDecl),
}

/// Declaración de apodo de tipo: `apodo Nombre = Tipo;`
#[derive(Debug, Clone)]
pub struct ApodoDecl {
    pub nombre: String,
    pub tipo: Tipo,
    pub span: Span,
}

/// Nivel de verificación de ownership (borrow checker gradual)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NivelVerificacion {
    Permisivo,   // Nivel 0: sin verificación (default, como C)
    Verificado,  // Nivel 1: moves + use-after-move
    Estricto,    // Nivel 2: moves + borrows + lifetimes
}

/// Anotación de efecto: el compiler razona entre funciones
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Efecto {
    Puro,                    // No muta nada fuera de su scope
    Muta(Vec<String>),       // Solo muta los campos especificados
    Lee(Vec<String>),        // Solo lee los campos especificados
    Conservador,             // Puede mutar cualquier cosa (default)
}

/// Declaración de función
#[derive(Debug, Clone)]
pub struct FuncionDecl {
    pub nombre: String,
    pub parametros_genericos: Vec<ParametroGenerico>,
    pub parametros: Vec<Parametro>,
    pub retorno: Option<Tipo>,
    pub cuerpo: Bloque,
    pub es_insegura: bool,
    /// Nivel de verificación de ownership (borrow checker gradual)
    pub nivel_verificacion: NivelVerificacion,
    /// Anotación de efecto (puro, muta(campo), lee(campo))
    pub efecto: Efecto,
    /// Visibilidad basada en artículo:
    /// - `None` o `Some(Articulo::El)` = pública (default)
    /// - `Some(Articulo::La)` = privada (solo accesible dentro del módulo)
    pub visibilidad: Option<Articulo>,
    /// true si es `fut función` (async)
    pub es_futuro: bool,
    pub span: Span,
}

/// Parámetro de función
#[derive(Debug, Clone)]
pub struct Parametro {
    pub articulo: Articulo,
    pub nombre: String,
    pub tipo: Tipo,
    pub span: Span,
}

/// Parámetro genérico: type param (T) o const param (N: Entero32)
#[derive(Debug, Clone)]
pub struct ParametroGenerico {
    pub nombre: String,
    pub tipo: Option<Tipo>, // None = type param (T), Some(t) = const param (N: Entero32)
    pub bounds: Vec<String>, // traits requeridos (ej: "Comparable")
    pub span: Span,
}

/// Artículo: codifica ownership
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Articulo {
    El,   // owned, mutable
    La,   // borrowed, inmutable
    Un,   // optional
    Los,  // colección owned
    Las,  // colección borrowed
}

/// Bloque de código: { sentencias }
#[derive(Debug, Clone)]
pub struct Bloque {
    pub sentencias: Vec<Sentencia>,
    pub span: Span,
}

/// Sentencia
#[derive(Debug, Clone)]
pub enum Sentencia {
    Expresion(Expresion),
    DeclaracionVariable(DeclaracionVariable),
    Asignacion(Asignacion),
    Retornar(Option<Expresion>, Span),
    Condicional(Condicional),
    BucleMientras(BucleMientras),
    BuclePara(BuclePara),
    Region { nombre: String, cuerpo: Vec<Sentencia>, span: Span },
    Seleccionar(Seleccionar),
    ConExecutor { hilos: Expresion, cuerpo: Vec<Sentencia>, span: Span },
}

/// Rama de un `seleccionar`: canal como variable => { cuerpo }
#[derive(Debug, Clone)]
pub struct RamaSeleccionar {
    pub canal: Expresion,
    pub variable: Option<String>, // None para la rama default `_`
    pub cuerpo: Bloque,
    pub span: Span,
}

/// seleccionar { canal como v => { ... }, _ => { ... } }
#[derive(Debug, Clone)]
pub struct Seleccionar {
    pub ramas: Vec<RamaSeleccionar>,
    pub span: Span,
}

/// Lugar de asignación: identificador simple o elemento de array
#[derive(Debug, Clone)]
pub enum Lugar {
    Identificador(String),
    Array(Box<Expresion>, Box<Expresion>), // array, índice
    Campo(Box<Expresion>, String),         // expr.campo (Fase 15B: bitfield write)
}

/// Asignación: lugar = expresion;
#[derive(Debug, Clone)]
pub struct Asignacion {
    pub lugar: Lugar,
    pub valor: Expresion,
    pub span: Span,
}

/// Modo verbal para condicionales (innovación lingüística)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModoVerbal {
    Indicativo,   // si x > 0  — hecho verificable, branch normal
    Estativo,     // si x está  — estado temporal, truthiness / state check
    Subjuntivo,   // si x fuese > 0  — hipótesis improbable, cold path
}

/// Condicional: si condicion { bloque } [sino { bloque }]
#[derive(Debug, Clone)]
pub struct Condicional {
    pub condicion: Expresion,
    pub bloque_entonces: Bloque,
    pub bloque_sino: Option<Bloque>,
    pub modo: ModoVerbal,
    pub span: Span,
}

/// Bucle mientras: mientras condicion { bloque }
#[derive(Debug, Clone)]
pub struct BucleMientras {
    pub condicion: Expresion,
    pub bloque: Bloque,
    pub span: Span,
}

/// Bucle para: para variable en iterable { bloque }
#[derive(Debug, Clone)]
pub struct BuclePara {
    pub variable: String,
    pub iterable: Expresion,
    pub bloque: Bloque,
    pub span: Span,
}

/// Declaración de variable: el x: Entero32 = 42;
#[derive(Debug, Clone)]
pub struct DeclaracionVariable {
    pub articulo: Articulo,
    pub nombre: String,
    pub tipo: Option<Tipo>,
    pub valor: Expresion,
    pub span: Span,
}

/// Declaración de prueba: prueba "nombre" { ... }
#[derive(Debug, Clone)]
pub struct PruebaDecl {
    pub nombre: String,
    pub bloque: Bloque,
    pub span: Span,
}

// INNOVACIÓN FUTURA: Condicionales Modales (Subjuntivo como Semántica de Certeza)
// 
// El español distingue entre indicativo (hechos verificables) y subjuntivo 
// (duda, hipótesis, deseos). Ningún lenguaje de programación explota esto.
// 
// PROPUESTA PARA FASE 3-4:
// 
// 1. INDICATIVO (default `si`): hecho verificable
//    ```falcato
//    si x > 0 {
//        // Branch normal, compilador optimiza con confianza
//    }
//    ```
//    Semántica: El compilador asume esta rama es probable (hot path).
//    
// 2. SUBJUNTIVO (`si fuese`): hipótesis / condición improbable
//    ```falcato
//    si x fuese > 0 {
//        // Branch especulativo, compilador genera [[unlikely]]
//    }
//    ```
//    Semántica: Branch prediction = cold path. Útil para errores, edge cases.
//    El programador comunica INTENCIÓN de probabilidad al hardware.
//    
// 3. IMPERATIVO DE CORTESÍA (`sea`): contrato / assertion
//    ```falcato
//    sea x > 0;  // Assertion: si falla, panic/UB
//    ```
//    Semántica: El programador DECLARA un invariante. El compilador puede
//    optimizar asumiendo esta condición SIEMPRE verdadera (como `assume`).
//    
// 4. SER/ESTAR EN CONDICIONES (Pilar II extendido):
//    ```falcato
//    si x es 5 {       // Comparación estructural/permanente (==)
//        // Identidad de valor inmutable
//    }
//    si x está vacío { // Estado temporal, mutable
//        // Verificación de estado que puede cambiar
//    }
//    ```
//    Semántica: `es` compara identidad/igualdad estructural.
//    `está` verifica estado mutable (como `isEmpty()`, `isReady()`).
//    Obliga al programador a distinguir identidad vs estado.
//
// IMPLEMENTACIÓN TÉCNICA:
// - Agregar `ModoVerbal` enum: Indicativo | Subjuntivo | Imperativo
// - `Expresion::Condicional` almacena modo + condición + bloques
// - Codegen: Subjuntivo → llvm.expect/cranelift cold block hint
// - Ser/Estar: análisis semántico distingue tipos comparables vs estados
//
// REFERENCIAS:
// - Papers PLDI 2025: branch prediction hints mejoran rendimiento 15-30%
// - Lenguajes con contracts (Dafny, Whiley) usan aserciones como optimización
// - El subjuntivo español es gramaticalmente una máquina de estados de certeza
//
// NOTA: Implementar en FASE 3 (ownership) o FASE 4 (optimizaciones).
// FASE 2 usa `si`/`si_no` básico (solo indicativo).

/// Patrón de match
#[derive(Debug, Clone)]
pub enum PatronMatch {
    /// Literal: 0, 1, 42, "hola"
    Literal(Literal),
    /// Variante de enum: Estado.Activo, Resultado.Exito(x)
    VarianteEnum(String, String, Option<String>, Span), // enum_nombre, variante, binding opcional
    /// Wildcard: _
    Comodin(Span),
}

/// Brazo de match: patron => expresion
#[derive(Debug, Clone)]
pub struct BrazoMatch {
    pub patron: PatronMatch,
    pub cuerpo: Expresion,
    pub span: Span,
}

/// Expresión
#[derive(Debug, Clone)]
pub enum Expresion {
    Literal(Literal),
    Identificador(String, Span),
    Llamada(Llamada),
    Binaria(Box<Expresion>, OperadorBinario, Box<Expresion>, Span),
    Unaria(OperadorUnario, Box<Expresion>, Span),
    AccesoArray(Box<Expresion>, Box<Expresion>, Span),  // array, índice
    LiteralArray(Vec<Expresion>, Span),
    ArrayRelleno(Box<Expresion>, usize, Span), // todos expr, tamaño (rellenado por semántica)
    InicializacionStruct(String, Vec<(String, Expresion)>, Span), // NombreStruct, campos: valores
    AccesoCampo(Box<Expresion>, String, Span), // expr.campo
    ConstructorEnum(String, String, Vec<Expresion>, Span), // enum_nombre, variante_nombre, argumentos
    EsVariante(Box<Expresion>, String, String, Option<String>, Span), // expr, enum_nombre, variante_nombre, binding (como x)
    Propagacion(Box<Expresion>, Span), // expr? — propagación de errores
    Mover(String, Option<Box<Expresion>>, Span), // mover x [a destino] — transferencia de ownership
    Copiar(Box<Expresion>, Span), // copiar expr — clone explícito
    /// Ruta cualificada: modulo::simbolo (pertenencia, no llamado)
    Ruta(Vec<String>, Span),
    /// Rango: inicio..fin (exclusivo) o inicio..=fin (inclusivo)
    Rango(Box<Expresion>, Box<Expresion>, bool, Span), // inicio, fin, inclusivo, span
    /// Closure: |params| cuerpo
    Closure(Vec<(String, Option<Tipo>)>, Box<Expresion>, Span), // params (nombre, tipo opcional), cuerpo, span
    /// Match exhaustivo: coincidir sujeto { patron => expr, ... }
    Coincidir(Box<Expresion>, Vec<BrazoMatch>, Span),
    /// Async (Fase 18): esperar expr — suspende hasta que el futuro complete
    Esperar(Box<Expresion>, Span),
    /// Async (Fase 18): lanzar expr — spawn de tarea independiente
    Lanzar(Box<Expresion>, Span),
    /// Async (Fase 18): bloquear(expr) — bridge sync→async (bloquea thread)
    Bloquear(Box<Expresion>, Span),
    /// Método general: x.metodo(args) — desugarea a llamada built-in según tipo del receptor
    Metodo(Box<Expresion>, String, Vec<Expresion>, Span), // receptor, nombre_método, args, span
    /// Bloque como expresión: { sentencias } — retorna el valor de la última expresión
    Bloque(Bloque),
    /// DireccionDe(nombre) — obtiene la dirección (puntero) de una función
    DireccionDe(String, Span),
}

impl Expresion {
    pub fn span(&self) -> &Span {
        match self {
            Expresion::Literal(lit) => lit.span(),
            Expresion::Identificador(_, span) => span,
            Expresion::Llamada(call) => &call.span,
            Expresion::Binaria(_, _, _, span) => span,
            Expresion::Unaria(_, _, span) => span,
            Expresion::AccesoArray(_, _, span) => span,
            Expresion::LiteralArray(_, span) => span,
            Expresion::ArrayRelleno(_, _, span) => span,
            Expresion::InicializacionStruct(_, _, span) => span,
            Expresion::AccesoCampo(_, _, span) => span,
            Expresion::ConstructorEnum(_, _, _, span) => span,
            Expresion::EsVariante(_, _, _, _, span) => span,
            Expresion::Propagacion(_, span) => span,
            Expresion::Mover(_, _, span) => span,
            Expresion::Copiar(_, span) => span,
            Expresion::Ruta(_, span) => span,
            Expresion::Rango(_, _, _, span) => span,
            Expresion::Closure(_, _, span) => span,
            Expresion::Coincidir(_, _, span) => span,
            Expresion::Esperar(_, span) => span,
            Expresion::Lanzar(_, span) => span,
            Expresion::Bloquear(_, span) => span,
            Expresion::Metodo(_, _, _, span) => span,
            Expresion::Bloque(bloque) => &bloque.span,
            Expresion::DireccionDe(_, span) => span,
        }
    }
}

/// Literal
#[derive(Debug, Clone)]
pub enum Literal {
    Entero(i64, Span),
    Flotante(f64, Span),
    Palabra(String, Span),
    Caracter(char, Span),
    Booleano(bool, Span),
}

impl Literal {
    pub fn span(&self) -> &Span {
        match self {
            Literal::Entero(_, s) |
            Literal::Flotante(_, s) |
            Literal::Palabra(_, s) |
            Literal::Caracter(_, s) |
            Literal::Booleano(_, s) => s,
        }
    }
}

/// Llamada a función
#[derive(Debug, Clone)]
pub struct Llamada {
    pub funcion: String,
    pub tipo_args: Vec<Tipo>,  // argumentos de tipo para llamadas genéricas: f<T>(args)
    pub argumentos: Vec<Expresion>,
    pub span: Span,
}

/// Operador binario
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperadorBinario {
    Suma,
    Resta,
    Multiplicacion,
    Division,
    Modulo,
    Igual,
    Distinto,
    Menor,
    Mayor,
    MenorIgual,
    MayorIgual,
    Y,
    O,
    // Bitwise
    BitAnd,       // &
    BitOr,        // |
    BitXor,       // ^
    ShiftLeft,    // <<
    ShiftRight,   // >> (aritmético para signed, lógico para unsigned)
    ShiftRightLogico, // >>> (zero-fill siempre)
}

/// Operador unario
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperadorUnario {
    Negacion,
    NegacionLogica,
    BitNot,        // ~ (bitwise NOT)
    Referencia,
    ReferenciaMut, // &mut expr — referencia mutable
    Desreferencia,
}

/// Tipo
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tipo {
    Entero8,
    Entero16,
    Entero32,
    Entero64,
    Natural8,
    Natural16,
    Natural32,
    Natural64,
    Flotante32,
    Flotante64,
    Booleano,
    Caracter,
    Palabra,
    Texto,
    Vacio,
    Puntero(Box<Tipo>),
    Referencia(Box<Tipo>),
    ReferenciaMut(Box<Tipo>), // &mut T — referencia mutable
    ReferenciaConLifetime(String, Box<Tipo>), // &nombre T — referencia con lifetime léxico
    ReferenciaMutConLifetime(String, Box<Tipo>), // &mut nombre T — referencia mutable con lifetime léxico
    ReferenciaSelf(Box<Tipo>), // &self T — referencia self-referential
    ReferenciaMutSelf(Box<Tipo>), // &mut self T — referencia mutable self-referential
    Array(Box<Tipo>, usize),  // tipo, longitud conocida
    ArrayGenerico(Box<Tipo>, String), // tipo, nombre del parámetro const genérico
    Vector(Box<Tipo>),        // vector dinámico heap-allocado
    Resultado(Box<Tipo>, Box<Tipo>), // Resultado<T, E> para manejo de errores
    Diccionario(Box<Tipo>, Box<Tipo>), // Diccionario<K, V> — hash map
    Conjunto(Box<Tipo>), // Conjunto<T> — hash set (wrapper de Diccionario<T, Booleano>)
    Generico(String), // parámetro de tipo genérico (T)
    Nombre(String),
    NombreGenerico(String, Vec<Tipo>), // Enum<Tipo1, Tipo2> → nombre instanciado
}

/// Declaración estructural
#[derive(Debug, Clone)]
pub struct EstructuralDecl {
    pub nombre: String,
    pub campos: Vec<Campo>,
    /// Fase 15B: campos de bits (empaquetados en un entero)
    pub campos_bits: Vec<CampoBits>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Campo {
    pub nombre: String,
    pub tipo: Tipo,
    pub span: Span,
}

/// Campo de bits dentro de un struct (Fase 15B)
/// `habilitado: Natural1` → 1 bit en offset 0
#[derive(Debug, Clone)]
pub struct CampoBits {
    pub nombre: String,
    pub ancho_bits: u32,   // 1-32
    pub offset_bits: u32,  // calculado por parser (secuencial desde LSB)
    pub span: Span,
}

/// Declaración de enumeración
#[derive(Debug, Clone)]
pub struct EnumeracionDecl {
    pub nombre: String,
    pub parametros_genericos: Vec<ParametroGenerico>,
    pub variantes: Vec<Variante>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Variante {
    pub nombre: String,
    pub datos: Option<Vec<(String, Tipo)>>, // None = sin datos, Some = campos con nombre y tipo
    pub span: Span,
}

/// Declaración de módulo
#[derive(Debug, Clone)]
pub struct ModuloDecl {
    pub nombre: String,
    pub contenido: Vec<Declaracion>,
    pub span: Span,
}

/// Declaración de importación
#[derive(Debug, Clone)]
pub struct UsarDecl {
    pub ruta: Vec<String>,
    pub span: Span,
}

/// Declaración de rasgo (trait): define una interfaz que tipos pueden implementar
#[derive(Debug, Clone)]
pub struct RasgoDecl {
    pub nombre: String,
    /// Firmas de métodos requeridos (sin cuerpo)
    pub metodos: Vec<FirmaMetodo>,
    pub span: Span,
}

/// Firma de un método de rasgo (sin cuerpo)
#[derive(Debug, Clone)]
pub struct FirmaMetodo {
    pub nombre: String,
    pub parametros: Vec<Parametro>,
    pub retorno: Option<Tipo>,
    pub span: Span,
}

/// Implementación de un rasgo para un tipo concreto
#[derive(Debug, Clone)]
pub struct ImplDecl {
    pub rasgo: String,
    pub tipo: Tipo,
    /// Métodos implementados (con cuerpo)
    pub metodos: Vec<FuncionDecl>,
    pub span: Span,
}
