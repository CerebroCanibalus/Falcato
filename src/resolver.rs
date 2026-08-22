use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;

use crate::ast::Programa;
use crate::lexer::LexerFalcato;
use crate::parser::ParserFalcato;
use crate::semantic::{AnalizadorSemantico, FirmaFuncion, InfoEnum, InfoStruct};

/// Representa una unidad de compilación: archivo fuente → AST → código objeto.
#[derive(Debug, Clone)]
pub struct UnidadCompilacion {
    /// Ruta absoluta al archivo fuente `.fc`
    pub ruta: PathBuf,
    /// Nombre del módulo (derivado del archivo o del bloque `módulo`)
    pub nombre_modulo: String,
    /// Dependencias: nombres de módulos que este archivo importa
    pub dependencias: Vec<String>,
    /// Código fuente
    pub fuente: String,
}

/// Resuelve módulos y archivos, construye el grafo de compilación.
pub struct Resolver {
    /// Directorio base para búsqueda de módulos
    pub base_dir: PathBuf,
    /// Módulos resueltos: nombre → unidad de compilación
    pub modulos: HashMap<String, UnidadCompilacion>,
    /// Orden topológico de compilación
    pub orden: Vec<String>,
}

impl Resolver {
    /// Crea un resolver nuevo con directorio base.
    pub fn nuevo(base_dir: &Path) -> Self {
        Resolver {
            base_dir: base_dir.to_path_buf(),
            modulos: HashMap::new(),
            orden: Vec::new(),
        }
    }

    /// Resuelve un módulo por nombre, buscando el archivo `.fc`
    /// Reglas de búsqueda:
    /// 1. `<nombre>.fc` en base_dir
    /// 2. `<base_dir>/<nombre>/mod.fc` (módulo como directorio)
    fn resolver_ruta_modulo(&self, nombre: &str) -> Option<PathBuf> {
        // Intentar <nombre>.fc
        let ruta1 = self.base_dir.join(format!("{}.fc", nombre));
        if ruta1.exists() {
            return Some(ruta1);
        }

        // Intentar <nombre>/mod.fc
        let ruta2 = self.base_dir.join(nombre).join("mod.fc");
        if ruta2.exists() {
            return Some(ruta2);
        }

        None
    }

    /// Agrega un archivo fuente al resolver. Si el archivo tiene imports (`usar`),
    /// los resuelve recursivamente.
    pub fn agregar_archivo(&mut self, ruta: &Path) -> Result<(), String> {
        let ruta_canonica = fs::canonicalize(ruta)
            .map_err(|e| format!("No se puede resolver ruta '{}': {}", ruta.display(), e))?;

        // El nombre del módulo es el filename sin extensión
        let nombre_modulo = ruta_canonica
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| format!("Nombre de archivo inválido: {}", ruta.display()))?
            .to_string();

        // Si ya está resuelto, no repetir
        if self.modulos.contains_key(&nombre_modulo) {
            return Ok(());
        }

        let fuente = fs::read_to_string(&ruta_canonica)
            .map_err(|e| format!("No se pudo leer '{}': {}", ruta_canonica.display(), e))?;

        // Analizar el fuente para extraer imports (usar declaraciones)
        let dependencias = self.extraer_imports(&fuente, &ruta_canonica)?;

        let unidad = UnidadCompilacion {
            ruta: ruta_canonica.clone(),
            nombre_modulo: nombre_modulo.clone(),
            dependencias: dependencias.clone(),
            fuente,
        };

        self.modulos.insert(nombre_modulo.clone(), unidad);

        // Resolver dependencias recursivamente
        for dep in &dependencias {
            if !self.modulos.contains_key(dep) {
                let ruta_dep = self.resolver_ruta_modulo(dep)
                    .ok_or_else(|| format!("Módulo '{}' no encontrado (importado desde '{}')", dep, ruta.display()))?;
                self.agregar_archivo(&ruta_dep)?;
            }
        }

