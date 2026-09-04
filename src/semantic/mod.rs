//! Análisis semántico de Falcato — Concordancia Lingüística
//! 
//! Este módulo implementa el análisis semántico del lenguaje Falcato,
//! basado en el principio de "Concordancia Lingüística": los tipos,
//! ownership y mutabilidad deben concordar gramaticalmente.

use crate::ast::*;
use crate::error::{CategoriaError, ErrorCompilador, Errores};
use crate::span::Span;
use std::collections::HashMap;

// Submódulos
mod tipos;
mod ownership;
mod funciones;
mod sentencias;

// Re-exports
pub use tipos::*;
pub use ownership::*;
pub use funciones::*;
pub use sentencias::*;

// ============================================
// CONSTANTES DE CÓDIGOS DE ERROR SEMÁNTICO
// ============================================

/// Códigos de error de tipo [T###]
pub mod codigos {
    // T001-T009: Declaración y asignación
    pub const CLAVE_SOBREESCRITA: u32 = 100;
    pub const DISCONCORDANCIA_TIPO: u32 = 1;
    pub const DISCONCORDANCIA_RETORNO: u32 = 2;
    pub const RETORNO_FALTANTE: u32 = 3;
    pub const VARIABLE_NO_DECLARADA: u32 = 4;
    pub const DISCONCORDANCIA_OPERANDOS: u32 = 5;
    pub const OPERACION_ARITMETICA_INVALIDA: u32 = 6;
    pub const COMPARACION_INVALIDA: u32 = 7;
    pub const OPERACION_LOGICA_INVALIDA: u32 = 8;
    pub const NEGACION_ARITMETICA_INVALIDA: u32 = 9;
    pub const NEGACION_LOGICA_INVALIDA: u32 = 10;
    pub const CONDICIONAL_NO_BOOLEANO: u32 = 11;
    pub const BUCLE_NO_BOOLEANO: u32 = 12;
    pub const ASIGNACION_INCOMPATIBLE: u32 = 13;

    // T031-T039: Shadowing / declaraciones duplicadas (warnings, no errores)
    pub const FUNCION_DUPLICADA: u32 = 31;
    pub const STRUCT_DUPLICADO: u32 = 32;
    pub const ENUM_DUPLICADO: u32 = 33;
    pub const RASGO_DUPLICADO: u32 = 34;
    pub const APODO_DUPLICADO: u32 = 35;

    // M001-M099: Módulos
    pub const VISIBILIDAD_PRIVADA: u32 = 1;
    pub const SIMBOLO_NO_ENCONTRADO: u32 = 2;
}

use codigos::*;

// ============================================
// CONCORDANCIA LINGÜÍSTICA
// ============================================

/// INNOVACIÓN SEMÁNTICA: Concordancia Lingüística en el Análisis de Tipos
///
/// El español requiere que adjetivos, artículos y sustantivos "concuerden"
/// en género y número. Esta innovación aplica el mismo principio al análisis
/// semántico de Falcato:
///
/// 1. **Concordancia de Género (Ownership)**: Los valores deben concordar
///    en su artículo (el/la/un) con el contexto. No puedes pasar 'el valor'
///    (owned) donde se espera 'la referencia' (borrowed).
///
/// 2. **Concordancia de Estado (Mutabilidad)**: 'ser' (inmutable) y 'estar'
///    (mutable) deben concordar con las operaciones. No puedes mutar algo
///    que 'es' (permanente).
///
/// 3. **Blame Tracking Lingüístico**: Cuando hay error, el mensaje indica
///    qué "categoría gramatical" falló, haciendo los errores intuitivos para
///    hispanohablantes.

/// Tabla de métodos: (nombre_tipo, nombre_método) → nombre_builtin
/// Permite sintaxis t.metodo(args) → se desugarea a llamada built-in
pub(crate) fn metodo_a_builtin(tipo: &Tipo, metodo: &str) -> Option<&'static str> {
    match tipo {
        Tipo::Texto => match metodo {
            "agregar" => Some("texto_agregar"),
            "tam" => Some("texto_longitud"),
            "liberar" => Some("texto_liberar"),
            "obtener" => Some("texto_obtener_byte"),
            "concatenar" => Some("texto_concatenar"),
            "subtexto" => Some("texto_subtexto"),
            "comparar" => Some("texto_comparar"),
            _ => None,
        },
        Tipo::Diccionario(_, _) => match metodo {
            "insertar" => Some("diccionario_insertar"),
            "obtener" => Some("diccionario_obtener"),
            "existe" => Some("diccionario_existe"),
            "eliminar" => Some("diccionario_eliminar"),
            "tam" => Some("diccionario_longitud"),
            "liberar" => Some("diccionario_liberar"),
            _ => None,
        },
        Tipo::Conjunto(_) => match metodo {
            "insertar" => Some("conjunto_insertar"),
            "contiene" => Some("conjunto_contiene"),
            "eliminar" => Some("conjunto_eliminar"),
            "tam" => Some("conjunto_longitud"),
            "liberar" => Some("conjunto_liberar"),
            _ => None,
        },
        Tipo::Vector(_) => match metodo {
            "agregar" => Some("vector_agregar"),
            "tam" => Some("vector_longitud"),
            "obtener" => Some("vector_obtener"),
            "liberar" => Some("vector_liberar"),
            _ => None,
        },
        _ => None,
    }
}

/// Información semántica de una variable
#[derive(Debug, Clone)]
pub struct InfoVariable {
    pub nombre: String,
    pub tipo: Tipo,
    pub articulo: Articulo,
    pub span: Span,
}

/// Entorno de variables (scope)
#[derive(Debug, Clone, Default)]
pub struct Entorno {
    variables: HashMap<String, InfoVariable>,
    tipos: HashMap<String, Tipo>,      // type params: T -> Generico("T")
    consts: HashMap<String, (Tipo, Option<usize>)>, // const params: N -> (Entero32, None)
    padre: Option<Box<Entorno>>,
}

impl Entorno {
    pub fn nuevo() -> Self {
        Self::default()
    }

    pub fn con_padre(padre: Entorno) -> Self {
        Self {
            variables: HashMap::new(),
            tipos: HashMap::new(),
            consts: HashMap::new(),
            padre: Some(Box::new(padre)),
        }
    }

    pub fn declarar(&mut self, info: InfoVariable) {
        self.variables.insert(info.nombre.clone(), info);
    }

    pub fn declarar_tipo(&mut self, nombre: String, tipo: Tipo) {
        self.tipos.insert(nombre, tipo);
    }

    pub fn declarar_const(&mut self, nombre: String, tipo: Tipo) {
        self.consts.insert(nombre, (tipo, None));
    }

    pub fn buscar(&self, nombre: &str) -> Option<&InfoVariable> {
        self.variables.get(nombre).or_else(|| {
            self.padre.as_ref().and_then(|p| p.buscar(nombre))
        })
    }

    pub fn buscar_tipo(&self, nombre: &str) -> Option<&Tipo> {
        self.tipos.get(nombre).or_else(|| {
            self.padre.as_ref().and_then(|p| p.buscar_tipo(nombre))
        })
    }

    pub fn buscar_const(&self, nombre: &str) -> Option<&(Tipo, Option<usize>)> {
        self.consts.get(nombre).or_else(|| {
            self.padre.as_ref().and_then(|p| p.buscar_const(nombre))
        })
    }

    /// Recolecta todos los nombres de variables en este scope y padres
    pub fn todos_nombres(&self) -> Vec<String> {
        let mut nombres: Vec<String> = self.variables.keys().cloned().collect();
        if let Some(ref padre) = self.padre {
            nombres.extend(padre.todos_nombres());
        }
        nombres
    }
}

/// Distancia de Levenshtein simple para sugerencias de nombres
pub(crate) fn distancia_levenshtein(a: &str, b: &str) -> usize {
    let la = a.len();
    let lb = b.len();
    if la == 0 { return lb; }
    if lb == 0 { return la; }
    
    let mut fila: Vec<usize> = (0..=lb).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut prev = fila[0];
        fila[0] = i + 1;
        for (j, cb) in b.chars().enumerate() {
            let temp = fila[j + 1];
            fila[j + 1] = if ca == cb {
                prev
            } else {
                1 + prev.min(fila[j]).min(fila[j + 1])
            };
            prev = temp;
        }
    }
    fila[lb]
}

/// Encuentra el nombre más similar en una lista
pub(crate) fn sugerir_nombre(escrito: &str, disponibles: &[String]) -> Option<String> {
    let mut mejor: Option<(usize, &String)> = None;
    for nombre in disponibles {
        let d = distancia_levenshtein(escrito, nombre);
        let limite = if escrito.len() <= 3 { 1 } else { (escrito.len() + 2) / 3 };
        if d <= limite {
            match mejor {
                Some((d_mejor, _)) if d < d_mejor => mejor = Some((d, nombre)),
                None => mejor = Some((d, nombre)),
                _ => {}
            }
        }
    }
    mejor.map(|(_, n)| n.clone())
}

/// Información de un struct declarado
#[derive(Debug, Clone)]
pub struct InfoStruct {
    pub nombre: String,
    pub campos: Vec<Campo>,
    pub campos_bits: Vec<CampoBits>,
    pub span: Span,
}

/// Firma de función para verificación de llamadas
#[derive(Debug, Clone)]
pub struct FirmaFuncion {
    pub nombre: String,
    pub parametros_genericos: Vec<ParametroGenerico>,
    pub parametros: Vec<(String, Tipo)>, // nombre, tipo
    pub retorno: Option<Tipo>,
    pub span: Span,
    pub es_publica: bool,
}

/// Información de una enumeración
#[derive(Debug, Clone)]
pub struct InfoEnum {
    pub nombre: String,
    pub parametros_genericos: Vec<ParametroGenerico>,
    pub variantes: Vec<Variante>,
    pub span: Span,
}

/// Información de un rasgo (trait)
#[derive(Debug, Clone)]
pub struct InfoRasgo {
    pub nombre: String,
    pub metodos: Vec<crate::ast::FirmaMetodo>,
    pub span: Span,
}

/// Estado de borrow de una variable
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorrowState {
    None,           // Sin borrows
    Shared(usize),  // N borrows inmutables (&T)
    Exclusive,      // 1 borrow mutable (&mut T)
}

/// Analizador semántico con concordancia lingüística
pub struct AnalizadorSemantico {
    pub(crate) errores: Errores,
    pub(crate) entorno: Entorno,
    pub(crate) funcion_actual: Option<FuncionDecl>,
    pub(crate) structs: HashMap<String, InfoStruct>,
    pub(crate) enums: HashMap<String, InfoEnum>,
    pub(crate) funciones: HashMap<String, FirmaFuncion>,
    /// Imports: nombre_corto → nombre_cualificado (ej: "suma" → "matematicas::suma")
    pub(crate) imports: HashMap<String, String>,
    /// Imports glob: lista de prefijos de módulo (ej: "matematicas")
    pub(crate) glob_imports: Vec<String>,
    /// Stack de módulos actual para registro de nombres cualificados
    pub(crate) modulo_actual: Vec<String>,
    /// Símbolos públicos de otros módulos (nombre cualificado → firma)
    pub(crate) simbolos_publicos_importados: HashMap<String, FirmaFuncion>,
    /// Structs públicos de otros módulos (nombre cualificado → info) — para `usar modulo::*`
    pub(crate) structs_importados: HashMap<String, InfoStruct>,
    /// Enums públicos de otros módulos (nombre cualificado → info) — para `usar modulo::*`
    pub(crate) enums_importados: HashMap<String, InfoEnum>,
    /// Variables movidas en la función actual (para use-after-move detection)
    pub(crate) variables_movidas: std::collections::HashSet<String>,
    /// Nivel de verificación de ownership de la función actual
    pub(crate) nivel_verificacion_actual: crate::ast::NivelVerificacion,
    /// Estado de borrow de cada variable (para borrowing rules)
    pub(crate) borrows: HashMap<String, BorrowState>,
    /// Profundidad de bucles activos — para validar romper/continuar
    pub(crate) profundidad_bucle: u32,
    /// Efecto de la función actual (para verificación de anotaciones)
    pub(crate) efecto_actual: crate::ast::Efecto,
    /// Rasgos (traits) registrados: nombre → InfoRasgo
    pub(crate) rasgos: HashMap<String, InfoRasgo>,
    /// Impls registrados: (rasgo, tipo) → métodos
    pub(crate) impls: HashMap<(String, String), Vec<String>>,
    /// Alias de tipos: nombre → Tipo (ej: "Entero" → Entero32)
    pub(crate) aliases: HashMap<String, Tipo>,
}

impl AnalizadorSemantico {
    pub fn nuevo() -> Self {
        let mut analizador = Self {
            errores: Errores::nuevo(),
            entorno: Entorno::nuevo(),
            funcion_actual: None,
            structs: HashMap::new(),
            enums: HashMap::new(),
            funciones: HashMap::new(),
            imports: HashMap::new(),
            glob_imports: Vec::new(),
            modulo_actual: Vec::new(),
            simbolos_publicos_importados: HashMap::new(),
            structs_importados: HashMap::new(),
            enums_importados: HashMap::new(),
            variables_movidas: std::collections::HashSet::new(),
            nivel_verificacion_actual: crate::ast::NivelVerificacion::Permisivo,
            borrows: HashMap::new(),
            profundidad_bucle: 0,
            efecto_actual: crate::ast::Efecto::Conservador,
            rasgos: HashMap::new(),
            impls: HashMap::new(),
            aliases: HashMap::new(),
        };
        analizador.registrar_builtins();
        analizador
    }

