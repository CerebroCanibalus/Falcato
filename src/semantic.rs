use crate::ast::*;
use crate::error::{CategoriaError, ErrorCompilador, Errores};
use crate::span::Span;
use std::collections::HashMap;

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

/// Información semántica de una variable

/// Tabla de métodos: (nombre_tipo, nombre_método) → nombre_builtin
/// Permite sintaxis t.metodo(args) → se desugarea a llamada built-in
fn metodo_a_builtin(tipo: &Tipo, metodo: &str) -> Option<&'static str> {
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
fn distancia_levenshtein(a: &str, b: &str) -> usize {
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
fn sugerir_nombre(escrito: &str, disponibles: &[String]) -> Option<String> {
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
    errores: Errores,
    entorno: Entorno,
    funcion_actual: Option<FuncionDecl>,
    structs: HashMap<String, InfoStruct>,
    enums: HashMap<String, InfoEnum>,
    funciones: HashMap<String, FirmaFuncion>,
    /// Imports: nombre_corto → nombre_cualificado (ej: "suma" → "matematicas::suma")
    imports: HashMap<String, String>,
    /// Imports glob: lista de prefijos de módulo (ej: "matematicas")
    glob_imports: Vec<String>,
    /// Stack de módulos actual para registro de nombres cualificados
    modulo_actual: Vec<String>,
    /// Símbolos públicos de otros módulos (nombre cualificado → firma)
    simbolos_publicos_importados: HashMap<String, FirmaFuncion>,
    /// Structs públicos de otros módulos (nombre cualificado → info) — para `usar modulo::*`
    structs_importados: HashMap<String, InfoStruct>,
    /// Enums públicos de otros módulos (nombre cualificado → info) — para `usar modulo::*`
    enums_importados: HashMap<String, InfoEnum>,
    /// Variables movidas en la función actual (para use-after-move detection)
    variables_movidas: std::collections::HashSet<String>,
    /// Nivel de verificación de ownership de la función actual
    nivel_verificacion_actual: crate::ast::NivelVerificacion,
    /// Estado de borrow de cada variable (para borrowing rules)
    borrows: HashMap<String, BorrowState>,
    /// Profundidad de bucles activos — para validar romper/continuar
    profundidad_bucle: u32,
    /// Efecto de la función actual (para verificación de anotaciones)
    efecto_actual: crate::ast::Efecto,
    /// Rasgos (traits) registrados: nombre → InfoRasgo
    rasgos: HashMap<String, InfoRasgo>,
    /// Impls registrados: (rasgo, tipo) → métodos
    impls: HashMap<(String, String), Vec<String>>,
    /// Alias de tipos: nombre → Tipo (ej: "Entero" → Entero32)
    aliases: HashMap<String, Tipo>,
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
    /// Usado por el resolver multi-archivo para compartir APIs entre módulos.
    pub fn con_simbolos_publicos(simbolos: HashMap<String, FirmaFuncion>) -> Self {
        Self::con_simbolos_publicos_completo(simbolos, HashMap::new(), HashMap::new())
    }

    /// Crea analizador con símbolos públicos completos de otros módulos:
    /// funciones, structs y enums (nombres cualificados).
    /// Usado por el resolver multi-archivo para type-checking cross-file real.
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
            retorno: Some(Tipo::Entero64), // socket handle
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("tcp_aceptar".to_string(), FirmaFuncion {
            nombre: "tcp_aceptar".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("listener".to_string(), Tipo::Entero64)],
            retorno: Some(Tipo::Entero64), // client socket handle
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("tcp_leer".to_string(), FirmaFuncion {
            nombre: "tcp_leer".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("socket".to_string(), Tipo::Entero64),
                ("buffer".to_string(), Tipo::Entero64), // puntero a buffer
                ("tam".to_string(), Tipo::Entero32),
            ],
            retorno: Some(Tipo::Entero32), // bytes leídos (-1 = error, 0 = cerrado)
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("tcp_escribir".to_string(), FirmaFuncion {
            nombre: "tcp_escribir".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("socket".to_string(), Tipo::Entero64),
                ("buffer".to_string(), Tipo::Entero64), // puntero a buffer
                ("tam".to_string(), Tipo::Entero32),
            ],
            retorno: Some(Tipo::Entero32), // bytes escritos
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

        // Procesos (R7.1): lanzar comandos y capturar salida
        self.funciones.insert("proceso_crear".to_string(), FirmaFuncion {
            nombre: "proceso_crear".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("comando".to_string(), Tipo::Palabra)],
            retorno: Some(Tipo::Entero64), // handle (0 = error)
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("proceso_esperar".to_string(), FirmaFuncion {
            nombre: "proceso_esperar".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("handle".to_string(), Tipo::Entero64)],
            retorno: Some(Tipo::Entero32), // exit code
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("proceso_leer_salida_completa".to_string(), FirmaFuncion {
            nombre: "proceso_leer_salida_completa".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("handle".to_string(), Tipo::Entero64)],
            retorno: Some(Tipo::Texto), // salida capturada (bloquea hasta EOF)
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
            retorno: Some(Tipo::Entero64), // handle (0 = error)
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("proceso_escribir".to_string(), FirmaFuncion {
            nombre: "proceso_escribir".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("handle".to_string(), Tipo::Entero64),
                ("datos".to_string(), Tipo::Entero64), // puntero a datos
                ("n".to_string(), Tipo::Entero32),     // número de bytes
            ],
            retorno: Some(Tipo::Entero32), // bytes escritos (-1 = error)
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("proceso_leer_salida_chunk".to_string(), FirmaFuncion {
            nombre: "proceso_leer_salida_chunk".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("handle".to_string(), Tipo::Entero64),
                ("buf".to_string(), Tipo::Entero64), // puntero a buffer
                ("n".to_string(), Tipo::Entero32),   // capacidad del buffer
            ],
            retorno: Some(Tipo::Entero32), // bytes leídos (0 = EOF)
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("proceso_leer_error_chunk".to_string(), FirmaFuncion {
            nombre: "proceso_leer_error_chunk".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("handle".to_string(), Tipo::Entero64),
                ("buf".to_string(), Tipo::Entero64), // puntero a buffer
                ("n".to_string(), Tipo::Entero32),   // capacidad del buffer
            ],
            retorno: Some(Tipo::Entero32), // bytes leídos (0 = EOF)
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("proceso_cerrar_entrada".to_string(), FirmaFuncion {
            nombre: "proceso_cerrar_entrada".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("handle".to_string(), Tipo::Entero64)],
            retorno: Some(vacio.clone()), // cierra stdin (envía EOF)
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("proceso_listo_para_leer".to_string(), FirmaFuncion {
            nombre: "proceso_listo_para_leer".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("handle".to_string(), Tipo::Entero64),
                ("ms".to_string(), Tipo::Entero32), // timeout en milisegundos
            ],
            retorno: Some(Tipo::Booleano), // 1 = hay datos, 0 = no
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
                ("host".to_string(), Tipo::Palabra),
                ("puerto".to_string(), Tipo::Entero32),
            ],
            retorno: Some(Tipo::Entero64), // socket handle (0 = error)
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("dns_resolver".to_string(), FirmaFuncion {
            nombre: "dns_resolver".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("host".to_string(), Tipo::Palabra)],
            retorno: Some(Tipo::Texto), // IP resuelta
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
            retorno: Some(Tipo::Booleano), // 1 = hay datos, 0 = no
            span: span_vacio.clone(),
            es_publica: true,
        });

        // Texto dinámico (R7.8 FASE 2): operaciones eficientes sobre strings
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
            retorno: Some(Tipo::Entero64), // puntero interno
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

        // Conversión numérica (R7.8 FASE 3): número → texto
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

        // Terminal (R7.2): modo raw + lectura de teclas
        self.funciones.insert("terminal_modo_raw".to_string(), FirmaFuncion {
            nombre: "terminal_modo_raw".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("activo".to_string(), Tipo::Entero32)],
            retorno: Some(Tipo::Entero32), // 1 = OK, 0 = error
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("terminal_leer_tecla".to_string(), FirmaFuncion {
            nombre: "terminal_leer_tecla".to_string(),
            parametros_genericos: vec![],
            parametros: vec![],
            retorno: Some(Tipo::Entero32), // código de tecla (ver terminal.rs)
            span: span_vacio.clone(),
            es_publica: true,
        });

        // Entrada estándar (R7.3)
        self.funciones.insert("entrada_leer".to_string(), FirmaFuncion {
            nombre: "entrada_leer".to_string(),
            parametros_genericos: vec![],
            parametros: vec![],
            retorno: Some(Tipo::Texto), // TODO stdin hasta EOF
            span: span_vacio.clone(),
            es_publica: true,
        });

        // Argumentos de línea de comandos (R7.5)
        self.funciones.insert("argumentos".to_string(), FirmaFuncion {
            nombre: "argumentos".to_string(),
            parametros_genericos: vec![],
            parametros: vec![],
            retorno: Some(Tipo::Vector(Box::new(Tipo::Texto))), // argv crudo
            span: span_vacio.clone(),
            es_publica: true,
        });

        // Reloj de pared (R7.4)
        self.funciones.insert("fecha_unix".to_string(), FirmaFuncion {
            nombre: "fecha_unix".to_string(),
            parametros_genericos: vec![],
            parametros: vec![],
            retorno: Some(Tipo::Entero64), // segundos desde epoch
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("fecha_ms".to_string(), FirmaFuncion {
            nombre: "fecha_ms".to_string(),
            parametros_genericos: vec![],
            parametros: vec![],
            retorno: Some(Tipo::Entero64), // ms desde epoch
            span: span_vacio.clone(),
            es_publica: true,
        });

        // DHT (R8.2): índice P2P distribuido
        self.funciones.insert("dht_nuevo".to_string(), FirmaFuncion {
            nombre: "dht_nuevo".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("puerto".to_string(), Tipo::Entero32)],
            retorno: Some(Tipo::Entero64), // handle (0 = error)
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
            retorno: Some(Tipo::Entero32), // 1 = OK, 0 = error
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
            retorno: Some(Tipo::Entero64), // puntero al valor (NULL = no encontrado)
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
            retorno: Some(Tipo::Entero32), // 1 = OK, 0 = error
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

        // Canales (Fase 18C): comunicación entre tareas
        self.funciones.insert("canal_nuevo".to_string(), FirmaFuncion {
            nombre: "canal_nuevo".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("capacidad".to_string(), Tipo::Entero32)],
            retorno: Some(Tipo::Entero64), // puntero al canal
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
            retorno: Some(Tipo::Entero32), // valor recibido
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
            retorno: Some(Tipo::Entero32), // valor o -2147483648 si vacío
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

        // Texto: cadena heap-allocada con longitud/capacidad
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
        // Alias: texto_tam = texto_longitud
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
        // Fase 15C: String ops adicionales
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
        // R7.5 Fase 2: conversión de Texto a número
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
        // Fase GUI-1: como_entero64(e: Entero32) -> Entero64
        // Convierte un Entero32 a Entero64 con signo. Útil para FFI donde se esperan punteros NULL.
        self.funciones.insert("como_entero64".to_string(), FirmaFuncion {
            nombre: "como_entero64".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("valor".to_string(), Tipo::Entero32)],
            retorno: Some(Tipo::Entero64),
            span: span_vacio.clone(),
            es_publica: true,
        });
        // R7.5 Fase 2: como_entero32(e: Entero64) -> Entero32
        // Trunca Entero64 a Entero32 (para parseo tipado de args).
        self.funciones.insert("como_entero32".to_string(), FirmaFuncion {
            nombre: "como_entero32".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("valor".to_string(), Tipo::Entero32)],
            retorno: Some(Tipo::Entero32),
            span: span_vacio.clone(),
            es_publica: true,
        });
        // R9.0.3 — Conversión numérica completa (para WAV 16/24-bit y DSP).
        // como_entero8/16/32/64(x: número) — entero→entero (extend/trunc) o flotante→entero (trunca).
        // como_flotante32/64(x: número) — entero→flotante o flotante→flotante (promueve/degrada).
        // La verificación de argumentos usa es_conversion_numerica (acepta cualquier tipo numérico).
        for (nombre, ret) in [
            ("como_entero8", Tipo::Entero8),
            ("como_entero16", Tipo::Entero16),
            ("como_entero32", Tipo::Entero32),
            ("como_entero64", Tipo::Entero64),
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
        // Fase GUI-1: texto_a_puntero — obtiene la dirección de un literal de cadena
        self.funciones.insert("texto_a_puntero".to_string(), FirmaFuncion {
            nombre: "texto_a_puntero".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("texto".to_string(), Tipo::Palabra)],
            retorno: Some(Tipo::Entero64),
            span: span_vacio.clone(),
            es_publica: true,
        });
        // Fase 15D: File I/O
        self.funciones.insert("archivo_leer".to_string(), FirmaFuncion {
            nombre: "archivo_leer".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("ruta".to_string(), Tipo::Palabra)],
            retorno: Some(Tipo::Texto),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("archivo_escribir".to_string(), FirmaFuncion {
            nombre: "archivo_escribir".to_string(),
            parametros_genericos: vec![],
            parametros: vec![
                ("ruta".to_string(), Tipo::Palabra),
                ("contenido".to_string(), Tipo::Texto),
            ],
            retorno: Some(Tipo::Entero32),
            span: span_vacio.clone(),
            es_publica: true,
        });
        self.funciones.insert("archivo_existe".to_string(), FirmaFuncion {
            nombre: "archivo_existe".to_string(),
            parametros_genericos: vec![],
            parametros: vec![("ruta".to_string(), Tipo::Palabra)],
            retorno: Some(Tipo::Booleano),
            span: span_vacio.clone(),
            es_publica: true,
        });
        // Fase 15E: Matemáticas
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

        // Trigonometría — F1: libm (preciso)
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

        // Trigonometría precisa (F1) — libm
        self.funciones.insert("seno_preciso".to_string(), trig_unario("seno_preciso"));
        self.funciones.insert("coseno_preciso".to_string(), trig_unario("coseno_preciso"));
        self.funciones.insert("tangente_preciso".to_string(), trig_unario("tangente_preciso"));
        self.funciones.insert("exp_preciso".to_string(), trig_unario("exp_preciso"));
        self.funciones.insert("log_preciso".to_string(), trig_unario("log_preciso"));

        // Trigonometría rápida (F2/F3) — math.rs
        self.funciones.insert("seno_rapido".to_string(), trig_unario("seno_rapido"));
        self.funciones.insert("coseno_rapido".to_string(), trig_unario("coseno_rapido"));
        self.funciones.insert("seno_2pi".to_string(), trig_unario("seno_2pi"));
        self.funciones.insert("coseno_2pi".to_string(), trig_unario("coseno_2pi"));
        self.funciones.insert("exp_rapido".to_string(), trig_unario("exp_rapido"));
        self.funciones.insert("log_rapido".to_string(), trig_unario("log_rapido"));
        self.funciones.insert("seno_aprox".to_string(), trig_unario("seno_aprox"));

        // Vector<T>: arreglo dinámico heap-allocado
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
        // Alias: vector_tam = vector_longitud
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

        // Diccionario<K, V>: hash map
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
            retorno: Some(Tipo::Entero64), // devuelve el puntero al diccionario para chaining
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
            retorno: Some(tipo_v.clone()), // WARNING: si no existe, devuelve basura — usar existe() primero
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
            retorno: Some(Tipo::Booleano), // true si se eliminó
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

        // Alias de tipo para Conjunto = Diccionario<T, Booleano>
        let s_generico = ParametroGenerico {
            nombre: "T".to_string(),
            tipo: None,
            bounds: vec![],
            span: span_vacio.clone(),
        };
        let tipo_s = Tipo::Generico("T".to_string());
        let _bool_tipo = Tipo::Booleano;

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

        // Resultado<T, E>: enum genérico para manejo de errores
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

        // Option<T>: enum genérico para valores opcionales (R7.7 F2 — `un x = a + b`)
        // Algo(valor) | Nada — el artículo `un` (incierto) codifica checked con None
        let t_generico_opt = ParametroGenerico {
            nombre: "T".to_string(),
            tipo: None,
            bounds: vec![],
            span: span_vacio.clone(),
        };
        let tipo_t_opt = Tipo::Generico("T".to_string());

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
    }

    /// Determina si una función es pública según su artículo de visibilidad y contexto.
    /// - `el función` → pública
    /// - `la función` → privada
    /// - Sin artículo en top-level → pública (API del archivo-módulo)
    /// - Sin artículo dentro de módulo → privada (encapsulación por defecto)
    pub(crate) fn es_funcion_publica(func: &FuncionDecl, es_top_level: bool) -> bool {
        match func.visibilidad {
            Some(Articulo::El) => true,
            Some(Articulo::La) => false,
            // Otros artículos (un, los, las) no son visibilidad válida; default según contexto
            _ => es_top_level,
        }
    }

    /// Construye el nombre cualificado según el módulo actual.
    /// Top-level: nombre sin prefijo.
    /// Dentro de módulo: modulo::nombre.
    fn nombre_con_modulo(&self, nombre: &str) -> String {
        if self.modulo_actual.is_empty() {
            nombre.to_string()
        } else {
            format!("{}::{}", self.modulo_actual.join("::"), nombre)
        }
    }

    /// Busca una función por nombre, verificando visibilidad en referencias cruzadas.
    /// - `es_referencia_cruzada`: true si el acceso es vía ruta cualificada o import.
    ///   En ese caso, las funciones locales privadas no son accesibles.
    fn buscar_funcion(
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
    /// Busca `prefijo::nombre` en funciones locales públicas y símbolos importados.
    fn resolver_glob(&self, nombre: &str) -> Option<String> {
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

    /// Busca un struct: local, import específico (`usar modulo::Struct`), o glob (`usar modulo::*`).
    fn buscar_struct(&self, nombre: &str) -> Option<InfoStruct> {
        // 1. Local
        if let Some(info) = self.structs.get(nombre) {
            return Some(info.clone());
        }
        // 2. Import específico: atajo "Struct" → "modulo::Struct"
        if let Some(cualificado) = self.imports.get(nombre) {
            if let Some(info) = self.structs_importados.get(cualificado) {
                return Some(info.clone());
            }
        }
        // 3. Glob import: prefijo::Struct
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

    /// Busca un enum: local, import específico (`usar modulo::Enum`), o glob (`usar modulo::*`).
    fn buscar_enum(&self, nombre: &str) -> Option<InfoEnum> {
        // 1. Local
        if let Some(info) = self.enums.get(nombre) {
            return Some(info.clone());
        }
        // 2. Import específico: atajo "Enum" → "modulo::Enum"
        if let Some(cualificado) = self.imports.get(nombre) {
            if let Some(info) = self.enums_importados.get(cualificado) {
                return Some(info.clone());
            }
        }
        // 3. Glob import: prefijo::Enum
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

    pub fn analizar(&mut self, programa: &Programa) -> Result<(), Errores> {
        for decl in &programa.declaraciones {
            self.analizar_declaracion(decl);
        }

        if self.errores.hay_errores() {
            Err(self.errores.clone())
        } else {
            Ok(())
        }
    }

    fn analizar_declaracion(&mut self, decl: &Declaracion) {
        match decl {
            Declaracion::Funcion(func) => {
                let es_top_level = self.modulo_actual.is_empty();
                let nombre_registro = self.nombre_con_modulo(&func.nombre);
                let es_publica = Self::es_funcion_publica(func, es_top_level);

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
                self.analizar_funcion(func);
            }
            Declaracion::Estructural(s) => {
                let nombre_registro = self.nombre_con_modulo(&s.nombre);
                self.structs.insert(nombre_registro, InfoStruct {
                    nombre: s.nombre.clone(),
                    campos: s.campos.clone(),
                    campos_bits: s.campos_bits.clone(),
                    span: s.span.clone(),
                });
            }
            Declaracion::Enumeracion(e) => {
                let nombre_registro = self.nombre_con_modulo(&e.nombre);
                self.enums.insert(nombre_registro, InfoEnum {
                    nombre: e.nombre.clone(),
                    parametros_genericos: e.parametros_genericos.clone(),
                    variantes: e.variantes.clone(),
                    span: e.span.clone(),
                });
            }
            Declaracion::Apodo(a) => {
                let nombre_registro = self.nombre_con_modulo(&a.nombre);
                self.aliases.insert(nombre_registro, a.tipo.clone());
            }
            Declaracion::Modulo(modulo) => {
                self.modulo_actual.push(modulo.nombre.clone());
                for decl in &modulo.contenido {
                    self.analizar_declaracion(decl);
                }
                self.modulo_actual.pop();
            }
            Declaracion::Usar(usar) => {
                // Import: usar modulo::funcion → crear atajo en imports
                let cualificado = usar.ruta.join("::");
                if let Some(atajo) = usar.ruta.last() {
                    if *atajo == "*" {
                        // Glob import: usar modulo::* → guardar prefijo para resolución lazy
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
                // Registrar rasgo: nombre → firmas de métodos
                self.rasgos.insert(rasgo.nombre.clone(), InfoRasgo {
                    nombre: rasgo.nombre.clone(),
                    metodos: rasgo.metodos.clone(),
                    span: rasgo.span.clone(),
                });
            }
            Declaracion::Implementacion(imp) => {
                // Registrar implementación: (rasgo, tipo) → métodos
                let tipo_nombre = self.nombre_tipo_string(&imp.tipo);
                self.impls.insert(
                    (imp.rasgo.clone(), tipo_nombre.clone()),
                    imp.metodos.iter().map(|m| m.nombre.clone()).collect()
                );
                
                // Verificar que el rasgo existe
                if !self.rasgos.contains_key(&imp.rasgo) {
                    self.reportar_error(
                        CategoriaError::Tipo,
                        60,
                        &imp.span,
                        format!("El rasgo '{}' no existe", imp.rasgo),
                        Some(format!("Declara el rasgo con: rasgo {} {{ ... }}", imp.rasgo))
                    );
                }
                
                // Verificar que todos los métodos del rasgo están implementados
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
                
                // Analizar cada método como función normal
                for metodo in &imp.metodos {
                    self.analizar_funcion(metodo);
                }
            }
            Declaracion::Prueba(prueba) => {
                // Analizar el bloque de la prueba como un scope nuevo
                let entorno_anterior = std::mem::take(&mut self.entorno);
                self.entorno = Entorno::con_padre(entorno_anterior);
                self.analizar_bloque(&prueba.bloque);
                self.entorno = *self.entorno.padre.take().unwrap_or_else(|| Box::new(Entorno::nuevo()));
            }
        }
    }

    /// Extrae el path de acceso de una expresión (ej: "punto.x" desde AccesoCampo)
    fn extraer_path(&self, expr: &Expresion) -> Option<String> {
        match expr {
            Expresion::Identificador(nombre, _) => Some(nombre.clone()),
            Expresion::AccesoCampo(base, campo, _) => {
                if let Some(base_path) = self.extraer_path(base) {
                    Some(format!("{}.{}", base_path, campo))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn analizar_funcion(&mut self, func: &FuncionDecl) {
        self.funcion_actual = Some(func.clone());
        
        // Establecer nivel de verificación de ownership y limpiar estado anterior
        self.nivel_verificacion_actual = func.nivel_verificacion;
        self.variables_movidas.clear();
        self.borrows.clear();
        self.efecto_actual = func.efecto.clone();
        
        // Nuevo entorno para la función
        let entorno_anterior = std::mem::take(&mut self.entorno);
        self.entorno = Entorno::con_padre(entorno_anterior);

        // Registrar parámetros genéricos
        for gen in &func.parametros_genericos {
            if let Some(ref tipo) = gen.tipo {
                // Const param: N: Entero32
                self.entorno.declarar_const(gen.nombre.clone(), tipo.clone());
            } else {
                // Type param: T
                self.entorno.declarar_tipo(gen.nombre.clone(), Tipo::Generico(gen.nombre.clone()));
            }
        }

        // Registrar parámetros
        for param in &func.parametros {
            self.entorno.declarar(InfoVariable {
                nombre: param.nombre.clone(),
                tipo: param.tipo.clone(),
                articulo: param.articulo,
                span: param.span.clone(),
            });
        }

        // Analizar cuerpo
        self.analizar_bloque(&func.cuerpo);

        // Restaurar entorno
        self.entorno = *self.entorno.padre.take().unwrap_or_else(|| Box::new(Entorno::nuevo()));
        self.funcion_actual = None;
    }

    fn analizar_bloque(&mut self, bloque: &Bloque) {
        for sentencia in &bloque.sentencias {
            self.analizar_sentencia(sentencia);
        }
    }

    fn analizar_sentencia(&mut self, sentencia: &Sentencia) {
        match sentencia {
            Sentencia::Expresion(expr) => {
                let _ = self.inferir_tipo(expr);
            }
            Sentencia::DeclaracionVariable(decl) => {
                // Manejo especial para ArrayRelleno: si hay tipo explícito Array, 
                // verificamos solo compatibilidad de tipo de elemento
                let tipo_valor = match (&decl.tipo, &decl.valor) {
                    (Some(Tipo::Array(tipo_esperado, n)), Expresion::ArrayRelleno(elem, _, _)) => {
                        let tipo_elem = self.inferir_tipo(elem);
                        if tipo_elem != **tipo_esperado {
                            self.reportar_error(
                                CategoriaError::Tipo,
                                DISCONCORDANCIA_TIPO,
                                &decl.span,
                                format!("Disconcordancia de tipo en 'todos': elemento es '{:?}' pero arreglo espera '{:?}'",
                                    tipo_elem, tipo_esperado),
                                Some(format!("Cambia el tipo a '{:?}' o el valor de relleno", tipo_elem))
                            );
                        }
                        Tipo::Array(tipo_esperado.clone(), *n)
                    }
                    (Some(Tipo::ArrayGenerico(tipo_esperado, _)), Expresion::ArrayRelleno(elem, _, _)) => {
                        let tipo_elem = self.inferir_tipo(elem);
                        if tipo_elem != **tipo_esperado {
                            self.reportar_error(
                                CategoriaError::Tipo,
                                DISCONCORDANCIA_TIPO,
                                &decl.span,
                                format!("Disconcordancia de tipo en 'todos': elemento es '{:?}' pero arreglo espera '{:?}'",
                                    tipo_elem, tipo_esperado),
                                Some(format!("Cambia el tipo a '{:?}' o el valor de relleno", tipo_elem))
                            );
                        }
                        Tipo::ArrayGenerico(tipo_esperado.clone(), String::new())
                    }
                    _ => self.inferir_tipo(&decl.valor)
                };
                
                // R7.7 F2 — artículo incierto: `un x = a + b` → Option<T> (checked con None)
                // El artículo `un` (Pilar I: opcional) + aritmética entera → Algo(valor) | Nada
                // si la operación desborda. Coherente: `un` = incierto = puede no haber valor.
                let mut tipo_valor = tipo_valor;
                if decl.articulo == Articulo::Un {
                    if let Expresion::Binaria(_, op, _, _) = &decl.valor {
                        let es_aritmetica = matches!(
                            op,
                            OperadorBinario::Suma
                                | OperadorBinario::Resta
                                | OperadorBinario::Multiplicacion
                        );
                        if es_aritmetica {
                            let es_entero_32 = matches!(
                                self.resolver_alias(&tipo_valor),
                                Tipo::Entero8 | Tipo::Entero16 | Tipo::Entero32
                                    | Tipo::Natural8 | Tipo::Natural16 | Tipo::Natural32
                            );
                            if es_entero_32 {
                                tipo_valor = Tipo::Option(Box::new(tipo_valor));
                            } else {
                                self.reportar_error(
                                    CategoriaError::Tipo,
                                    109, // T109: un requiere enteros 32-bit
                                    &decl.span,
                                    format!(
                                        "'un' (opcional checked) requiere enteros de 32 bits o menos, pero la operación es '{:?}'",
                                        tipo_valor
                                    ),
                                    Some("F2: soporta Entero8/16/32 y Natural8/16/32; flotantes no desbordan (inf)".to_string())
                                );
                            }
                        }
                    }
                }
                
                // Verificar concordancia de tipo explícito
                if let Some(ref tipo_declarado) = decl.tipo {
                    // Resolver apodos (ej: apodo ID = Entero64  Entero64)
                    let tipo_declarado_resuelto = self.resolver_alias(tipo_declarado);
                    // Adaptar literal numérico al tipo declarado (ej: el x: Entero64 = 5)
                    // Los literales enteros se infieren como Entero32; si el tipo declarado
                    // es numérico, el literal se re-tipifica para concordar.
                    let es_literal_entero = |e: &Expresion| match e {
                        Expresion::Literal(Literal::Entero(_, _)) => true,
                        Expresion::Unaria(_, inner, _) => matches!(inner.as_ref(), Expresion::Literal(Literal::Entero(_, _))),
                        _ => false,
                    };
                    let es_literal_flotante = |e: &Expresion| matches!(e, Expresion::Literal(Literal::Flotante(_, _)));
                    let tipo_valor_adaptado = match &decl.valor {
                        e if es_literal_entero(e) && self.es_entero(&tipo_declarado_resuelto) => {
                            tipo_declarado_resuelto.clone()
                        }
                        e if es_literal_entero(e) && self.es_flotante(&tipo_declarado_resuelto) => {
                            tipo_declarado_resuelto.clone()
                        }
                        e if es_literal_flotante(e) && self.es_flotante(&tipo_declarado_resuelto) => {
                            tipo_declarado_resuelto.clone()
                        }
                        _ => tipo_valor.clone(),
                    };
                    let tipo_valor = &tipo_valor_adaptado;
                    if !self.tipos_compatibles(tipo_declarado, tipo_valor) {
                        self.reportar_error(
                            CategoriaError::Tipo,
                            DISCONCORDANCIA_TIPO,
                            &decl.span,
                            format!("Disconcordancia de tipo: '{}' es '{:?}' pero se declaró como '{:?}'",
                                decl.nombre, tipo_valor, tipo_declarado),
                            Some(format!("Cambia el tipo a '{:?}' o el valor", tipo_valor))
                        );
                    }
                }

                // Resolver alias de tipo al declarar (ej: alias Entero = Entero32)
                let tipo_final = match &decl.tipo {
                    Some(t) => self.resolver_alias(t),
                    None => tipo_valor.clone(),
                };

                self.entorno.declarar(InfoVariable {
                    nombre: decl.nombre.clone(),
                    tipo: tipo_final,
                    articulo: decl.articulo,
                    span: decl.span.clone(),
                });
            }
            Sentencia::Asignacion(asig) => {
                // Verificación de efecto 'puro': no muta nada fuera de su scope
                if self.efecto_actual == crate::ast::Efecto::Puro {
                    if let Lugar::Identificador(nombre) = &asig.lugar {
                        // Verificar si es un parámetro (no local)
                        if let Some(func) = &self.funcion_actual {
                            if func.parametros.iter().any(|p| p.nombre == *nombre) {
                                self.reportar_error(
                                    CategoriaError::Ownership,
                                    50,
                                    &asig.span,
                                    format!("Función 'puro' no puede mutar parámetro '{}'", nombre),
                                    Some("Una función pura no muta estado externo. Usa una variable local.".to_string())
                                );
                            }
                        }
                    }
                }
                
                match &asig.lugar {
                    Lugar::Identificador(nombre) => {
                        let tipo_valor = self.inferir_tipo(&asig.valor);
                        let info_opt = self.entorno.buscar(nombre).cloned();
                        
                        match info_opt {
                            Some(info) => {
                                if !self.tipos_compatibles(&info.tipo, &tipo_valor) {
                                    self.reportar_error(
                                        CategoriaError::Tipo,
                                        ASIGNACION_INCOMPATIBLE,
                                        &asig.span,
                                        format!("Disconcordancia en asignación: '{}' es '{:?}' pero se asigna '{:?}'",
                                            nombre, info.tipo, tipo_valor),
                                        None
                                    );
                                }
                                if !self.es_mutable(info.articulo) {
                                    self.reportar_error(
                                        CategoriaError::Ownership,
                                        1,
                                        &asig.span,
                                        format!("Disconcordancia de estado: '{}' se declaró con '{}' (inmutable/prestada). \
No puedes modificar algo que no es 'tuyo'.", 
                                            nombre, self.articulo_a_str(info.articulo)),
                                        Some(format!("Usa 'el {}' para hacerlo mutable (owned)", nombre))
                                    );
                                }
                            }
                            None => {
                                self.reportar_error(
                                    CategoriaError::Tipo,
                                    VARIABLE_NO_DECLARADA,
                                    &asig.span,
                                    format!("'{}' no tiene concordancia en este contexto. ¿Olvidaste declararlo con artículo?",
                                        nombre),
                                    Some("Los identificadores deben declararse con artículo: el, la, un, los, las".to_string())
                                );
                            }
                        }
                    }
                    Lugar::Array(array_expr, indice_expr) => {
                        let tipo_array = self.inferir_tipo(array_expr);
                        let tipo_indice = self.inferir_tipo(indice_expr);
                        let tipo_valor = self.inferir_tipo(&asig.valor);
                        
                        if tipo_indice != Tipo::Entero32 && tipo_indice != Tipo::Entero64 {
                            self.reportar_error(
                                CategoriaError::Tipo,
                                15,
                                &asig.span,
                                "Índice de arreglo debe ser Entero".to_string(),
                                None
                            );
                        }
                        
                        match &tipo_array {
                            Tipo::Array(tipo_elem, _) | Tipo::ArrayGenerico(tipo_elem, _) => {
                                if !self.tipos_compatibles(tipo_elem, &tipo_valor) {
                                    self.reportar_error(
                                        CategoriaError::Tipo,
                                        ASIGNACION_INCOMPATIBLE,
                                        &asig.span,
                                        format!("Disconcordancia: arreglo almacena '{:?}' pero se asigna '{:?}'",
                                            tipo_elem, tipo_valor),
                                        None
                                    );
                                }
                            }
                            _ => {
                                self.reportar_error(
                                    CategoriaError::Tipo,
                                    16,
                                    &asig.span,
                                    format!("Asignación a arreglo en tipo '{:?}' que no es arreglo", tipo_array),
                                    None
                                );
                            }
                        }
                    }
                    // Fase 15B: asignación a campo de struct (bitfield o normal)
                    Lugar::Campo(base_expr, _nombre_campo) => {
                        // Verificar que el valor sea entero (para bitfields)
                        let _tipo_valor = self.inferir_tipo(&asig.valor);
                        let _tipo_base = self.inferir_tipo(base_expr);
                        // TODO: verificar que el campo existe en el struct
                    }
                }
            }
            Sentencia::Retornar(expr, span) => {
                let func = self.funcion_actual.clone();
                if let Some(func) = func {
                    if let Some(ref tipo_retorno) = func.retorno {
                        if let Some(expr) = expr {
                            let tipo_expr = self.inferir_tipo(expr);
                            if !self.tipos_compatibles(tipo_retorno, &tipo_expr) {
                                self.reportar_error(
                                    CategoriaError::Tipo,
                                    DISCONCORDANCIA_RETORNO,
                                    span,
                                    format!("Disconcordancia en retorno: función '{}' devuelve '{:?}' pero se retorna '{:?}'",
                                        func.nombre, tipo_retorno, tipo_expr),
                                    None
                                );
                            }
                        } else {
                            self.reportar_error(
                                CategoriaError::Tipo,
                                RETORNO_FALTANTE,
                                span,
                                format!("Función '{}' debe retornar '{:?}'", func.nombre, tipo_retorno),
                                None
                            );
                        }
                    }
                }
            }
            Sentencia::Condicional(cond) => {
                let tipo_cond = self.inferir_tipo(&cond.condicion);
                let es_estativo_bare = matches!(&cond.condicion, Expresion::Identificador(_, _))
                    && cond.modo == ModoVerbal::Estativo;
                
                // Bare "está" (sin RHS): truthiness check, permitido en enteros, booleanos y punteros
                if es_estativo_bare {
                    let es_valido_para_estado = matches!(&tipo_cond,
                        Tipo::Entero8 | Tipo::Entero16 | Tipo::Entero32 | Tipo::Entero64 |
                        Tipo::Natural8 | Tipo::Natural16 | Tipo::Natural32 | Tipo::Natural64 |
                        Tipo::Booleano | Tipo::Caracter |
                        Tipo::Palabra | Tipo::Puntero(_) | Tipo::Generico(_)
                    );
                    if !es_valido_para_estado {
                        self.reportar_error(
                            CategoriaError::Tipo,
                            24,
                            &cond.span,
                            format!("'está' (bare) requiere tipo entero, Booleano o puntero, encontrado '{:?}'", tipo_cond),
                            Some("Usa una comparación explícita (==, !=) o cambia el tipo de la variable".to_string())
                        );
                    }
                } else if tipo_cond != Tipo::Booleano {
                    self.reportar_error(
                        CategoriaError::Tipo,
                        CONDICIONAL_NO_BOOLEANO,
                        &cond.span,
                        format!("La condición 'si' requiere un valor Booleano, encontrado '{:?}'", tipo_cond),
                        Some("Usa una comparación (==, !=, <, >) o una variable Booleano".to_string())
                    );
                }
                
                // Branch-aware borrowing: guardar estado de borrows antes de cada rama
                // Los borrows creados dentro de una rama mueren al final de esa rama
                let borrows_antes = self.borrows.clone();
                
                self.analizar_bloque(&cond.bloque_entonces);
                
                // Restaurar borrows (los de la rama 'entonces' no escapan)
                self.borrows = borrows_antes.clone();
                
                if let Some(ref bloque_sino) = cond.bloque_sino {
                    self.analizar_bloque(bloque_sino);
                }
                
                // Restaurar borrows al estado original (los de ambas ramas no escapan)
                self.borrows = borrows_antes;
            }
            Sentencia::BucleMientras(bucle) => {
                let tipo_cond = self.inferir_tipo(&bucle.condicion);
                if tipo_cond != Tipo::Booleano {
                    self.reportar_error(
                        CategoriaError::Tipo,
                        BUCLE_NO_BOOLEANO,
                        &bucle.span,
                        format!("La condición 'mientras' requiere un valor Booleano, encontrado '{:?}'", tipo_cond),
                        Some("Usa una comparación (==, !=, <, >) o una variable Booleano".to_string())
                    );
                }
                
                // Branch-aware: borrows dentro del bucle se resetean cada iteración
                let borrows_antes = self.borrows.clone();
                self.profundidad_bucle += 1;
                self.analizar_bloque(&bucle.bloque);
                self.profundidad_bucle -= 1;
                self.borrows = borrows_antes;
            }
            Sentencia::BuclePara(bucle) => {
                // Determinar tipo del elemento según el iterable
                let tipo_elem = match &bucle.iterable {
                    Expresion::Rango(inicio, fin, _, _) => {
                        // Rango: verificar que ambos extremos sean enteros
                        let tipo_inicio = self.inferir_tipo(inicio);
                        let tipo_fin = self.inferir_tipo(fin);
                        if !self.es_entero(&tipo_inicio) || !self.es_entero(&tipo_fin) {
                            self.reportar_error(
                                CategoriaError::Tipo,
                                23,
                                &bucle.span,
                                format!("Rango requiere extremos enteros, encontrado '{:?}' y '{:?}'", tipo_inicio, tipo_fin),
                                Some("Usa enteros: para i en 0..10 {{ ... }}".to_string())
                            );
                        }
                        tipo_inicio
                    }
                    _ => {
                        let tipo_iterable = self.inferir_tipo(&bucle.iterable);
                        match &tipo_iterable {
                            Tipo::Array(t, _) | Tipo::ArrayGenerico(t, _) => *t.clone(),
                            _ => {
                                self.reportar_error(
                                    CategoriaError::Tipo,
                                    23,
                                    &bucle.span,
                                    format!("'para' requiere un arreglo o rango, encontrado '{:?}'", tipo_iterable),
                                    Some("Usa un arreglo [T; N] o un rango: 0..10".to_string())
                                );
                                Tipo::Entero32
                            }
                        }
                    }
                };
                
                // Nuevo entorno con la variable de iteración
                let entorno_anterior = std::mem::take(&mut self.entorno);
                self.entorno = Entorno::con_padre(entorno_anterior);
                
                // Declarar variable de iteración (mutable por defecto)
                self.entorno.declarar(InfoVariable {
                    nombre: bucle.variable.clone(),
                    tipo: tipo_elem,
                    articulo: Articulo::El,
                    span: bucle.span.clone(),
                });
                
                // Branch-aware: borrows dentro del bucle se resetean cada iteración
                let borrows_antes = self.borrows.clone();
                self.profundidad_bucle += 1;
                self.analizar_bloque(&bucle.bloque);
                self.profundidad_bucle -= 1;
                self.borrows = borrows_antes;
                
                // Restaurar entorno
                self.entorno = *self.entorno.padre.take().unwrap_or_else(|| Box::new(Entorno::nuevo()));
            }
            Sentencia::Romper(span) => {
                if self.profundidad_bucle == 0 {
                    self.reportar_error(
                        CategoriaError::Tipo,
                        107, // T107: romper fuera de bucle
                        span,
                        "'romper' solo puede usarse dentro de un bucle (para/mientras)".to_string(),
                        Some("Coloca 'romper' dentro del cuerpo de un bucle para salir temprano".to_string())
                    );
                }
            }
            Sentencia::Continuar(span) => {
                if self.profundidad_bucle == 0 {
                    self.reportar_error(
                        CategoriaError::Tipo,
                        108, // T108: continuar fuera de bucle
                        span,
                        "'continuar' solo puede usarse dentro de un bucle (para/mientras)".to_string(),
                        Some("Coloca 'continuar' dentro del cuerpo de un bucle para saltar a la siguiente iteración".to_string())
                    );
                }
            }
            Sentencia::Region { nombre: _, cuerpo, span: _ } => {
                // Región: nuevo entorno léxico (arena allocation scope)
                let entorno_anterior = std::mem::take(&mut self.entorno);
                self.entorno = Entorno::con_padre(entorno_anterior);
                
                for sentencia in cuerpo {
                    self.analizar_sentencia(sentencia);
                }
                
                // Restaurar entorno (variables de la región mueren aquí)
                self.entorno = *self.entorno.padre.take().unwrap_or_else(|| Box::new(Entorno::nuevo()));
            }
            Sentencia::Seleccionar(seleccionar) => {
                // seleccionar { canal como v => { ... }, _ => { ... } }
                for rama in &seleccionar.ramas {
                    // Verificar que la expresión del canal es válida
                    if rama.variable.is_some() {
                        let tipo_canal = self.inferir_tipo(&rama.canal);
                        if tipo_canal != Tipo::Entero64 && tipo_canal != Tipo::Vacio {
                            self.reportar_error(
                                CategoriaError::Tipo,
                                90,
                                &rama.span,
                                format!(
                                    "El canal en 'seleccionar' debe ser Entero64, pero se encontró '{:?}'",
                                    tipo_canal
                                ),
                                Some("Usa canal_nuevo() que retorna Entero64".to_string()),
                            );
                        }
                    }
                    // Nuevo scope para la variable de la rama
                    let entorno_anterior = std::mem::take(&mut self.entorno);
                    self.entorno = Entorno::con_padre(entorno_anterior);
                    
                    if let Some(ref var) = rama.variable {
                        self.entorno.declarar(InfoVariable {
                            nombre: var.clone(),
                            tipo: Tipo::Entero32,
                            articulo: crate::ast::Articulo::La,
                            span: rama.span.clone(),
                        });
                    }
                    
                    for sentencia in &rama.cuerpo.sentencias {
                        self.analizar_sentencia(sentencia);
                    }
                    
                    self.entorno = *self.entorno.padre.take().unwrap_or_else(|| Box::new(Entorno::nuevo()));
                }
            }
            Sentencia::ConExecutor { hilos, cuerpo, span } => {
                // con_executor(N) { ... } — validar que N es entero
                let tipo_hilos = self.inferir_tipo(hilos);
                match tipo_hilos {
                    Tipo::Entero32 | Tipo::Entero64 | Tipo::Natural32 | Tipo::Natural64 => {}
                    _ => {
                        self.reportar_error(
                            CategoriaError::Tipo,
                            91,
                            span,
                            format!(
                                "con_executor requiere un número entero de hilos, pero se encontró '{:?}'",
                                tipo_hilos
                            ),
                            Some("Usa un literal entero: con_executor(4) { ... }".to_string()),
                        );
                    }
                }
                // Analizar cuerpo en nuevo scope
                let entorno_anterior = std::mem::take(&mut self.entorno);
                self.entorno = Entorno::con_padre(entorno_anterior);
                for sentencia in cuerpo {
                    self.analizar_sentencia(sentencia);
                }
                self.entorno = *self.entorno.padre.take().unwrap_or_else(|| Box::new(Entorno::nuevo()));
            }
        }
    }

    /// Helper para reportar errores de forma consistente
    fn reportar_error(
        &mut self,
        categoria: CategoriaError,
        codigo: u32,
        span: &Span,
        mensaje: String,
        sugerencia: Option<String>,
    ) {
        let mut error = ErrorCompilador::nuevo(categoria, codigo, span.clone(), mensaje);
        if let Some(sug) = sugerencia {
            error = error.con_sugerencia(sug);
        }
        self.errores.agregar(error);
    }

    /// Sustituye parámetros genéricos de tipo por tipos concretos en una firma.
    /// Usado para llamadas built-in genéricas como vector_nuevo<Entero32>().
    fn aplicar_tipo_args_a_firma(
        &mut self,
        firma: &FirmaFuncion,
        tipo_args: &Vec<Tipo>,
        span: &Span,
    ) -> Option<FirmaFuncion> {
        if firma.parametros_genericos.is_empty() {
            return Some(firma.clone());
        }

        if tipo_args.len() != firma.parametros_genericos.len() {
            self.reportar_error(
                CategoriaError::Tipo,
                70,
                span,
                format!("Función '{}' espera {} argumentos de tipo, pero se pasaron {}",
                    firma.nombre, firma.parametros_genericos.len(), tipo_args.len()),
                Some("Proporciona los tipos genéricos requeridos, e.g., vector_nuevo<Entero32>()".to_string()),
            );
            return None;
        }

        let mut sustituciones: HashMap<String, Tipo> = HashMap::new();
        for (gen, tipo) in firma.parametros_genericos.iter().zip(tipo_args.iter()) {
            sustituciones.insert(gen.nombre.clone(), tipo.clone());
        }

        let parametros = firma.parametros.iter()
            .map(|(n, t)| (n.clone(), self.sustituir_genericos(t, &sustituciones)))
            .collect();
        let retorno = firma.retorno.as_ref()
            .map(|t| self.sustituir_genericos(t, &sustituciones));

        Some(FirmaFuncion {
            nombre: firma.nombre.clone(),
            parametros_genericos: vec![], // ya instanciados
            parametros,
            retorno,
            span: firma.span.clone(),
            es_publica: firma.es_publica,
        })
    }

    /// Reemplaza Tipo::Generico(n) por el tipo concreto asociado.
    fn sustituir_genericos(
        &self,
        tipo: &Tipo,
        sustituciones: &HashMap<String, Tipo>,
    ) -> Tipo {
        match tipo {
            Tipo::Generico(nombre) => {
                sustituciones.get(nombre).cloned().unwrap_or(tipo.clone())
            }
            Tipo::Vector(t) => Tipo::Vector(Box::new(self.sustituir_genericos(t, sustituciones))),
            Tipo::Diccionario(k, v) => Tipo::Diccionario(
                Box::new(self.sustituir_genericos(k, sustituciones)),
                Box::new(self.sustituir_genericos(v, sustituciones)),
            ),
            Tipo::Conjunto(t) => Tipo::Conjunto(Box::new(self.sustituir_genericos(t, sustituciones))),
            Tipo::Resultado(t, e) => Tipo::Resultado(
                Box::new(self.sustituir_genericos(t, sustituciones)),
                Box::new(self.sustituir_genericos(e, sustituciones)),
            ),
            Tipo::Puntero(t) => Tipo::Puntero(Box::new(self.sustituir_genericos(t, sustituciones))),
            Tipo::Referencia(t) => Tipo::Referencia(Box::new(self.sustituir_genericos(t, sustituciones))),
            Tipo::ReferenciaMut(t) => Tipo::ReferenciaMut(Box::new(self.sustituir_genericos(t, sustituciones))),
            Tipo::ReferenciaConLifetime(n, t) => Tipo::ReferenciaConLifetime(n.clone(), Box::new(self.sustituir_genericos(t, sustituciones))),
            Tipo::ReferenciaMutConLifetime(n, t) => Tipo::ReferenciaMutConLifetime(n.clone(), Box::new(self.sustituir_genericos(t, sustituciones))),
            Tipo::ReferenciaSelf(t) => Tipo::ReferenciaSelf(Box::new(self.sustituir_genericos(t, sustituciones))),
            Tipo::ReferenciaMutSelf(t) => Tipo::ReferenciaMutSelf(Box::new(self.sustituir_genericos(t, sustituciones))),
            Tipo::Array(t, n) => Tipo::Array(Box::new(self.sustituir_genericos(t, sustituciones)), *n),
            Tipo::ArrayGenerico(t, n) => Tipo::ArrayGenerico(Box::new(self.sustituir_genericos(t, sustituciones)), n.clone()),
            Tipo::NombreGenerico(n, args) => {
                let nuevos = args.iter()
                    .map(|a| self.sustituir_genericos(a, sustituciones))
                    .collect();
                Tipo::NombreGenerico(n.clone(), nuevos)
            }
            _ => tipo.clone(),
        }
    }

    /// Inferir tipo de expresión con verificación de concordancia
    fn inferir_tipo(&mut self, expr: &Expresion) -> Tipo {
        match expr {
            Expresion::Literal(lit) => self.tipo_literal(lit),
            Expresion::Ruta(path, span) => {
                // Ruta cualificada: modulo::simbolo (siempre referencia cruzada)
                let nombre_cualificado = path.join("::");
                if let Some(firma) = self.buscar_funcion(&nombre_cualificado, true, span) {
                    firma.retorno.clone().unwrap_or(Tipo::Entero32)
                } else if let Some(_ts) = self.structs.get(&nombre_cualificado).or_else(|| self.structs_importados.get(&nombre_cualificado)) {
                    Tipo::Nombre(nombre_cualificado)
                } else if let Some(_te) = self.enums.get(&nombre_cualificado).or_else(|| self.enums_importados.get(&nombre_cualificado)) {
                    Tipo::Nombre(nombre_cualificado)
                } else {
                    let sugerencia = sugerir_nombre(&path[0], &self.entorno.todos_nombres());
                    let msg = match sugerencia {
                        Some(ref s) => format!("'{}' no tiene concordancia en este contexto. ¿Quizás quisiste decir '{}'?", path[0], s),
                        None => format!("'{}' no tiene concordancia en este contexto", path[0]),
                    };
                    self.reportar_error(
                        CategoriaError::Tipo,
                        VARIABLE_NO_DECLARADA,
                        span,
                        msg,
                        Some(format!("¿Olvidaste declarar '{}' como módulo?", path[0]))
                    );
                    Tipo::Entero32
                }
            }
            Expresion::Propagacion(expr, span) => {
                // Verificar que expr es Resultado<T, E> u Option<T> y retornar T
                let tipo_expr = self.inferir_tipo(expr);
                match tipo_expr {
                    Tipo::Resultado(tipo_exito, _) => *tipo_exito,
                    Tipo::Option(tipo_exito) => *tipo_exito,
                    _ => {
                        self.reportar_error(
                            CategoriaError::Tipo,
                            29,
                            span,
                            format!("El operador '?' requiere Resultado<T, E> u Option<T>, pero se encontr� '{:?}'", tipo_expr),
                            Some("Usa '?' solo en expresiones de tipo Resultado u Option".to_string())
                        );
                        Tipo::Entero32
                    }
                }
            }
            Expresion::Identificador(nombre, span) => {
                // Verificar use-after-move (solo en Nivel 1+)
                if self.nivel_verificacion_actual != crate::ast::NivelVerificacion::Permisivo 
                    && self.variables_movidas.contains(nombre) {
                    self.reportar_error(
                        CategoriaError::Ownership,
                        1,
                        span,
                        format!("'{}' fue movido y ya no es válido", nombre),
                        Some(format!(
                            "Si necesitas usar '{}' después:\n       │   opción A: copiar {} antes de pasar\n       │   opción B: pasar por referencia (&{})\n       │   opción C: reordenar para usar {} antes del move",
                            nombre, nombre, nombre, nombre
                        ))
                    );
                }
                
                match self.entorno.buscar(nombre) {
                    Some(info) => info.tipo.clone(),
                    None => {
                        // Buscar como const genérico
                        if let Some((tipo, _)) = self.entorno.buscar_const(nombre) {
                            tipo.clone()
                        } else {
                            let sugerencia = sugerir_nombre(nombre, &self.entorno.todos_nombres());
                            let msg = match sugerencia {
                                Some(ref s) => format!("'{}' no tiene concordancia en este contexto. ¿Quizás quisiste decir '{}'?", nombre, s),
                                None => format!("'{}' no tiene concordancia en este contexto. ¿Olvidaste declararlo con artículo?", nombre),
                            };
                            self.reportar_error(
                                CategoriaError::Tipo,
                                VARIABLE_NO_DECLARADA,
                                span,
                                msg,
                                Some("Los identificadores deben declararse con artículo: el, la, un, los, las".to_string())
                            );
                            Tipo::Entero32 // Tipo por defecto para continuar análisis
                        }
                    }
                }
            }
            Expresion::Binaria(izq, op, der, span) => {
                let tipo_izq = self.inferir_tipo(izq);
                let tipo_der = self.inferir_tipo(der);

                // Verificar concordancia de tipos en operación binaria.
                // Los literales numéricos se adaptan al tipo del otro operando
                // (ej: id: Entero64 + 1 → el 1 se trata como Entero64).
                let (tipo_izq, tipo_der) = self.adaptar_literales_binaria(izq, &tipo_izq, der, &tipo_der);
                if tipo_izq != tipo_der {
                    self.reportar_error(
                        CategoriaError::Tipo,
                        DISCONCORDANCIA_OPERANDOS,
                        span,
                        format!("Disconcordancia de tipo en operación '{:?}': izquierda '{:?}', derecha '{:?}'",
                            op, tipo_izq, tipo_der),
                        Some("Ambos operandos deben ser del mismo tipo".to_string())
                    );
                }

                // Verificar división por cero en constantes
                if matches!(op, OperadorBinario::Division | OperadorBinario::Modulo) {
                    if let Expresion::Literal(Literal::Entero(valor, _)) = der.as_ref() {
                        if *valor == 0 {
                            self.reportar_error(
                                CategoriaError::Tipo,
                                99, // T099: división por cero
                                span,
                                format!("División por cero en operación '{:?}'", op),
                                Some("El divisor no puede ser cero. Usa un valor distinto de cero.".to_string())
                            );
                        }
                    }
                }

                self.tipo_operacion(*op, &tipo_izq, span)
            }
            // R7.7 F1 — Subjuntivo aritmético: `a + b fuese` → Resultado<T, Entero32>
            // Solo aritmética entera que pueda desbordar. "si la suma se desbordara..."
            Expresion::Checked(inner, span) => {
                match inner.as_ref() {
                    Expresion::Binaria(izq, op, der, span_bin) => {
                        // Verificar que es aritmética entera (suma/resta/multiplicación)
                        let es_aritmetica = matches!(
                            op,
                            OperadorBinario::Suma
                                | OperadorBinario::Resta
                                | OperadorBinario::Multiplicacion
                        );
                        if !es_aritmetica {
                            self.reportar_error(
                                CategoriaError::Tipo,
                                104, // T104: fuese solo en aritmética
                                span,
                                format!(
                                    "'fuese' (checked) solo aplica a suma, resta o multiplicación, no a '{:?}'",
                                    op
                                ),
                                Some("Usa 'fuese' para detectar desbordamiento: el x = a + b fuese".to_string())
                            );
                            return Tipo::Entero32;
                        }

                        let tipo_izq = self.inferir_tipo(izq);
                        let tipo_der = self.inferir_tipo(der);
                        let (tipo_izq, tipo_der) = self.adaptar_literales_binaria(izq, &tipo_izq, der, &tipo_der);
                        if tipo_izq != tipo_der {
                            self.reportar_error(
                                CategoriaError::Tipo,
                                DISCONCORDANCIA_OPERANDOS,
                                span_bin,
                                format!("Disconcordancia de tipo en operación '{:?}': izquierda '{:?}', derecha '{:?}'",
                                    op, tipo_izq, tipo_der),
                                Some("Ambos operandos deben ser del mismo tipo".to_string())
                            );
                        }

                        // Debe ser entero de ≤32 bits (los flotantes no desbordan en el sentido checked;
                        // Entero64/Natural64 → Resultado de 12 bytes, layout enum grande — pendiente F1.1)
                        let tipo_operacion = self.tipo_operacion(*op, &tipo_izq, span_bin);
                        let es_entero_32 = matches!(
                            tipo_operacion,
                            Tipo::Entero8 | Tipo::Entero16 | Tipo::Entero32
                                | Tipo::Natural8 | Tipo::Natural16 | Tipo::Natural32
                        );
                        if !es_entero_32 {
                            self.reportar_error(
                                CategoriaError::Tipo,
                                105, // T105: fuese requiere enteros 32-bit
                                span,
                                format!(
                                    "'fuese' (checked) requiere enteros de 32 bits o menos, pero la operación es '{:?}'",
                                    tipo_operacion
                                ),
                                Some("F1: soporta Entero8/16/32 y Natural8/16/32; Entero64/Natural64 pendiente (F1.1)".to_string())
                            );
                            return Tipo::Entero32;
                        }

                        // Resultado<T, Entero32>: Exito(valor) | Error(1 = desbordamiento)
                        Tipo::Resultado(Box::new(tipo_operacion), Box::new(Tipo::Entero32))
                    }
                    _ => {
                        self.reportar_error(
                            CategoriaError::Tipo,
                            106, // T106: fuese requiere operación binaria
                            span,
                            format!("'fuese' (checked) solo aplica a operaciones aritméticas, no a esta expresión"),
                            Some("Escribe la operación completa: el x = a + b fuese".to_string())
                        );
                        Tipo::Entero32
                    }
                }
            }
            Expresion::Unaria(op, expr, span) => {
                let tipo = self.inferir_tipo(expr);
                match op {
                    OperadorUnario::Referencia => {
                        // &expr: retorna Referencia(T)
                        // Verificar borrowing rules (solo en Nivel 2: estricto)
                        if self.nivel_verificacion_actual == crate::ast::NivelVerificacion::Estricto {
                            if let Some(path) = self.extraer_path(expr) {
                                let estado = self.borrows.get(&path).copied().unwrap_or(BorrowState::None);
                                match estado {
                                    BorrowState::Exclusive => {
                                        self.reportar_error(
                                            CategoriaError::Ownership,
                                            2,
                                            span,
                                            format!("No se puede crear referencia inmutable a '{}': ya tiene borrow mutable (&mut)", path),
                                            Some("Espera a que el borrow mutable termine antes de crear uno inmutable".to_string())
                                        );
                                    }
                                    BorrowState::Shared(n) => {
                                        self.borrows.insert(path.clone(), BorrowState::Shared(n + 1));
                                    }
                                    BorrowState::None => {
                                        self.borrows.insert(path.clone(), BorrowState::Shared(1));
                                    }
                                }
                            }
                        }
                        Tipo::Referencia(Box::new(tipo))
                    }
                    OperadorUnario::ReferenciaMut => {
                        // &mut expr: retorna ReferenciaMut(T)
                        // Verificar borrowing rules (solo en Nivel 2: estricto)
                        if self.nivel_verificacion_actual == crate::ast::NivelVerificacion::Estricto {
                            if let Some(path) = self.extraer_path(expr) {
                                let estado = self.borrows.get(&path).copied().unwrap_or(BorrowState::None);
                                match estado {
                                    BorrowState::Exclusive => {
                                        self.reportar_error(
                                            CategoriaError::Ownership,
                                            3,
                                            span,
                                            format!("No se puede crear referencia mutable a '{}': ya tiene borrow mutable (&mut)", path),
                                            Some(format!(
                                                "Solo puede existir un borrow mutable a la vez.\n       │   opción A: usa el borrow mutable existente\n       │   opción B: reordena para que los borrows no se solapen\n       │   opción C: usa 'copiar {}' para trabajar con una copia", path
                                            ))
                                        );
                                    }
                                    BorrowState::Shared(_) => {
                                        self.reportar_error(
                                            CategoriaError::Ownership,
                                            4,
                                            span,
                                            format!("No se puede crear referencia mutable a '{}': ya tiene borrows inmutables (&)", path),
                                            Some(format!(
                                                "Espera a que los borrows inmutables terminen.\n       │   opción A: reordena para que el borrow mutable vaya primero\n       │   opción B: usa un scope ({{ ... }}) para limitar el borrow inmutable\n       │   opción C: usa 'copiar {}' para mutar una copia", path
                                            ))
                                        );
                                    }
                                    BorrowState::None => {
                                        self.borrows.insert(path.clone(), BorrowState::Exclusive);
                                    }
                                }
                            }
                        }
                        Tipo::ReferenciaMut(Box::new(tipo))
                    }
                    OperadorUnario::Desreferencia => {
                        // *expr: extrae T desde Referencia(T), ReferenciaMut(T), ReferenciaConLifetime, ReferenciaMutConLifetime, ReferenciaSelf, o ReferenciaMutSelf
                        match tipo {
                            Tipo::Referencia(t) | Tipo::ReferenciaMut(t) |
                            Tipo::ReferenciaConLifetime(_, t) | Tipo::ReferenciaMutConLifetime(_, t) |
                            Tipo::ReferenciaSelf(t) | Tipo::ReferenciaMutSelf(t) => *t,
                            _ => {
                                self.reportar_error(
                                    CategoriaError::Tipo,
                                    30,
                                    span,
                                    format!("No se puede desreferenciar tipo '{:?}' (no es una referencia)", tipo),
                                    Some("Usa '*' solo en referencias (&T o &mut T)".to_string())
                                );
                                Tipo::Entero32
                            }
                        }
                    }
                    _ => self.tipo_operacion_unaria(*op, &tipo, span)
                }
            }
            Expresion::Llamada(llamada) => {
                // Resolver nombre de función considerando:
                // 1. Local simple (mismo módulo): no check de visibilidad
                // 2. Local con prefijo de módulo actual (funciones dentro de módulo inline)
                // 3. Import (usar modulo::funcion): referencia cruzada, requiere pública
                // 4. Ruta cualificada directa (modulo::funcion()): referencia cruzada, requiere pública
                let (nombre_resuelto, es_referencia_cruzada, viene_de_import) =
                    if llamada.funcion.contains("::") {
                        // Ya es ruta cualificada
                        (llamada.funcion.clone(), true, false)
                    } else if self.funciones.contains_key(&llamada.funcion) {
                        // Función local simple (top-level o del módulo actual)
                        (llamada.funcion.clone(), false, false)
                    } else {
                        // Intentar con prefijo de módulo actual
                        let nombre_con_modulo = self.nombre_con_modulo(&llamada.funcion);
                        if self.funciones.contains_key(&nombre_con_modulo) {
                            (nombre_con_modulo, false, false)
                        } else if let Some(cualificado) = self.imports.get(&llamada.funcion) {
                            // Import cruzado explícito
                            (cualificado.clone(), true, true)
                        } else if let Some(cualificado) = self.resolver_glob(&llamada.funcion) {
                            // Import cruzado glob
                            (cualificado, true, true)
                        } else {
                            // No se encontró; devolver nombre original para fallback FFI
                            (llamada.funcion.clone(), false, false)
                        }
                    };

                let firma_opt = self.buscar_funcion(&nombre_resuelto, es_referencia_cruzada, &llamada.span);
                match firma_opt {
                    Some(firma) => {
                        // Aplicar argumentos de tipo explícitos para built-ins genéricos
                        let firma_efectiva = if llamada.tipo_args.is_empty() {
                            firma.clone()
                        } else {
                            match self.aplicar_tipo_args_a_firma(&firma, &llamada.tipo_args, &llamada.span) {
                                Some(f) => f,
                                None => return Tipo::Entero32,
                            }
                        };

                        // Verificar cantidad de argumentos
                        if llamada.argumentos.len() != firma_efectiva.parametros.len() {
                            self.reportar_error(
                                CategoriaError::Tipo,
                                22, // cantidad de argumentos incorrecta
                                &llamada.span,
                                format!("Función '{}' espera {} argumentos, pero se pasaron {}",
                                    llamada.funcion, firma_efectiva.parametros.len(), llamada.argumentos.len()),
                                None
                            );
                        } else {
                            // Verificar tipos de cada argumento
                            // imprimir/imprimir_linea/decir son polimórficos (aceptan cualquier tipo)
                            // pero aún así debemos inferir tipos para detectar variables no declaradas
                            let es_polimorfica = llamada.funcion == "imprimir" || llamada.funcion == "imprimir_linea" || llamada.funcion == "decir";
                            // R9.0.3 — conversiones numéricas aceptan cualquier número (entero o flotante)
                            let es_conversion_numerica = matches!(llamada.funcion.as_str(),
                                "como_entero8" | "como_entero16" | "como_entero32" | "como_entero64" |
                                "como_flotante32" | "como_flotante64");
                            if !es_polimorfica {
                                for (i, (arg, (nombre_param, tipo_param))) in 
                                    llamada.argumentos.iter().zip(firma_efectiva.parametros.iter()).enumerate() {
                                    let tipo_arg = self.inferir_tipo(arg);
                                    // Conversión numérica: aceptar cualquier tipo numérico
                                    let compatible = if es_conversion_numerica {
                                        matches!(self.resolver_alias(&tipo_arg),
                                            Tipo::Entero8 | Tipo::Entero16 | Tipo::Entero32 | Tipo::Entero64 |
                                            Tipo::Natural8 | Tipo::Natural16 | Tipo::Natural32 | Tipo::Natural64 |
                                            Tipo::Flotante32 | Tipo::Flotante64)
                                    } else {
                                        self.tipos_compatibles(tipo_param, &tipo_arg)
                                    };
                                    if !compatible {
                                        self.reportar_error(
                                            CategoriaError::Tipo,
                                            DISCONCORDANCIA_TIPO,
                                            &llamada.span,
                                            format!("Argumento {} ('{}') de '{}': espera '{:?}', encontrado '{:?}'",
                                                i + 1, nombre_param, llamada.funcion, tipo_param, tipo_arg),
                                            Some(format!("Cambia el argumento a tipo '{:?}'", tipo_param))
                                        );
                                    }
                                }
                            } else {
                                // Aún para funciones polimórficas, inferir tipos de argumentos
                                // para detectar variables no declaradas
                                for arg in &llamada.argumentos {
                                    self.inferir_tipo(arg);
                                }
                            }
                        }
                        firma_efectiva.retorno.clone().unwrap_or(Tipo::Entero32)
                    }
                    None => {
                        if viene_de_import {
                            self.reportar_error(
                                CategoriaError::Modulos,
                                SIMBOLO_NO_ENCONTRADO,
                                &llamada.span,
                                format!("Función importada '{}' no encontrada o no es pública", llamada.funcion),
                                Some("Verifica que el módulo exporte la función con 'el función'".to_string())
                            );
                        }
                        // Podría ser función FFI, asumimos Entero32
                        Tipo::Entero32
                    }
                }
            }
            Expresion::ArrayRelleno(elem, _, _span) => {
                // Sin contexto, inferimos Array(tipo_elem, 0)
                // El tamaño real se resuelve en analizar_sentencia si hay tipo explícito
                let tipo_elem = self.inferir_tipo(elem);
                Tipo::Array(Box::new(tipo_elem), 0)
            }
            Expresion::AccesoArray(array, indice, span) => {
                let tipo_array = self.inferir_tipo(array);
                
                // Texto[i] → Entero8 (byte), Texto[inicio..fin] → Texto (subtexto)
                if tipo_array == Tipo::Texto {
                    let es_rango = matches!(indice.as_ref(), Expresion::Rango(_, _, _, _));
                    if es_rango {
                        // slicing: t[0..5] → Texto
                        return Tipo::Texto;
                    }
                    // t[i] → Entero8
                    return Tipo::Entero8;
                }
                
                // Vector<T>[i] → T
                if let Tipo::Vector(tipo_elem) = &tipo_array {
                    // Por ahora solo índices enteros
                    return *tipo_elem.clone();
                }
                
                let tipo_indice = self.inferir_tipo(indice);
                if tipo_indice != Tipo::Entero32 && tipo_indice != Tipo::Entero64 {
                    self.reportar_error(
                        CategoriaError::Tipo,
                        15,
                        span,
                            format!("Índice de arreglo debe ser Entero, encontrado '{:?}'", tipo_indice),
                        Some("Usa un valor Entero como índice".to_string())
                    );
                }
                
                match tipo_array {
                    Tipo::Array(tipo_elem, _) | Tipo::ArrayGenerico(tipo_elem, _) => *tipo_elem,
                    _ => {
                        self.reportar_error(
                            CategoriaError::Tipo,
                            16,
                            span,
                            format!("Acceso a arreglo en tipo '{:?}' que no es arreglo", tipo_array),
                            None
                        );
                        Tipo::Entero32
                    }
                }
            }
            Expresion::LiteralArray(elementos, span) => {
                if elementos.is_empty() {
                    Tipo::Array(Box::new(Tipo::Entero32), 0) // tipo por defecto
                } else {
                    // Inferir tipo del primer elemento
                    let tipo = self.inferir_tipo(&elementos[0]);
                    // Verificar que todos sean del mismo tipo
                    for (i, elem) in elementos.iter().enumerate().skip(1) {
                        let tipo_elem = self.inferir_tipo(elem);
                        if tipo_elem != tipo {
                            self.reportar_error(
                                CategoriaError::Tipo,
                                17, // nuevo código para arreglo heterogéneo
                                span,
                                format!("Elemento {} del arreglo es '{:?}' pero se espera '{:?}'", i, tipo_elem, tipo),
                                Some("Todos los elementos de un arreglo deben ser del mismo tipo".to_string())
                            );
                        }
                    }
                    Tipo::Array(Box::new(tipo), elementos.len())
                }
            }
            Expresion::InicializacionStruct(nombre, campos, span) => {
                let info_opt = self.buscar_struct(nombre);
                match info_opt {
                    Some(info) => {
                        // Fase 15B: struct de bitfields
                        if !info.campos_bits.is_empty() && info.campos.is_empty() {
                            for (nombre_campo, valor) in campos {
                                let _tipo_valor = self.inferir_tipo(valor);
                                if !info.campos_bits.iter().any(|c| c.nombre == *nombre_campo) {
                                    self.reportar_error(
                                        CategoriaError::Tipo,
                                        18,
                                        span,
                                        format!("El struct '{}' no tiene campo '{}'", nombre, nombre_campo),
                                        None
                                    );
                                }
                            }
                            return Tipo::Nombre(nombre.clone());
                        }

                        let mut campos_vistos = std::collections::HashSet::new();
                        for (nombre_campo, valor) in campos {
                            let tipo_valor = self.inferir_tipo(valor);
                            match info.campos.iter().find(|c| c.nombre == *nombre_campo) {
                                Some(campo) => {
                                    if !self.tipos_compatibles(&campo.tipo, &tipo_valor) {
                                        self.reportar_error(
                                            CategoriaError::Tipo,
                                            DISCONCORDANCIA_TIPO,
                                            span,
                                            format!("Campo '{}' de struct '{}' es '{:?}' pero se asigna '{:?}'",
                                                nombre_campo, nombre, campo.tipo, tipo_valor),
                                            Some(format!("Cambia el tipo a '{:?}' o el valor", tipo_valor))
                                        );
                                    }
                                }
                                None => {
                                    self.reportar_error(
                                        CategoriaError::Tipo,
                                        18, // campo no existe
                                        span,
                                        format!("El struct '{}' no tiene campo '{}'", nombre, nombre_campo),
                                        None
                                    );
                                }
                            }
                            campos_vistos.insert(nombre_campo.clone());
                        }
                        // Verificar campos faltantes
                        for campo in &info.campos {
                            if !campos_vistos.contains(&campo.nombre) {
                                self.reportar_error(
                                    CategoriaError::Tipo,
                                    19, // campo faltante
                                    span,
                                    format!("Falta campo '{}' en inicialización de struct '{}'", campo.nombre, nombre),
                                    None
                                );
                            }
                        }
                        Tipo::Nombre(nombre.clone())
                    }
                    None => {
                        self.reportar_error(
                            CategoriaError::Tipo,
                            20, // struct no declarado
                            span,
                            format!("Struct '{}' no declarado", nombre),
                            Some("Declara el struct con 'estructural {} {{ ... }}'".to_string())
                        );
                        Tipo::Entero32
                    }
                }
            }
            Expresion::ConstructorEnum(enum_nombre, variante_nombre, argumentos, span) => {
                let info_opt = self.buscar_enum(enum_nombre);
                match info_opt {
                    Some(info) => {
                        match info.variantes.iter().find(|v| v.nombre == *variante_nombre) {
                            Some(variante) => {
                                if let Some(ref campos) = variante.datos {
                                    // Verificar cantidad de argumentos
                                    if argumentos.len() != campos.len() {
                                        self.reportar_error(
                                            CategoriaError::Tipo,
                                            24,
                                            span,
                                            format!("Constructor '{}' de '{}' espera {} argumentos, pero se pasaron {}",
                                                variante_nombre, enum_nombre, campos.len(), argumentos.len()),
                                            None
                                        );
                                    } else {
                                        // Verificar tipos de cada argumento
                                        for (i, (arg, (nombre_campo, tipo_campo))) in
                                            argumentos.iter().zip(campos.iter()).enumerate() {
                                            let tipo_arg = self.inferir_tipo(arg);
                                            // Si el tipo del campo es genérico, aceptar cualquier tipo concreto
                                            if matches!(tipo_campo, Tipo::Generico(_)) {
                                                // Aceptado: el tipo genérico se instanciará con el tipo concreto
                                            } else if tipo_arg != *tipo_campo {
                                                self.reportar_error(
                                                    CategoriaError::Tipo,
                                                    DISCONCORDANCIA_TIPO,
                                                    span,
                                                    format!("Argumento {} ('{}') de '{}.{}': espera '{:?}', encontrado '{:?}'",
                                                        i + 1, nombre_campo, enum_nombre, variante_nombre, tipo_campo, tipo_arg),
                                                    Some(format!("Cambia el argumento a tipo '{:?}'", tipo_campo))
                                                );
                                            }
                                        }
                                    }
                                } else if !argumentos.is_empty() {
                                    self.reportar_error(
                                        CategoriaError::Tipo,
                                        25,
                                        span,
                                        format!("Variante '{}.{}' no tiene datos, pero se pasaron {} argumentos",
                                            enum_nombre, variante_nombre, argumentos.len()),
                                        None
                                    );
                                }
                                
                                // Si el enum es genérico, intentar inferir los tipos
                                if !info.parametros_genericos.is_empty() {
                                    // Para Resultado<T, E>, inferir T del primer argumento de Exito
                                    // o E del primer argumento de Error
                                    if enum_nombre == "Resultado" && info.parametros_genericos.len() == 2 {
                                        if variante_nombre == "Exito" && !argumentos.is_empty() {
                                            let tipo_t = self.inferir_tipo(&argumentos[0]);
                                            // E no se puede inferir, usar Entero32 por defecto
                                            Tipo::Resultado(Box::new(tipo_t), Box::new(Tipo::Entero32))
                                        } else if variante_nombre == "Error" && !argumentos.is_empty() {
                                            let tipo_e = self.inferir_tipo(&argumentos[0]);
                                            // T no se puede inferir, usar Entero32 por defecto
                                            Tipo::Resultado(Box::new(Tipo::Entero32), Box::new(tipo_e))
                                        } else {
                                            // No hay argumentos, usar Entero32 por defecto
                                            Tipo::Resultado(Box::new(Tipo::Entero32), Box::new(Tipo::Entero32))
                                        }
                                    } else {
                                        let tipos_inferidos: Vec<Tipo> = info.parametros_genericos.iter()
                                            .map(|_| Tipo::Entero32)
                                            .collect();
                                        Tipo::NombreGenerico(enum_nombre.clone(), tipos_inferidos)
                                    }
                                } else {
                                    Tipo::Nombre(enum_nombre.clone())
                                }
                            }
                            None => {
                                self.reportar_error(
                                    CategoriaError::Tipo,
                                    26,
                                    span,
                                    format!("La enumeración '{}' no tiene variante '{}'", enum_nombre, variante_nombre),
                                    None
                                );
                                Tipo::Entero32
                            }
                        }
                    }
                    None => {
                        self.reportar_error(
                            CategoriaError::Tipo,
                            27,
                            span,
                            format!("Enumeración '{}' no declarada", enum_nombre),
                            Some("Declara la enumeración con 'enumeración {} { ... }'".to_string())
                        );
                        Tipo::Entero32
                    }
                }
            }
            Expresion::EsVariante(expr, enum_nombre, variante_nombre, _binding, span) => {
                let tipo_expr = self.inferir_tipo(expr);
                // Verificar que el tipo de la expresión es el enum
                let tipo_es_enum = match &tipo_expr {
                    Tipo::Nombre(n) if n == enum_nombre => true,
                    Tipo::Resultado(_, _) if enum_nombre == "Resultado" => true,
                    Tipo::Option(_) if enum_nombre == "Option" => true,
                    _ => false,
                };
                
                if !tipo_es_enum {
                    self.reportar_error(
                        CategoriaError::Tipo,
                        28,
                        span,
                        format!("Pattern matching en tipo '{:?}', pero se esperaba '{}'", tipo_expr, enum_nombre),
                        None
                    );
                }
                // Verificar que la variante existe
                if let Some(info) = self.buscar_enum(enum_nombre) {
                    if !info.variantes.iter().any(|v| v.nombre == *variante_nombre) {
                        self.reportar_error(
                            CategoriaError::Tipo,
                            26,
                            span,
                            format!("La enumeración '{}' no tiene variante '{}'", enum_nombre, variante_nombre),
                            None
                        );
                    }
                }
                Tipo::Booleano
            }
            Expresion::AccesoCampo(expr, nombre_campo, span) => {
                let tipo_expr = self.inferir_tipo(expr);
                match &tipo_expr {
                    Tipo::Nombre(nombre_struct) => {
                        let info_opt = self.buscar_struct(nombre_struct);
                        match info_opt {
                            Some(info) => {
                                // Fase 15B: verificar campos de bits primero
                                if let Some(campo_bit) = info.campos_bits.iter().find(|c| c.nombre == *nombre_campo) {
                                    // El tipo de retorno es Entero32 (valor extraído)
                                    let _ = campo_bit;
                                    return Tipo::Entero32;
                                }
                                match info.campos.iter().find(|c| c.nombre == *nombre_campo) {
                                    Some(campo) => campo.tipo.clone(),
                                    None => {
                                        self.reportar_error(
                                            CategoriaError::Tipo,
                                            18,
                                            span,
                                            format!("El struct '{}' no tiene campo '{}'", nombre_struct, nombre_campo),
                                            None
                                        );
                                        Tipo::Entero32
                                    }
                                }
                            }
                            None => {
                                self.reportar_error(
                                    CategoriaError::Tipo,
                                    20,
                                    span,
                                    format!("Struct '{}' no declarado", nombre_struct),
                                    None
                                );
                                Tipo::Entero32
                            }
                        }
                    }
                    _ => {
                        self.reportar_error(
                            CategoriaError::Tipo,
                            21, // acceso a campo en no-struct
                            span,
                            format!("Acceso a campo '{}' en tipo '{:?}' que no es struct", nombre_campo, tipo_expr),
                            None
                        );
                        Tipo::Entero32
                    }
                }
            }
            Expresion::Mover(nombre, _destino, span) => {
                // Verificar que la variable existe
                let tipo = match self.entorno.buscar(nombre) {
                    Some(info) => info.tipo.clone(),
                    None => {
                        self.reportar_error(
                            CategoriaError::Tipo,
                            VARIABLE_NO_DECLARADA,
                            span,
                            format!("'{}' no tiene concordancia en este contexto (mover)", nombre),
                            Some("Declara la variable con artículo antes de moverla".to_string())
                        );
                        Tipo::Entero32
                    }
                };
                
                // Marcar variable como movida (para use-after-move detection)
                self.variables_movidas.insert(nombre.clone());
                
                tipo
            }
            Expresion::Copiar(expr, _span) => {
                // copiar x tiene el mismo tipo que x
                self.inferir_tipo(expr)
            }
            Expresion::Rango(inicio, _fin, _inclusivo, _span) => {
                // Un rango tiene el tipo de sus extremos (entero)
                self.inferir_tipo(inicio)
            }
            Expresion::Closure(params, cuerpo, _span) => {
                // Nuevo scope con los parámetros del closure
                let entorno_anterior = std::mem::take(&mut self.entorno);
                self.entorno = Entorno::con_padre(entorno_anterior);

                // Registrar parámetros del closure
                for (nombre, tipo_opt) in params {
                    let tipo = tipo_opt.clone().unwrap_or(Tipo::Entero32);
                    self.entorno.declarar(InfoVariable {
                        nombre: nombre.clone(),
                        tipo,
                        articulo: Articulo::La,
                        span: _span.clone(),
                    });
                }

                // Verificar cuerpo (pero el tipo del closure es function pointer = Entero64)
                let _tipo_cuerpo = self.inferir_tipo(cuerpo);

                // Restaurar entorno
                self.entorno = *self.entorno.padre.take().unwrap_or_else(|| Box::new(Entorno::nuevo()));

                // Un closure es un function pointer (I64)
                Tipo::Entero64
            }
            Expresion::Coincidir(sujeto, brazos, span) => {
                // Verificar tipo del sujeto
                let tipo_sujeto = self.inferir_tipo(sujeto);

                if brazos.is_empty() {
                    self.reportar_error(
                        CategoriaError::Tipo,
                        70,
                        span,
                        "'coincidir' requiere al menos un brazo".to_string(),
                        Some("Agrega al menos un patrón: coincidir x { _ => ... }".to_string()),
                    );
                    return Tipo::Entero32;
                }

                // Verificar cada brazo
                let mut tiene_comodin = false;
                let mut tipo_resultado: Option<Tipo> = None;

                for brazo in brazos {
                    // Verificar patrón contra tipo del sujeto
                    match &brazo.patron {
                        crate::ast::PatronMatch::Comodin(_) => {
                            tiene_comodin = true;
                        }
                        crate::ast::PatronMatch::Literal(lit) => {
                            let tipo_lit = self.tipo_literal(lit);
                            if tipo_lit != tipo_sujeto {
                                self.reportar_error(
                                    CategoriaError::Tipo,
                                    71,
                                    &brazo.span,
                                    format!("Disconcordancia en patrón: el sujeto es '{}' pero el patrón es '{}'", self.nombre_tipo_string(&tipo_sujeto), self.nombre_tipo_string(&tipo_lit)),
                                    Some("El patrón debe ser del mismo tipo que el sujeto".to_string()),
                                );
                            }
                        }
                        crate::ast::PatronMatch::VarianteEnum(enum_nombre, variante, binding, span_pat) => {
                            // Verificar que el enum existe y la variante es válida
                            if let Some(info_enum) = self.buscar_enum(enum_nombre) {
                                if !info_enum.variantes.iter().any(|v| &v.nombre == variante) {
                                    self.reportar_error(
                                        CategoriaError::Tipo,
                                        72,
                                        span_pat,
                                        format!("La variante '{}' no existe en la enumeración '{}'", variante, enum_nombre),
                                        Some(format!("Variantes disponibles: {}", info_enum.variantes.iter().map(|v| v.nombre.as_str()).collect::<Vec<_>>().join(", "))),
                                    );
                                }
                            }
                            // Si hay binding, declararlo en un scope temporal
                            if let Some(nombre_binding) = binding {
                                self.entorno.declarar(InfoVariable {
                                    nombre: nombre_binding.clone(),
                                    tipo: Tipo::Entero32, // tipo del dato de la variante
                                    articulo: Articulo::La,
                                    span: span_pat.clone(),
                                });
                            }
                        }
                    }

                    // Verificar tipo del cuerpo del brazo
                    let tipo_cuerpo = self.inferir_tipo(&brazo.cuerpo);
                    if let Some(ref tipo_previo) = tipo_resultado {
                        if *tipo_previo != tipo_cuerpo {
                            self.reportar_error(
                                CategoriaError::Tipo,
                                73,
                                &brazo.span,
                                format!("Todos los brazos de 'coincidir' deben retornar el mismo tipo: se esperaba '{}' pero este brazo retorna '{}'", self.nombre_tipo_string(tipo_previo), self.nombre_tipo_string(&tipo_cuerpo)),
                                Some("Unifica los tipos de retorno de todos los brazos".to_string()),
                            );
                        }
                    } else {
                        tipo_resultado = Some(tipo_cuerpo);
                    }
                }

                // Verificar exhaustividad: para enteros, requerir comodín
                if !tiene_comodin && matches!(tipo_sujeto, Tipo::Entero32 | Tipo::Entero64 | Tipo::Natural32 | Tipo::Natural64) {
                    self.reportar_error(
                        CategoriaError::Tipo,
                        74,
                        span,
                        "'coincidir' no es exhaustivo: faltan casos por cubrir".to_string(),
                        Some("Agrega un brazo comodín: _ => ...".to_string()),
                    );
                }

                tipo_resultado.unwrap_or(Tipo::Entero32)
            }

            // Async (Fase 18A): esperar expr
            Expresion::Esperar(expr_interno, span) => {
                // [T080] verificar que estamos dentro de una fut función
                let dentro_de_fut = self.funcion_actual.as_ref()
                    .map(|f| f.es_futuro)
                    .unwrap_or(false);
                if !dentro_de_fut {
                    self.reportar_error(
                        CategoriaError::Tipo,
                        80,
                        span,
                        "'esperar' solo puede usarse dentro de 'fut función'".to_string(),
                        Some("Marca la función como async: 'fut función nombre(...) { ... }'".to_string()),
                    );
                }
                let tipo = self.inferir_tipo(expr_interno);
                // TODO: si tipo es Futuro<T>, extraer T
                tipo
            }

            // Async (Fase 18A): lanzar expr
            Expresion::Lanzar(expr_interno, _span) => {
                // TODO: verificar que la expresión es Futuro<T> [T081]
                let _tipo = self.inferir_tipo(expr_interno);
                // lanzar retorna Tarea<T> — por ahora Entero64 (handle)
                Tipo::Entero64
            }

            // Async (Fase 18A): bloquear(expr)
            Expresion::Bloquear(expr_interno, span) => {
                // [T084] bloquear dentro de fut función causaría deadlock
                let dentro_de_fut = self.funcion_actual.as_ref()
                    .map(|f| f.es_futuro)
                    .unwrap_or(false);
                if dentro_de_fut {
                    self.reportar_error(
                        CategoriaError::Tipo,
                        84,
                        span,
                        "'bloquear()' dentro de 'fut función' causaría deadlock".to_string(),
                        Some("Usa 'esperar' en su lugar dentro de funciones async".to_string()),
                    );
                }
                let tipo = self.inferir_tipo(expr_interno);
                tipo
            }

            // GUI (Fase GUI-1): direccion_de(funcion)
            Expresion::DireccionDe(nombre_funcion, span) => {
                // Verificar que la función existe en el ámbito actual
                let funcion_existe = self.funciones.contains_key(nombre_funcion)
                    || self.simbolos_publicos_importados.contains_key(nombre_funcion);
                if !funcion_existe {
                    self.reportar_error(
                        CategoriaError::Tipo,
                        85,
                        span,
                        format!("Función '{}' no encontrada para 'direccion_de'", nombre_funcion),
                        Some("Asegúrate de que la función exista y sea accesible en el ámbito actual".to_string()),
                    );
                }
                // direccion_de retorna un puntero (Entero64)
                Tipo::Entero64
            }

            // Fase 15A: métodos bitwise en enteros
            Expresion::Bloque(bloque) => {
                // Analizar todas las sentencias del bloque
                for sentencia in &bloque.sentencias {
                    self.analizar_sentencia(sentencia);
                }
                // El tipo del bloque es el de la última expresión (o Vacio si no hay)
                if let Some(ultima) = bloque.sentencias.last() {
                    match ultima {
                        Sentencia::Expresion(expr) => self.inferir_tipo(expr),
                        Sentencia::Retornar(Some(expr), _) => self.inferir_tipo(expr),
                        _ => Tipo::Vacio,
                    }
                } else {
                    Tipo::Vacio
                }
            }
            Expresion::Metodo(receptor, nombre, args, span) => {
                let tipo_receptor = self.inferir_tipo(receptor);
                
                // Intentar resolver como método de tipo (Texto, Vector, etc.)
                if let Some(builtin) = metodo_a_builtin(&tipo_receptor, nombre) {
                    // Buscar la firma del builtin
                    if let Some(firma) = self.funciones.get(builtin) {
                        // Verificar número de argumentos
                        let esperado_args = if builtin.ends_with("_nuevo") || builtin.ends_with("_desde") {
                            let total_params = firma.parametros.len();
                            if total_params > 0 { total_params - 1 } else { 0 }
                        } else if builtin.ends_with("_concatenar") || builtin.ends_with("_comparar") {
                            let total_params = firma.parametros.len();
                            if total_params > 0 { total_params - 1 } else { 0 }
                        } else {
                            let total_params = firma.parametros.len();
                            if total_params > 0 { total_params - 1 } else { 0 }
                        };
                        let tipo_retorno = firma.retorno.clone().unwrap_or(Tipo::Entero32);
                        // firma se suelta aquí, ya podemos mutar self
                        
                        if args.len() != esperado_args {
                            self.reportar_error(
                                CategoriaError::Tipo, 1, span,
                                format!(".{} requiere {} argumento(s), se pasaron {}", nombre, esperado_args, args.len()),
                                None,
                            );
                        }
                        
                        tipo_retorno
                    } else {
                        Tipo::Entero32 // fallback
                    }
                } else {
                    // No es método de tipo built-in → verificar método bitwise
                    let es_entero = matches!(&tipo_receptor,
                        Tipo::Entero8 | Tipo::Entero16 | Tipo::Entero32 | Tipo::Entero64 |
                        Tipo::Natural8 | Tipo::Natural16 | Tipo::Natural32 | Tipo::Natural64
                    );
                    
                    if es_entero {
                        // Validar método bitwise
                        match nombre.as_str() {
                            "poner_bit" | "quitar_bit" | "alternar_bit" => {
                                if args.len() != 1 {
                                    self.reportar_error(CategoriaError::Tipo, 1, span,
                                        format!(".{} requiere exactamente 1 argumento (posición del bit)", nombre), None);
                                }
                            }
                            "extraer_bits" => {
                                if args.len() != 2 {
                                    self.reportar_error(CategoriaError::Tipo, 1, span,
                                        ".extraer_bits requiere 2 argumentos (offset, cantidad)".to_string(), None);
                                }
                            }
                            "ceros_izquierda" | "unos" => {
                                if !args.is_empty() {
                                    self.reportar_error(CategoriaError::Tipo, 1, span,
                                        format!(".{} no acepta argumentos", nombre), None);
                                }
                            }
                            _ => {
                                self.reportar_error(CategoriaError::Tipo, 1, span,
                                    format!("Tipo '{:?}' no tiene método '.{}'", tipo_receptor, nombre),
                                    Some("Revisa el nombre del método. Para enteros: poner_bit, quitar_bit, alternar_bit, extraer_bits, ceros_izquierda, unos. Para Texto: agregar, tam, liberar, obtener, concatenar, subtexto, comparar, desde. Para Vector: agregar, tam, obtener, liberar.".to_string()),
                                );
                            }
                        }
                        tipo_receptor
                    } else {
                        self.reportar_error(CategoriaError::Tipo, 1, span,
                            format!("Tipo '{:?}' no tiene método '.{}'", tipo_receptor, nombre),
                            Some("Los métodos disponibles dependen del tipo. Para enteros: poner_bit, quitar_bit, etc. Para Texto: agregar, tam, etc.".to_string()),
                        );
                        Tipo::Entero32
                    }
                }
            }
        }
    }

    fn tipo_literal(&self, lit: &Literal) -> Tipo {
        match lit {
            Literal::Entero(_, _) => Tipo::Entero32,
            Literal::Flotante(_, _) => Tipo::Flotante64,
            Literal::Palabra(_, _) => Tipo::Palabra,
            Literal::Caracter(_, _) => Tipo::Caracter,
            Literal::Booleano(_, _) => Tipo::Booleano,
        }
    }

    fn tipo_operacion(&mut self, op: OperadorBinario, tipo: &Tipo, span: &Span) -> Tipo {
        match op {
            OperadorBinario::Suma => {
                // Suma polimórfica: numérica + Texto
                if *tipo == Tipo::Texto {
                    // Texto + Texto → concatenación
                    tipo.clone()
                } else if !self.es_numerico(tipo) {
                    self.reportar_error(
                        CategoriaError::Tipo,
                        OPERACION_ARITMETICA_INVALIDA,
                        span,
                        format!("Operación '+' no válida para tipo '{:?}'. Se requiere tipo numérico (Entero o Real) o Texto para concatenación", tipo),
                        None
                    );
                    Tipo::Entero32
                } else {
                    tipo.clone()
                }
            }
            OperadorBinario::Resta |
            OperadorBinario::Multiplicacion |
            OperadorBinario::Division |
            OperadorBinario::Modulo => {
                if !self.es_numerico(tipo) {
                    self.reportar_error(
                        CategoriaError::Tipo,
                        OPERACION_ARITMETICA_INVALIDA,
                        span,
                        format!("Operación aritmética no válida para tipo '{:?}'. Se requiere tipo numérico (Entero o Real)", tipo),
                        None
                    );
                }
                tipo.clone()
            }
            OperadorBinario::Igual |
            OperadorBinario::Distinto |
            OperadorBinario::Menor |
            OperadorBinario::Mayor |
            OperadorBinario::MenorIgual |
            OperadorBinario::MayorIgual => {
                if !self.es_comparable(tipo) {
                    self.reportar_error(
                        CategoriaError::Tipo,
                        COMPARACION_INVALIDA,
                        span,
                        format!("Comparación no válida para tipo '{:?}'", tipo),
                        None
                    );
                }
                Tipo::Booleano
            }
            OperadorBinario::Y |
            OperadorBinario::O => {
                if *tipo != Tipo::Booleano {
                    self.reportar_error(
                        CategoriaError::Tipo,
                        OPERACION_LOGICA_INVALIDA,
                        span,
                        format!("Operación lógica requiere Booleano, encontrado '{:?}'", tipo),
                        None
                    );
                }
                Tipo::Booleano
            }
            OperadorBinario::BitAnd |
            OperadorBinario::BitOr |
            OperadorBinario::BitXor |
            OperadorBinario::ShiftLeft |
            OperadorBinario::ShiftRight |
            OperadorBinario::ShiftRightLogico => {
                if !self.es_entero(tipo) {
                    self.reportar_error(
                        CategoriaError::Tipo,
                        OPERACION_ARITMETICA_INVALIDA,
                        span,
                        format!("Operación bitwise requiere tipo entero, encontrado '{:?}'", tipo),
                        Some("Los operadores &, |, ^, <<, >> solo funcionan con Entero8/16/32/64 o Natural8/16/32/64".to_string())
                    );
                }
                tipo.clone()
            }
        }
    }

    fn tipo_operacion_unaria(&mut self, op: OperadorUnario, tipo: &Tipo, span: &Span) -> Tipo {
        match op {
            OperadorUnario::Negacion => {
                if !self.es_numerico(tipo) {
                    self.reportar_error(
                        CategoriaError::Tipo,
                        NEGACION_ARITMETICA_INVALIDA,
                        span,
                        format!("Negación aritmética no válida para tipo '{:?}'", tipo),
                        None
                    );
                }
                tipo.clone()
            }
            OperadorUnario::NegacionLogica => {
                if *tipo != Tipo::Booleano {
                    self.reportar_error(
                        CategoriaError::Tipo,
                        NEGACION_LOGICA_INVALIDA,
                        span,
                        format!("Negación lógica requiere Booleano, encontrado '{:?}'", tipo),
                        None
                    );
                }
                Tipo::Booleano
            }
            OperadorUnario::BitNot => {
                if !self.es_entero(tipo) {
                    self.reportar_error(
                        CategoriaError::Tipo,
                        OPERACION_ARITMETICA_INVALIDA,
                        span,
                        format!("Operador ~ (bitwise NOT) requiere tipo entero, encontrado '{:?}'", tipo),
                        Some("Usa ~ solo con Entero8/16/32/64 o Natural8/16/32/64".to_string())
                    );
                }
                tipo.clone()
            }
            _ => tipo.clone(), // Referencia y desreferencia para FASE 3
        }
    }

    /// Verifica si un tipo de argumento es compatible con un tipo de parámetro,
    /// permitiendo genéricos en el parámetro.
    /// Resuelve un alias de tipo recursivamente hasta el tipo base.
    /// Ej: alias Entero = Entero32 → resolver_alias(Entero) = Entero32.
    /// Si el nombre no es un alias, devuelve el tipo sin cambios.
    fn resolver_alias(&self, tipo: &Tipo) -> Tipo {
        match tipo {
            Tipo::Nombre(nombre) => {
                if let Some(alias) = self.aliases.get(nombre) {
                    self.resolver_alias(alias)
                } else {
                    tipo.clone()
                }
            }
            // Resolver también dentro de contenedores
            Tipo::Vector(inner) => Tipo::Vector(Box::new(self.resolver_alias(inner))),
            Tipo::Array(inner, n) => Tipo::Array(Box::new(self.resolver_alias(inner)), *n),
            Tipo::Puntero(inner) => Tipo::Puntero(Box::new(self.resolver_alias(inner))),
            Tipo::Referencia(inner) => Tipo::Referencia(Box::new(self.resolver_alias(inner))),
            Tipo::ReferenciaMut(inner) => Tipo::ReferenciaMut(Box::new(self.resolver_alias(inner))),
            _ => tipo.clone(),
        }
    }

    fn tipos_compatibles(&self,
        tipo_param: &Tipo,
        tipo_arg: &Tipo,
    ) -> bool {
        // Resolver aliases de tipo antes de comparar (ej: alias Entero = Entero32)
        let tipo_param = self.resolver_alias(tipo_param);
        let tipo_arg = self.resolver_alias(tipo_arg);
        if tipo_param == tipo_arg {
            return true;
        }

        match (&tipo_param, &tipo_arg) {
            // Type params concuerdan con cualquier tipo en ambas direcciones
            (Tipo::Generico(_), _) | (_, Tipo::Generico(_)) => true,
            // Array-to-pointer decay: array es compatible con Entero64 (puntero raw)
            (Tipo::Entero64, Tipo::Array(_, _)) => true,
            // Array genérico concuerda con array de tamaño conocido si el elemento concuerda
            (Tipo::ArrayGenerico(elem_param, _), Tipo::Array(elem_arg, _)) => self.tipos_compatibles(elem_param, elem_arg),
            // Arrays concuerdan si elementos concuerdan (tamaño puede variar por genérico)
            (Tipo::Array(elem_param, _), Tipo::Array(elem_arg, _)) => self.tipos_compatibles(elem_param, elem_arg),
            // Recursión para punteros/referencias
            (Tipo::Puntero(p), Tipo::Puntero(a)) => self.tipos_compatibles(p, a),
            (Tipo::Referencia(p), Tipo::Referencia(a)) => self.tipos_compatibles(p, a),
            // Lifetimes léxicos son compatibles con referencias normales (el lifetime se ignora en compatibilidad)
            (Tipo::ReferenciaConLifetime(_, p), Tipo::Referencia(a)) => self.tipos_compatibles(p, a),
            (Tipo::Referencia(p), Tipo::ReferenciaConLifetime(_, a)) => self.tipos_compatibles(p, a),
            (Tipo::ReferenciaConLifetime(_, p), Tipo::ReferenciaConLifetime(_, a)) => self.tipos_compatibles(p, a),
            (Tipo::ReferenciaMutConLifetime(_, p), Tipo::ReferenciaMut(a)) => self.tipos_compatibles(p, a),
            (Tipo::ReferenciaMut(p), Tipo::ReferenciaMutConLifetime(_, a)) => self.tipos_compatibles(p, a),
            (Tipo::ReferenciaMutConLifetime(_, p), Tipo::ReferenciaMutConLifetime(_, a)) => self.tipos_compatibles(p, a),
            // Self-referential: &self T es compatible con &T y &self T
            (Tipo::ReferenciaSelf(p), Tipo::Referencia(a)) => self.tipos_compatibles(p, a),
            (Tipo::Referencia(p), Tipo::ReferenciaSelf(a)) => self.tipos_compatibles(p, a),
            (Tipo::ReferenciaSelf(p), Tipo::ReferenciaSelf(a)) => self.tipos_compatibles(p, a),
            (Tipo::ReferenciaMutSelf(p), Tipo::ReferenciaMut(a)) => self.tipos_compatibles(p, a),
            (Tipo::ReferenciaMut(p), Tipo::ReferenciaMutSelf(a)) => self.tipos_compatibles(p, a),
            (Tipo::ReferenciaMutSelf(p), Tipo::ReferenciaMutSelf(a)) => self.tipos_compatibles(p, a),
            // &self T también compatible con &nombre T (lifetime léxico)
            (Tipo::ReferenciaSelf(p), Tipo::ReferenciaConLifetime(_, a)) => self.tipos_compatibles(p, a),
            (Tipo::ReferenciaConLifetime(_, p), Tipo::ReferenciaSelf(a)) => self.tipos_compatibles(p, a),
            (Tipo::ReferenciaMutSelf(p), Tipo::ReferenciaMutConLifetime(_, a)) => self.tipos_compatibles(p, a),
            (Tipo::ReferenciaMutConLifetime(_, p), Tipo::ReferenciaMutSelf(a)) => self.tipos_compatibles(p, a),
            // FFI/GUI: Referencia(T) ≡ Entero64 (ambos son puntero de 8 bytes en x64)
            // &expr produce stack_addr → I64, compatible con parámetros Entero64 de FFI
            (Tipo::Entero64, Tipo::Referencia(_)) |
            (Tipo::Entero64, Tipo::ReferenciaMut(_)) |
            (Tipo::Entero64, Tipo::ReferenciaConLifetime(_, _)) |
            (Tipo::Entero64, Tipo::ReferenciaMutConLifetime(_, _)) |
            (Tipo::Entero64, Tipo::ReferenciaSelf(_)) |
            (Tipo::Entero64, Tipo::ReferenciaMutSelf(_)) => true,
            _ => false,
        }
    }

    fn es_numerico(&self, tipo: &Tipo) -> bool {
        match tipo {
            Tipo::Generico(nombre) => self.tiene_bound(nombre, "Numérico"),
            _ => matches!(tipo,
                Tipo::Entero8 | Tipo::Entero16 | Tipo::Entero32 | Tipo::Entero64 |
                Tipo::Natural8 | Tipo::Natural16 | Tipo::Natural32 | Tipo::Natural64 |
                Tipo::Flotante32 | Tipo::Flotante64
            ),
        }
    }

    fn es_entero(&self, tipo: &Tipo) -> bool {
        matches!(tipo,
            Tipo::Entero8 | Tipo::Entero16 | Tipo::Entero32 | Tipo::Entero64 |
            Tipo::Natural8 | Tipo::Natural16 | Tipo::Natural32 | Tipo::Natural64
        )
    }

    fn es_flotante(&self, tipo: &Tipo) -> bool {
        matches!(tipo, Tipo::Flotante32 | Tipo::Flotante64)
    }

    /// En operaciones binarias, los literales numéricos se adaptan al tipo del
    /// otro operando (ej: `x: Entero64 + 1` → el 1 se trata como Entero64).
    /// Devuelve los tipos adaptados.
    fn adaptar_literales_binaria(&self, izq: &Expresion, tipo_izq: &Tipo, der: &Expresion, tipo_der: &Tipo) -> (Tipo, Tipo) {
        let es_literal_entero = |e: &Expresion| match e {
            Expresion::Literal(Literal::Entero(_, _)) => true,
            Expresion::Unaria(_, inner, _) => matches!(inner.as_ref(), Expresion::Literal(Literal::Entero(_, _))),
            _ => false,
        };
        let es_literal_flotante = |e: &Expresion| matches!(e, Expresion::Literal(Literal::Flotante(_, _)));

        match (es_literal_entero(izq), es_literal_entero(der), es_literal_flotante(izq), es_literal_flotante(der)) {
            // Literal entero + tipo entero/flotante
            (true, false, _, _) if self.es_numerico(tipo_der) && !es_literal_flotante(izq) => (tipo_der.clone(), tipo_der.clone()),
            (false, true, _, _) if self.es_numerico(tipo_izq) && !es_literal_flotante(der) => (tipo_izq.clone(), tipo_izq.clone()),
            // Literal flotante + tipo flotante
            (_, _, true, false) if self.es_flotante(tipo_der) => (tipo_der.clone(), tipo_der.clone()),
            (_, _, false, true) if self.es_flotante(tipo_izq) => (tipo_izq.clone(), tipo_izq.clone()),
            _ => (tipo_izq.clone(), tipo_der.clone()),
        }
    }

    fn es_comparable(&self, tipo: &Tipo) -> bool {
        match tipo {
            Tipo::Generico(nombre) => {
                self.tiene_bound(nombre, "Comparable") || self.tiene_bound(nombre, "Ordenable")
            }
            _ => self.es_numerico(tipo) || matches!(tipo, Tipo::Caracter | Tipo::Booleano),
        }
    }

    fn tiene_bound(&self, nombre: &str, bound: &str) -> bool {
        if let Some(func) = &self.funcion_actual {
            func.parametros_genericos.iter().any(|pg| {
                pg.nombre == nombre && pg.bounds.iter().any(|b| b == bound)
            })
        } else {
            false
        }
    }

    fn es_mutable(&self, articulo: Articulo) -> bool {
        // el = owned mutable, la = borrowed immutable
        // un = optional (mutable by default)
        // los = shared ownership (mutable, reference-counted)
        // las = shared borrowed (inmutable, solo lectura)
        matches!(articulo, Articulo::El | Articulo::Un | Articulo::Los)
    }

    fn articulo_a_str(&self, articulo: Articulo) -> &'static str {
        match articulo {
            Articulo::El => "el",
            Articulo::La => "la",
            Articulo::Un => "un",
            Articulo::Los => "los",
            Articulo::Las => "las",
        }
    }

    /// Convierte un Tipo a string para usar como clave en impls
    fn nombre_tipo_string(&self, tipo: &Tipo) -> String {
        match tipo {
            Tipo::Nombre(n) => n.clone(),
            Tipo::Generico(n) => n.clone(),
            _ => format!("{:?}", tipo),
        }
    }
}

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
        let fuente = r#"función principal() -> Entero32 {
    el x: Entero32 = 10;
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
        // 'el' es mutable, asignación permitida
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
        // 'la' es inmutable, asignación prohibida
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
        // Simula ops.fc con función privada
        let mut simbolos_publicos: HashMap<String, FirmaFuncion> = HashMap::new();
        // No agregamos 'ops::secreto' porque es privado
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
        // texto_desde espera Palabra, no Entero32
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
        // vector_agregar<Entero32> no acepta Booleano
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