        Ok(())
    }

    /// Extrae los nombres de módulos importados de un fuente usando el lexer (rápido, sin parse completo).
    fn extraer_imports(&self, fuente: &str, ruta: &Path) -> Result<Vec<String>, String> {
        let ruta_str = ruta.to_string_lossy().to_string();
        let lexer = LexerFalcato::nuevo(fuente, &ruta_str);
        let tokens = lexer.tokenizar();

        let mut imports = Vec::new();
        let mut i = 0;
        while i < tokens.len() {
            // Buscar token Usar (Debug representation)
            if format!("{:?}", tokens[i].token) == "Usar" {
                // El siguiente token debería ser el nombre del módulo
                i += 1;
                if i < tokens.len() {
                    // Extraer el nombre del módulo
                    let modulo = self.extraer_nombre_modulo(&tokens[i..]);
                    if let Some(nombre) = modulo {
                        if !imports.contains(&nombre) {
                            imports.push(nombre);
                        }
                    }
                }
            }
            i += 1;
        }

        Ok(imports)
    }

    /// Extrae el nombre del módulo de una secuencia de tokens que comienza con un identificador.
    /// Maneja: `modulo::simbolo` y `modulo::*`
    fn extraer_nombre_modulo(&self, tokens: &[crate::lexer::TokenConSpan]) -> Option<String> {
        use crate::lexer::Token;
        if tokens.is_empty() { return None; }

        match &tokens[0].token {
            Token::Identificador(nombre) => Some(nombre.clone()),
            _ => None,
        }
    }

    /// Calcula el orden topológico de compilación (las dependencias primero).
    pub fn calcular_orden(&mut self) -> Result<(), String> {
        let mut visitados: HashMap<String, bool> = HashMap::new();
        let mut orden: Vec<String> = Vec::new();

        for nombre in self.modulos.keys() {
            if !visitados.contains_key(nombre) {
                self.visitar(nombre, &mut visitados, &mut orden)?;
            }
        }

        self.orden = orden;
        Ok(())
    }

    fn visitar(
        &self,
        nombre: &str,
        visitados: &mut HashMap<String, bool>,
        orden: &mut Vec<String>,
    ) -> Result<(), String> {
        if let Some(&en_proceso) = visitados.get(nombre) {
            if en_proceso {
                return Err(format!("Dependencia circular detectada en módulo '{}'", nombre));
            }
            return Ok(());
        }

        visitados.insert(nombre.to_string(), true);

        let unidad = self.modulos.get(nombre)
            .ok_or_else(|| format!("Módulo '{}' no encontrado", nombre))?;

        for dep in &unidad.dependencias {
            if self.modulos.contains_key(dep) || self.resolver_ruta_modulo(dep).is_some() {
                self.visitar(dep, visitados, orden)?;
            }
            // Si no existe, el error se reportará en semántica
        }

        visitados.insert(nombre.to_string(), false);
        orden.push(nombre.to_string());
        Ok(())
    }

    /// Compila todos los módulos en orden topológico en una sola unidad de código.
    /// Todos los módulos comparten el mismo backend, así que las funciones son visibles entre sí.
    /// La semántica comparte símbolos públicos entre módulos para type-checking cross-file real.
    pub fn compilar_todo(&mut self) -> Result<Vec<(String, String)>, String> {
        let mut objetos: Vec<(String, String)> = Vec::new();

        // 1. Parsear todos los módulos primero
        let mut programas: Vec<(String, Programa)> = Vec::new();
        for nombre_modulo in &self.orden.clone() {
            let unidad = self.modulos.get(nombre_modulo)
                .ok_or_else(|| format!("Módulo '{}' no encontrado", nombre_modulo))?;

            let ruta_str = unidad.ruta.to_string_lossy().to_string();
            let lexer = LexerFalcato::nuevo(&unidad.fuente, &ruta_str);
            let tokens = lexer.tokenizar();

            let programa = ParserFalcato::parse(tokens)
                .map_err(|errores| {
                    let msgs: Vec<String> = errores.iter()
                        .map(|e| e.error.to_string())
                        .collect();
                    format!("Errores de parseo en '{}':\n{}", unidad.ruta.display(), msgs.join("\n"))
                })?;

            programas.push((nombre_modulo.clone(), programa));
        }

        // 2. Colectar símbolos públicos de todos los módulos en un mapa global
        let mut simbolos_publicos: HashMap<String, FirmaFuncion> = HashMap::new();
        let mut structs_publicos: HashMap<String, InfoStruct> = HashMap::new();
        let mut enums_publicos: HashMap<String, InfoEnum> = HashMap::new();
        for (nombre_modulo, programa) in &programas {
            for decl in &programa.declaraciones {
                Self::colectar_simbolos_publicos_decl(
                    decl, nombre_modulo, "",
                    &mut simbolos_publicos, &mut structs_publicos, &mut enums_publicos,
                );
            }
        }

        // 3. Analizar semánticamente cada módulo con acceso a los símbolos públicos globales
        for (nombre_modulo, programa) in &programas {
            let mut semantica = AnalizadorSemantico::con_simbolos_publicos_completo(
                simbolos_publicos.clone(),
                structs_publicos.clone(),
                enums_publicos.clone(),
            );
            semantica.analizar(programa)
                .map_err(|e| format!("Errores semánticos en '{}':\n{}", nombre_modulo, e))?;
        }

        // 4. Codegen unificado
        let mut codegen = crate::codegen::Codegen::nuevo("falcato_programa")?;
        for (_, programa) in &programas {
            codegen.compilar_programa(programa)
                .map_err(|e| format!("Errores de compilación:\n{:?}", e))?;
        }

        // 5. Escribir un solo objeto con todo
        let obj_ruta = "falcato_programa.o".to_string();
        codegen.escribir_objeto(&obj_ruta)?;
        objetos.push(("falcato_programa".to_string(), obj_ruta));

        Ok(objetos)
    }

    /// Recorre declaraciones y registra símbolos públicos con nombre cualificado:
    /// funciones, structs y enums.
    /// - Top-level de archivo: `nombre_modulo::funcion` / `nombre_modulo::Struct`
    /// - Dentro de módulo inline: `prefijo::simbolo`
    pub fn colectar_simbolos_publicos_decl(
        decl: &crate::ast::Declaracion,
        nombre_modulo_archivo: &str,
        prefijo: &str,
        simbolos: &mut HashMap<String, FirmaFuncion>,
        structs: &mut HashMap<String, InfoStruct>,
        enums: &mut HashMap<String, InfoEnum>,
    ) {
        use crate::ast::Declaracion;
        let cualificar = |nombre: &str| -> String {
            if prefijo.is_empty() {
                format!("{}::{}", nombre_modulo_archivo, nombre)
            } else {
                format!("{}::{}", prefijo, nombre)
            }
        };
        match decl {
            Declaracion::Funcion(func) => {
                let es_top_level = prefijo.is_empty();
                if AnalizadorSemantico::es_funcion_publica(func, es_top_level) {
                    let nombre_cualificado = cualificar(&func.nombre);
                    let firma = FirmaFuncion {
                        nombre: nombre_cualificado.clone(),
                        parametros_genericos: func.parametros_genericos.clone(),
                        parametros: func.parametros.iter()
                            .map(|p| (p.nombre.clone(), p.tipo.clone()))
                            .collect(),
                        retorno: func.retorno.clone(),
                        span: func.span.clone(),
                        es_publica: true,
                    };
                    simbolos.insert(nombre_cualificado, firma);
                }
            }
            Declaracion::Estructural(s) => {
                let nombre_cualificado = cualificar(&s.nombre);
                structs.insert(nombre_cualificado, InfoStruct {
                    nombre: s.nombre.clone(),
                    campos: s.campos.clone(),
                    campos_bits: s.campos_bits.clone(),
                    span: s.span.clone(),
                });
            }
            Declaracion::Enumeracion(e) => {
                let nombre_cualificado = cualificar(&e.nombre);
                enums.insert(nombre_cualificado, InfoEnum {
                    nombre: e.nombre.clone(),
                    parametros_genericos: e.parametros_genericos.clone(),
                    variantes: e.variantes.clone(),
                    span: e.span.clone(),
                });
            }
            Declaracion::Modulo(modulo) => {
                let nuevo_prefijo = if prefijo.is_empty() {
                    modulo.nombre.clone()
                } else {
                    format!("{}::{}", prefijo, modulo.nombre)
                };
                for decl in &modulo.contenido {
                    Self::colectar_simbolos_publicos_decl(
                        decl, nombre_modulo_archivo, &nuevo_prefijo,
                        simbolos, structs, enums,
                    );
                }
            }
            _ => {}
        }
    }
}