    /// Crea analizador con símbolos públicos de otros módulos pre-cargados (solo funciones).
    pub fn con_simbolos_publicos(simbolos: HashMap<String, FirmaFuncion>) -> Self {
        Self::con_simbolos_publicos_completo(simbolos, HashMap::new(), HashMap::new())
    }

    /// Crea analizador con símbolos públicos completos de otros módulos.
    pub fn con_simbolos_publicos_completo(
        simbolos: HashMap<String, FirmaFuncion>,
        structs: HashMap<String, InfoStruct>,
        enums: HashMap<String, InfoEnum>,
    ) -> Self {
        let mut analizador = Self {
            errores: Errores::nuevo(),
            entorno: Entorno::nuevo(),
            funcion_actual: None,
            structs: HashMap::new(),
            enums: HashMap::new(),
            funciones: HashMap::new(),
            imports: HashMap::new(),
            glob_imports: Vec::new(),
            modulo_actual: Vec::new(),
            simbolos_publicos_importados: simbolos,
            structs_importados: structs,
            enums_importados: enums,
            variables_movidas: std::collections::HashSet::new(),
            nivel_verificacion_actual: crate::ast::NivelVerificacion::Permisivo,
            borrows: HashMap::new(),
            profundidad_bucle: 0,
            efecto_actual: crate::ast::Efecto::Conservador,
            rasgos: HashMap::new(),
            impls: HashMap::new(),
            aliases: HashMap::new(),
        };
        analizador.registrar_builtins();
        analizador
    }

    /// Registra funciones built-in del compilador: operaciones sobre Texto y Vector<T>.
    fn registrar_builtins(&mut self) {
        let vacio = Tipo::Vacio;
        let span_vacio = Span::vacio();

        // I/O básico: imprimir / imprimir_linea
        self.funciones.insert("imprimir".to_string(), FirmaFuncion {
            nombre: "imprimir".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("mensaje".to_string(), Tipo::Palabra)],
            retorno: Some(vacio.clone()),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("imprimir_linea".to_string(), FirmaFuncion {
            nombre: "imprimir_linea".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("mensaje".to_string(), Tipo::Palabra)],
            retorno: Some(vacio.clone()),
            span: span_vacio.clone(),
            es_publica: true,
        });
        // Alias: decir = imprimir_linea
        self.funciones.insert("decir".to_string(), FirmaFuncion {
            nombre: "decir".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("mensaje".to_string(), Tipo::Palabra)],
            retorno: Some(vacio.clone()),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("afirmar".to_string(), FirmaFuncion {
            nombre: "afirmar".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("condicion".to_string(), Tipo::Booleano)],
            retorno: Some(vacio.clone()),
            span: span_vacio.clone(),
            es_publica: true,
        });

        // tamaño_de::<T>() — sizeof comptime
        let t_size = ParametroGenerico {
            nombre: "T".to_string(),
            tipo: None,
            bounds: vec![],
            span: span_vacio.clone(),
        };
        self.funciones.insert("tamaño_de".to_string(), FirmaFuncion {
            nombre: "tamaño_de".to_string(),
            parametros_genericos: vec![t_size],
            parametros: vec![],
            retorno: Some(Tipo::Entero64),
            span: span_vacio.clone(),
            es_publica: true,
        });

        // Async (Fase 18A): dormir(ms) — suspende la tarea actual
        self.funciones.insert("dormir".to_string(), FirmaFuncion {
            nombre: "dormir".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("ms".to_string(), Tipo::Entero32)],
            retorno: Some(vacio.clone()),
            span: span_vacio.clone(),
            es_publica: true,
        });

        // TCP (Fase 18B): I/O de red
        self.funciones.insert("tcp_vincular".to_string(), FirmaFuncion {
            nombre: "tcp_vincular".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("puerto".to_string(), Tipo::Entero32)],
            retorno: Some(Tipo::Entero64),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("tcp_aceptar".to_string(), FirmaFuncion {
            nombre: "tcp_aceptar".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("listener".to_string(), Tipo::Entero64)],
            retorno: Some(Tipo::Entero64),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("tcp_leer".to_string(), FirmaFuncion {
            nombre: "tcp_leer".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("socket".to_string(), Tipo::Entero64),
                ("buffer".to_string(), Tipo::Entero64),
                ("tam".to_string(), Tipo::Entero32),
            ],
            retorno: Some(Tipo::Entero32),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("tcp_escribir".to_string(), FirmaFuncion {
            nombre: "tcp_escribir".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("socket".to_string(), Tipo::Entero64),
                ("buffer".to_string(), Tipo::Entero64),
                ("tam".to_string(), Tipo::Entero32),
            ],
            retorno: Some(Tipo::Entero32),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("tcp_cerrar".to_string(), FirmaFuncion {
            nombre: "tcp_cerrar".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("socket".to_string(), Tipo::Entero64)],
            retorno: Some(vacio.clone()),
            span: span_vacio.clone(),
            es_publica: true,
        });

        // Procesos (R7.1)
        self.funciones.insert("proceso_crear".to_string(), FirmaFuncion {
            nombre: "proceso_crear".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("comando".to_string(), Tipo::Palabra)],
            retorno: Some(Tipo::Entero64),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("proceso_esperar".to_string(), FirmaFuncion {
            nombre: "proceso_esperar".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("handle".to_string(), Tipo::Entero64)],
            retorno: Some(Tipo::Entero32),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("proceso_leer_salida_completa".to_string(), FirmaFuncion {
            nombre: "proceso_leer_salida_completa".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("handle".to_string(), Tipo::Entero64)],
            retorno: Some(Tipo::Texto),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("proceso_cerrar".to_string(), FirmaFuncion {
            nombre: "proceso_cerrar".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("handle".to_string(), Tipo::Entero64)],
            retorno: Some(vacio.clone()),
            span: span_vacio.clone(),
            es_publica: true,
        });

        // Proceso bidireccional (pipes para MCP servers)
        self.funciones.insert("proceso_crear_con_pipes".to_string(), FirmaFuncion {
            nombre: "proceso_crear_con_pipes".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("comando".to_string(), Tipo::Palabra)],
            retorno: Some(Tipo::Entero64),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("proceso_escribir".to_string(), FirmaFuncion {
            nombre: "proceso_escribir".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("handle".to_string(), Tipo::Entero64),
                ("datos".to_string(), Tipo::Entero64),
                ("n".to_string(), Tipo::Entero32),
            ],
            retorno: Some(Tipo::Entero32),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("proceso_leer_salida_chunk".to_string(), FirmaFuncion {
            nombre: "proceso_leer_salida_chunk".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("handle".to_string(), Tipo::Entero64),
                ("buf".to_string(), Tipo::Entero64),
                ("n".to_string(), Tipo::Entero32),
            ],
            retorno: Some(Tipo::Entero32),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("proceso_leer_error_chunk".to_string(), FirmaFuncion {
            nombre: "proceso_leer_error_chunk".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("handle".to_string(), Tipo::Entero64),
                ("buf".to_string(), Tipo::Entero64),
                ("n".to_string(), Tipo::Entero32),
            ],
            retorno: Some(Tipo::Entero32),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("proceso_cerrar_entrada".to_string(), FirmaFuncion {
            nombre: "proceso_cerrar_entrada".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("handle".to_string(), Tipo::Entero64)],
            retorno: Some(vacio.clone()),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("proceso_listo_para_leer".to_string(), FirmaFuncion {
            nombre: "proceso_listo_para_leer".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("handle".to_string(), Tipo::Entero64),
                ("ms".to_string(), Tipo::Entero32),
            ],
            retorno: Some(Tipo::Booleano),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("proceso_cerrar_bidireccional".to_string(), FirmaFuncion {
            nombre: "proceso_cerrar_bidireccional".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("handle".to_string(), Tipo::Entero64)],
            retorno: Some(vacio.clone()),
            span: span_vacio.clone(),
            es_publica: true,
        });

        // TCP Cliente + DNS
        self.funciones.insert("tcp_conectar".to_string(), FirmaFuncion {
            nombre: "tcp_conectar".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("host".to_string(), Tipo::Texto),
                ("puerto".to_string(), Tipo::Entero32),
            ],
            retorno: Some(Tipo::Entero64),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("dns_resolver".to_string(), FirmaFuncion {
            nombre: "dns_resolver".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("host".to_string(), Tipo::Texto)],
            retorno: Some(Tipo::Texto),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("tcp_establecer_timeout".to_string(), FirmaFuncion {
            nombre: "tcp_establecer_timeout".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("sock".to_string(), Tipo::Entero64),
                ("ms".to_string(), Tipo::Entero32),
            ],
            retorno: Some(vacio.clone()),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("tcp_datos_disponibles".to_string(), FirmaFuncion {
            nombre: "tcp_datos_disponibles".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("sock".to_string(), Tipo::Entero64)],
            retorno: Some(Tipo::Booleano),
            span: span_vacio.clone(),
            es_publica: true,
        });

        // Texto dinámico (R7.8 FASE 2)
        self.funciones.insert("texto_agregar_texto".to_string(), FirmaFuncion {
            nombre: "texto_agregar_texto".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("texto".to_string(), Tipo::ReferenciaMut(Box::new(Tipo::Texto))),
                ("fragmento".to_string(), Tipo::Texto),
            ],
            retorno: Some(vacio.clone()),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("texto_poner_byte".to_string(), FirmaFuncion {
            nombre: "texto_poner_byte".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("texto".to_string(), Tipo::ReferenciaMut(Box::new(Tipo::Texto))),
                ("indice".to_string(), Tipo::Entero32),
                ("byte".to_string(), Tipo::Entero32),
            ],
            retorno: Some(vacio.clone()),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("texto_puntero".to_string(), FirmaFuncion {
            nombre: "texto_puntero".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("texto".to_string(), Tipo::Texto)],
            retorno: Some(Tipo::Entero64),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("texto_desde_bytes".to_string(), FirmaFuncion {
            nombre: "texto_desde_bytes".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("ptr".to_string(), Tipo::Entero64),
                ("longitud".to_string(), Tipo::Entero32),
            ],
            retorno: Some(Tipo::Texto),
            span: span_vacio.clone(),
            es_publica: true,
        });

        // Conversión numérica (R7.8 FASE 3)
        self.funciones.insert("entero_a_texto".to_string(), FirmaFuncion {
            nombre: "entero_a_texto".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("n".to_string(), Tipo::Entero64)],
            retorno: Some(Tipo::Texto),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("flotante_a_texto".to_string(), FirmaFuncion {
            nombre: "flotante_a_texto".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("f".to_string(), Tipo::Flotante64)],
            retorno: Some(Tipo::Texto),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("booleano_a_texto".to_string(), FirmaFuncion {
            nombre: "booleano_a_texto".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("b".to_string(), Tipo::Booleano)],
            retorno: Some(Tipo::Texto),
            span: span_vacio.clone(),
            es_publica: true,
        });

        // Archivos avanzados + entorno (R7.8 FASE 4)
        self.funciones.insert("archivo_agregar".to_string(), FirmaFuncion {
            nombre: "archivo_agregar".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("ruta".to_string(), Tipo::Texto),
                ("texto".to_string(), Tipo::Texto),
            ],
            retorno: Some(vacio.clone()),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("archivo_borrar".to_string(), FirmaFuncion {
            nombre: "archivo_borrar".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("ruta".to_string(), Tipo::Texto)],
            retorno: Some(vacio.clone()),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("archivo_renombrar".to_string(), FirmaFuncion {
            nombre: "archivo_renombrar".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("vieja".to_string(), Tipo::Texto),
                ("nueva".to_string(), Tipo::Texto),
            ],
            retorno: Some(vacio.clone()),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("archivo_escribir_bytes".to_string(), FirmaFuncion {
            nombre: "archivo_escribir_bytes".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("ruta".to_string(), Tipo::Texto),
                ("datos".to_string(), Tipo::Entero64),
                ("n".to_string(), Tipo::Entero32),
            ],
            retorno: Some(vacio.clone()),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("entorno_obtener".to_string(), FirmaFuncion {
            nombre: "entorno_obtener".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("nombre".to_string(), Tipo::Texto)],
            retorno: Some(Tipo::Texto),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("directorio_actual".to_string(), FirmaFuncion {
            nombre: "directorio_actual".to_string(),
            parametros_genericos: vec![],
            parametros: vec![],
            retorno: Some(Tipo::Texto),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("aleatorio".to_string(), FirmaFuncion {
            nombre: "aleatorio".to_string(),
            parametros_genericos: vec![],
            parametros: vec![],
            retorno: Some(Tipo::Entero64),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("archivo_listar".to_string(), FirmaFuncion {
            nombre: "archivo_listar".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("dir".to_string(), Tipo::Texto)],
            retorno: Some(Tipo::Vector(Box::new(Tipo::Texto))),
            span: span_vacio.clone(),
            es_publica: true,
        });

        // TLS/HTTPS (R7.8 FASE 5)
        self.funciones.insert("tls_conectar".to_string(), FirmaFuncion {
            nombre: "tls_conectar".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("host".to_string(), Tipo::Texto),
                ("puerto".to_string(), Tipo::Entero32),
            ],
            retorno: Some(Tipo::Entero64),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("tls_escribir".to_string(), FirmaFuncion {
            nombre: "tls_escribir".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("conn".to_string(), Tipo::Entero64),
                ("datos".to_string(), Tipo::Entero64),
                ("n".to_string(), Tipo::Entero32),
            ],
            retorno: Some(Tipo::Entero32),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("tls_leer".to_string(), FirmaFuncion {
            nombre: "tls_leer".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("conn".to_string(), Tipo::Entero64),
                ("buf".to_string(), Tipo::Entero64),
                ("n".to_string(), Tipo::Entero32),
            ],
            retorno: Some(Tipo::Entero32),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("tls_datos_disponibles".to_string(), FirmaFuncion {
            nombre: "tls_datos_disponibles".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("conn".to_string(), Tipo::Entero64)],
            retorno: Some(Tipo::Entero32),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("tls_cerrar".to_string(), FirmaFuncion {
            nombre: "tls_cerrar".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("conn".to_string(), Tipo::Entero64)],
            retorno: Some(vacio.clone()),
            span: span_vacio.clone(),
            es_publica: true,
        });

        // Terminal (R7.2)
        self.funciones.insert("terminal_modo_raw".to_string(), FirmaFuncion {
            nombre: "terminal_modo_raw".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("activo".to_string(), Tipo::Entero32)],
            retorno: Some(Tipo::Entero32),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("terminal_leer_tecla".to_string(), FirmaFuncion {
            nombre: "terminal_leer_tecla".to_string(),
            parametros_genericos: vec![],
            parametros: vec![],
            retorno: Some(Tipo::Entero32),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("terminal_dimensiones".to_string(), FirmaFuncion {
            nombre: "terminal_dimensiones".to_string(),
            parametros_genericos: vec![],
            parametros: vec![],
            retorno: Some(Tipo::Entero64),
            span: span_vacio.clone(),
            es_publica: true,
        });

        // Entrada estándar (R7.3)
        self.funciones.insert("entrada_leer".to_string(), FirmaFuncion {
            nombre: "entrada_leer".to_string(),
            parametros_genericos: vec![],
            parametros: vec![],
            retorno: Some(Tipo::Texto),
            span: span_vacio.clone(),
            es_publica: true,
        });

        // Argumentos de línea de comandos (R7.5)
        self.funciones.insert("argumentos".to_string(), FirmaFuncion {
            nombre: "argumentos".to_string(),
            parametros_genericos: vec![],
            parametros: vec![],
            retorno: Some(Tipo::Vector(Box::new(Tipo::Texto))),
            span: span_vacio.clone(),
            es_publica: true,
        });

        // Reloj de pared (R7.4)
        self.funciones.insert("fecha_unix".to_string(), FirmaFuncion {
            nombre: "fecha_unix".to_string(),
            parametros_genericos: vec![],
            parametros: vec![],
            retorno: Some(Tipo::Entero64),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("fecha_ms".to_string(), FirmaFuncion {
            nombre: "fecha_ms".to_string(),
            parametros_genericos: vec![],
            parametros: vec![],
            retorno: Some(Tipo::Entero64),
            span: span_vacio.clone(),
            es_publica: true,
        });

        // DHT (R8.2)
        self.funciones.insert("dht_nuevo".to_string(), FirmaFuncion {
            nombre: "dht_nuevo".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("puerto".to_string(), Tipo::Entero32)],
            retorno: Some(Tipo::Entero64),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("dht_publicar".to_string(), FirmaFuncion {
            nombre: "dht_publicar".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("handle".to_string(), Tipo::Entero64),
                ("clave".to_string(), Tipo::Palabra),
                ("valor".to_string(), Tipo::Palabra),
            ],
            retorno: Some(Tipo::Entero32),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("dht_consultar".to_string(), FirmaFuncion {
            nombre: "dht_consultar".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("handle".to_string(), Tipo::Entero64),
                ("clave".to_string(), Tipo::Palabra),
            ],
            retorno: Some(Tipo::Entero64),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("dht_bootstrap".to_string(), FirmaFuncion {
            nombre: "dht_bootstrap".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("handle".to_string(), Tipo::Entero64),
                ("direccion".to_string(), Tipo::Palabra),
                ("puerto".to_string(), Tipo::Entero32),
            ],
            retorno: Some(Tipo::Entero32),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("dht_cerrar".to_string(), FirmaFuncion {
            nombre: "dht_cerrar".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("handle".to_string(), Tipo::Entero64)],
            retorno: Some(vacio.clone()),
            span: span_vacio.clone(),
            es_publica: true,
        });

        // Canales (Fase 18C)
        self.funciones.insert("canal_nuevo".to_string(), FirmaFuncion {
            nombre: "canal_nuevo".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("capacidad".to_string(), Tipo::Entero32)],
            retorno: Some(Tipo::Entero64),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("canal_enviar".to_string(), FirmaFuncion {
            nombre: "canal_enviar".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("canal".to_string(), Tipo::Entero64),
                ("valor".to_string(), Tipo::Entero32),
            ],
            retorno: Some(vacio.clone()),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("canal_recibir".to_string(), FirmaFuncion {
            nombre: "canal_recibir".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("canal".to_string(), Tipo::Entero64)],
            retorno: Some(Tipo::Entero32),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("canal_cerrar".to_string(), FirmaFuncion {
            nombre: "canal_cerrar".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("canal".to_string(), Tipo::Entero64)],
            retorno: Some(vacio.clone()),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("canal_intentar".to_string(), FirmaFuncion {
            nombre: "canal_intentar".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("canal".to_string(), Tipo::Entero64)],
            retorno: Some(Tipo::Entero32),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("cancelar".to_string(), FirmaFuncion {
            nombre: "cancelar".to_string(),
            parametros_genericos: vec![],
            parametros: vec![],
            retorno: Some(vacio.clone()),
            span: span_vacio.clone(),
            es_publica: true,
        });

        // Texto: cadena heap-allocada
        self.funciones.insert("texto_nuevo".to_string(), FirmaFuncion {
            nombre: "texto_nuevo".to_string(),
            parametros_genericos: vec![],
            parametros: vec![],
            retorno: Some(Tipo::Texto),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("texto_desde".to_string(), FirmaFuncion {
            nombre: "texto_desde".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("mensaje".to_string(), Tipo::Palabra)],
            retorno: Some(Tipo::Texto),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("texto_agregar".to_string(), FirmaFuncion {
            nombre: "texto_agregar".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("texto".to_string(), Tipo::Texto),
                ("fragmento".to_string(), Tipo::Palabra),
            ],
            retorno: Some(vacio.clone()),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("texto_longitud".to_string(), FirmaFuncion {
            nombre: "texto_longitud".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("texto".to_string(), Tipo::Texto)],
            retorno: Some(Tipo::Entero32),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("texto_tam".to_string(), FirmaFuncion {
            nombre: "texto_tam".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("texto".to_string(), Tipo::Texto)],
            retorno: Some(Tipo::Entero32),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("texto_liberar".to_string(), FirmaFuncion {
            nombre: "texto_liberar".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("texto".to_string(), Tipo::Texto)],
            retorno: Some(vacio.clone()),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("texto_concatenar".to_string(), FirmaFuncion {
            nombre: "texto_concatenar".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("a".to_string(), Tipo::Texto),
                ("b".to_string(), Tipo::Texto),
            ],
            retorno: Some(Tipo::Texto),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("texto_subtexto".to_string(), FirmaFuncion {
            nombre: "texto_subtexto".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("texto".to_string(), Tipo::Texto),
                ("inicio".to_string(), Tipo::Entero32),
                ("fin".to_string(), Tipo::Entero32),
            ],
            retorno: Some(Tipo::Texto),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("texto_comparar".to_string(), FirmaFuncion {
            nombre: "texto_comparar".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("a".to_string(), Tipo::Texto),
                ("b".to_string(), Tipo::Texto),
            ],
            retorno: Some(Tipo::Entero32),
            span: span_vacio.clone(),
            es_publica: true,
        });
        // R9.0.x Â BUG-003: texto_igual/desigual retornan Booleano, no Entero32
        self.funciones.insert("texto_igual".to_string(), FirmaFuncion {
            nombre: "texto_igual".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("a".to_string(), Tipo::Texto),
                ("b".to_string(), Tipo::Texto),
            ],
            retorno: Some(Tipo::Booleano),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("texto_desigual".to_string(), FirmaFuncion {
            nombre: "texto_desigual".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("a".to_string(), Tipo::Texto),
                ("b".to_string(), Tipo::Texto),
            ],
            retorno: Some(Tipo::Booleano),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("texto_obtener_byte".to_string(), FirmaFuncion {
            nombre: "texto_obtener_byte".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("texto".to_string(), Tipo::Texto),
                ("indice".to_string(), Tipo::Entero32),
            ],
            retorno: Some(Tipo::Entero8),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("texto_a_entero".to_string(), FirmaFuncion {
            nombre: "texto_a_entero".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("texto".to_string(), Tipo::Texto)],
            retorno: Some(Tipo::Entero64),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("texto_a_natural".to_string(), FirmaFuncion {
            nombre: "texto_a_natural".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("texto".to_string(), Tipo::Texto)],
            retorno: Some(Tipo::Entero64),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("texto_a_flotante".to_string(), FirmaFuncion {
            nombre: "texto_a_flotante".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("texto".to_string(), Tipo::Texto)],
            retorno: Some(Tipo::Flotante64),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("texto_a_booleano".to_string(), FirmaFuncion {
            nombre: "texto_a_booleano".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("texto".to_string(), Tipo::Texto)],
            retorno: Some(Tipo::Booleano),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("como_entero64".to_string(), FirmaFuncion {
            nombre: "como_entero64".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("valor".to_string(), Tipo::Entero32)],
            retorno: Some(Tipo::Entero64),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("como_entero32".to_string(), FirmaFuncion {
            nombre: "como_entero32".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("valor".to_string(), Tipo::Entero32)],
            retorno: Some(Tipo::Entero32),
            span: span_vacio.clone(),
            es_publica: true,
        });
        for (nombre, ret) in [
            ("como_entero8", Tipo::Entero8),
            ("como_entero16", Tipo::Entero16),
            ("como_entero32", Tipo::Entero32),
            ("como_entero64", Tipo::Entero64),
            ("como_natural8", Tipo::Natural8),
            ("como_natural16", Tipo::Natural16),
            ("como_natural32", Tipo::Natural32),
            ("como_natural64", Tipo::Natural64),
            ("como_flotante32", Tipo::Flotante32),
            ("como_flotante64", Tipo::Flotante64),
        ] {
            self.funciones.insert(nombre.to_string(), FirmaFuncion {
                nombre: nombre.to_string(),
                parametros_genericos: vec![],
                parametros: vec![("valor".to_string(), Tipo::Entero32)],
                retorno: Some(ret),
                span: span_vacio.clone(),
                es_publica: true,
            });
        }
        self.funciones.insert("texto_a_puntero".to_string(), FirmaFuncion {
            nombre: "texto_a_puntero".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("texto".to_string(), Tipo::Palabra)],
            retorno: Some(Tipo::Entero64),
            span: span_vacio.clone(),
            es_publica: true,
        });

        // File I/O
        self.funciones.insert("archivo_leer".to_string(), FirmaFuncion {
            nombre: "archivo_leer".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("ruta".to_string(), Tipo::Texto)],
            retorno: Some(Tipo::Texto),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("archivo_escribir".to_string(), FirmaFuncion {
            nombre: "archivo_escribir".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("ruta".to_string(), Tipo::Texto),
                ("contenido".to_string(), Tipo::Texto),
            ],
            retorno: Some(Tipo::Entero32),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("archivo_existe".to_string(), FirmaFuncion {
            nombre: "archivo_existe".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("ruta".to_string(), Tipo::Texto)],
            retorno: Some(Tipo::Booleano),
            span: span_vacio.clone(),
            es_publica: true,
        });

        // Matemáticas
        self.funciones.insert("abs".to_string(), FirmaFuncion {
            nombre: "abs".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("x".to_string(), Tipo::Entero32)],
            retorno: Some(Tipo::Entero32),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("max".to_string(), FirmaFuncion {
            nombre: "max".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("a".to_string(), Tipo::Entero32),
                ("b".to_string(), Tipo::Entero32),
            ],
            retorno: Some(Tipo::Entero32),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("min".to_string(), FirmaFuncion {
            nombre: "min".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("a".to_string(), Tipo::Entero32),
                ("b".to_string(), Tipo::Entero32),
            ],
            retorno: Some(Tipo::Entero32),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("raiz".to_string(), FirmaFuncion {
            nombre: "raiz".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("x".to_string(), Tipo::Flotante64)],
            retorno: Some(Tipo::Flotante64),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("potencia".to_string(), FirmaFuncion {
            nombre: "potencia".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("base".to_string(), Tipo::Flotante64),
                ("exponente".to_string(), Tipo::Flotante64),
            ],
            retorno: Some(Tipo::Flotante64),
            span: span_vacio.clone(),
            es_publica: true,
        });

        // Trigonometría
        let trig_unario = |nombre: &str| FirmaFuncion {
            nombre: nombre.to_string(),
            parametros_genericos: vec![],
            parametros: vec![("x".to_string(), Tipo::Flotante64)],
            retorno: Some(Tipo::Flotante64),
            span: span_vacio.clone(),
            es_publica: true,
        };
        let trig_binario = |nombre: &str| FirmaFuncion {
            nombre: nombre.to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("y".to_string(), Tipo::Flotante64),
                ("x".to_string(), Tipo::Flotante64),
            ],
            retorno: Some(Tipo::Flotante64),
            span: span_vacio.clone(),
            es_publica: true,
        };

        self.funciones.insert("seno".to_string(), trig_unario("seno"));
        self.funciones.insert("coseno".to_string(), trig_unario("coseno"));
        self.funciones.insert("tangente".to_string(), trig_unario("tangente"));
        self.funciones.insert("arcseno".to_string(), trig_unario("arcseno"));
        self.funciones.insert("arccoseno".to_string(), trig_unario("arccoseno"));
        self.funciones.insert("arctangente".to_string(), trig_unario("arctangente"));
        self.funciones.insert("arctangente2".to_string(), trig_binario("arctangente2"));
        self.funciones.insert("senoh".to_string(), trig_unario("senoh"));
        self.funciones.insert("cosenoh".to_string(), trig_unario("cosenoh"));
        self.funciones.insert("tangenteh".to_string(), trig_unario("tangenteh"));
        self.funciones.insert("exp".to_string(), trig_unario("exp"));
        self.funciones.insert("log".to_string(), trig_unario("log"));
        self.funciones.insert("log10".to_string(), trig_unario("log10"));
        self.funciones.insert("piso".to_string(), trig_unario("piso"));
        self.funciones.insert("techo".to_string(), trig_unario("techo"));
        self.funciones.insert("fabs".to_string(), trig_unario("fabs"));
        self.funciones.insert("fmod".to_string(), trig_binario("fmod"));

        self.funciones.insert("seno_preciso".to_string(), trig_unario("seno_preciso"));
        self.funciones.insert("coseno_preciso".to_string(), trig_unario("coseno_preciso"));
        self.funciones.insert("tangente_preciso".to_string(), trig_unario("tangente_preciso"));
        self.funciones.insert("exp_preciso".to_string(), trig_unario("exp_preciso"));
        self.funciones.insert("log_preciso".to_string(), trig_unario("log_preciso"));

        self.funciones.insert("seno_rapido".to_string(), trig_unario("seno_rapido"));
        self.funciones.insert("coseno_rapido".to_string(), trig_unario("coseno_rapido"));
        self.funciones.insert("seno_2pi".to_string(), trig_unario("seno_2pi"));
        self.funciones.insert("coseno_2pi".to_string(), trig_unario("coseno_2pi"));
        self.funciones.insert("exp_rapido".to_string(), trig_unario("exp_rapido"));
        self.funciones.insert("log_rapido".to_string(), trig_unario("log_rapido"));
        self.funciones.insert("seno_aprox".to_string(), trig_unario("seno_aprox"));

        // Vector<T>
        let t_generico = ParametroGenerico {
            nombre: "T".to_string(),
            tipo: None,
            bounds: vec![],
            span: span_vacio.clone(),
        };
        let tipo_t = Tipo::Generico("T".to_string());

        self.funciones.insert("vector_nuevo".to_string(), FirmaFuncion {
            nombre: "vector_nuevo".to_string(),
            parametros_genericos: vec![t_generico.clone()],
            parametros: vec![],
            retorno: Some(Tipo::Vector(Box::new(tipo_t.clone()))),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("vector_agregar".to_string(), FirmaFuncion {
            nombre: "vector_agregar".to_string(),
            parametros_genericos: vec![t_generico.clone()],
            parametros: vec![
                ("vector".to_string(), Tipo::Vector(Box::new(tipo_t.clone()))),
                ("valor".to_string(), tipo_t.clone()),
            ],
            retorno: Some(vacio.clone()),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("vector_obtener".to_string(), FirmaFuncion {
            nombre: "vector_obtener".to_string(),
            parametros_genericos: vec![t_generico.clone()],
            parametros: vec![
                ("vector".to_string(), Tipo::Vector(Box::new(tipo_t.clone()))),
                ("indice".to_string(), Tipo::Entero32),
            ],
            retorno: Some(tipo_t.clone()),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("vector_longitud".to_string(), FirmaFuncion {
            nombre: "vector_longitud".to_string(),
            parametros_genericos: vec![t_generico.clone()],
            parametros: vec![
                ("vector".to_string(), Tipo::Vector(Box::new(tipo_t.clone()))),
            ],
            retorno: Some(Tipo::Entero32),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("vector_tam".to_string(), FirmaFuncion {
            nombre: "vector_tam".to_string(),
            parametros_genericos: vec![t_generico.clone()],
            parametros: vec![
                ("vector".to_string(), Tipo::Vector(Box::new(tipo_t.clone()))),
            ],
            retorno: Some(Tipo::Entero32),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("vector_liberar".to_string(), FirmaFuncion {
            nombre: "vector_liberar".to_string(),
            parametros_genericos: vec![t_generico.clone()],
            parametros: vec![
                ("vector".to_string(), Tipo::Vector(Box::new(tipo_t.clone()))),
            ],
            retorno: Some(vacio.clone()),
            span: span_vacio.clone(),
            es_publica: true,
        });

        // Diccionario<K, V>
        let k_generico = ParametroGenerico {
            nombre: "K".to_string(),
            tipo: None,
            bounds: vec![],
            span: span_vacio.clone(),
        };
        let v_generico = ParametroGenerico {
            nombre: "V".to_string(),
            tipo: None,
            bounds: vec![],
            span: span_vacio.clone(),
        };
        let tipo_k = Tipo::Generico("K".to_string());
        let tipo_v = Tipo::Generico("V".to_string());

        self.funciones.insert("diccionario_nuevo".to_string(), FirmaFuncion {
            nombre: "diccionario_nuevo".to_string(),
            parametros_genericos: vec![k_generico.clone(), v_generico.clone()],
            parametros: vec![],
            retorno: Some(Tipo::Diccionario(Box::new(tipo_k.clone()), Box::new(tipo_v.clone()))),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("diccionario_insertar".to_string(), FirmaFuncion {
            nombre: "diccionario_insertar".to_string(),
            parametros_genericos: vec![k_generico.clone(), v_generico.clone()],
            parametros: vec![
                ("diccionario".to_string(), Tipo::Diccionario(Box::new(tipo_k.clone()), Box::new(tipo_v.clone()))),
                ("clave".to_string(), tipo_k.clone()),
                ("valor".to_string(), tipo_v.clone()),
            ],
            retorno: Some(Tipo::Entero64),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("diccionario_obtener".to_string(), FirmaFuncion {
            nombre: "diccionario_obtener".to_string(),
            parametros_genericos: vec![k_generico.clone(), v_generico.clone()],
            parametros: vec![
                ("diccionario".to_string(), Tipo::Diccionario(Box::new(tipo_k.clone()), Box::new(tipo_v.clone()))),
                ("clave".to_string(), tipo_k.clone()),
            ],
            retorno: Some(tipo_v.clone()),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("diccionario_existe".to_string(), FirmaFuncion {
            nombre: "diccionario_existe".to_string(),
            parametros_genericos: vec![k_generico.clone(), v_generico.clone()],
            parametros: vec![
                ("diccionario".to_string(), Tipo::Diccionario(Box::new(tipo_k.clone()), Box::new(tipo_v.clone()))),
                ("clave".to_string(), tipo_k.clone()),
            ],
            retorno: Some(Tipo::Booleano),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("diccionario_eliminar".to_string(), FirmaFuncion {
            nombre: "diccionario_eliminar".to_string(),
            parametros_genericos: vec![k_generico.clone(), v_generico.clone()],
            parametros: vec![
                ("diccionario".to_string(), Tipo::Diccionario(Box::new(tipo_k.clone()), Box::new(tipo_v.clone()))),
                ("clave".to_string(), tipo_k.clone()),
            ],
            retorno: Some(Tipo::Booleano),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("diccionario_longitud".to_string(), FirmaFuncion {
            nombre: "diccionario_longitud".to_string(),
            parametros_genericos: vec![k_generico.clone(), v_generico.clone()],
            parametros: vec![
                ("diccionario".to_string(), Tipo::Diccionario(Box::new(tipo_k.clone()), Box::new(tipo_v.clone()))),
            ],
            retorno: Some(Tipo::Entero32),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("diccionario_liberar".to_string(), FirmaFuncion {
            nombre: "diccionario_liberar".to_string(),
            parametros_genericos: vec![k_generico.clone(), v_generico.clone()],
            parametros: vec![
                ("diccionario".to_string(), Tipo::Diccionario(Box::new(tipo_k.clone()), Box::new(tipo_v.clone()))),
            ],
            retorno: Some(vacio.clone()),
            span: span_vacio.clone(),
            es_publica: true,
        });

        // Conjunto<T>
        let s_generico = ParametroGenerico {
            nombre: "T".to_string(),
            tipo: None,
            bounds: vec![],
            span: span_vacio.clone(),
        };
        let tipo_s = Tipo::Generico("T".to_string());

        self.funciones.insert("conjunto_nuevo".to_string(), FirmaFuncion {
            nombre: "conjunto_nuevo".to_string(),
            parametros_genericos: vec![s_generico.clone()],
            parametros: vec![],
            retorno: Some(Tipo::Conjunto(Box::new(tipo_s.clone()))),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("conjunto_insertar".to_string(), FirmaFuncion {
            nombre: "conjunto_insertar".to_string(),
            parametros_genericos: vec![s_generico.clone()],
            parametros: vec![
                ("conjunto".to_string(), Tipo::Conjunto(Box::new(tipo_s.clone()))),
                ("valor".to_string(), tipo_s.clone()),
            ],
            retorno: Some(vacio.clone()),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("conjunto_contiene".to_string(), FirmaFuncion {
            nombre: "conjunto_contiene".to_string(),
            parametros_genericos: vec![s_generico.clone()],
            parametros: vec![
                ("conjunto".to_string(), Tipo::Conjunto(Box::new(tipo_s.clone()))),
                ("valor".to_string(), tipo_s.clone()),
            ],
            retorno: Some(Tipo::Booleano),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("conjunto_eliminar".to_string(), FirmaFuncion {
            nombre: "conjunto_eliminar".to_string(),
            parametros_genericos: vec![s_generico.clone()],
            parametros: vec![
                ("conjunto".to_string(), Tipo::Conjunto(Box::new(tipo_s.clone()))),
                ("valor".to_string(), tipo_s.clone()),
            ],
            retorno: Some(Tipo::Booleano),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("conjunto_longitud".to_string(), FirmaFuncion {
            nombre: "conjunto_longitud".to_string(),
            parametros_genericos: vec![s_generico.clone()],
            parametros: vec![
                ("conjunto".to_string(), Tipo::Conjunto(Box::new(tipo_s.clone()))),
            ],
            retorno: Some(Tipo::Entero32),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("conjunto_liberar".to_string(), FirmaFuncion {
            nombre: "conjunto_liberar".to_string(),
            parametros_genericos: vec![s_generico.clone()],
            parametros: vec![
                ("conjunto".to_string(), Tipo::Conjunto(Box::new(tipo_s.clone()))),
            ],
            retorno: Some(vacio.clone()),
            span: span_vacio.clone(),
            es_publica: true,
        });

        // Resultado<T, E>
        let t_generico_res = ParametroGenerico {
            nombre: "T".to_string(),
            tipo: None,
            bounds: vec![],
            span: span_vacio.clone(),
        };
        let e_generico_res = ParametroGenerico {
            nombre: "E".to_string(),
            tipo: None,
            bounds: vec![],
            span: span_vacio.clone(),
        };
        let tipo_t_res = Tipo::Generico("T".to_string());
        let tipo_e_res = Tipo::Generico("E".to_string());

        self.enums.insert("Resultado".to_string(), InfoEnum {
            nombre: "Resultado".to_string(),
            parametros_genericos: vec![t_generico_res, e_generico_res],
            variantes: vec![
                Variante {
                    nombre: "Exito".to_string(),
                    datos: Some(vec![("valor".to_string(), tipo_t_res)]),
                    span: span_vacio.clone(),
                },
                Variante {
                    nombre: "Error".to_string(),
                    datos: Some(vec![("error".to_string(), tipo_e_res)]),
                    span: span_vacio.clone(),
                },
            ],
            span: span_vacio.clone(),
        });

        // Option<T>
        let t_generico_opt = ParametroGenerico {
            nombre: "T".to_string(),
            tipo: None,
            bounds: vec![],
            span: span_vacio.clone(),
        };
        let tipo_t_opt = Tipo::Generico("T".to_string());

        // Memoria debug — lente graduable (niveles 0-3)
        self.funciones.insert("memoria_usada".to_string(), FirmaFuncion {
            nombre: "memoria_usada".to_string(),
            parametros_genericos: vec![],
            parametros: vec![],
            retorno: Some(Tipo::Entero64),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("memoria_volcar".to_string(), FirmaFuncion {
            nombre: "memoria_volcar".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("ptr".to_string(), Tipo::Entero64),
                ("n".to_string(), Tipo::Entero32),
            ],
            retorno: Some(Tipo::Vacio),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("memoria_rastrear".to_string(), FirmaFuncion {
            nombre: "memoria_rastrear".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("ptr".to_string(), Tipo::Entero64)],
            retorno: Some(Tipo::Vacio),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("memoria_canario_verificar".to_string(), FirmaFuncion {
            nombre: "memoria_canario_verificar".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("ptr".to_string(), Tipo::Entero64)],
            retorno: Some(Tipo::Booleano),
            span: span_vacio.clone(),
            es_publica: true,
        });

        // Perfil — reloj monotónico + marcas (nivel 0/1 nativo)
        self.funciones.insert("reloj_mono_ns".to_string(), FirmaFuncion {
            nombre: "reloj_mono_ns".to_string(),
            parametros_genericos: vec![],
            parametros: vec![],
            retorno: Some(Tipo::Entero64),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("perfil_inicio".to_string(), FirmaFuncion {
            nombre: "perfil_inicio".to_string(),
            parametros_genericos: vec![],
            parametros: vec![],
            retorno: Some(vacio.clone()),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("perfil_marca".to_string(), FirmaFuncion {
            nombre: "perfil_marca".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("etiqueta".to_string(), Tipo::Texto)],
            retorno: Some(vacio.clone()),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("perfil_reporte".to_string(), FirmaFuncion {
            nombre: "perfil_reporte".to_string(),
            parametros_genericos: vec![],
            parametros: vec![],
            retorno: Some(vacio.clone()),
            span: span_vacio.clone(),
            es_publica: true,
        });

        self.enums.insert("Option".to_string(), InfoEnum {
            nombre: "Option".to_string(),
            parametros_genericos: vec![t_generico_opt],
            variantes: vec![
                Variante {
                    nombre: "Algo".to_string(),
                    datos: Some(vec![("valor".to_string(), tipo_t_opt)]),
                    span: span_vacio.clone(),
                },
                Variante {
                    nombre: "Nada".to_string(),
                    datos: None,
                    span: span_vacio.clone(),
                },
            ],
            span: span_vacio.clone(),
        });

        // ============================================================
        // BUILTINS NUEVOS (v0.8.0) — Texto avanzado
        // ============================================================
        self.funciones.insert("texto_contiene".to_string(), FirmaFuncion {
            nombre: "texto_contiene".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("texto".to_string(), Tipo::Texto),
                ("sub".to_string(), Tipo::Texto),
            ],
            retorno: Some(Tipo::Booleano),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("texto_empieza_con".to_string(), FirmaFuncion {
            nombre: "texto_empieza_con".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("texto".to_string(), Tipo::Texto),
                ("prefijo".to_string(), Tipo::Texto),
            ],
            retorno: Some(Tipo::Booleano),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("texto_termina_con".to_string(), FirmaFuncion {
            nombre: "texto_termina_con".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("texto".to_string(), Tipo::Texto),
                ("sufijo".to_string(), Tipo::Texto),
            ],
            retorno: Some(Tipo::Booleano),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("texto_esta_vacio".to_string(), FirmaFuncion {
            nombre: "texto_esta_vacio".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("texto".to_string(), Tipo::Texto)],
            retorno: Some(Tipo::Booleano),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("texto_recortar".to_string(), FirmaFuncion {
            nombre: "texto_recortar".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("texto".to_string(), Tipo::Texto)],
            retorno: Some(Tipo::Texto),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("texto_mayusculas".to_string(), FirmaFuncion {
            nombre: "texto_mayusculas".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("texto".to_string(), Tipo::Texto)],
            retorno: Some(Tipo::Texto),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("texto_minusculas".to_string(), FirmaFuncion {
            nombre: "texto_minusculas".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("texto".to_string(), Tipo::Texto)],
            retorno: Some(Tipo::Texto),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("texto_reemplazar".to_string(), FirmaFuncion {
            nombre: "texto_reemplazar".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("texto".to_string(), Tipo::Texto),
                ("de".to_string(), Tipo::Texto),
                ("a".to_string(), Tipo::Texto),
            ],
            retorno: Some(Tipo::Texto),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("texto_dividir".to_string(), FirmaFuncion {
            nombre: "texto_dividir".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("texto".to_string(), Tipo::Texto),
                ("separador".to_string(), Tipo::Texto),
            ],
            retorno: Some(Tipo::Vector(Box::new(Tipo::Texto))),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("texto_a_bytes".to_string(), FirmaFuncion {
            nombre: "texto_a_bytes".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("texto".to_string(), Tipo::Texto)],
            retorno: Some(Tipo::Vector(Box::new(Tipo::Entero8))),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("texto_codificar_base64".to_string(), FirmaFuncion {
            nombre: "texto_codificar_base64".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("texto".to_string(), Tipo::Texto)],
            retorno: Some(Tipo::Texto),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("texto_decodificar_base64".to_string(), FirmaFuncion {
            nombre: "texto_decodificar_base64".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("texto".to_string(), Tipo::Texto)],
            retorno: Some(Tipo::Texto),
            span: span_vacio.clone(),
            es_publica: true,
        });

        // ============================================================
        // BUILTINS NUEVOS (v0.8.0) — Vector avanzado
        // ============================================================
        self.funciones.insert("vector_poner".to_string(), FirmaFuncion {
            nombre: "vector_poner".to_string(),
            parametros_genericos: vec![t_generico.clone()],
            parametros: vec![
                ("vector".to_string(), Tipo::Vector(Box::new(tipo_t.clone()))),
                ("indice".to_string(), Tipo::Entero32),
                ("valor".to_string(), tipo_t.clone()),
            ],
            retorno: Some(vacio.clone()),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("vector_intercambiar".to_string(), FirmaFuncion {
            nombre: "vector_intercambiar".to_string(),
            parametros_genericos: vec![t_generico.clone()],
            parametros: vec![
                ("vector".to_string(), Tipo::Vector(Box::new(tipo_t.clone()))),
                ("i".to_string(), Tipo::Entero32),
                ("j".to_string(), Tipo::Entero32),
            ],
            retorno: Some(vacio.clone()),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("vector_insertar".to_string(), FirmaFuncion {
            nombre: "vector_insertar".to_string(),
            parametros_genericos: vec![t_generico.clone()],
            parametros: vec![
                ("vector".to_string(), Tipo::Vector(Box::new(tipo_t.clone()))),
                ("indice".to_string(), Tipo::Entero32),
                ("valor".to_string(), tipo_t.clone()),
            ],
            retorno: Some(vacio.clone()),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("vector_eliminar".to_string(), FirmaFuncion {
            nombre: "vector_eliminar".to_string(),
            parametros_genericos: vec![t_generico.clone()],
            parametros: vec![
                ("vector".to_string(), Tipo::Vector(Box::new(tipo_t.clone()))),
                ("indice".to_string(), Tipo::Entero32),
            ],
            retorno: Some(vacio.clone()),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("vector_extender".to_string(), FirmaFuncion {
            nombre: "vector_extender".to_string(),
            parametros_genericos: vec![t_generico.clone()],
            parametros: vec![
                ("vector".to_string(), Tipo::Vector(Box::new(tipo_t.clone()))),
                ("otro".to_string(), Tipo::Vector(Box::new(tipo_t.clone()))),
            ],
            retorno: Some(vacio.clone()),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("vector_contiene".to_string(), FirmaFuncion {
            nombre: "vector_contiene".to_string(),
            parametros_genericos: vec![t_generico.clone()],
            parametros: vec![
                ("vector".to_string(), Tipo::Vector(Box::new(tipo_t.clone()))),
                ("valor".to_string(), tipo_t.clone()),
            ],
            retorno: Some(Tipo::Booleano),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("vector_indice_de".to_string(), FirmaFuncion {
            nombre: "vector_indice_de".to_string(),
            parametros_genericos: vec![t_generico.clone()],
            parametros: vec![
                ("vector".to_string(), Tipo::Vector(Box::new(tipo_t.clone()))),
                ("valor".to_string(), tipo_t.clone()),
            ],
            retorno: Some(Tipo::Entero32),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("vector_clonar".to_string(), FirmaFuncion {
            nombre: "vector_clonar".to_string(),
            parametros_genericos: vec![t_generico.clone()],
            parametros: vec![
                ("vector".to_string(), Tipo::Vector(Box::new(tipo_t.clone()))),
            ],
            retorno: Some(Tipo::Vector(Box::new(tipo_t.clone()))),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("vector_invertir".to_string(), FirmaFuncion {
            nombre: "vector_invertir".to_string(),
            parametros_genericos: vec![t_generico.clone()],
            parametros: vec![
                ("vector".to_string(), Tipo::Vector(Box::new(tipo_t.clone()))),
            ],
            retorno: Some(vacio.clone()),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("vector_limpiar".to_string(), FirmaFuncion {
            nombre: "vector_limpiar".to_string(),
            parametros_genericos: vec![t_generico.clone()],
            parametros: vec![
                ("vector".to_string(), Tipo::Vector(Box::new(tipo_t.clone()))),
            ],
            retorno: Some(vacio.clone()),
            span: span_vacio.clone(),
            es_publica: true,
        });

        // ============================================================
        // BUILTINS NUEVOS (v0.8.0) — Diccionario avanzado
        // ============================================================
        self.funciones.insert("diccionario_claves".to_string(), FirmaFuncion {
            nombre: "diccionario_claves".to_string(),
            parametros_genericos: vec![k_generico.clone(), v_generico.clone()],
            parametros: vec![
                ("diccionario".to_string(), Tipo::Diccionario(Box::new(tipo_k.clone()), Box::new(tipo_v.clone()))),
            ],
            retorno: Some(Tipo::Vector(Box::new(Tipo::Texto))),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("diccionario_valores".to_string(), FirmaFuncion {
            nombre: "diccionario_valores".to_string(),
            parametros_genericos: vec![k_generico.clone(), v_generico.clone()],
            parametros: vec![
                ("diccionario".to_string(), Tipo::Diccionario(Box::new(tipo_k.clone()), Box::new(tipo_v.clone()))),
            ],
            retorno: Some(Tipo::Vector(Box::new(Tipo::Texto))),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("diccionario_limpiar".to_string(), FirmaFuncion {
            nombre: "diccionario_limpiar".to_string(),
            parametros_genericos: vec![k_generico.clone(), v_generico.clone()],
            parametros: vec![
                ("diccionario".to_string(), Tipo::Diccionario(Box::new(tipo_k.clone()), Box::new(tipo_v.clone()))),
            ],
            retorno: Some(vacio.clone()),
            span: span_vacio.clone(),
            es_publica: true,
        });

        // ============================================================
        // BUILTINS NUEVOS (v0.8.0) — Conjunto avanzado
        // ============================================================
        self.funciones.insert("conjunto_elementos".to_string(), FirmaFuncion {
            nombre: "conjunto_elementos".to_string(),
            parametros_genericos: vec![s_generico.clone()],
            parametros: vec![
                ("conjunto".to_string(), Tipo::Conjunto(Box::new(tipo_s.clone()))),
            ],
            retorno: Some(Tipo::Vector(Box::new(Tipo::Texto))),
            span: span_vacio.clone(),
            es_publica: true,
        });

        // ============================================================
        // BUILTINS NUEVOS (v0.8.0) — Opción/Resultado
        // ============================================================
        self.funciones.insert("opcion_es_alguno".to_string(), FirmaFuncion {
            nombre: "opcion_es_alguno".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("opcion".to_string(), Tipo::Option(Box::new(Tipo::Entero64)))],
            retorno: Some(Tipo::Booleano),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("opcion_es_ninguno".to_string(), FirmaFuncion {
            nombre: "opcion_es_ninguno".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("opcion".to_string(), Tipo::Option(Box::new(Tipo::Entero64)))],
            retorno: Some(Tipo::Booleano),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("resultado_es_exito".to_string(), FirmaFuncion {
            nombre: "resultado_es_exito".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("resultado".to_string(), Tipo::Resultado(Box::new(Tipo::Entero64), Box::new(Tipo::Entero64)))],
            retorno: Some(Tipo::Booleano),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("resultado_es_error".to_string(), FirmaFuncion {
            nombre: "resultado_es_error".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("resultado".to_string(), Tipo::Resultado(Box::new(Tipo::Entero64), Box::new(Tipo::Entero64)))],
            retorno: Some(Tipo::Booleano),
            span: span_vacio.clone(),
            es_publica: true,
        });

        // ============================================================
        // BUILTINS NUEVOS (v0.8.0) — HTTP
        // ============================================================
        self.funciones.insert("http_get".to_string(), FirmaFuncion {
            nombre: "http_get".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("host".to_string(), Tipo::Texto),
                ("puerto".to_string(), Tipo::Entero32),
                ("path".to_string(), Tipo::Texto),
            ],
            retorno: Some(Tipo::Texto),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("http_post".to_string(), FirmaFuncion {
            nombre: "http_post".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("host".to_string(), Tipo::Texto),
                ("puerto".to_string(), Tipo::Entero32),
                ("path".to_string(), Tipo::Texto),
                ("cuerpo".to_string(), Tipo::Texto),
            ],
            retorno: Some(Tipo::Texto),
            span: span_vacio.clone(),
            es_publica: true,
        });

        // ============================================================
        // BUILTINS NUEVOS (v0.8.0) — JSON
        // ============================================================
        self.funciones.insert("json_parsear".to_string(), FirmaFuncion {
            nombre: "json_parsear".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("json".to_string(), Tipo::Texto)],
            retorno: Some(Tipo::Texto),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("json_serializar".to_string(), FirmaFuncion {
            nombre: "json_serializar".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("valor".to_string(), Tipo::Texto)],
            retorno: Some(Tipo::Texto),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("json_escapar".to_string(), FirmaFuncion {
            nombre: "json_escapar".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("texto".to_string(), Tipo::Texto)],
            retorno: Some(Tipo::Texto),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("json_obtener".to_string(), FirmaFuncion {
            nombre: "json_obtener".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("json".to_string(), Tipo::Texto),
                ("clave".to_string(), Tipo::Texto),
            ],
            retorno: Some(Tipo::Texto),
            span: span_vacio.clone(),
            es_publica: true,
        });

        // ============================================================
        // BUILTINS NUEVOS (v0.8.0) — Tiempo
        // ============================================================
        self.funciones.insert("fecha_anio".to_string(), FirmaFuncion {
            nombre: "fecha_anio".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("unix".to_string(), Tipo::Entero64)],
            retorno: Some(Tipo::Entero32),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("fecha_mes".to_string(), FirmaFuncion {
            nombre: "fecha_mes".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("unix".to_string(), Tipo::Entero64)],
            retorno: Some(Tipo::Entero32),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("fecha_dia".to_string(), FirmaFuncion {
            nombre: "fecha_dia".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("unix".to_string(), Tipo::Entero64)],
            retorno: Some(Tipo::Entero32),
            span: span_vacio.clone(),
            es_publica: true,
        });

        // ============================================================
        // BUILTINS NUEVOS (v0.8.0) — Archivo avanzado
        // ============================================================
        self.funciones.insert("archivo_tamano".to_string(), FirmaFuncion {
            nombre: "archivo_tamano".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("ruta".to_string(), Tipo::Texto)],
            retorno: Some(Tipo::Entero64),
            span: span_vacio.clone(),
            es_publica: true,
        });

        // ============================================================
        // BUILTINS NUEVOS (v0.8.0) — TCP avanzado
        // ============================================================
        self.funciones.insert("tcp_enviar".to_string(), FirmaFuncion {
            nombre: "tcp_enviar".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("conn".to_string(), Tipo::Entero64),
                ("datos".to_string(), Tipo::Texto),
            ],
            retorno: Some(Tipo::Entero32),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("tcp_recibir".to_string(), FirmaFuncion {
            nombre: "tcp_recibir".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("conn".to_string(), Tipo::Entero64),
                ("tamano".to_string(), Tipo::Entero32),
            ],
            retorno: Some(Tipo::Texto),
            span: span_vacio.clone(),
            es_publica: true,
        });

        // ============================================================
        // BUILTINS NUEVOS (v0.8.0) — TLS avanzado
        // ============================================================
        self.funciones.insert("tls_escribir_texto".to_string(), FirmaFuncion {
            nombre: "tls_escribir_texto".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("conn".to_string(), Tipo::Entero64),
                ("datos".to_string(), Tipo::Texto),
            ],
            retorno: Some(Tipo::Entero32),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("tls_leer_texto".to_string(), FirmaFuncion {
            nombre: "tls_leer_texto".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("conn".to_string(), Tipo::Entero64),
                ("tamano".to_string(), Tipo::Entero32),
            ],
            retorno: Some(Tipo::Texto),
            span: span_vacio.clone(),
            es_publica: true,
        });

        // ============================================================
        // BUILTINS NUEVOS (v0.8.0) — Sistema
        // ============================================================
        self.funciones.insert("argumentos_cantidad".to_string(), FirmaFuncion {
            nombre: "argumentos_cantidad".to_string(),
            parametros_genericos: vec![],
            parametros: vec![],
            retorno: Some(Tipo::Entero32),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("argumentos_obtener".to_string(), FirmaFuncion {
            nombre: "argumentos_obtener".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("i".to_string(), Tipo::Entero32)],
            retorno: Some(Tipo::Texto),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("consola_imprimir".to_string(), FirmaFuncion {
            nombre: "consola_imprimir".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("texto".to_string(), Tipo::Texto)],
            retorno: Some(vacio.clone()),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("consola_imprimir_linea".to_string(), FirmaFuncion {
            nombre: "consola_imprimir_linea".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("texto".to_string(), Tipo::Texto)],
            retorno: Some(vacio.clone()),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("aleatorio_entero".to_string(), FirmaFuncion {
            nombre: "aleatorio_entero".to_string(),
            parametros_genericos: vec![],
            parametros: vec![],
            retorno: Some(Tipo::Entero64),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("aleatorio_entero_entre".to_string(), FirmaFuncion {
            nombre: "aleatorio_entero_entre".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("min".to_string(), Tipo::Entero64),
                ("max".to_string(), Tipo::Entero64),
            ],
            retorno: Some(Tipo::Entero64),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("timestamp".to_string(), FirmaFuncion {
            nombre: "timestamp".to_string(),
            parametros_genericos: vec![],
            parametros: vec![],
            retorno: Some(Tipo::Entero64),
            span: span_vacio.clone(),
            es_publica: true,
        });

        // ============================================================
        // BUILTINS NUEVOS (v0.8.0) — Matemáticas avanzadas
        // ============================================================
        self.funciones.insert("mate_abs".to_string(), FirmaFuncion {
            nombre: "mate_abs".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("x".to_string(), Tipo::Flotante64)],
            retorno: Some(Tipo::Flotante64),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("mate_maximo".to_string(), FirmaFuncion {
            nombre: "mate_maximo".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("a".to_string(), Tipo::Flotante64),
                ("b".to_string(), Tipo::Flotante64),
            ],
            retorno: Some(Tipo::Flotante64),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("mate_minimo".to_string(), FirmaFuncion {
            nombre: "mate_minimo".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("a".to_string(), Tipo::Flotante64),
                ("b".to_string(), Tipo::Flotante64),
            ],
            retorno: Some(Tipo::Flotante64),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("mate_piso".to_string(), FirmaFuncion {
            nombre: "mate_piso".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("x".to_string(), Tipo::Flotante64)],
            retorno: Some(Tipo::Entero64),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("mate_techo".to_string(), FirmaFuncion {
            nombre: "mate_techo".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("x".to_string(), Tipo::Flotante64)],
            retorno: Some(Tipo::Entero64),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("mate_pi".to_string(), FirmaFuncion {
            nombre: "mate_pi".to_string(),
            parametros_genericos: vec![],
            parametros: vec![],
            retorno: Some(Tipo::Flotante64),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("mate_e".to_string(), FirmaFuncion {
            nombre: "mate_e".to_string(),
            parametros_genericos: vec![],
            parametros: vec![],
            retorno: Some(Tipo::Flotante64),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("mate_grados_a_radianes".to_string(), FirmaFuncion {
            nombre: "mate_grados_a_radianes".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("grados".to_string(), Tipo::Flotante64)],
            retorno: Some(Tipo::Flotante64),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("mate_radianes_a_grados".to_string(), FirmaFuncion {
            nombre: "mate_radianes_a_grados".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("radianes".to_string(), Tipo::Flotante64)],
            retorno: Some(Tipo::Flotante64),
            span: span_vacio.clone(),
            es_publica: true,
        });

        // ============================================================
        // BUILTINS NUEVOS (v0.8.0) — Visual (stubs)
        // ============================================================
        let tipos_visual = vec![
            ("ventana_nuevo", vec![("titulo".to_string(), Tipo::Texto), ("ancho".to_string(), Tipo::Entero32), ("alto".to_string(), Tipo::Entero32)], Tipo::Entero64),
            ("ventana_mostrar", vec![("ventana".to_string(), Tipo::Entero64)], Tipo::Vacio),
            ("ventana_cerrar", vec![("ventana".to_string(), Tipo::Entero64)], Tipo::Vacio),
            ("lienzo_nuevo", vec![("ancho".to_string(), Tipo::Entero32), ("alto".to_string(), Tipo::Entero32)], Tipo::Entero64),
            ("lienzo_limpiar", vec![("lienzo".to_string(), Tipo::Entero64), ("color".to_string(), Tipo::Entero32)], Tipo::Vacio),
            ("lienzo_linea", vec![("lienzo".to_string(), Tipo::Entero64), ("x1".to_string(), Tipo::Entero32), ("y1".to_string(), Tipo::Entero32), ("x2".to_string(), Tipo::Entero32), ("y2".to_string(), Tipo::Entero32)], Tipo::Vacio),
            ("lienzo_rectangulo", vec![("lienzo".to_string(), Tipo::Entero64), ("x".to_string(), Tipo::Entero32), ("y".to_string(), Tipo::Entero32), ("w".to_string(), Tipo::Entero32), ("h".to_string(), Tipo::Entero32)], Tipo::Vacio),
            ("lienzo_circulo", vec![("lienzo".to_string(), Tipo::Entero64), ("cx".to_string(), Tipo::Entero32), ("cy".to_string(), Tipo::Entero32), ("radio".to_string(), Tipo::Entero32)], Tipo::Vacio),
            ("lienzo_texto", vec![("lienzo".to_string(), Tipo::Entero64), ("x".to_string(), Tipo::Entero32), ("y".to_string(), Tipo::Entero32), ("texto".to_string(), Tipo::Texto)], Tipo::Vacio),
            ("lienzo_guardar_png", vec![("lienzo".to_string(), Tipo::Entero64), ("ruta".to_string(), Tipo::Texto)], Tipo::Entero32),
            ("lienzo_liberar", vec![("lienzo".to_string(), Tipo::Entero64)], Tipo::Vacio),
            ("imagen_desde_archivo", vec![("ruta".to_string(), Tipo::Texto)], Tipo::Entero64),
            ("imagen_ancho", vec![("imagen".to_string(), Tipo::Entero64)], Tipo::Entero32),
            ("imagen_alto", vec![("imagen".to_string(), Tipo::Entero64)], Tipo::Entero32),
            ("imagen_liberar", vec![("imagen".to_string(), Tipo::Entero64)], Tipo::Vacio),
            ("audio_nuevo", vec![("canales".to_string(), Tipo::Entero32), ("frecuencia".to_string(), Tipo::Entero32)], Tipo::Entero64),
            ("audio_tono", vec![("frecuencia".to_string(), Tipo::Flotante64), ("duracion_ms".to_string(), Tipo::Entero32)], Tipo::Entero64),
            ("audio_reproducir", vec![("audio".to_string(), Tipo::Entero64)], Tipo::Entero32),
            ("audio_guardar_wav", vec![("audio".to_string(), Tipo::Entero64), ("ruta".to_string(), Tipo::Texto)], Tipo::Entero32),
            ("audio_liberar", vec![("audio".to_string(), Tipo::Entero64)], Tipo::Vacio),
        ];
        for (nombre, params, ret) in tipos_visual {
            self.funciones.insert(nombre.to_string(), FirmaFuncion {
                nombre: nombre.to_string(),
                parametros_genericos: vec![],
                parametros: params,
                retorno: Some(ret),
                span: span_vacio.clone(),
                es_publica: true,
            });
        }

        // ============================================================
        // BUILTINS NUEVOS (v0.8.0) — Proceso avanzado
        // ============================================================
        self.funciones.insert("proceso_listo_para_leer".to_string(), FirmaFuncion {
            nombre: "proceso_listo_para_leer".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("handle".to_string(), Tipo::Entero64),
                ("ms".to_string(), Tipo::Entero32),
            ],
            retorno: Some(Tipo::Booleano),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("proceso_cerrar_bidireccional".to_string(), FirmaFuncion {
            nombre: "proceso_cerrar_bidireccional".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("handle".to_string(), Tipo::Entero64)],
            retorno: Some(vacio.clone()),
            span: span_vacio.clone(),
            es_publica: true,
        });
    }

    /// Analiza un programa completo en dos pasadas (F-006).
    ///
    /// **Pasada 1 — Recolección de firmas**: registra TODAS las firmas
    /// (funciones, structs, enums, rasgos, apodos, imports, métodos de impl)
    /// ANTES de analizar cualquier cuerpo. Esto resuelve el bug F-006
    /// (forward references) y previene el bug silencioso de shadowing:
    /// una segunda declaración del mismo símbolo ahora genera un warning
    /// con span, en vez de sobrescribirse silenciosamente.
    ///
    /// **Pasada 2 — Análisis de cuerpos**: analiza los cuerpos de funciones,
    /// métodos de impl y bloques `prueba`. Con todas las firmas registradas,
    /// la inferencia de tipos de llamadas a funciones forward-referenceadas
    /// resuelve correctamente.
    ///
    /// Patrón estándar (Rust, Go, Zig hacen equivalente).
    pub fn analizar(&mut self, programa: &Programa) -> Result<(), Errores> {
        // Pasada 1: recolectar firmas (sin analizar cuerpos)
        for decl in &programa.declaraciones {
            self.recolectar_firmas_decl(decl);
        }

        // Si la pasada 1 ya detectó errores fatales (duplicados de nivel grave),
        // cortamos aquí — analizar cuerpos con tablas corruptas solo empeoraría.
        if self.errores.hay_errores() {
            return Err(self.errores.clone());
        }

        // Pasada 2: analizar cuerpos (funciones, métodos, pruebas)
        for decl in &programa.declaraciones {
            self.analizar_cuerpos_decl(decl);
        }

        if self.errores.hay_errores() {
            Err(self.errores.clone())
        } else {
            Ok(())
        }
    }

    /// **Pasada 1 del análisis (F-006)**: recolecta firmas sin tocar cuerpos.
    ///
    /// Para cada declaración, registra su firma/nombre en la tabla correspondiente.
    /// Detecta shadowing (declaración duplicada) y emite un **warning** con span
    /// — antes el shadowing se silenciaba y la última definición ganaba, lo que
    /// producía bugs difíciles de rastrear (ej. `json_reparador.fc` tiene
    /// `_jr_copiar` y `_jr_es_igual` duplicadas; antes ganaba la última en orden
    /// de archivo sin avisar).
    ///
    /// Patrón estándar de compilers: Rust, Go, Zig recolectan firmas antes
    /// de analizar cuerpos.
    fn recolectar_firmas_decl(&mut self, decl: &Declaracion) {
        match decl {
            Declaracion::Funcion(func) => {
                let es_top_level = self.modulo_actual.is_empty();
                let nombre_registro = self.nombre_con_modulo(&func.nombre);
                let es_publica = Self::es_funcion_publica(func, es_top_level);

                // Detección de shadowing (warning, no error)
                if self.funciones.contains_key(&nombre_registro) {
                    self.reportar_warning(
                        FUNCION_DUPLICADA,
                        &func.span,
                        format!("Función '{}' ya fue declarada; la nueva definición reemplaza a la anterior", func.nombre),
                        Some(format!("Renombra la función o elimina una de las dos declaraciones")),
                    );
                }

                let firma = FirmaFuncion {
                    nombre: nombre_registro.clone(),
                    parametros_genericos: func.parametros_genericos.clone(),
                    parametros: func.parametros.iter()
                        .map(|p| (p.nombre.clone(), p.tipo.clone()))
                        .collect(),
                    retorno: func.retorno.clone(),
                    span: func.span.clone(),
                    es_publica,
                };
                self.funciones.insert(nombre_registro, firma);
            }
            Declaracion::Estructural(s) => {
                let nombre_registro = self.nombre_con_modulo(&s.nombre);

                if self.structs.contains_key(&nombre_registro) {
                    self.reportar_warning(
                        STRUCT_DUPLICADO,
                        &s.span,
                        format!("Struct '{}' ya fue declarado", s.nombre),
                        Some("Renombra el struct o elimina la declaración duplicada".to_string()),
                    );
                }

                self.structs.insert(nombre_registro, InfoStruct {
                    nombre: s.nombre.clone(),
                    campos: s.campos.clone(),
                    campos_bits: s.campos_bits.clone(),
                    span: s.span.clone(),
                });
            }
            Declaracion::Enumeracion(e) => {
                let nombre_registro = self.nombre_con_modulo(&e.nombre);

                if self.enums.contains_key(&nombre_registro) {
                    self.reportar_warning(
                        ENUM_DUPLICADO,
                        &e.span,
                        format!("Enum '{}' ya fue declarado", e.nombre),
                        Some("Renombra el enum o elimina la declaración duplicada".to_string()),
                    );
                }

                self.enums.insert(nombre_registro, InfoEnum {
                    nombre: e.nombre.clone(),
                    parametros_genericos: e.parametros_genericos.clone(),
                    variantes: e.variantes.clone(),
                    span: e.span.clone(),
                });
            }
            Declaracion::Apodo(a) => {
                let nombre_registro = self.nombre_con_modulo(&a.nombre);

                if self.aliases.contains_key(&nombre_registro) {
                    self.reportar_warning(
                        APODO_DUPLICADO,
                        &a.span,
                        format!("Apodo '{}' ya fue declarado", a.nombre),
                        Some("Renombra el apodo o elimina la declaración duplicada".to_string()),
                    );
                }

                self.aliases.insert(nombre_registro, a.tipo.clone());
            }
            Declaracion::Modulo(modulo) => {
                // Recursión: entrar al módulo, recolectar firmas, salir
                self.modulo_actual.push(modulo.nombre.clone());
                for decl in &modulo.contenido {
                    self.recolectar_firmas_decl(decl);
                }
                self.modulo_actual.pop();
            }
            Declaracion::Usar(usar) => {
                let cualificado = usar.ruta.join("::");
                if let Some(atajo) = usar.ruta.last() {
                    if *atajo == "*" {
                        if usar.ruta.len() > 1 {
                            let prefijo = usar.ruta[..usar.ruta.len() - 1].join("::");
                            self.glob_imports.push(prefijo);
                        }
                    } else {
                        self.imports.insert(atajo.clone(), cualificado);
                    }
                }
            }
            Declaracion::Rasgo(rasgo) => {
                if self.rasgos.contains_key(&rasgo.nombre) {
                    self.reportar_warning(
                        RASGO_DUPLICADO,
                        &rasgo.span,
                        format!("Rasgo '{}' ya fue declarado", rasgo.nombre),
                        Some("Renombra el rasgo o elimina la declaración duplicada".to_string()),
                    );
                }

                self.rasgos.insert(rasgo.nombre.clone(), InfoRasgo {
                    nombre: rasgo.nombre.clone(),
                    metodos: rasgo.metodos.clone(),
                    span: rasgo.span.clone(),
                });
            }
            Declaracion::Implementacion(imp) => {
                // Registrar la impl (la asociación rasgo→tipo)
                let tipo_nombre = self.nombre_tipo_string(&imp.tipo);
                self.impls.insert(
                    (imp.rasgo.clone(), tipo_nombre.clone()),
                    imp.metodos.iter().map(|m| m.nombre.clone()).collect()
                );

                // Registrar cada método como función para que sea llamable
                // (análisis de tipos en el cuerpo viene en la pasada 2)
                let prefijo = format!("{}::{}", tipo_nombre, imp.rasgo);
                for metodo in &imp.metodos {
                    let nombre_registro = format!("{}::{}", prefijo, metodo.nombre);
                    if self.funciones.contains_key(&nombre_registro) {
                        self.reportar_warning(
                            FUNCION_DUPLICADA,
                            &metodo.span,
                            format!("Método '{}' del impl '{}' para '{}' ya fue declarado",
                                metodo.nombre, imp.rasgo, tipo_nombre),
                            Some("Renombra el método o elimina el impl duplicado".to_string()),
                        );
                    }

                    let firma = FirmaFuncion {
                        nombre: nombre_registro.clone(),
                        parametros_genericos: metodo.parametros_genericos.clone(),
                        parametros: metodo.parametros.iter()
                            .map(|p| (p.nombre.clone(), p.tipo.clone()))
                            .collect(),
                        retorno: metodo.retorno.clone(),
                        span: metodo.span.clone(),
                        es_publica: true,
                    };
                    self.funciones.insert(nombre_registro, firma);
                }
            }
            Declaracion::Prueba(_) => {
                // Las pruebas se analizan en pasada 2; nada que recolectar aquí
            }
        }
    }

    /// **Pasada 2 del análisis (F-006)**: analiza los cuerpos de funciones,
    /// métodos de impl y bloques `prueba`. Para entonces, todas las firmas
    /// ya están en las tablas, así que las llamadas forward-referenceadas
    /// (como `json_parsear` llamando a `_jr_validar_balance` declarada más
    /// abajo) infieren correctamente el tipo de retorno en vez de caer en
    /// el default `Entero32`.
    fn analizar_cuerpos_decl(&mut self, decl: &Declaracion) {
        match decl {
            Declaracion::Funcion(func) => {
                self.analizar_funcion(func);
            }
            Declaracion::Modulo(modulo) => {
                self.modulo_actual.push(modulo.nombre.clone());
                for decl in &modulo.contenido {
                    self.analizar_cuerpos_decl(decl);
                }
                self.modulo_actual.pop();
            }
            Declaracion::Implementacion(imp) => {
                // Validar consistencia rasgo↔impl
                if !self.rasgos.contains_key(&imp.rasgo) {
                    self.reportar_error(
                        CategoriaError::Tipo,
                        60,
                        &imp.span,
                        format!("El rasgo '{}' no existe", imp.rasgo),
                        Some(format!("Declara el rasgo con: rasgo {} {{ ... }}", imp.rasgo))
                    );
                }

                let metodos_requeridos: Vec<String> = if let Some(rasgo_info) = self.rasgos.get(&imp.rasgo) {
                    rasgo_info.metodos.iter().map(|m| m.nombre.clone()).collect()
                } else {
                    Vec::new()
                };

                for nombre_metodo in &metodos_requeridos {
                    if !imp.metodos.iter().any(|m| m.nombre == *nombre_metodo) {
                        self.reportar_error(
                            CategoriaError::Tipo,
                            61,
                            &imp.span,
                            format!("Impl incompleta: falta método '{}' del rasgo '{}'",
                                nombre_metodo, imp.rasgo),
                            Some(format!("Agrega: función {}(...) {{ ... }}", nombre_metodo))
                        );
                    }
                }

                // Analizar cada método
                for metodo in &imp.metodos {
                    self.analizar_funcion(metodo);
                }
            }
            Declaracion::Prueba(prueba) => {
                let entorno_anterior = std::mem::take(&mut self.entorno);
                self.entorno = Entorno::con_padre(entorno_anterior);
                self.analizar_bloque(&prueba.bloque);
                self.entorno = *self.entorno.padre.take().unwrap_or_else(|| Box::new(Entorno::nuevo()));
            }
            _ => {
                // Estructural, Enumeracion, Apodo, Usar, Rasgo: ya procesados en pasada 1
            }
        }
    }

    /// Determina si una función es pública según su artículo de visibilidad y contexto.
    pub(crate) fn es_funcion_publica(func: &FuncionDecl, es_top_level: bool) -> bool {
        match func.visibilidad {
            Some(Articulo::El) => true,
            Some(Articulo::La) => false,
            _ => es_top_level,
        }
    }

    /// Construye el nombre cualificado según el módulo actual.
    fn nombre_con_modulo(&self, nombre: &str) -> String {
        if self.modulo_actual.is_empty() {
            nombre.to_string()
        } else {
            format!("{}::{}", self.modulo_actual.join("::"), nombre)
        }
    }

    /// Busca una función por nombre, verificando visibilidad en referencias cruzadas.
    pub(crate) fn buscar_funcion(
        &mut self,
        nombre: &str,
        es_referencia_cruzada: bool,
        span: &Span,
    ) -> Option<FirmaFuncion> {
        if let Some(firma) = self.funciones.get(nombre) {
            if es_referencia_cruzada && !firma.es_publica {
                let error = ErrorCompilador::nuevo(
                    CategoriaError::Modulos,
                    VISIBILIDAD_PRIVADA,
                    span.clone(),
                    format!("Función '{}' es privada", nombre),
                ).con_sugerencia("Usa 'el función' para hacerla pública, o accede solo dentro del mismo módulo".to_string());
                self.errores.agregar(error);
            }
            return Some(firma.clone());
        }
        if let Some(firma) = self.simbolos_publicos_importados.get(nombre) {
            return Some(firma.clone());
        }
        None
    }

    /// Resuelve un nombre simple mediante imports glob.
    pub(crate) fn resolver_glob(&self, nombre: &str) -> Option<String> {
        for prefijo in &self.glob_imports {
            let cualificado = format!("{}::{}", prefijo, nombre);
            if let Some(firma) = self.funciones.get(&cualificado) {
                if firma.es_publica {
                    return Some(cualificado);
                }
            }
            if self.simbolos_publicos_importados.contains_key(&cualificado) {
                return Some(cualificado);
            }
        }
        None
    }

    /// Busca un struct: local, import específico, o glob.
    pub(crate) fn buscar_struct(&self, nombre: &str) -> Option<InfoStruct> {
        if let Some(info) = self.structs.get(nombre) {
            return Some(info.clone());
        }
        if let Some(cualificado) = self.imports.get(nombre) {
            if let Some(info) = self.structs_importados.get(cualificado) {
                return Some(info.clone());
            }
        }
        for prefijo in &self.glob_imports {
            let cualificado = format!("{}::{}", prefijo, nombre);
            if let Some(info) = self.structs_importados.get(&cualificado) {
                return Some(info.clone());
            }
            if let Some(info) = self.structs.get(&cualificado) {
                return Some(info.clone());
            }
        }
        None
    }

    /// Busca un enum: local, import específico, o glob.
    pub(crate) fn buscar_enum(&self, nombre: &str) -> Option<InfoEnum> {
        if let Some(info) = self.enums.get(nombre) {
            return Some(info.clone());
        }
        if let Some(cualificado) = self.imports.get(nombre) {
            if let Some(info) = self.enums_importados.get(cualificado) {
                return Some(info.clone());
            }
        }
        for prefijo in &self.glob_imports {
            let cualificado = format!("{}::{}", prefijo, nombre);
            if let Some(info) = self.enums_importados.get(&cualificado) {
                return Some(info.clone());
            }
            if let Some(info) = self.enums.get(&cualificado) {
                return Some(info.clone());
            }
        }
        None
    }
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::LexerFalcato;
    use crate::parser::ParserFalcato;

    #[test]
    fn test_semantica_correcta() {
        let fuente = r#"función principal() -> Entero32 {
    el a: Entero32 = 10;
    el b: Entero32 = 20;
    retornar a + b;
}"#;
        let lexer = LexerFalcato::nuevo(fuente, "test.fc");
        let tokens = lexer.tokenizar();
        let programa = ParserFalcato::parse(tokens).unwrap();
        
        let mut semantica = AnalizadorSemantico::nuevo();
        assert!(semantica.analizar(&programa).is_ok());
    }

    #[test]
    fn test_error_tipo_mismatch() {
        let fuente = r#"función principal() -> Entero32 {
    el a: Booleano = 10;
    retornar 0;
}"#;
        let lexer = LexerFalcato::nuevo(fuente, "test.fc");
        let tokens = lexer.tokenizar();
        let programa = ParserFalcato::parse(tokens).unwrap();
        
        let mut semantica = AnalizadorSemantico::nuevo();
        let resultado = semantica.analizar(&programa);
        assert!(resultado.is_err());
        
        let errores = resultado.unwrap_err();
        assert!(errores.errores.iter().any(|e| e.codigo == DISCONCORDANCIA_TIPO));
    }

    #[test]
    fn test_error_variable_no_declarada() {
        let fuente = r#"función principal() -> Entero32 {
    retornar x + 1;
}"#;
        let lexer = LexerFalcato::nuevo(fuente, "test.fc");
        let tokens = lexer.tokenizar();
        let programa = ParserFalcato::parse(tokens).unwrap();
        
        let mut semantica = AnalizadorSemantico::nuevo();
        let resultado = semantica.analizar(&programa);
        assert!(resultado.is_err());
    }

    #[test]
    fn test_error_retorno_incorrecto() {
        let fuente = r#"función principal() -> Booleano {
    retornar 42;
}"#;
        let lexer = LexerFalcato::nuevo(fuente, "test.fc");
        let tokens = lexer.tokenizar();
        let programa = ParserFalcato::parse(tokens).unwrap();
        
        let mut semantica = AnalizadorSemantico::nuevo();
        let resultado = semantica.analizar(&programa);
        assert!(resultado.is_err());
    }

    #[test]
    fn test_condicional_correcto() {
        let fuente = r#"función principal() -> Entero32 {
    el x: Entero32 = 10;
    si x > 5 {
        retornar 100;
    } sino {
        retornar 0;
    }
}"#;
        let lexer = LexerFalcato::nuevo(fuente, "test.fc");
        let tokens = lexer.tokenizar();
        let programa = ParserFalcato::parse(tokens).unwrap();
        
        let mut semantica = AnalizadorSemantico::nuevo();
        assert!(semantica.analizar(&programa).is_ok());
    }

    #[test]
    fn test_condicional_tipo_invalido() {
        // MEJORA-002: enteros ahora son válidos en `si` (truthy).
        // Para que el test siga detectando tipos inválidos, usamos Texto (no aceptable).
        let fuente = r#"función principal() -> Entero32 {
    el x: Texto = texto_nuevo();
    si x {
        retornar 100;
    }
    retornar 0;
}"#;
        let lexer = LexerFalcato::nuevo(fuente, "test.fc");
        let tokens = lexer.tokenizar();
        let programa = ParserFalcato::parse(tokens).unwrap();

        let mut semantica = AnalizadorSemantico::nuevo();
        let resultado = semantica.analizar(&programa);
        assert!(resultado.is_err());

        let errores = resultado.unwrap_err();
        assert!(errores.errores.iter().any(|e| e.codigo == CONDICIONAL_NO_BOOLEANO));
    }

    #[test]
    fn test_ownership_mutable_ok() {
        let fuente = r#"función principal() -> Entero32 {
    el x: Entero32 = 10;
    x = 20;
    retornar x;
}"#;
        let lexer = LexerFalcato::nuevo(fuente, "test.fc");
        let tokens = lexer.tokenizar();
        let programa = ParserFalcato::parse(tokens).unwrap();
        
        let mut semantica = AnalizadorSemantico::nuevo();
        assert!(semantica.analizar(&programa).is_ok());
    }

    #[test]
    fn test_ownership_inmutable_error() {
        let fuente = r#"función principal() -> Entero32 {
    la x: Entero32 = 10;
    x = 20;
    retornar x;
}"#;
        let lexer = LexerFalcato::nuevo(fuente, "test.fc");
        let tokens = lexer.tokenizar();
        let programa = ParserFalcato::parse(tokens).unwrap();
        
        let mut semantica = AnalizadorSemantico::nuevo();
        let resultado = semantica.analizar(&programa);
        assert!(resultado.is_err());
        
        let errores = resultado.unwrap_err();
        assert!(errores.errores.iter().any(|e| e.codigo == 1 && e.categoria == CategoriaError::Ownership));
    }

    #[test]
    fn test_bucle_mientras_correcto() {
        let fuente = r#"función principal() -> Entero32 {
    el i: Entero32 = 0;
    mientras i < 5 {
        i = i + 1;
    }
    retornar i;
}"#;
        let lexer = LexerFalcato::nuevo(fuente, "test.fc");
        let tokens = lexer.tokenizar();
        let programa = ParserFalcato::parse(tokens).unwrap();
        
        let mut semantica = AnalizadorSemantico::nuevo();
        assert!(semantica.analizar(&programa).is_ok());
    }

    #[test]
    fn test_enum_correcto() {
        let fuente = r#"enumeración Estado {
    Activo,
    Inactivo
}

función principal() -> Entero32 {
    el estado: Estado = Estado.Activo;
    si estado es Estado.Activo {
        retornar 1;
    }
    retornar 0;
}"#;
        let lexer = LexerFalcato::nuevo(fuente, "test.fc");
        let tokens = lexer.tokenizar();
        let programa = ParserFalcato::parse(tokens).unwrap();
        
        let mut semantica = AnalizadorSemantico::nuevo();
        assert!(semantica.analizar(&programa).is_ok());
    }

    #[test]
    fn test_enum_con_datos() {
        let fuente = r#"enumeración MiResultado {
    Exito(valor: Entero32),
    Error(codigo: Entero32)
}

función principal() -> Entero32 {
    el r: MiResultado = MiResultado.Exito(42);
    si r es MiResultado.Exito {
        retornar 1;
    }
    retornar 0;
}"#;
        let lexer = LexerFalcato::nuevo(fuente, "test.fc");
        let tokens = lexer.tokenizar();
        let programa = ParserFalcato::parse(tokens).unwrap();
        
        let mut semantica = AnalizadorSemantico::nuevo();
        assert!(semantica.analizar(&programa).is_ok());
    }

    #[test]
    fn test_enum_variante_inexistente() {
        let fuente = r#"enumeración Estado {
    Activo,
    Inactivo
}

función principal() -> Entero32 {
    el estado: Estado = Estado.Desconocido;
    retornar 0;
}"#;
        let lexer = LexerFalcato::nuevo(fuente, "test.fc");
        let tokens = lexer.tokenizar();
        let programa = ParserFalcato::parse(tokens).unwrap();
        
        let mut semantica = AnalizadorSemantico::nuevo();
        let resultado = semantica.analizar(&programa);
        assert!(resultado.is_err());
    }

    #[test]
    fn test_const_generico_correcto() {
        let fuente = r#"función longitud<N: Entero32>(los nums: [Entero32; N]) -> Entero32 {
    retornar N;
}

función principal() -> Entero32 {
    los nums: [Entero32; 5] = [1, 2, 3, 4, 5];
    retornar longitud(nums);
}"#;
        let lexer = LexerFalcato::nuevo(fuente, "test.fc");
        let tokens = lexer.tokenizar();
        let programa = ParserFalcato::parse(tokens).unwrap();
        
        let mut semantica = AnalizadorSemantico::nuevo();
        assert!(semantica.analizar(&programa).is_ok());
    }

    #[test]
    fn test_bound_comparable_correcto() {
        let fuente = r#"función máximo<T que Comparable>(el a: T, el b: T) -> T {
    si a > b {
        retornar a;
    } sino {
        retornar b;
    }
}

función principal() -> Entero32 {
    retornar máximo(5, 3);
}"#;
        let lexer = LexerFalcato::nuevo(fuente, "test.fc");
        let tokens = lexer.tokenizar();
        let programa = ParserFalcato::parse(tokens).unwrap();
        
        let mut semantica = AnalizadorSemantico::nuevo();
        let resultado = semantica.analizar(&programa);
        assert!(resultado.is_ok(), "Errores: {:?}", resultado.err());
    }

    #[test]
    fn test_modulo_publico_ok() {
        let fuente = r#"módulo matematicas {
    el función suma(el a: Entero32, el b: Entero32) -> Entero32 {
        retornar a + b;
    }
}

función principal() -> Entero32 {
    retornar matematicas::suma(1, 2);
}"#;
        let lexer = LexerFalcato::nuevo(fuente, "test.fc");
        let tokens = lexer.tokenizar();
        let programa = ParserFalcato::parse(tokens).unwrap();

        let mut semantica = AnalizadorSemantico::nuevo();
        let resultado = semantica.analizar(&programa);
        assert!(resultado.is_ok(), "Errores: {:?}", resultado.err());
    }

    #[test]
    fn test_modulo_privado_error() {
        let fuente = r#"módulo matematicas {
    función secreto(el n: Entero32) -> Entero32 {
        retornar n * 2;
    }
}

función principal() -> Entero32 {
    retornar matematicas::secreto(5);
}"#;
        let lexer = LexerFalcato::nuevo(fuente, "test.fc");
        let tokens = lexer.tokenizar();
        let programa = ParserFalcato::parse(tokens).unwrap();

        let mut semantica = AnalizadorSemantico::nuevo();
        let resultado = semantica.analizar(&programa);
        assert!(resultado.is_err());
        let errores = resultado.unwrap_err();
        assert!(errores.errores.iter().any(|e| e.codigo == VISIBILIDAD_PRIVADA));
    }

    #[test]
    fn test_usar_glob_ok() {
        let fuente = r#"módulo matematicas {
    el función suma(el a: Entero32, el b: Entero32) -> Entero32 {
        retornar a + b;
    }
}

usar matematicas::*;

función principal() -> Entero32 {
    retornar suma(1, 2);
}"#;
        let lexer = LexerFalcato::nuevo(fuente, "test.fc");
        let tokens = lexer.tokenizar();
        let programa = ParserFalcato::parse(tokens).unwrap();

        let mut semantica = AnalizadorSemantico::nuevo();
        let resultado = semantica.analizar(&programa);
        assert!(resultado.is_ok(), "Errores: {:?}", resultado.err());
    }

    #[test]
    fn test_import_cross_file_privado_error() {
        let mut simbolos_publicos: HashMap<String, FirmaFuncion> = HashMap::new();
        simbolos_publicos.insert("ops::doble".to_string(), FirmaFuncion {
            nombre: "ops::doble".to_string(),
            parametros_genericos: Vec::new(),
            parametros: vec![("x".to_string(), Tipo::Entero32)],
            retorno: Some(Tipo::Entero32),
            span: Span::vacio(),
            es_publica: true,
        });

        let fuente = r#"usar ops::secreto;

función principal() -> Entero32 {
    retornar secreto(1);
}"#;
        let lexer = LexerFalcato::nuevo(fuente, "principal.fc");
        let tokens = lexer.tokenizar();
        let programa = ParserFalcato::parse(tokens).unwrap();

        let mut semantica = AnalizadorSemantico::con_simbolos_publicos(simbolos_publicos);
        let resultado = semantica.analizar(&programa);
        assert!(resultado.is_err());
        let errores = resultado.unwrap_err();
        assert!(errores.errores.iter().any(|e| e.codigo == SIMBOLO_NO_ENCONTRADO));
    }

    #[test]
    fn test_import_cross_file_publico_ok() {
        let mut simbolos_publicos: HashMap<String, FirmaFuncion> = HashMap::new();
        simbolos_publicos.insert("ops::doble".to_string(), FirmaFuncion {
            nombre: "ops::doble".to_string(),
            parametros_genericos: Vec::new(),
            parametros: vec![("x".to_string(), Tipo::Entero32)],
            retorno: Some(Tipo::Entero32),
            span: Span::vacio(),
            es_publica: true,
        });

        let fuente = r#"usar ops::doble;

función principal() -> Entero32 {
    retornar doble(21);
}"#;
        let lexer = LexerFalcato::nuevo(fuente, "principal.fc");
        let tokens = lexer.tokenizar();
        let programa = ParserFalcato::parse(tokens).unwrap();

        let mut semantica = AnalizadorSemantico::con_simbolos_publicos(simbolos_publicos);
        let resultado = semantica.analizar(&programa);
        assert!(resultado.is_ok(), "Errores: {:?}", resultado.err());
    }

    #[test]
    fn test_texto_builtin_ok() {
        let fuente = r#"función principal() -> Entero32 {
    el t: Texto = texto_desde("Hola");
    texto_agregar(t, ", mundo");
    el len: Entero32 = texto_longitud(t);
    texto_liberar(t);
    retornar len;
}"#;
        let lexer = LexerFalcato::nuevo(fuente, "test.fc");
        let tokens = lexer.tokenizar();
        let programa = ParserFalcato::parse(tokens).unwrap();

        let mut semantica = AnalizadorSemantico::nuevo();
        let resultado = semantica.analizar(&programa);
        assert!(resultado.is_ok(), "Errores: {:?}", resultado.err());
    }

    #[test]
    fn test_texto_tipo_erroneo() {
        let fuente = r#"función principal() -> Entero32 {
    el t: Texto = texto_desde(42);
    retornar 0;
}"#;
        let lexer = LexerFalcato::nuevo(fuente, "test.fc");
        let tokens = lexer.tokenizar();
        let programa = ParserFalcato::parse(tokens).unwrap();

        let mut semantica = AnalizadorSemantico::nuevo();
        let resultado = semantica.analizar(&programa);
        assert!(resultado.is_err());
    }

    #[test]
    fn test_vector_generico_ok() {
        let fuente = r#"función principal() -> Entero32 {
    el v: Vector<Entero32> = vector_nuevo<Entero32>();
    vector_agregar<Entero32>(v, 10);
    vector_agregar<Entero32>(v, 20);
    el x: Entero32 = vector_obtener<Entero32>(v, 1);
    vector_liberar<Entero32>(v);
    retornar x;
}"#;
        let lexer = LexerFalcato::nuevo(fuente, "test.fc");
        let tokens = lexer.tokenizar();
        let programa = ParserFalcato::parse(tokens).unwrap();

        let mut semantica = AnalizadorSemantico::nuevo();
        let resultado = semantica.analizar(&programa);
        assert!(resultado.is_ok(), "Errores: {:?}", resultado.err());
    }

    #[test]
    fn test_vector_tipo_erroneo() {
        let fuente = r#"función principal() -> Entero32 {
    el v: Vector<Entero32> = vector_nuevo<Entero32>();
    vector_agregar<Entero32>(v, verdadero);
    retornar 0;
}"#;
        let lexer = LexerFalcato::nuevo(fuente, "test.fc");
        let tokens = lexer.tokenizar();
        let programa = ParserFalcato::parse(tokens).unwrap();

        let mut semantica = AnalizadorSemantico::nuevo();
        let resultado = semantica.analizar(&programa);
        assert!(resultado.is_err());
    }
}
