use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::ast::*;
use crate::error::ErrorCompilador;
use crate::lexer::LexerFalcato;
use crate::parser::ParserFalcato;
use crate::semantic::AnalizadorSemantico;
use crate::span::Span;

// ============================================
// ÍNDICE SEMÁNTICO PARA LSP
// ============================================

#[allow(deprecated)]

/// Información de una variable para hover/definition
#[derive(Debug, Clone)]
pub struct InfoVariableLsp {
    pub nombre: String,
    pub tipo: String,
    pub articulo: String,
    pub articulo_raw: String,
    pub span_declaracion: Span,
}

/// Información de una función para hover/definition
#[derive(Debug, Clone)]
pub struct InfoFuncionLsp {
    pub nombre: String,
    pub retorno: Option<String>,
    pub parametros: Vec<String>,
    pub parametros_raw: Vec<(String, String)>, // (nombre, tipo_string)
    pub span_declaracion: Span,
}

/// Información de un struct para outline/completion
#[derive(Debug, Clone)]
pub struct InfoStructLsp {
    pub nombre: String,
    pub campos: Vec<(String, String)>, // (nombre, tipo_string)
    pub span_declaracion: Span,
}

/// Información de un enum para outline/completion
#[derive(Debug, Clone)]
pub struct InfoEnumLsp {
    pub nombre: String,
    pub variantes: Vec<(String, Option<String>)>, // (nombre, tipo_dato_opcional)
    pub span_declaracion: Span,
}

/// Información de un trait para outline/completion
#[derive(Debug, Clone)]
pub struct InfoTraitLsp {
    pub nombre: String,
    pub metodos: Vec<String>, // firmas
    pub span_declaracion: Span,
}

/// Índice semántico de un documento
#[derive(Debug, Clone, Default)]
pub struct IndiceSemantico {
    pub variables: HashMap<String, InfoVariableLsp>,
    pub funciones: HashMap<String, InfoFuncionLsp>,
    pub structs: HashMap<String, InfoStructLsp>,
    pub enums: HashMap<String, InfoEnumLsp>,
    pub traits: HashMap<String, InfoTraitLsp>,
}

impl IndiceSemantico {
    pub fn nuevo() -> Self {
        Self::default()
    }

    /// Construye el índice a partir del AST
    pub fn desde_ast(programa: &Programa) -> Self {
        let mut indice = Self::nuevo();

        for decl in &programa.declaraciones {
            match decl {
                Declaracion::Funcion(func) => {
                    indice.indexar_funcion(func);
                }
                Declaracion::Estructural(estructural) => {
                    indice.indexar_estructural(estructural);
                }
                Declaracion::Enumeracion(enumeracion) => {
                    indice.indexar_enumeracion(enumeracion);
                }
                Declaracion::Rasgo(rasgo) => {
                    indice.indexar_rasgo(rasgo);
                }
                _ => {}
            }
        }

        indice
    }

    fn indexar_funcion(&mut self,
        func: &FuncionDecl,
    ) {
        // Registrar función
        let params: Vec<String> = func.parametros.iter()
            .map(|p| format!("{} {}: {:?}", self.articulo_str(p.articulo), p.nombre, p.tipo))
            .collect();
        let params_raw: Vec<(String, String)> = func.parametros.iter()
            .map(|p| (p.nombre.clone(), format!("{:?}", p.tipo)))
            .collect();

        self.funciones.insert(func.nombre.clone(), InfoFuncionLsp {
            nombre: func.nombre.clone(),
            retorno: func.retorno.as_ref().map(|t| format!("{:?}", t)),
            parametros: params,
            parametros_raw: params_raw,
            span_declaracion: func.span.clone(),
        });

        // Registrar parámetros como variables
        for param in &func.parametros {
            self.variables.insert(param.nombre.clone(), InfoVariableLsp {
                nombre: param.nombre.clone(),
                tipo: format!("{:?}", param.tipo),
                articulo: self.articulo_str(param.articulo).to_string(),
                articulo_raw: format!("{:?}", param.articulo),
                span_declaracion: param.span.clone(),
            });
        }

        // Registrar variables del cuerpo
        for sentencia in &func.cuerpo.sentencias {
            self.indexar_sentencia(sentencia);
        }
    }

    fn indexar_estructural(&mut self,
        decl: &EstructuralDecl,
    ) {
        let campos: Vec<(String, String)> = decl.campos.iter()
            .map(|c| (c.nombre.clone(), format!("{:?}", c.tipo)))
            .collect();
        self.structs.insert(decl.nombre.clone(), InfoStructLsp {
            nombre: decl.nombre.clone(),
            campos,
            span_declaracion: decl.span.clone(),
        });
    }

    fn indexar_enumeracion(&mut self,
        decl: &EnumeracionDecl,
    ) {
        let variantes: Vec<(String, Option<String>)> = decl.variantes.iter()
            .map(|v| {
                let tipo_dato = v.datos.as_ref().map(|d| format!("{:?}", d));
                (v.nombre.clone(), tipo_dato)
            })
            .collect();
        self.enums.insert(decl.nombre.clone(), InfoEnumLsp {
            nombre: decl.nombre.clone(),
            variantes,
            span_declaracion: decl.span.clone(),
        });
    }

    fn indexar_rasgo(&mut self,
        decl: &RasgoDecl,
    ) {
        let metodos: Vec<String> = decl.metodos.iter()
            .map(|m| {
                let params: Vec<String> = m.parametros.iter()
                    .map(|p| format!("{} {}: {:?}", self.articulo_str(p.articulo), p.nombre, p.tipo))
                    .collect();
                let ret = m.retorno.as_ref().map(|t| format!(" -> {:?}", t)).unwrap_or_default();
                format!("fn {}({}){}", m.nombre, params.join(", "), ret)
            })
            .collect();
        self.traits.insert(decl.nombre.clone(), InfoTraitLsp {
            nombre: decl.nombre.clone(),
            metodos,
            span_declaracion: decl.span.clone(),
        });
    }

    fn indexar_sentencia(&mut self,
        sentencia: &Sentencia,
    ) {
        match sentencia {
            Sentencia::DeclaracionVariable(decl) => {
                self.variables.insert(decl.nombre.clone(), InfoVariableLsp {
                    nombre: decl.nombre.clone(),
                    tipo: decl.tipo.as_ref().map(|t| format!("{:?}", t))
                        .unwrap_or_else(|| "inferido".to_string()),
                    articulo: self.articulo_str(decl.articulo).to_string(),
                    articulo_raw: format!("{:?}", decl.articulo),
                    span_declaracion: decl.span.clone(),
                });
            }
            Sentencia::Condicional(cond) => {
                for s in &cond.bloque_entonces.sentencias {
                    self.indexar_sentencia(s);
                }
                if let Some(ref sino) = cond.bloque_sino {
                    for s in &sino.sentencias {
                        self.indexar_sentencia(s);
                    }
                }
            }
            Sentencia::BucleMientras(bucle) => {
                for s in &bucle.bloque.sentencias {
                    self.indexar_sentencia(s);
                }
            }
            _ => {}
        }
    }

    fn articulo_str(&self,
        articulo: Articulo,
    ) -> &'static str {
        match articulo {
            Articulo::El => "el",
            Articulo::La => "la",
            Articulo::Un => "un",
            Articulo::Los => "los",
            Articulo::Las => "las",
        }
    }

    /// Busca qué identificador está en la posición dada
    pub fn identificador_en_posicion(
        &self,
        programa: &Programa,
        linea: u32,      // 1-indexed
        columna: u32,    // 1-indexed
    ) -> Option<String> {
        for decl in &programa.declaraciones {
            if let Declaracion::Funcion(func) = decl {
                // Buscar en el cuerpo de la función
                if let Some(nombre) = self.buscar_en_bloque(&func.cuerpo, linea, columna) {
                    return Some(nombre);
                }
            }
        }
        None
    }

    fn buscar_en_bloque(&self,
        bloque: &Bloque,
        linea: u32,
        columna: u32,
    ) -> Option<String> {
        for sentencia in &bloque.sentencias {
            if let Some(nombre) = self.buscar_en_sentencia(sentencia, linea, columna) {
                return Some(nombre);
            }
        }
        None
    }

    fn buscar_en_sentencia(&self,
        sentencia: &Sentencia,
        linea: u32,
        columna: u32,
    ) -> Option<String> {
        match sentencia {
            Sentencia::Expresion(expr) => self.buscar_en_expresion(expr, linea, columna),
            Sentencia::Romper(_) | Sentencia::Continuar(_) => None,
            Sentencia::DeclaracionVariable(decl) => {
                // Buscar en el valor
                if let Some(nombre) = self.buscar_en_expresion(&decl.valor, linea, columna) {
                    return Some(nombre);
                }
                // ¿Es el nombre de la variable en sí?
                if self.posicion_en_span(linea, columna, &decl.span) {
                    // Verificar si el cursor está específicamente sobre el identificador
                    // (simplificación: si está en la línea de declaración)
                    return Some(decl.nombre.clone());
                }
                None
            }
            Sentencia::Asignacion(asig) => {
                if self.posicion_en_span(linea, columna, &asig.span) {
                    match &asig.lugar {
                        crate::ast::Lugar::Identificador(nombre) => return Some(nombre.clone()),
                        crate::ast::Lugar::Array(array, _) => {
                            if let Some(n) = self.buscar_en_expresion(array, linea, columna) {
                                return Some(n);
                            }
                        }
                        crate::ast::Lugar::Campo(base, _campo) => {
                            if let Some(n) = self.buscar_en_expresion(base, linea, columna) {
                                return Some(n);
                            }
                        }
                    }
                }
                self.buscar_en_expresion(&asig.valor, linea, columna)
            }
            Sentencia::Retornar(expr, _) => {
                expr.as_ref().and_then(|e| self.buscar_en_expresion(e, linea, columna))
            }
            Sentencia::Condicional(cond) => {
                if let Some(n) = self.buscar_en_expresion(&cond.condicion, linea, columna) {
                    return Some(n);
                }
                if let Some(n) = self.buscar_en_bloque(&cond.bloque_entonces, linea, columna) {
                    return Some(n);
                }
                if let Some(ref sino) = cond.bloque_sino {
                    if let Some(n) = self.buscar_en_bloque(sino, linea, columna) {
                        return Some(n);
                    }
                }
                None
            }
            Sentencia::BucleMientras(bucle) => {
                if let Some(n) = self.buscar_en_expresion(&bucle.condicion, linea, columna) {
                    return Some(n);
                }
                if let Some(n) = self.buscar_en_bloque(&bucle.bloque, linea, columna) {
                    return Some(n);
                }
                None
            }
            Sentencia::BuclePara(bucle) => {
                if let Some(n) = self.buscar_en_expresion(&bucle.iterable, linea, columna) {
                    return Some(n);
                }
                if let Some(n) = self.buscar_en_bloque(&bucle.bloque, linea, columna) {
                    return Some(n);
                }
                None
            }
            Sentencia::Region { nombre: _, cuerpo, span: _ } => {
                for sent in cuerpo {
                    if let Some(n) = self.buscar_en_sentencia(sent, linea, columna) {
                        return Some(n);
                    }
                }
                None
            }
            Sentencia::Seleccionar(seleccionar) => {
                for rama in &seleccionar.ramas {
                    if let Some(n) = self.buscar_en_expresion(&rama.canal, linea, columna) {
                        return Some(n);
                    }
                    for sent in &rama.cuerpo.sentencias {
                        if let Some(n) = self.buscar_en_sentencia(sent, linea, columna) {
                            return Some(n);
                        }
                    }
                }
                None
            }
            Sentencia::ConExecutor { hilos, cuerpo, span: _ } => {
                if let Some(n) = self.buscar_en_expresion(hilos, linea, columna) {
                    return Some(n);
                }
                for sent in cuerpo {
                    if let Some(n) = self.buscar_en_sentencia(sent, linea, columna) {
                        return Some(n);
                    }
                }
                None
            }
        }
    }

    fn buscar_en_expresion(&self,
        expr: &Expresion,
        linea: u32,
        columna: u32,
    ) -> Option<String> {
        match expr {
            Expresion::Identificador(nombre, span) => {
                if self.posicion_en_span(linea, columna, span) {
                    Some(nombre.clone())
                } else {
                    None
                }
            }
            Expresion::Llamada(llamada) => {
                // ¿Está sobre el nombre de la función?
                if self.posicion_en_span(linea, columna, &llamada.span) {
                    return Some(llamada.funcion.clone());
                }
                // Buscar en argumentos
                for arg in &llamada.argumentos {
                    if let Some(n) = self.buscar_en_expresion(arg, linea, columna) {
                        return Some(n);
                    }
                }
                None
            }
            Expresion::Binaria(izq, _, der, _span) => {
                if let Some(n) = self.buscar_en_expresion(izq, linea, columna) {
                    return Some(n);
                }
                if let Some(n) = self.buscar_en_expresion(der, linea, columna) {
                    return Some(n);
                }
                None
            }
            Expresion::Unaria(_, expr, _) => {
                self.buscar_en_expresion(expr, linea, columna)
            }
            _ => None,
        }
    }

    fn posicion_en_span(
        &self,
        linea: u32,
        columna: u32,
        span: &Span,
    ) -> bool {
        linea >= span.inicio.linea && linea <= span.fin.linea
            && columna >= span.inicio.columna && columna <= span.fin.columna
    }

    // === Find References ===

    pub fn encontrar_referencias(
        &self,
        programa: &Programa,
        nombre: &str,
    ) -> Vec<Span> {
        let mut referencias = Vec::new();

        if let Some(v) = self.variables.get(nombre) {
            referencias.push(v.span_declaracion.clone());
        }
        if let Some(f) = self.funciones.get(nombre) {
            referencias.push(f.span_declaracion.clone());
        }

        for decl in &programa.declaraciones {
            if let Declaracion::Funcion(func) = decl {
                Self::colectar_referencias_en_bloque(&func.cuerpo, nombre, &mut referencias);
            }
        }

        referencias
    }

    fn colectar_referencias_en_bloque(bloque: &Bloque, nombre: &str, refs: &mut Vec<Span>) {
        for sentencia in &bloque.sentencias {
            Self::colectar_referencias_en_sentencia(sentencia, nombre, refs);
        }
    }

    fn colectar_referencias_en_sentencia(sentencia: &Sentencia, nombre: &str, refs: &mut Vec<Span>) {
        match sentencia {
            Sentencia::Expresion(expr) => Self::colectar_referencias_en_expresion(expr, nombre, refs),
            Sentencia::Romper(_) | Sentencia::Continuar(_) => {},
            Sentencia::DeclaracionVariable(decl) => {
                Self::colectar_referencias_en_expresion(&decl.valor, nombre, refs);
            }
            Sentencia::Asignacion(asig) => {
                if let crate::ast::Lugar::Array(array, _) = &asig.lugar {
                    Self::colectar_referencias_en_expresion(array, nombre, refs);
                }
                Self::colectar_referencias_en_expresion(&asig.valor, nombre, refs);
            }
            Sentencia::Retornar(expr, _) => {
                if let Some(e) = expr { Self::colectar_referencias_en_expresion(e, nombre, refs); }
            }
            Sentencia::Condicional(cond) => {
                Self::colectar_referencias_en_expresion(&cond.condicion, nombre, refs);
                Self::colectar_referencias_en_bloque(&cond.bloque_entonces, nombre, refs);
                if let Some(ref sino) = cond.bloque_sino {
                    Self::colectar_referencias_en_bloque(sino, nombre, refs);
                }
            }
            Sentencia::BucleMientras(bucle) => {
                Self::colectar_referencias_en_expresion(&bucle.condicion, nombre, refs);
                Self::colectar_referencias_en_bloque(&bucle.bloque, nombre, refs);
            }
            Sentencia::BuclePara(bucle) => {
                Self::colectar_referencias_en_expresion(&bucle.iterable, nombre, refs);
                Self::colectar_referencias_en_bloque(&bucle.bloque, nombre, refs);
            }
            Sentencia::Region { nombre: _, cuerpo, span: _ } => {
                for sent in cuerpo {
                    Self::colectar_referencias_en_sentencia(sent, nombre, refs);
                }
            }
            Sentencia::Seleccionar(seleccionar) => {
                for rama in &seleccionar.ramas {
                    Self::colectar_referencias_en_expresion(&rama.canal, nombre, refs);
                    for sent in &rama.cuerpo.sentencias {
                        Self::colectar_referencias_en_sentencia(sent, nombre, refs);
                    }
                }
            }
            Sentencia::ConExecutor { hilos, cuerpo, span: _ } => {
                Self::colectar_referencias_en_expresion(hilos, nombre, refs);
                for sent in cuerpo {
                    Self::colectar_referencias_en_sentencia(sent, nombre, refs);
                }
            }
        }
    }

    fn colectar_referencias_en_expresion(expr: &Expresion, nombre: &str, refs: &mut Vec<Span>) {
        match expr {
            Expresion::Identificador(n, span) => {
                if n == nombre { refs.push(span.clone()); }
            }
            Expresion::Llamada(llamada) => {
                if llamada.funcion == nombre { refs.push(llamada.span.clone()); }
                for arg in &llamada.argumentos {
                    Self::colectar_referencias_en_expresion(arg, nombre, refs);
                }
            }
            Expresion::Binaria(izq, _, der, _) => {
                Self::colectar_referencias_en_expresion(izq, nombre, refs);
                Self::colectar_referencias_en_expresion(der, nombre, refs);
            }
            Expresion::Unaria(_, expr, _) => Self::colectar_referencias_en_expresion(expr, nombre, refs),
            Expresion::LiteralArray(elementos, _) => {
                for elem in elementos { Self::colectar_referencias_en_expresion(elem, nombre, refs); }
            }
            Expresion::ArrayRelleno(elem, _, _) => Self::colectar_referencias_en_expresion(elem, nombre, refs),
            Expresion::AccesoArray(base, indice, _) => {
                Self::colectar_referencias_en_expresion(base, nombre, refs);
                Self::colectar_referencias_en_expresion(indice, nombre, refs);
            }
            Expresion::InicializacionStruct(_, campos, _) => {
                for (_, valor) in campos { Self::colectar_referencias_en_expresion(valor, nombre, refs); }
            }
            Expresion::ConstructorEnum(_, _, args, _) => {
                for arg in args { Self::colectar_referencias_en_expresion(arg, nombre, refs); }
            }
            Expresion::AccesoCampo(base, _, _) => Self::colectar_referencias_en_expresion(base, nombre, refs),
            Expresion::EsVariante(base, _, _, _, _) => Self::colectar_referencias_en_expresion(base, nombre, refs),
            Expresion::Propagacion(expr, _) => Self::colectar_referencias_en_expresion(expr, nombre, refs),
            Expresion::Checked(inner, _) => Self::colectar_referencias_en_expresion(inner, nombre, refs),
            Expresion::Mover(nombre_var, destino, span) => {
                if nombre_var == nombre { refs.push(span.clone()); }
                if let Some(dest) = destino {
                    Self::colectar_referencias_en_expresion(dest, nombre, refs);
                }
            }
            Expresion::Copiar(expr, _) => Self::colectar_referencias_en_expresion(expr, nombre, refs),
            Expresion::Ruta(path, span) => {
                if path.iter().any(|p| p == nombre) {
                    refs.push(span.clone());
                }
            }
            Expresion::Rango(inicio, fin, _, _) => {
                Self::colectar_referencias_en_expresion(inicio, nombre, refs);
                Self::colectar_referencias_en_expresion(fin, nombre, refs);
            }
            Expresion::Closure(_, cuerpo, _) => {
                Self::colectar_referencias_en_expresion(cuerpo, nombre, refs);
            }
            Expresion::Coincidir(sujeto, brazos, _) => {
                Self::colectar_referencias_en_expresion(sujeto, nombre, refs);
                for brazo in brazos {
                    Self::colectar_referencias_en_expresion(&brazo.cuerpo, nombre, refs);
                }
            }
            Expresion::Esperar(expr, _) => Self::colectar_referencias_en_expresion(expr, nombre, refs),
            Expresion::Lanzar(expr, _) => Self::colectar_referencias_en_expresion(expr, nombre, refs),
            Expresion::Bloquear(expr, _) => Self::colectar_referencias_en_expresion(expr, nombre, refs),
            Expresion::DireccionDe(_, _) => {},  // referencia a función, no a variable
            Expresion::Bloque(bloque) => {
                for sentencia in &bloque.sentencias {
                    if let Sentencia::Expresion(expr) = sentencia {
                        Self::colectar_referencias_en_expresion(expr, nombre, refs);
                    }
                }
            }
            Expresion::Metodo(receptor, _, args, _) => {
                Self::colectar_referencias_en_expresion(receptor, nombre, refs);
                for arg in args {
                    Self::colectar_referencias_en_expresion(arg, nombre, refs);
                }
            }
            Expresion::Literal(_) => {}
        }
    }
}

// ============================================
// BACKEND LSP
// ============================================

/// Estado de un documento abierto
#[derive(Debug, Clone)]
pub struct DocumentoLsp {
    pub contenido: String,
    pub indice: IndiceSemantico,
    pub ast: Option<Programa>,
}

/// Backend del Language Server Protocol para Falcato
pub struct Backend {
    client: Client,
    documentos: Arc<RwLock<HashMap<Url, DocumentoLsp>>>,
    /// Índice global del workspace: todos los archivos .fc indexados
    indice_global: Arc<RwLock<HashMap<String, IndiceSemantico>>>,
    /// Raíz del workspace (directorio del proyecto)
    workspace_root: Arc<RwLock<Option<String>>>,
}

impl Clone for Backend {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            documentos: Arc::clone(&self.documentos),
            indice_global: Arc::clone(&self.indice_global),
            workspace_root: Arc::clone(&self.workspace_root),
        }
    }
}

impl Backend {
    pub fn nuevo(client: Client) -> Self {
        Self {
            client,
            documentos: Arc::new(RwLock::new(HashMap::new())),
            indice_global: Arc::new(RwLock::new(HashMap::new())),
            workspace_root: Arc::new(RwLock::new(None)),
        }
    }

    /// Escanea el workspace en busca de archivos .fc y construye el índice global
    async fn escanear_workspace(&self, root: &str) {
        use std::fs;
        use std::path::Path;

        let root_path = Path::new(root);
        if !root_path.exists() {
            return;
        }

        let mut archivos = Vec::new();
        self.recopilar_fc(root_path, &mut archivos, 0);

        let mut indice_global = self.indice_global.write().await;
        indice_global.clear();

        for archivo in &archivos {
            if let Ok(contenido) = fs::read_to_string(archivo) {
                let uri_path = archivo.to_string_lossy().replace('\\', "/");
                let lexer = LexerFalcato::nuevo(&contenido, &uri_path);
                let tokens = lexer.tokenizar();
                if let Ok(programa) = ParserFalcato::parse(tokens) {
                    let indice = IndiceSemantico::desde_ast(&programa);
                    indice_global.insert(uri_path, indice);
                }
            }
        }

        self.client
            .log_message(
                MessageType::INFO,
                format!("Workspace escaneado: {} archivos .fc", archivos.len()),
            )
            .await;
    }

    /// Recopila archivos .fc recursivamente (máx 3 niveles)
    fn recopilar_fc(&self, dir: &std::path::Path, archivos: &mut Vec<std::path::PathBuf>, depth: u32) {
        use std::fs;
        if depth > 3 {
            return;
        }
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let name = path.file_name().unwrap_or_default().to_string_lossy();
                    // Ignorar directorios especiales
                    if name == ".git" || name == "target" || name == "node_modules" || name == ".falcato-cache" {
                        continue;
                    }
                    self.recopilar_fc(&path, archivos, depth + 1);
                } else if path.extension().map_or(false, |e| e == "fc") {
                    archivos.push(path);
                }
            }
        }
    }

    /// Resuelve imports de un AST usando el índice global
    async fn resolver_imports(&self, ast: &Programa) -> IndiceSemantico {
        let mut indice = IndiceSemantico::desde_ast(ast);
        let global = self.indice_global.read().await;

        // Buscar declaraciones Usar en el AST
        for decl in &ast.declaraciones {
            if let Declaracion::Usar(usar) = decl {
                let es_glob = usar.ruta.last().map_or(false, |s| s == "*");

                if es_glob {
                    // `usar modulo::*` — importar todo del módulo
                    let modulo = usar.ruta[..usar.ruta.len() - 1].join("::");
                    if let Some(mod_indice) = global.get(&modulo) {
                        for (nombre, func) in &mod_indice.funciones {
                            if !indice.funciones.contains_key(nombre) {
                                indice.funciones.insert(nombre.clone(), func.clone());
                            }
                        }
                        for (nombre, s) in &mod_indice.structs {
                            if !indice.structs.contains_key(nombre) {
                                indice.structs.insert(nombre.clone(), s.clone());
                            }
                        }
                        for (nombre, e) in &mod_indice.enums {
                            if !indice.enums.contains_key(nombre) {
                                indice.enums.insert(nombre.clone(), e.clone());
                            }
                        }
                    }
                } else if let Some(simbolo) = usar.ruta.last() {
                    // `usar modulo::simbolo` — importar símbolo específico
                    let ruta_completa = usar.ruta.join("::");
                    if let Some(mod_indice) = global.get(&ruta_completa) {
                        if let Some(func) = mod_indice.funciones.get(simbolo) {
                            indice.funciones.insert(simbolo.clone(), func.clone());
                        }
                        if let Some(s) = mod_indice.structs.get(simbolo) {
                            indice.structs.insert(simbolo.clone(), s.clone());
                        }
                        if let Some(e) = mod_indice.enums.get(simbolo) {
                            indice.enums.insert(simbolo.clone(), e.clone());
                        }
                    }
                }
            }
        }

        indice
    }

    /// Busca llamadas a una función en un bloque AST
    fn buscar_llamadas_en_bloque(
        bloque: &Bloque,
        nombre_funcion: &str,
        llamadas: &mut HashMap<String, Vec<Range>>,
    ) {
        for sentencia in &bloque.sentencias {
            match sentencia {
                Sentencia::Expresion(expr) => {
                    Self::buscar_llamadas_en_expresion(expr, nombre_funcion, llamadas);
                }
                Sentencia::Condicional(cond) => {
                    Self::buscar_llamadas_en_expresion(&cond.condicion, nombre_funcion, llamadas);
                    Self::buscar_llamadas_en_bloque(&cond.bloque_entonces, nombre_funcion, llamadas);
                    if let Some(sino_bloque) = &cond.bloque_sino {
                        Self::buscar_llamadas_en_bloque(sino_bloque, nombre_funcion, llamadas);
                    }
                }
                Sentencia::BucleMientras(bucle) => {
                    Self::buscar_llamadas_en_expresion(&bucle.condicion, nombre_funcion, llamadas);
                    Self::buscar_llamadas_en_bloque(&bucle.bloque, nombre_funcion, llamadas);
                }
                Sentencia::BuclePara(para) => {
                    Self::buscar_llamadas_en_bloque(&para.bloque, nombre_funcion, llamadas);
                }
                Sentencia::Retornar(Some(expr), _) => {
                    Self::buscar_llamadas_en_expresion(expr, nombre_funcion, llamadas);
                }
                _ => {}
            }
        }
    }

    /// Busca llamadas a una función en una expresión
    fn buscar_llamadas_en_expresion(
        expr: &Expresion,
        nombre_funcion: &str,
        llamadas: &mut HashMap<String, Vec<Range>>,
    ) {
        match expr {
            Expresion::Llamada(llamada) => {
                // Si la expresión es una llamada a la función buscada
                if llamada.funcion == nombre_funcion {
                    // Encontramos una llamada
                    let range = Range {
                        start: Position { line: 0, character: 0 },
                        end: Position { line: 0, character: 0 },
                    };
                    llamadas.entry(llamada.funcion.clone()).or_insert_with(Vec::new).push(range);
                }
                // Buscar en argumentos
                for arg in &llamada.argumentos {
                    Self::buscar_llamadas_en_expresion(arg, nombre_funcion, llamadas);
                }
            }
            Expresion::Binaria(izquierda, _, derecha, _) => {
                Self::buscar_llamadas_en_expresion(izquierda, nombre_funcion, llamadas);
                Self::buscar_llamadas_en_expresion(derecha, nombre_funcion, llamadas);
            }
            Expresion::Unaria(_, operando, _) => {
                Self::buscar_llamadas_en_expresion(operando, nombre_funcion, llamadas);
            }
            _ => {}
        }
    }

    /// Formatea código Falcato: normaliza indentación, espacios, líneas vacías
    fn formatear_falcato(contenido: &str) -> String {
        let mut resultado = String::with_capacity(contenido.len());
        let mut indentacion: usize = 0;
        let mut lineas_vacias_consecutivas: u32 = 0;
        let indent_str = "    "; // 4 espacios

        for linea in contenido.lines() {
            let recortada = linea.trim();

            // Línea vacía
            if recortada.is_empty() {
                lineas_vacias_consecutivas += 1;
                if lineas_vacias_consecutivas <= 1 {
                    resultado.push('\n');
                }
                continue;
            }

            lineas_vacias_consecutivas = 0;

            // Detectar si la línea cierra un bloque (empieza con } o ])
            let cierra_bloque = recortada.starts_with('}') || recortada.starts_with(']');

            if cierra_bloque && indentacion > 0 {
                indentacion -= 1;
            }

            // Escribir indentación
            for _ in 0..indentacion {
                resultado.push_str(indent_str);
            }

            // Escribir la línea recortada
            resultado.push_str(recortada);
            resultado.push('\n');

            // Detectar si la línea abre un bloque (termina con { o ])
            let abre_bloque = recortada.ends_with('{') || recortada.ends_with('[');

            if abre_bloque {
                indentacion += 1;
            }
        }

        // Asegurar que termine con newline
        if !resultado.ends_with('\n') {
            resultado.push('\n');
        }

        resultado
    }

    /// Analiza un documento y devuelve diagnósticos + índice
    async fn analizar_documento(
        &self,
        uri: &Url,
        contenido: &str,
    ) -> (Vec<Diagnostic>, IndiceSemantico, Option<Programa>) {
        let mut diagnosticos = Vec::new();

        // 1. Lexer
        let lexer = LexerFalcato::nuevo(contenido, uri.path());
        let tokens = lexer.tokenizar();

        // 2. Parser
        let programa = match ParserFalcato::parse(tokens) {
            Ok(p) => p,
            Err(errores) => {
                for e in errores {
                    diagnosticos.push(self.error_a_diagnostico(&e.error));
                }
                return (diagnosticos, IndiceSemantico::nuevo(), None);
            }
        };

        // 3. Construir índice semántico (con imports resueltos del workspace)
        let indice = self.resolver_imports(&programa).await;

        // 4. Análisis semántico
        let mut semantica = AnalizadorSemantico::nuevo();
        if let Err(errores) = semantica.analizar(&programa) {
            for e in &errores.errores {
                diagnosticos.push(self.error_a_diagnostico(e));
            }
        }

        (diagnosticos, indice, Some(programa))
    }

    /// Convierte un ErrorCompilador a Diagnostic de LSP
    fn error_a_diagnostico(
        &self,
        error: &ErrorCompilador,
    ) -> Diagnostic {
        let severity = match error.categoria {
            crate::error::CategoriaError::Sintaxis |
            crate::error::CategoriaError::Tipo |
            crate::error::CategoriaError::Ownership => DiagnosticSeverity::ERROR,
            crate::error::CategoriaError::Warning => DiagnosticSeverity::WARNING,
            _ => DiagnosticSeverity::ERROR,
        };

        let mut message = format!("[{}] {}", error.codigo_str(), error.mensaje);
        if let Some(ref sug) = error.sugerencia {
            message.push_str(format!("\n💡 {}", sug).as_str());
        }

        Diagnostic {
            range: Range {
                start: Position {
                    line: error.span.inicio.linea.saturating_sub(1),
                    character: error.span.inicio.columna.saturating_sub(1),
                },
                end: Position {
                    line: error.span.fin.linea.saturating_sub(1),
                    character: error.span.fin.columna.saturating_sub(1),
                },
            },
            severity: Some(severity),
            code: Some(NumberOrString::String(error.codigo_str())),
            source: Some("falcato".to_string()),
            message,
            ..Default::default()
        }
    }

    /// Genera el contenido de hover para un identificador
    fn hover_para_identificador(
        &self,
        indice: &IndiceSemantico,
        nombre: &str,
    ) -> Option<Hover> {
        // Buscar como variable
        if let Some(var) = indice.variables.get(nombre) {
            let mut contenido = format!(
                "**{}** | `{}`\n\n| Propiedad | Valor |\n|-----------|-------|\n\
                 | Artículo | `{}` → {} |\n| Tipo | `{}` |\n",
                var.nombre, var.articulo,
                var.articulo_raw, self.explicar_articulo(&var.articulo_raw),
                var.tipo
            );
            // Si también es función (mismo nombre), mostrar declaración
            if let Some(func) = indice.funciones.get(nombre) {
                let params = func.parametros.join(", ");
                let ret = func.retorno.as_deref().unwrap_or("Vacío");
                contenido.push_str(&format!(
                    "\n---\n*Declaración*: `{}({}) -> {}`\n",
                    func.nombre, params, ret
                ));
            }

            return Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: contenido,
                }),
                range: None,
            });
        }

        // Buscar como función
        if let Some(func) = indice.funciones.get(nombre) {
            let params = func.parametros.join(", ");
            let ret = func.retorno.as_deref().unwrap_or("Vacío");
            let mut contenido = format!(
                "**fn** `{}({}) -> {}`\n\n| Parámetro | Tipo |\n|-----------|------|\n",
                func.nombre, params, ret
            );
            for (n, t) in &func.parametros_raw {
                contenido.push_str(&format!("| `{}` | `{}` |\n", n, t));
            }
            contenido.push_str(&format!("\n---\n*Función de Falcato*"));

            return Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: contenido,
                }),
                range: None,
            });
        }

        // Buscar como struct
        if let Some(s) = indice.structs.get(nombre) {
            let mut contenido = format!("**estructural** `{}`\n\n| Campo | Tipo |\n|-------|------|\n", s.nombre);
            for (n, t) in &s.campos {
                contenido.push_str(&format!("| `{}` | `{}` |\n", n, t));
            }
            return Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: contenido,
                }),
                range: None,
            });
        }

        // Buscar como enum
        if let Some(e) = indice.enums.get(nombre) {
            let mut contenido = format!("**enumeración** `{}`\n\n| Variante | Dato |\n|----------|------|\n", e.nombre);
            for (v, t) in &e.variantes {
                let tipo_str = t.as_deref().unwrap_or("—");
                contenido.push_str(&format!("| `{}` | `{}` |\n", v, tipo_str));
            }
            return Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: contenido,
                }),
                range: None,
            });
        }

        // Buscar como trait
        if let Some(t) = indice.traits.get(nombre) {
            let mut contenido = format!("**rasgo** `{}`\n\n| Método |\n|--------|\n", t.nombre);
            for m in &t.metodos {
                contenido.push_str(&format!("| `{}` |\n", m));
            }
            return Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: contenido,
                }),
                range: None,
            });
        }

        None
    }

    fn explicar_articulo(&self,
        articulo: &str,
    ) -> &'static str {
        match articulo {
            "el" => "dueño único, mutable",
            "la" => "prestado, solo lectura",
            "un" => "opcional (quizás existe)",
            "los" => "compartido (ref-counted), múltiples dueños",
            "las" => "compartido, solo lectura (todos leen)",
            _ => "desconocido",
        }
    }

    /// Lista de items para autocompletado
    /// Genera items de autocompletado basados en el contexto del documento
    fn items_autocompletado_contexto(
        &self,
        indice: &IndiceSemantico,
        contenido: &str,
        linea_actual: u32,
    ) -> Vec<CompletionItem> {
        let mut items = Vec::new();

        // Variables del documento
        for (nombre, var) in &indice.variables {
            items.push(CompletionItem {
                label: nombre.clone(),
                kind: Some(CompletionItemKind::VARIABLE),
                detail: Some(format!("{} {}: {}", var.articulo, nombre, var.tipo)),
                ..Default::default()
            });
        }

        // Funciones del documento
        for (nombre, func) in &indice.funciones {
            let params = func.parametros.join(", ");
            let ret = func.retorno.as_deref().unwrap_or("Vacío");
            items.push(CompletionItem {
                label: nombre.clone(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some(format!("{}({}) -> {}", nombre, params, ret)),
                ..Default::default()
            });
        }

        // Structs del documento
        for (nombre, s) in &indice.structs {
            items.push(CompletionItem {
                label: nombre.clone(),
                kind: Some(CompletionItemKind::STRUCT),
                detail: Some(format!("estructural {} ({} campos)", nombre, s.campos.len())),
                ..Default::default()
            });
        }

        // Enums del documento
        for (nombre, e) in &indice.enums {
            items.push(CompletionItem {
                label: nombre.clone(),
                kind: Some(CompletionItemKind::ENUM),
                detail: Some(format!("enumeración {} ({} variantes)", nombre, e.variantes.len())),
                ..Default::default()
            });
        }

        // Traits del documento
        for (nombre, t) in &indice.traits {
            items.push(CompletionItem {
                label: nombre.clone(),
                kind: Some(CompletionItemKind::INTERFACE),
                detail: Some(format!("rasgo {} ({} métodos)", nombre, t.metodos.len())),
                ..Default::default()
            });
        }

        // Si estamos después de un `.` (acceso a campo/método), completar campos de struct
        let line_prefix: String = contenido.lines()
            .nth(linea_actual as usize)
            .and_then(|l| {
                let before_cursor = if (l.len() as u32) < 50 { l } else { &l[..50.min(l.len())] };
                let dot_pos = before_cursor.rfind('.');
                dot_pos.map(|p| before_cursor[..p].trim().to_string())
            })
            .unwrap_or_default();

        if line_prefix.ends_with('.') {
            let type_name = line_prefix.trim_end_matches('.');
            // Buscar struct con ese nombre
            if let Some(s) = indice.structs.get(type_name) {
                for (campo, tipo) in &s.campos {
                    items.push(CompletionItem {
                        label: campo.clone(),
                        kind: Some(CompletionItemKind::FIELD),
                        detail: Some(format!("{}: {}", campo, tipo)),
                        ..Default::default()
                    });
                }
            }
            // Métodos de Texto después de .
            if type_name == "t" || type_name.contains("texto") || type_name.contains("Texto") {
                let text_methods = vec![
                    ("longitud", "() -> Entero32", "Cantidad de bytes"),
                    ("esta_vacio", "() -> Booleano", "¿Longitud == 0?"),
                    ("contiene", "(sub: Texto) -> Booleano", "Búsqueda subcadena"),
                    ("empieza_con", "(prefijo: Texto) -> Booleano", "Prefix check"),
                    ("termina_con", "(sufijo: Texto) -> Booleano", "Suffix check"),
                    ("reemplazar", "(de: Texto, a: Texto) -> Texto", "Reemplazo global"),
                    ("recortar", "() -> Texto", "Trim espacios"),
                    ("mayusculas", "() -> Texto", "Uppercase"),
                    ("minusculas", "() -> Texto", "Lowercase"),
                    ("dividir", "(sep: Texto) -> Vector<Texto>", "Split"),
                    ("subtexto", "(i: Entero32, f: Entero32) -> Texto", "Substring"),
                    ("a_entero", "() -> Entero64", "Parsing entero"),
                    ("a_flotante", "() -> Flotante64", "Parsing flotante"),
                    ("a_bytes", "() -> Vector<Entero8>", "A bytes"),
                    ("codificar_base64", "() -> Texto", "Encode Base64"),
                    ("decodificar_base64", "() -> Texto", "Decode Base64"),
                    ("concatenar", "(b: Texto) -> Texto", "Unir textos"),
                ];
                for (name, sig, doc) in text_methods {
                    items.push(CompletionItem {
                        label: name.to_string(),
                        kind: Some(CompletionItemKind::METHOD),
                        detail: Some(format!("{}{}", name, sig)),
                        documentation: Some(Documentation::String(doc.to_string())),
                        ..Default::default()
                    });
                }
            }
            // Métodos de Vector después de .
            if type_name.contains("vector") || type_name.contains("Vector") || type_name.contains("v") {
                let vec_methods = vec![
                    ("longitud", "() -> Entero32", "Cantidad de elementos"),
                    ("obtener", "(i: Entero32) -> Option<T>", "Acceso por índice"),
                    ("contiene", "(item: T) -> Booleano", "Búsqueda lineal"),
                    ("indice_de", "(item: T) -> Entero32", "Índice del item"),
                    ("clonar", "() -> Vector<T>", "Deep copy"),
                    ("invertir", "() -> Vacío", "Invertir orden"),
                    ("limpiar", "() -> Vacío", "Vaciar"),
                    ("insertar", "(i: Entero32, item: T) -> Vacío", "Insertar en posición"),
                    ("eliminar", "(i: Entero32) -> Vacío", "Eliminar en posición"),
                ];
                for (name, sig, doc) in vec_methods {
                    items.push(CompletionItem {
                        label: name.to_string(),
                        kind: Some(CompletionItemKind::METHOD),
                        detail: Some(format!("{}{}", name, sig)),
                        documentation: Some(Documentation::String(doc.to_string())),
                        ..Default::default()
                    });
                }
            }
            // Métodos de Diccionario después de .
            if type_name.contains("diccionario") || type_name.contains("Diccionario") || type_name.contains("d") {
                let dict_methods = vec![
                    ("obtener", "(k: K) -> Option<V>", "Buscar por clave"),
                    ("existe", "(k: K) -> Booleano", "Verificar clave"),
                    ("longitud", "() -> Entero32", "Cantidad de pares"),
                    ("claves", "() -> Vector<Texto>", "Extraer claves"),
                    ("valores", "() -> Vector<Texto>", "Extraer valores"),
                ];
                for (name, sig, doc) in dict_methods {
                    items.push(CompletionItem {
                        label: name.to_string(),
                        kind: Some(CompletionItemKind::METHOD),
                        detail: Some(format!("{}{}", name, sig)),
                        documentation: Some(Documentation::String(doc.to_string())),
                        ..Default::default()
                    });
                }
            }
            // Métodos de Resultado después de .
            if type_name.contains("resultado") || type_name.contains("Resultado") || type_name.contains("r") {
                let result_methods = vec![
                    ("es_exito", "() -> Booleano", "¿Es Exito?"),
                    ("es_error", "() -> Booleano", "¿Es Error?"),
                ];
                for (name, sig, doc) in result_methods {
                    items.push(CompletionItem {
                        label: name.to_string(),
                        kind: Some(CompletionItemKind::METHOD),
                        detail: Some(format!("{}{}", name, sig)),
                        documentation: Some(Documentation::String(doc.to_string())),
                        ..Default::default()
                    });
                }
            }
            // Variantes de enum después de .
            if let Some(e) = indice.enums.get(type_name) {
                for (variante, tipo_dato) in &e.variantes {
                    let detail = match tipo_dato {
                        Some(t) => format!("{}({})", variante, t),
                        None => variante.clone(),
                    };
                    items.push(CompletionItem {
                        label: variante.clone(),
                        kind: Some(CompletionItemKind::ENUM_MEMBER),
                        detail: Some(detail),
                        ..Default::default()
                    });
                }
            }
        }

        items
    }

    /// Convierte un Span de Falcato a Range de LSP
    fn span_a_rango(&self, span: &Span) -> Range {
        Range {
            start: Position {
                line: span.inicio.linea.saturating_sub(1),
                character: span.inicio.columna.saturating_sub(1),
            },
            end: Position {
                line: span.fin.linea.saturating_sub(1),
                character: span.fin.columna.saturating_sub(1),
            },
        }
    }

    /// Verifica si un Range de diagnóstico se solapa con otro Range
    fn span_en_rango(&self, diag_range: Range, request_range: &Range) -> bool {
        diag_range.start.line >= request_range.start.line
            && diag_range.start.line <= request_range.end.line
    }

    fn items_autocompletado() -> Vec<CompletionItem> {
        let mut items = Vec::new();

        // Keywords — todas las palabras reservadas del lenguaje
        let keywords = vec![
            ("función", "Declara una función", CompletionItemKind::KEYWORD),
            ("fn", "Alias de función", CompletionItemKind::KEYWORD),
            ("retornar", "Retorna un valor", CompletionItemKind::KEYWORD),
            ("devolver", "Alias de retornar", CompletionItemKind::KEYWORD),
            ("si", "Condicional si", CompletionItemKind::KEYWORD),
            ("sino", "Rama alternativa", CompletionItemKind::KEYWORD),
            ("es", "Comparación de identidad (==)", CompletionItemKind::KEYWORD),
            ("está", "Comparación de estado / truthiness", CompletionItemKind::KEYWORD),
            ("fuese", "Subjuntivo — cold path optimization", CompletionItemKind::KEYWORD),
            ("mientras", "Bucle mientras (condición)", CompletionItemKind::KEYWORD),
            ("para", "Bucle para (iteración)", CompletionItemKind::KEYWORD),
            ("en", "Separador para/bucle", CompletionItemKind::KEYWORD),
            ("coincidir", "Pattern matching exhaustivo", CompletionItemKind::KEYWORD),
            ("emparejar", "Alias de coincidir", CompletionItemKind::KEYWORD),
            ("inseguro", "Bloque o función FFI insegura", CompletionItemKind::KEYWORD),
            ("estructural", "Define un struct (layout C)", CompletionItemKind::KEYWORD),
            ("enumeración", "Define un enum (tag+union)", CompletionItemKind::KEYWORD),
            ("rasgo", "Define un trait/interface", CompletionItemKind::KEYWORD),
            ("implementar", "Implementa un trait para un tipo", CompletionItemKind::KEYWORD),
            ("módulo", "Define un módulo", CompletionItemKind::KEYWORD),
            ("usar", "Importa un símbolo de otro módulo", CompletionItemKind::KEYWORD),
            ("mover", "Transfiere ownership explícitamente", CompletionItemKind::KEYWORD),
            ("copiar", "Clona un valor explícitamente", CompletionItemKind::KEYWORD),
            ("prestar", "Presta una referencia explícitamente", CompletionItemKind::KEYWORD),
            ("región", "Bloque de arena allocation", CompletionItemKind::KEYWORD),
            ("puro", "Anotación de efecto: sin side effects", CompletionItemKind::KEYWORD),
            ("muta", "Anotación de efecto: muta campo(s)", CompletionItemKind::KEYWORD),
            ("lee", "Anotación de efecto: lee campo(s)", CompletionItemKind::KEYWORD),
            ("vectorizable", "Modificador de función: auto-vectorizable por el compiler", CompletionItemKind::KEYWORD),
            ("fut", "Función asíncrona (futuro)", CompletionItemKind::KEYWORD),
            ("esperar", "Espera un futuro (await)", CompletionItemKind::KEYWORD),
            ("lanzar", "Lanza un hilo/tarea", CompletionItemKind::KEYWORD),
            ("bloquear", "Bridge sync→async", CompletionItemKind::KEYWORD),
            ("seleccionar", "Select de canales", CompletionItemKind::KEYWORD),
            ("con_executor", "Crea un thread pool", CompletionItemKind::KEYWORD),
            ("cancelar", "Cancelación estructurada", CompletionItemKind::KEYWORD),
            ("prueba", "Define un test", CompletionItemKind::KEYWORD),
            ("afirmar", "Aserción en tests", CompletionItemKind::KEYWORD),
            ("como", "Binding en pattern matching", CompletionItemKind::KEYWORD),
            ("bits", "Campos de bits en struct", CompletionItemKind::KEYWORD),
            ("todos", "Inicialización de arreglo con valor", CompletionItemKind::KEYWORD),
            ("direccion_de", "Obtiene la dirección de una función", CompletionItemKind::KEYWORD),
            ("dir_de", "Obtiene la dirección de una función (abreviatura)", CompletionItemKind::KEYWORD),
            
        ];

        for (kw, doc, kind) in keywords {
            items.push(CompletionItem {
                label: kw.to_string(),
                kind: Some(kind),
                detail: Some(doc.to_string()),
                insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                ..Default::default()
            });
        }

        // Artículos (ownership — 5 tipos)
        let articulos = vec![
            ("el", "Owned mutable (dueño único, puedes modificar)", CompletionItemKind::KEYWORD),
            ("la", "Borrowed inmutable (prestado, solo lectura)", CompletionItemKind::KEYWORD),
            ("un", "Optional (quizás existe, quizás no)", CompletionItemKind::KEYWORD),
            ("los", "Shared owned (ref-counted, múltiples dueños)", CompletionItemKind::KEYWORD),
            ("las", "Shared borrowed (todos leen, nadie escribe)", CompletionItemKind::KEYWORD),
        ];

        for (art, doc, kind) in articulos {
            items.push(CompletionItem {
                label: art.to_string(),
                kind: Some(kind),
                detail: Some(doc.to_string()),
                ..Default::default()
            });
        }

        // Tipos — primitivos + compuestos + visuales
        let tipos = vec![
            // Numéricos
            ("Entero8", "Entero de 8 bits con signo (-128..127)"),
            ("Entero16", "Entero de 16 bits con signo"),
            ("Entero32", "Entero de 32 bits con signo"),
            ("Entero64", "Entero de 64 bits con signo"),
            ("Natural8", "Entero de 8 bits sin signo (0..255)"),
            ("Natural16", "Entero de 16 bits sin signo"),
            ("Natural32", "Entero de 32 bits sin signo"),
            ("Natural64", "Entero de 64 bits sin signo"),
            ("Flotante32", "Flotante de 32 bits (f32)"),
            ("Flotante64", "Flotante de 64 bits (f64)"),
            ("Real", "Flotante64 — alias"),
            // Booleano y caracteres
            ("Booleano", "Booleano: verdadero o falso"),
            ("Caracter", "Carácter Unicode de 32 bits"),
            // Strings
            ("Palabra", "String literal inmutable (&str)"),
            ("Texto", "String heap-allocado growable (24 bytes, ¡liberar!)"),
            // Colecciones
            ("Vector", "Vector dinámico genérico (heap, ¡liberar!)"),
            ("Diccionario", "Mapa hash genérico<K,V> (heap, ¡liberar!)"),
            ("Conjunto", "Set hash genérico<T> (heap, ¡liberar!)"),
            // Option/Result
            ("Option", "Option<T> — Algo(valor) o Nada"),
            ("Resultado", "Result<T,E> — Exito(valor) o Error(codigo)"),
            // Unit
            ("Vacío", "Tipo unitario (sin valor)"),
            // Visual
            ("Ventana", "Ventana del sistema operativo"),
            ("Lienzo", "Canvas de dibujo (GDI/bitmap)"),
            ("Imagen", "Imagen rasterizada"),
            ("Audio", "Buffer de audio (samples)"),
            ("Punto", "Punto 2D: { x: Entero32, y: Entero32 }"),
            ("Tamano", "Tamaño 2D: { w: Entero32, h: Entero32 }"),
            ("Rect", "Rectángulo: { x, y, w, h }"),
            // Red
            ("Canal", "Canal MPSC para concurrencia"),
            // Math aliases
            ("Fase", "Estado de oscilador: { valor: Real, inc: Real }"),
            ("Real_preciso", "apodo de Real — libm (error < 1 ULP)"),
            ("Real_rapido", "apodo de Real — polinomio grado 5"),
            ("Real_aprox", "apodo de Real — tabla 256 + lerp"),
        ];

        for (t, doc) in tipos {
            items.push(CompletionItem {
                label: t.to_string(),
                kind: Some(CompletionItemKind::TYPE_PARAMETER),
                detail: Some(doc.to_string()),
                ..Default::default()
            });
        }

        // Literales booleanos
        for b in ["verdadero", "falso"] {
            items.push(CompletionItem {
                label: b.to_string(),
                kind: Some(CompletionItemKind::CONSTANT),
                detail: Some("Literal booleano".to_string()),
                ..Default::default()
            });
        }

        // Snippets de template — boilerplate del lenguaje
        let snippets = vec![
            ("si", "si ${1:condicion} {\n\t$0\n}", "Condicional si"),
            ("sino", "si ${1:condicion} {\n\t$2\n} sino {\n\t$0\n}", "Si sino"),
            ("si-es", "si ${1:variable} es ${2:Variante} {\n\t$0\n}", "Si es (pattern match)"),
            ("mientras", "mientras ${1:condicion} {\n\t$0\n}", "Bucle mientras"),
            ("para-en", "para ${1:item} en ${2:coleccion} {\n\t$0\n}", "Bucle para-in"),
            ("para-rango", "para ${1:i} en 0..${2:10} {\n\t$0\n}", "Bucle para-rango"),
            ("fn", "función ${1:nombre}(${2:la parametro: Tipo}) -> ${3:Tipo} {\n\t$0\n}", "Declarar función"),
            ("fn-sin-retorno", "función ${1:nombre}(${2:la parametro: Tipo}) {\n\t$0\n}", "Función sin retorno"),
            ("estructural", "estructural ${1:Nombre} {\n\t${2:campo}: ${3:Tipo},\n}", "Declarar struct"),
            ("enumeracion", "enumeración ${1:Nombre} {\n\t${2:Variante},\n}", "Declarar enum"),
            ("rasgo", "rasgo ${1:Nombre} {\n\t${2:método}(): ${3:Tipo};\n}", "Declarar trait"),
            ("implementar", "implementar ${1:Trait} para ${2:Tipo} {\n\t$0\n}", "Implementar trait"),
            ("resultado", "Resultado<${1:TipoExito}, ${2:TipoError}>", "Tipo Resultado"),
            ("option", "Option<${1:Tipo}>", "Tipo Option"),
            ("vector", "Vector<${1:Tipo}>", "Tipo Vector"),
            ("diccionario", "Diccionario<${1:Clave}, ${2:Valor}>", "Tipo Diccionario"),
            ("fut", "fut función ${1:nombre}(${2:la parametro: Tipo}) -> ${3:Tipo} {\n\t$0\n}", "Función async"),
            ("lanzar", "lanzar ${1:funcion}(${2:args})", "Lanzar tarea"),
            ("usar", "usar ${1:libEst}::${2:modulos}::*;", "Importar módulo"),
            ("prueba", "prueba \"${1:nombre}\" {\n\t$0\n}", "Declarar test"),
            ("afirmar", "afirmar(${1:condicion});", "Aserción"),
            ("coincidir", "coincidir ${1:variable} {\n\t${2:Patron} => ${3:resultado},\n\t_ => ${4:default},\n}", "Pattern matching"),
        ];
        for (trigger, template, doc) in snippets {
            items.push(CompletionItem {
                label: trigger.to_string(),
                kind: Some(CompletionItemKind::SNIPPET),
                detail: Some(doc.to_string()),
                insert_text: Some(template.to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            });
        }

        // Built-in functions — 153 builtins completos
        let builtins = vec![
            // === I/O básico ===
            ("imprimir", "(mensaje: T) -> Vacío", CompletionItemKind::FUNCTION),
            ("imprimir_linea", "(mensaje: T) -> Vacío", CompletionItemKind::FUNCTION),
            ("decir", "(mensaje: T) -> Vacío — alias de imprimir_linea", CompletionItemKind::FUNCTION),
            ("entrada_leer", "() -> Texto — lee stdin completo", CompletionItemKind::FUNCTION),
            // === Sistema ===
            ("tamaño_de::<T>", "() -> Entero64 — sizeof comptime", CompletionItemKind::FUNCTION),
            ("dormir", "(ms: Entero32) -> Vacío — suspende hilo actual", CompletionItemKind::FUNCTION),
            ("directorio_actual", "() -> Texto — CWD", CompletionItemKind::FUNCTION),
            ("terminal_dimensiones", "() -> Entero64 — ancho/alto terminal", CompletionItemKind::FUNCTION),
            ("terminal_modo_raw", "(habilitar: Booleano) -> Vacío", CompletionItemKind::FUNCTION),
            ("terminal_leer_tecla", "() -> Entero32 — leer tecla sin echo", CompletionItemKind::FUNCTION),
            ("entorno_obtener", "(nombre: Texto) -> Texto — variable de entorno", CompletionItemKind::FUNCTION),
            ("argumentos_cantidad", "() -> Entero32 — argc", CompletionItemKind::FUNCTION),
            ("argumentos_obtener", "(i: Entero32) -> Texto — argv[i]", CompletionItemKind::FUNCTION),
            ("consola_imprimir", "(texto: Texto) -> Vacío — stdout sin newline", CompletionItemKind::FUNCTION),
            ("consola_imprimir_linea", "(texto: Texto) -> Vacío — stdout con newline", CompletionItemKind::FUNCTION),
            ("aleatorio_entero", "() -> Entero64 — número aleatorio", CompletionItemKind::FUNCTION),
            ("aleatorio_entero_entre", "(min: Entero64, max: Entero64) -> Entero64 — aleatorio en rango", CompletionItemKind::FUNCTION),
            ("aleatorio", "() -> Entero64 — alias de aleatorio_entero", CompletionItemKind::FUNCTION),
            ("timestamp", "() -> Entero64 — timestamp Unix", CompletionItemKind::FUNCTION),
            // === Texto (22) ===
            ("texto_nuevo", "() -> Texto", CompletionItemKind::FUNCTION),
            ("texto_desde", "(palabra: Palabra) -> Texto — literal → heap", CompletionItemKind::FUNCTION),
            ("texto_longitud", "(t: Texto) -> Entero32 — bytes", CompletionItemKind::FUNCTION),
            ("texto_esta_vacio", "(t: Texto) -> Booleano", CompletionItemKind::FUNCTION),
            ("texto_contiene", "(t: Texto, sub: Texto) -> Booleano — búsqueda subcadena", CompletionItemKind::FUNCTION),
            ("texto_empieza_con", "(t: Texto, prefijo: Texto) -> Booleano", CompletionItemKind::FUNCTION),
            ("texto_termina_con", "(t: Texto, sufijo: Texto) -> Booleano", CompletionItemKind::FUNCTION),
            ("texto_comparar", "(a: Texto, b: Texto) -> Entero32 — memcmp (-/0/+)", CompletionItemKind::FUNCTION),
            ("texto_es_igual", "(a: Texto, b: Texto) -> Booleano", CompletionItemKind::FUNCTION),
            ("texto_es_diferente", "(a: Texto, b: Texto) -> Booleano", CompletionItemKind::FUNCTION),
            ("texto_concatenar", "(a: Texto, b: Texto) -> Texto", CompletionItemKind::FUNCTION),
            ("texto_reemplazar", "(t: Texto, de: Texto, a: Texto) -> Texto — reemplazo global", CompletionItemKind::FUNCTION),
            ("texto_recortar", "(t: Texto) -> Texto — trim espacios", CompletionItemKind::FUNCTION),
            ("texto_mayusculas", "(t: Texto) -> Texto — uppercase ASCII", CompletionItemKind::FUNCTION),
            ("texto_minusculas", "(t: Texto) -> Texto — lowercase ASCII", CompletionItemKind::FUNCTION),
            ("texto_dividir", "(t: Texto, sep: Texto) -> Vector<Texto> — split", CompletionItemKind::FUNCTION),
            ("texto_subtexto", "(t: Texto, i: Entero32, f: Entero32) -> Texto — substring [i,f)", CompletionItemKind::FUNCTION),
            ("texto_a_entero", "(t: Texto) -> Entero64 — parsing entero", CompletionItemKind::FUNCTION),
            ("texto_a_natural", "(t: Texto) -> Natural64 — parsing natural", CompletionItemKind::FUNCTION),
            ("texto_a_flotante", "(t: Texto) -> Flotante64 — parsing flotante", CompletionItemKind::FUNCTION),
            ("texto_a_booleano", "(t: Texto) -> Booleano — parsing booleano", CompletionItemKind::FUNCTION),
            ("texto_a_bytes", "(t: Texto) -> Vector<Entero8> — conversión a bytes", CompletionItemKind::FUNCTION),
            ("texto_codificar_base64", "(t: Texto) -> Texto — encode Base64", CompletionItemKind::FUNCTION),
            ("texto_decodificar_base64", "(t: Texto) -> Texto — decode Base64", CompletionItemKind::FUNCTION),
            ("texto_agregar", "(texto: Texto, fragmento: Palabra) -> Vacío — append literal", CompletionItemKind::FUNCTION),
            ("texto_liberar", "(texto: Texto) -> Vacío", CompletionItemKind::FUNCTION),
            // === Matemáticas (14) ===
            ("abs", "(x: Entero32) -> Entero32 — valor absoluto", CompletionItemKind::FUNCTION),
            ("max", "(a: Entero32, b: Entero32) -> Entero32 — máximo", CompletionItemKind::FUNCTION),
            ("min", "(a: Entero32, b: Entero32) -> Entero32 — mínimo", CompletionItemKind::FUNCTION),
            ("raiz", "(x: Flotante64) -> Flotante64 — sqrt()", CompletionItemKind::FUNCTION),
            ("potencia", "(base: Flotante64, exp: Flotante64) -> Flotante64 — pow()", CompletionItemKind::FUNCTION),
            ("mate_abs", "(x: Flotante64) -> Flotante64 — abs flotante", CompletionItemKind::FUNCTION),
            ("mate_maximo", "(a: Flotante64, b: Flotante64) -> Flotante64 — máximo", CompletionItemKind::FUNCTION),
            ("mate_minimo", "(a: Flotante64, b: Flotante64) -> Flotante64 — mínimo", CompletionItemKind::FUNCTION),
            ("mate_raiz", "(x: Flotante64) -> Flotante64 — raíz cuadrada", CompletionItemKind::FUNCTION),
            ("mate_potencia", "(base: Flotante64, exp: Flotante64) -> Flotante64 — potencia", CompletionItemKind::FUNCTION),
            ("mate_piso", "(x: Flotante64) -> Entero64 — floor", CompletionItemKind::FUNCTION),
            ("mate_techo", "(x: Flotante64) -> Entero64 — ceil", CompletionItemKind::FUNCTION),
            ("mate_pi", "() -> Flotante64 — π", CompletionItemKind::FUNCTION),
            ("mate_e", "() -> Flotante64 — e", CompletionItemKind::FUNCTION),
            ("mate_grados_a_radianes", "(g: Flotante64) -> Flotante64", CompletionItemKind::FUNCTION),
            ("mate_radianes_a_grados", "(r: Flotante64) -> Flotante64", CompletionItemKind::FUNCTION),
            // === Trig (libm) ===
            ("seno", "(x: Real) -> Real — seno via libm", CompletionItemKind::FUNCTION),
            ("coseno", "(x: Real) -> Real — coseno via libm", CompletionItemKind::FUNCTION),
            ("tangente", "(x: Real) -> Real — tangente via libm", CompletionItemKind::FUNCTION),
            ("arcseno", "(x: Real) -> Real — arco seno via libm", CompletionItemKind::FUNCTION),
            ("arccoseno", "(x: Real) -> Real — arco coseno via libm", CompletionItemKind::FUNCTION),
            ("arctangente", "(x: Real) -> Real — arco tangente via libm", CompletionItemKind::FUNCTION),
            ("arctangente2", "(y: Real, x: Real) -> Real — arco tangente2", CompletionItemKind::FUNCTION),
            ("senoh", "(x: Real) -> Real — seno hiperbólico", CompletionItemKind::FUNCTION),
            ("cosenoh", "(x: Real) -> Real — coseno hiperbólico", CompletionItemKind::FUNCTION),
            ("tangenteh", "(x: Real) -> Real — tangente hiperbólica", CompletionItemKind::FUNCTION),
            ("exp", "(x: Real) -> Real — exponencial", CompletionItemKind::FUNCTION),
            ("log", "(x: Real) -> Real — logaritmo natural", CompletionItemKind::FUNCTION),
            ("log10", "(x: Real) -> Real — logaritmo base 10", CompletionItemKind::FUNCTION),
            ("piso", "(x: Real) -> Real — floor", CompletionItemKind::FUNCTION),
            ("techo", "(x: Real) -> Real — ceil", CompletionItemKind::FUNCTION),
            ("fabs", "(x: Real) -> Real — valor absoluto flotante", CompletionItemKind::FUNCTION),
            ("fmod", "(a: Real, b: Real) -> Real — módulo flotante", CompletionItemKind::FUNCTION),
            ("seno_preciso", "(x: Real) -> Real — seno preciso (libm)", CompletionItemKind::FUNCTION),
            ("coseno_preciso", "(x: Real) -> Real — coseno preciso (libm)", CompletionItemKind::FUNCTION),
            ("tangente_preciso", "(x: Real) -> Real — tangente precisa (libm)", CompletionItemKind::FUNCTION),
            ("exp_preciso", "(x: Real) -> Real — exp preciso (libm)", CompletionItemKind::FUNCTION),
            ("log_preciso", "(x: Real) -> Real — log preciso (libm)", CompletionItemKind::FUNCTION),
            ("seno_rapido", "(x: Real) -> Real — seno rápido (polinomio)", CompletionItemKind::FUNCTION),
            ("coseno_rapido", "(x: Real) -> Real — coseno rápido (polinomio)", CompletionItemKind::FUNCTION),
            ("seno_2pi", "(fase: Real) -> Real — seno(2π×fase), fase ∈ [0,1)", CompletionItemKind::FUNCTION),
            ("coseno_2pi", "(fase: Real) -> Real — coseno(2π×fase), fase ∈ [0,1)", CompletionItemKind::FUNCTION),
            ("exp_rapido", "(x: Real) -> Real — exp rápido (libm)", CompletionItemKind::FUNCTION),
            ("log_rapido", "(x: Real) -> Real — log rápido (libm)", CompletionItemKind::FUNCTION),
            ("seno_aprox", "(x: Real) -> Real — seno aproximado (placeholder)", CompletionItemKind::FUNCTION),
            // === Archivo (8) ===
            ("archivo_leer", "(ruta: Texto) -> Texto — leer contenido completo", CompletionItemKind::FUNCTION),
            ("archivo_escribir", "(ruta: Texto, contenido: Texto) -> Entero32 — escribir (sobrescribe)", CompletionItemKind::FUNCTION),
            ("archivo_agregar", "(ruta: Texto, contenido: Texto) -> Entero32 — append", CompletionItemKind::FUNCTION),
            ("archivo_existe", "(ruta: Texto) -> Booleano — verificar existencia", CompletionItemKind::FUNCTION),
            ("archivo_borrar", "(ruta: Texto) -> Entero32 — eliminar archivo", CompletionItemKind::FUNCTION),
            ("archivo_renombrar", "(de: Texto, a: Texto) -> Entero32 — mover/renombrar", CompletionItemKind::FUNCTION),
            ("archivo_listar", "(ruta: Texto) -> Vector<Texto> — listar directorio", CompletionItemKind::FUNCTION),
            ("archivo_tamano", "(ruta: Texto) -> Entero64 — tamaño en bytes", CompletionItemKind::FUNCTION),
            // === TCP (10) ===
            ("tcp_conectar", "(host: Texto, puerto: Entero32) -> Entero32 — conexión TCP", CompletionItemKind::FUNCTION),
            ("tcp_enviar", "(conn: Entero32, datos: Texto) -> Entero32 — enviar datos", CompletionItemKind::FUNCTION),
            ("tcp_recibir", "(conn: Entero32, tamano: Entero32) -> Texto — recibir datos", CompletionItemKind::FUNCTION),
            ("tcp_cerrar", "(conn: Entero32) -> Vacío — cerrar conexión", CompletionItemKind::FUNCTION),
            ("tcp_establecer_timeout", "(conn: Entero32, ms: Entero32) -> Vacío — timeout", CompletionItemKind::FUNCTION),
            ("tcp_datos_disponibles", "(conn: Entero32) -> Entero32 — bytes pendientes", CompletionItemKind::FUNCTION),
            ("dns_resolver", "(host: Texto) -> Texto — DNS lookup", CompletionItemKind::FUNCTION),
            ("tcp_vincular", "(host: Texto, puerto: Entero32) -> Entero32 — bind", CompletionItemKind::FUNCTION),
            ("tcp_escuchar", "(fd: Entero32, backlog: Entero32) -> Entero32 — listen", CompletionItemKind::FUNCTION),
            ("tcp_aceptar", "(fd: Entero32) -> Entero32 — accept", CompletionItemKind::FUNCTION),
            // === HTTP (2) ===
            ("http_get", "(host: Texto, puerto: Entero32, path: Texto) -> Texto — HTTP GET", CompletionItemKind::FUNCTION),
            ("http_post", "(host: Texto, puerto: Entero32, path: Texto, cuerpo: Texto) -> Texto — HTTP POST", CompletionItemKind::FUNCTION),
            // === TLS (5) ===
            ("tls_conectar", "(host: Texto, puerto: Entero32) -> Entero32 — conexión TLS", CompletionItemKind::FUNCTION),
            ("tls_escribir", "(conn: Entero32, datos: Texto, len: Entero32) -> Entero32 — escribir TLS", CompletionItemKind::FUNCTION),
            ("tls_leer", "(conn: Entero32, buf: Texto, len: Entero32) -> Entero32 — leer TLS", CompletionItemKind::FUNCTION),
            ("tls_datos_disponibles", "(conn: Entero32) -> Entero32 — bytes pendientes TLS", CompletionItemKind::FUNCTION),
            ("tls_cerrar", "(conn: Entero32) -> Vacío — cerrar TLS", CompletionItemKind::FUNCTION),
            // === JSON (4) ===
            ("json_parsear", "(json: Texto) -> Texto — parser JSON recursivo", CompletionItemKind::FUNCTION),
            ("json_serializar", "(valor: Texto) -> Texto — serializar a JSON", CompletionItemKind::FUNCTION),
            ("json_escapar", "(texto: Texto) -> Texto — escape de strings", CompletionItemKind::FUNCTION),
            ("json_obtener", "(json: Texto, clave: Texto) -> Texto — extraer campo", CompletionItemKind::FUNCTION),
            // === Tiempo (5) ===
            ("fecha_unix", "() -> Entero64 — timestamp Unix actual", CompletionItemKind::FUNCTION),
            ("fecha_ms", "() -> Entero64 — milisegundos actuales", CompletionItemKind::FUNCTION),
            ("fecha_anio", "(unix: Entero64) -> Entero32 — año", CompletionItemKind::FUNCTION),
            ("fecha_mes", "(unix: Entero64) -> Entero32 — mes (1-12)", CompletionItemKind::FUNCTION),
            ("fecha_dia", "(unix: Entero64) -> Entero32 — día (1-31)", CompletionItemKind::FUNCTION),
            // === Vector (15) ===
            ("vector_nuevo", "<T>() -> Vector<T> — vector vacío", CompletionItemKind::FUNCTION),
            ("vector_agregar", "<T>(v: &mut Vector<T>, item: T) -> Vacío — agregar elemento", CompletionItemKind::FUNCTION),
            ("vector_obtener", "<T>(v: Vector<T>, i: Entero32) -> Option<T> — acceso por índice", CompletionItemKind::FUNCTION),
            ("vector_poner", "<T>(v: &mut Vector<T>, i: Entero32, item: T) -> Vacío — asignar por índice", CompletionItemKind::FUNCTION),
            ("vector_longitud", "<T>(v: Vector<T>) -> Entero32 — cantidad de elementos", CompletionItemKind::FUNCTION),
            ("vector_liberar", "<T>(v: Vector<T>) -> Vacío — liberar memoria", CompletionItemKind::FUNCTION),
            ("vector_intercambiar", "<T>(v: &mut Vector<T>, i: Entero32, j: Entero32) -> Vacío — swap", CompletionItemKind::FUNCTION),
            ("vector_insertar", "<T>(v: &mut Vector<T>, i: Entero32, item: T) -> Vacío — insertar en posición", CompletionItemKind::FUNCTION),
            ("vector_eliminar", "<T>(v: &mut Vector<T>, i: Entero32) -> Vacío — eliminar en posición", CompletionItemKind::FUNCTION),
            ("vector_extender", "<T>(v: &mut Vector<T>, otro: Vector<T>) -> Vacío — extender", CompletionItemKind::FUNCTION),
            ("vector_contiene", "<T>(v: Vector<T>, item: T) -> Booleano — búsqueda lineal", CompletionItemKind::FUNCTION),
            ("vector_indice_de", "<T>(v: Vector<T>, item: T) -> Entero32 — índice del item (-1 si no)", CompletionItemKind::FUNCTION),
            ("vector_clonar", "<T>(v: Vector<T>) -> Vector<T> — deep copy", CompletionItemKind::FUNCTION),
            ("vector_invertir", "<T>(v: &mut Vector<T>) -> Vacío — invertir orden", CompletionItemKind::FUNCTION),
            ("vector_limpiar", "<T>(v: &mut Vector<T>) -> Vacío — vaciar sin deallocar", CompletionItemKind::FUNCTION),
            // === Diccionario (10) ===
            ("diccionario_nuevo", "<K,V>() -> Diccionario<K,V> — diccionario vacío", CompletionItemKind::FUNCTION),
            ("diccionario_insertar", "<K,V>(d: &mut Diccionario<K,V>, k: K, v: V) -> Vacío — insertar/actualizar", CompletionItemKind::FUNCTION),
            ("diccionario_obtener", "<K,V>(d: Diccionario<K,V>, k: K) -> Option<V> — buscar por clave", CompletionItemKind::FUNCTION),
            ("diccionario_existe", "<K,V>(d: Diccionario<K,V>, k: K) -> Booleano — verificar clave", CompletionItemKind::FUNCTION),
            ("diccionario_eliminar", "<K,V>(d: &mut Diccionario<K,V>, k: K) -> Vacío — eliminar clave", CompletionItemKind::FUNCTION),
            ("diccionario_longitud", "<K,V>(d: Diccionario<K,V>) -> Entero32 — cantidad de pares", CompletionItemKind::FUNCTION),
            ("diccionario_liberar", "<K,V>(d: Diccionario<K,V>) -> Vacío — liberar memoria", CompletionItemKind::FUNCTION),
            ("diccionario_claves", "<K,V>(d: Diccionario<K,V>) -> Vector<Texto> — extraer claves", CompletionItemKind::FUNCTION),
            ("diccionario_valores", "<K,V>(d: Diccionario<K,V>) -> Vector<Texto> — extraer valores", CompletionItemKind::FUNCTION),
            ("diccionario_limpiar", "<K,V>(d: &mut Diccionario<K,V>) -> Vacío — vaciar", CompletionItemKind::FUNCTION),
            // === Conjunto (7) ===
            ("conjunto_nuevo", "<T>() -> Conjunto<T> — conjunto vacío", CompletionItemKind::FUNCTION),
            ("conjunto_insertar", "<T>(c: &mut Conjunto<T>, item: T) -> Vacío — agregar", CompletionItemKind::FUNCTION),
            ("conjunto_contiene", "<T>(c: Conjunto<T>, item: T) -> Booleano — pertenencia", CompletionItemKind::FUNCTION),
            ("conjunto_eliminar", "<T>(c: &mut Conjunto<T>, item: T) -> Vacío — eliminar", CompletionItemKind::FUNCTION),
            ("conjunto_longitud", "<T>(c: Conjunto<T>) -> Entero32 — cantidad", CompletionItemKind::FUNCTION),
            ("conjunto_liberar", "<T>(c: Conjunto<T>) -> Vacío — liberar", CompletionItemKind::FUNCTION),
            ("conjunto_elementos", "<T>(c: Conjunto<T>) -> Vector<Texto> — extraer a vector", CompletionItemKind::FUNCTION),
            // === Opción/Resultado (4) ===
            ("opcion_es_alguno", "<T>(o: Option<T>) -> Booleano — ¿Es Algo?", CompletionItemKind::FUNCTION),
            ("opcion_es_ninguno", "<T>(o: Option<T>) -> Booleano — ¿Es Nada?", CompletionItemKind::FUNCTION),
            ("resultado_es_exito", "<T,E>(r: Resultado<T,E>) -> Booleano — ¿Es Exito?", CompletionItemKind::FUNCTION),
            ("resultado_es_error", "<T,E>(r: Resultado<T,E>) -> Booleano — ¿Es Error?", CompletionItemKind::FUNCTION),
            // === Proceso (8) ===
            ("proceso_crear", "(comando: Texto) -> Entero32 — ejecutar proceso", CompletionItemKind::FUNCTION),
            ("proceso_esperar", "(pid: Entero32) -> Entero32 — esperar proceso", CompletionItemKind::FUNCTION),
            ("proceso_cerrar", "(pid: Entero32) -> Vacío — cerrar proceso", CompletionItemKind::FUNCTION),
            ("proceso_crear_con_pipes", "(cmd: Texto) -> Entero32 — crear con pipes", CompletionItemKind::FUNCTION),
            ("proceso_escribir", "(pid: Entero32, datos: Texto) -> Vacío — escribir a stdin", CompletionItemKind::FUNCTION),
            ("proceso_leer_salida_completa", "(pid: Entero32) -> Texto — leer stdout", CompletionItemKind::FUNCTION),
            ("proceso_leer_error_chunk", "(pid: Entero32) -> Texto — leer stderr", CompletionItemKind::FUNCTION),
            ("proceso_cerrar_bidireccional", "(pid: Entero32) -> Vacío — cerrar pipes", CompletionItemKind::FUNCTION),
            // === Canales (5) ===
            ("canal_nuevo", "(capacidad: Entero32) -> Canal — crear canal", CompletionItemKind::FUNCTION),
            ("canal_enviar", "(canal: Canal, valor: Entero32) -> Vacío — enviar", CompletionItemKind::FUNCTION),
            ("canal_recibir", "(canal: Canal) -> Entero32 — recibir", CompletionItemKind::FUNCTION),
            ("canal_cerrar", "(canal: Canal) -> Vacío — cerrar canal", CompletionItemKind::FUNCTION),
            ("canal_intentar", "(canal: Canal) -> Option<Entero32> — recibir sin bloquear", CompletionItemKind::FUNCTION),
            // === Visual: Ventana (8) ===
            ("ventana_nueva", "(titulo: Texto, ancho: Entero32, alto: Entero32) -> Ventana — crear ventana", CompletionItemKind::FUNCTION),
            ("ventana_mostrar", "(v: Ventana) -> Vacío — mostrar", CompletionItemKind::FUNCTION),
            ("ventana_cerrar", "(v: Ventana) -> Vacío — cerrar", CompletionItemKind::FUNCTION),
            ("ventana_bucle_mensajes", "(v: Ventana) -> Entero32 — message loop", CompletionItemKind::FUNCTION),
            ("ventana_titulo", "(v: Ventana) -> Texto — obtener título", CompletionItemKind::FUNCTION),
            ("ventana_establecer_titulo", "(v: Ventana, titulo: Texto) -> Vacío — establecer título", CompletionItemKind::FUNCTION),
            ("ventana_posicion", "(v: Ventana) -> Punto — posición (x,y)", CompletionItemKind::FUNCTION),
            ("ventana_tamano", "(v: Ventana) -> Tamano — tamaño (w,h)", CompletionItemKind::FUNCTION),
            // === Visual: Lienzo (8) ===
            ("lienzo_nuevo", "(ancho: Entero32, alto: Entero32) -> Lienzo — crear canvas", CompletionItemKind::FUNCTION),
            ("lienzo_limpiar", "(l: Lienzo, color: Entero32) -> Vacío — fill rect", CompletionItemKind::FUNCTION),
            ("lienzo_linea", "(l: Lienzo, x1: Entero32, y1: Entero32, x2: Entero32, y2: Entero32) -> Vacío — dibujar línea", CompletionItemKind::FUNCTION),
            ("lienzo_rectangulo", "(l: Lienzo, x: Entero32, y: Entero32, w: Entero32, h: Entero32) -> Vacío — rectángulo", CompletionItemKind::FUNCTION),
            ("lienzo_circulo", "(l: Lienzo, cx: Entero32, cy: Entero32, radio: Entero32) -> Vacío — círculo", CompletionItemKind::FUNCTION),
            ("lienzo_texto", "(l: Lienzo, x: Entero32, y: Entero32, texto: Texto) -> Vacío — texto", CompletionItemKind::FUNCTION),
            ("lienzo_guardar_png", "(l: Lienzo, ruta: Texto) -> Entero32 — guardar PNG", CompletionItemKind::FUNCTION),
            ("lienzo_liberar", "(l: Lienzo) -> Vacío — liberar", CompletionItemKind::FUNCTION),
            // === Visual: Imagen (6) ===
            ("imagen_desde_archivo", "(ruta: Texto) -> Imagen — cargar imagen", CompletionItemKind::FUNCTION),
            ("imagen_ancho", "(img: Imagen) -> Entero32 — ancho", CompletionItemKind::FUNCTION),
            ("imagen_alto", "(img: Imagen) -> Entero32 — alto", CompletionItemKind::FUNCTION),
            ("imagen_redimensionar", "(img: Imagen, w: Entero32, h: Entero32) -> Imagen — redimensionar", CompletionItemKind::FUNCTION),
            ("imagen_guardar_png", "(img: Imagen, ruta: Texto) -> Entero32 — guardar PNG", CompletionItemKind::FUNCTION),
            ("imagen_liberar", "(img: Imagen) -> Vacío — liberar", CompletionItemKind::FUNCTION),
            // === Visual: Sonido (8) ===
            ("audio_nuevo", "(canales: Entero32, freq: Entero32) -> Audio — buffer vacío", CompletionItemKind::FUNCTION),
            ("audio_desde_archivo", "(ruta: Texto) -> Audio — cargar WAV", CompletionItemKind::FUNCTION),
            ("audio_tono", "(freq: Flotante64, dur_ms: Entero32, canales: Entero32, freq_m: Entero32) -> Audio — tono puro", CompletionItemKind::FUNCTION),
            ("audio_mezclar", "(a: Audio, b: Audio) -> Audio — mezclar", CompletionItemKind::FUNCTION),
            ("audio_fade_in", "(audio: Audio, dur_ms: Entero32) -> Vacío — fade in", CompletionItemKind::FUNCTION),
            ("audio_fade_out", "(audio: Audio, dur_ms: Entero32) -> Vacío — fade out", CompletionItemKind::FUNCTION),
            ("audio_guardar_wav", "(audio: Audio, ruta: Texto) -> Entero32 — guardar WAV", CompletionItemKind::FUNCTION),
            ("audio_reproducir", "(audio: Audio) -> Entero32 — reproducir", CompletionItemKind::FUNCTION),
        ];

        for (name, sig, kind) in builtins {
            items.push(CompletionItem {
                label: name.to_string(),
                kind: Some(kind),
                detail: Some(sig.to_string()),
                ..Default::default()
            });
        }

        items
    }

    /// Genera firma de función para signature help
    fn firma_a_signature_info(
        nombre: &str,
        params: &[(String, Tipo)],
        retorno: Option<&Tipo>,
    ) -> SignatureInformation {
        let params_str: Vec<String> = params.iter()
            .map(|(n, t)| format!("{}: {:?}", n, t))
            .collect();
        let ret_str = retorno.map(|t| format!("{:?}", t)).unwrap_or_else(|| "Vacío".to_string());
        let label = format!("{}({}) -> {}", nombre, params_str.join(", "), ret_str);

        let param_info: Vec<ParameterInformation> = params.iter()
            .map(|(n, t)| ParameterInformation {
                    label: ParameterLabel::LabelOffsets([
                    label.find(n).unwrap_or(0) as u32,
                    label.find(n).map(|i| i + n.len()).unwrap_or(0) as u32,
                ]),
                documentation: Some(Documentation::String(format!("{:?}", t))),
            })
            .collect();

        SignatureInformation {
            label,
            documentation: Some(Documentation::String(format!("Función `{}` de Falcato", nombre))),
            parameters: Some(param_info),
            active_parameter: Some(0),
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(
        &self,
        params: InitializeParams,
    ) -> Result<InitializeResult> {
        // Extraer workspace root para escaneo de archivos
        if let Some(folder) = params.workspace_folders.and_then(|f| f.into_iter().next()) {
            let root = folder.uri.to_file_path()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            {
                let mut ws = self.workspace_root.write().await;
                *ws = Some(root.clone());
            }
            // Escanear workspace en background
            let root_clone = root;
            let this = self.clone();
            tokio::spawn(async move {
                this.escanear_workspace(&root_clone).await;
            });
        }
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::FULL),
                        ..Default::default()
                    }
                )),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![
                        ":".to_string(),
                        ".".to_string(),
                    ]),
                    all_commit_characters: Some(vec![
                        "\n".to_string(),
                        ";".to_string(),
                        ",".to_string(),
                    ]),
                    ..Default::default()
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                signature_help_provider: Some(SignatureHelpOptions {
                    trigger_characters: Some(vec!["(".to_string(), ",".to_string()]),
                    retrigger_characters: Some(vec![",".to_string()]),
                    ..Default::default()
                }),
                document_symbol_provider: Some(OneOf::Left(true)),
                document_formatting_provider: Some(OneOf::Left(true)),
                inlay_hint_provider: Some(OneOf::Left(true)),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                rename_provider: Some(OneOf::Left(true)),
                code_lens_provider: Some(CodeLensOptions {
                    resolve_provider: Some(false),
                }),
                diagnostic_provider: Some(
                    DiagnosticServerCapabilities::Options(DiagnosticOptions {
                        identifier: Some("falcato".to_string()),
                        inter_file_dependencies: false,
                        workspace_diagnostics: false,
                        work_done_progress_options: WorkDoneProgressOptions {
                            work_done_progress: Some(false),
                        },
                    })
                ),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(
        &self,
        _: InitializedParams,
    ) {
        self.client
            .log_message(MessageType::INFO, "Servidor Falcato LSP iniciado")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    // === Manejo de documentos ===

    async fn did_open(
        &self,
        params: DidOpenTextDocumentParams,
    ) {
        let uri = params.text_document.uri;
        let contenido = params.text_document.text;

        // Analizar
        let (diagnosticos, indice, ast) = self.analizar_documento(&uri, &contenido).await;

        // Guardar documento con índice
        {
            let mut docs = self.documentos.write().await;
            docs.insert(uri.clone(), DocumentoLsp {
                contenido: contenido.clone(),
                indice,
                ast,
            });
        }

        // Enviar diagnósticos
        self.client
            .publish_diagnostics(uri, diagnosticos, None)
            .await;
    }

    async fn did_change(
        &self,
        params: DidChangeTextDocumentParams,
    ) {
        let uri = params.text_document.uri;

        // Actualizar contenido (FULL sync = solo un cambio con todo el texto)
        if let Some(change) = params.content_changes.into_iter().next() {
            let contenido = change.text;

            // Re-analizar
            let (diagnosticos, indice, ast) = self.analizar_documento(&uri, &contenido).await;

            {
                let mut docs = self.documentos.write().await;
                docs.insert(uri.clone(), DocumentoLsp {
                    contenido: contenido.clone(),
                    indice,
                    ast,
                });
            }

            self.client
                .publish_diagnostics(uri, diagnosticos, None)
                .await;
        }
    }

    async fn did_close(
        &self,
        params: DidCloseTextDocumentParams,
    ) {
        let uri = params.text_document.uri;

        {
            let mut docs = self.documentos.write().await;
            docs.remove(&uri);
        }

        // Limpiar diagnósticos
        self.client
            .publish_diagnostics(uri, vec![], None)
            .await;
    }

    // === Autocompletado (context-aware) ===

    async fn completion(
        &self,
        params: CompletionParams,
    ) -> Result<Option<CompletionResponse>> {
        // Siempre incluir items estáticos (keywords, tipos, builtins)
        let mut items = Self::items_autocompletado();

        // Añadir items contextuales del documento
        let uri = params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;

        let docs = self.documentos.read().await;
        if let Some(doc) = docs.get(&uri) {
            let contextuales = self.items_autocompletado_contexto(
                &doc.indice,
                &doc.contenido,
                pos.line,
            );
            items.extend(contextuales);
        }

        Ok(Some(CompletionResponse::Array(items)))
    }

    // === Signature Help ===

    async fn signature_help(
        &self,
        params: SignatureHelpParams,
    ) -> Result<Option<SignatureHelp>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;

        let docs = self.documentos.read().await;
        let doc = match docs.get(&uri) {
            Some(d) => d,
            None => return Ok(None),
        };

        // Buscar nombre de función alrededor de la posición (antes de '(')
        let line = doc.contenido.lines().nth(pos.line as usize).unwrap_or("");
        let before_paren = if let Some(paren_pos) = line[..pos.character as usize].rfind('(') {
            let before = line[..paren_pos].trim();
            before.split_whitespace().last().map(|s| s.to_string())
        } else {
            None
        };

        let func_name = match before_paren {
            Some(ref n) if !n.is_empty() && n.chars().all(|c| c.is_alphanumeric() || c == '_') => n.clone(),
            _ => return Ok(None),
        };

        // Buscar función en índice
        if let Some(func) = doc.indice.funciones.get(&func_name) {
            let params_info: Vec<ParameterInformation> = func.parametros_raw.iter()
                .map(|(n, t)| ParameterInformation {
                    label: ParameterLabel::Simple(format!("{}: {}", n, t)),
                    documentation: Some(Documentation::String(t.clone())),
                })
                .collect();

            let params_str = func.parametros_raw.iter()
                .map(|(n, t)| format!("{}: {}", n, t))
                .collect::<Vec<_>>().join(", ");
            let ret = func.retorno.as_deref().unwrap_or("Vacío");
            let label = format!("{}({}) -> {}", func.nombre, params_str, ret);

            return Ok(Some(SignatureHelp {
                signatures: vec![SignatureInformation {
                    label,
                    documentation: Some(Documentation::String(format!("Función `{}` de Falcato", func.nombre))),
                    parameters: Some(params_info),
                    active_parameter: Some(0),
                }],
                active_signature: Some(0),
                active_parameter: Some(0),
            }));
        }

        Ok(None)
    }

    // === Code Actions ===

    async fn code_action(
        &self,
        params: CodeActionParams,
    ) -> Result<Option<CodeActionResponse>> {
        let uri = params.text_document.uri;

        let docs = self.documentos.read().await;
        let doc = match docs.get(&uri) {
            Some(d) => d,
            None => return Ok(None),
        };

        // Re-analizar para obtener diagnósticos actualizados
        let (diagnosticos, _, _) = self.analizar_documento(&uri, &doc.contenido).await;

        let mut actions: Vec<CodeActionOrCommand> = Vec::new();

        for diag in &diagnosticos {
            // Solo acciones para errores en el rango solicitado
            if !self.span_en_rango(diag.range, &params.range) {
                continue;
            }

            let codigo = diag.code.as_ref()
                .and_then(|c| match c {
                    NumberOrString::String(s) => Some(s.as_str()),
                    _ => None,
                })
                .unwrap_or("");

            match codigo {
                "T001" | "T005" => {
                    // Error de tipo → sugerencia de cambio
                    actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                        title: "💡 Revisar tipo (abre hover)".to_string(),
                        kind: Some(CodeActionKind::QUICKFIX),
                        diagnostics: Some(vec![diag.clone()]),
                        ..Default::default()
                    }));
                }
                "O001" => {
                    // Error de ownership (usar después de mover / mutar inmutable)
                    actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                        title: "🔧 Usar `mover` / `copiar` antes del uso".to_string(),
                        kind: Some(CodeActionKind::QUICKFIX),
                        diagnostics: Some(vec![diag.clone()]),
                        ..Default::default()
                    }));
                }
                _ => {
                    // Genérico: mostrar sugerencia del compilador
                    let suggestion = diag.message.contains("💡");
                    if suggestion {
                        actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                            title: format!("💡 Seguir sugerencia del compilador"),
                            kind: Some(CodeActionKind::QUICKFIX),
                            diagnostics: Some(vec![diag.clone()]),
                            ..Default::default()
                        }));
                    }
                }
            }
        }

        if actions.is_empty() {
            Ok(None)
        } else {
            Ok(Some(actions))
        }
    }

    // === Document Symbols ===

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri;

        let docs = self.documentos.read().await;
        let doc = match docs.get(&uri) {
            Some(d) => d,
            None => return Ok(None),
        };

        let mut symbols: Vec<DocumentSymbol> = Vec::new();

        // Funciones
        for func in doc.indice.funciones.values() {
            let params_str = func.parametros_raw.iter()
                .map(|(n, t)| format!("{}: {}", n, t))
                .collect::<Vec<_>>().join(", ");
            let ret = func.retorno.as_deref().unwrap_or("Vacío");
            let detail = format!("{}({}) -> {}", func.nombre, params_str, ret);

            symbols.push(DocumentSymbol {
                name: func.nombre.clone(),
                kind: SymbolKind::FUNCTION,
                range: self.span_a_rango(&func.span_declaracion),
                selection_range: self.span_a_rango(&func.span_declaracion),
                detail: Some(detail),
                children: None,
                tags: None,
                deprecated: None,
            });
        }

        // Structs
        for s in doc.indice.structs.values() {
            let campos: Vec<DocumentSymbol> = s.campos.iter()
                .map(|(n, t)| DocumentSymbol {
                    name: n.clone(),
                    kind: SymbolKind::FIELD,
                    range: Range { start: Position { line: 0, character: 0 }, end: Position { line: 0, character: 0 } },
                    selection_range: Range { start: Position { line: 0, character: 0 }, end: Position { line: 0, character: 0 } },
                    detail: Some(t.clone()),
                    children: None,
                    tags: None,
                    deprecated: None,
                })
                .collect();

            symbols.push(DocumentSymbol {
                name: s.nombre.clone(),
                kind: SymbolKind::STRUCT,
                range: self.span_a_rango(&s.span_declaracion),
                selection_range: self.span_a_rango(&s.span_declaracion),
                detail: Some(format!("estructural ({} campos)", s.campos.len())),
                children: Some(campos),
                tags: None,
                deprecated: None,
            });
        }

        // Enums
        for e in doc.indice.enums.values() {
            let variantes: Vec<DocumentSymbol> = e.variantes.iter()
                .map(|(n, t)| {
                    let detail = t.as_deref().unwrap_or("—");
                    DocumentSymbol {
                        name: n.clone(),
                        kind: SymbolKind::ENUM_MEMBER,
                        range: Range { start: Position { line: 0, character: 0 }, end: Position { line: 0, character: 0 } },
                        selection_range: Range { start: Position { line: 0, character: 0 }, end: Position { line: 0, character: 0 } },
                        detail: Some(detail.to_string()),
                        children: None,
                        tags: None,
                        deprecated: None,
                    }
                })
                .collect();

            symbols.push(DocumentSymbol {
                name: e.nombre.clone(),
                kind: SymbolKind::ENUM,
                range: self.span_a_rango(&e.span_declaracion),
                selection_range: self.span_a_rango(&e.span_declaracion),
                detail: Some(format!("enumeración ({} variantes)", e.variantes.len())),
                children: Some(variantes),
                tags: None,
                deprecated: None,
            });
        }

        // Traits
        for t in doc.indice.traits.values() {
            symbols.push(DocumentSymbol {
                name: t.nombre.clone(),
                kind: SymbolKind::INTERFACE,
                range: self.span_a_rango(&t.span_declaracion),
                selection_range: self.span_a_rango(&t.span_declaracion),
                detail: Some(format!("rasgo ({} métodos)", t.metodos.len())),
                children: None,
                tags: None,
                deprecated: None,
            });
        }

        if symbols.is_empty() {
            Ok(None)
        } else {
            Ok(Some(DocumentSymbolResponse::Nested(symbols)))
        }
    }

    // === Hover ===

    async fn hover(
        &self,
        params: HoverParams,
    ) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;

        // Convertir posición LSP (0-indexed) a nuestro sistema (1-indexed)
        let linea = pos.line + 1;
        let columna = pos.character + 1;

        // Buscar documento
        let docs = self.documentos.read().await;
        let doc = match docs.get(&uri) {
            Some(d) => d,
            None => return Ok(None),
        };

        // Buscar identificador en la posición
        let ast = match &doc.ast {
            Some(a) => a,
            None => return Ok(None),
        };

        let ident = match doc.indice.identificador_en_posicion(ast, linea, columna) {
            Some(i) => i,
            None => return Ok(None),
        };

        // Generar hover
        Ok(self.hover_para_identificador(&doc.indice, &ident))
    }

    // === Find References ===

    async fn references(
        &self,
        params: ReferenceParams,
    ) -> Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;

        let linea = pos.line + 1;
        let columna = pos.character + 1;

        let docs = self.documentos.read().await;
        let doc = match docs.get(&uri) {
            Some(d) => d,
            None => return Ok(None),
        };

        let ast = match &doc.ast {
            Some(a) => a,
            None => return Ok(None),
        };

        // Encontrar identificador en la posición
        let ident = match doc.indice.identificador_en_posicion(ast, linea, columna) {
            Some(i) => i,
            None => return Ok(None),
        };

        // Encontrar todas las referencias
        let spans = doc.indice.encontrar_referencias(ast, &ident);

        let locations: Vec<Location> = spans.into_iter().map(|span| Location {
            uri: uri.clone(),
            range: Range {
                start: Position {
                    line: span.inicio.linea.saturating_sub(1),
                    character: span.inicio.columna.saturating_sub(1),
                },
                end: Position {
                    line: span.fin.linea.saturating_sub(1),
                    character: span.fin.columna.saturating_sub(1),
                },
            },
        }).collect();

        Ok(Some(locations))
    }

    // === Go to Definition ===

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;

        let linea = pos.line + 1;
        let columna = pos.character + 1;

        let docs = self.documentos.read().await;
        let doc = match docs.get(&uri) {
            Some(d) => d,
            None => return Ok(None),
        };

        let ast = match &doc.ast {
            Some(a) => a,
            None => return Ok(None),
        };

        let ident = match doc.indice.identificador_en_posicion(ast, linea, columna) {
            Some(i) => i,
            None => return Ok(None),
        };

        // Buscar span de declaración — ahora incluye structs/enums/traits
        let span = doc.indice.variables.get(&ident)
            .map(|v| v.span_declaracion.clone())
            .or_else(|| doc.indice.funciones.get(&ident)
                .map(|f| f.span_declaracion.clone()))
            .or_else(|| doc.indice.structs.get(&ident)
                .map(|s| s.span_declaracion.clone()))
            .or_else(|| doc.indice.enums.get(&ident)
                .map(|e| e.span_declaracion.clone()))
            .or_else(|| doc.indice.traits.get(&ident)
                .map(|t| t.span_declaracion.clone()));

        let span = match span {
            Some(s) => s,
            None => return Ok(None),
        };

        let location = Location {
            uri: uri.clone(),
            range: Range {
                start: Position {
                    line: span.inicio.linea.saturating_sub(1),
                    character: span.inicio.columna.saturating_sub(1),
                },
                end: Position {
                    line: span.fin.linea.saturating_sub(1),
                    character: span.fin.columna.saturating_sub(1),
                },
            },
        };

        Ok(Some(GotoDefinitionResponse::Scalar(location)))
    }

    // === Format Document ===

    async fn formatting(
        &self,
        params: DocumentFormattingParams,
    ) -> Result<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri;

        let docs = self.documentos.read().await;
        let doc = match docs.get(&uri) {
            Some(d) => d,
            None => return Ok(None),
        };

        let formateado = Self::formatear_falcato(&doc.contenido);

        // Si no cambió, no hay edits
        if formateado == doc.contenido {
            return Ok(None);
        }

        // Calcular el rango completo del documento
        let lineas = doc.contenido.lines().count().max(1);
        let ultima_linea = doc.contenido.lines().last().map(|l| l.len()).unwrap_or(0);

        let edit = TextEdit {
            range: Range {
                start: Position { line: 0, character: 0 },
                end: Position {
                    line: lineas as u32,
                    character: ultima_linea as u32,
                },
            },
            new_text: formateado,
        };

        Ok(Some(vec![edit]))
    }

    // === Inlay Hints ===

    async fn inlay_hint(
        &self,
        params: InlayHintParams,
    ) -> Result<Option<Vec<InlayHint>>> {
        let uri = params.text_document.uri;
        let range = params.range;

        let docs = self.documentos.read().await;
        let doc = match docs.get(&uri) {
            Some(d) => d,
            None => return Ok(None),
        };

        let mut hints = Vec::new();

        // Buscar variables con tipo inferido
        for (nombre, var) in &doc.indice.variables {
            // Solo mostrar hint si la variable está en el rango solicitado
            let linea = var.span_declaracion.inicio.linea.saturating_sub(1);
            if linea >= range.start.line && linea <= range.end.line {
                // Hint de tipo después del nombre
                let hint = InlayHint {
                    position: Position {
                        line: linea,
                        character: var.span_declaracion.inicio.columna as u32 + nombre.len() as u32,
                    },
                    label: InlayHintLabel::String(format!(": {}", var.tipo)),
                    kind: Some(InlayHintKind::TYPE),
                    tooltip: None,
                    text_edits: None,
                    data: None,
                    padding_left: Some(false),
                    padding_right: Some(true),
                };
                hints.push(hint);
            }
        }

        // Buscar funciones con retorno inferido
        for (nombre, func) in &doc.indice.funciones {
            if let Some(ref retorno) = func.retorno {
                let linea = func.span_declaracion.inicio.linea.saturating_sub(1);
                if linea >= range.start.line && linea <= range.end.line {
                    // Hint de retorno al final de la firma
                    let hint = InlayHint {
                        position: Position {
                            line: linea,
                            character: 999, // al final de la línea
                        },
                        label: InlayHintLabel::String(format!("-> {}", retorno)),
                        kind: Some(InlayHintKind::TYPE),
                        tooltip: None,
                        text_edits: None,
                        data: None,
                        padding_left: Some(true),
                        padding_right: Some(false),
                    };
                    hints.push(hint);
                }
            }
        }

        if hints.is_empty() {
            Ok(None)
        } else {
            Ok(Some(hints))
        }
    }

    // === Rename Symbol ===

    async fn rename(
        &self,
        params: RenameParams,
    ) -> Result<Option<WorkspaceEdit>> {
        let uri = params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;
        let nuevo_nombre = params.new_name;

        let linea = pos.line + 1;
        let columna = pos.character + 1;

        let docs = self.documentos.read().await;
        let doc = match docs.get(&uri) {
            Some(d) => d,
            None => return Ok(None),
        };

        let ast = match &doc.ast {
            Some(a) => a,
            None => return Ok(None),
        };

        let ident = match doc.indice.identificador_en_posicion(ast, linea, columna) {
            Some(i) => i,
            None => return Ok(None),
        };

        // Encontrar todas las referencias
        let spans = doc.indice.encontrar_referencias(ast, &ident);

        let mut changes = HashMap::new();
        let mut edits = Vec::new();

        for span in &spans {
            edits.push(TextEdit {
                range: Range {
                    start: Position {
                        line: span.inicio.linea.saturating_sub(1),
                        character: span.inicio.columna.saturating_sub(1),
                    },
                    end: Position {
                        line: span.fin.linea.saturating_sub(1),
                        character: span.fin.columna.saturating_sub(1),
                    },
                },
                new_text: nuevo_nombre.clone(),
            });
        }

        changes.insert(uri.clone(), edits);

        Ok(Some(WorkspaceEdit {
            changes: Some(changes),
            ..Default::default()
        }))
    }

    // === Code Lens ===

    async fn code_lens(
        &self,
        params: CodeLensParams,
    ) -> Result<Option<Vec<CodeLens>>> {
        let uri = params.text_document.uri;

        let docs = self.documentos.read().await;
        let doc = match docs.get(&uri) {
            Some(d) => d,
            None => return Ok(None),
        };

        let mut lenses = Vec::new();

        // Agregar "▶ Ejecutar" y "🧪 Test" en cada función
        for (nombre, func) in &doc.indice.funciones {
            let linea = func.span_declaracion.inicio.linea.saturating_sub(1);
            let range = Range {
                start: Position { line: linea, character: 0 },
                end: Position { line: linea, character: 0 },
            };

            // Botón "▶ Ejecutar"
            lenses.push(CodeLens {
                range,
                command: Some(Command {
                    title: format!("▶ Ejecutar {}", nombre),
                    command: "falcato.ejecutar".to_string(),
                    arguments: Some(vec![
                        serde_json::Value::String(uri.to_string()),
                        serde_json::Value::String(nombre.clone()),
                    ]),
                }),
                data: None,
            });

            // Botón "🧪 Test" si es función de test
            if nombre.starts_with("test_") || nombre.starts_with("prueba_") {
                lenses.push(CodeLens {
                    range,
                    command: Some(Command {
                        title: format!("🧪 Ejecutar test {}", nombre),
                        command: "falcato.test".to_string(),
                        arguments: Some(vec![
                            serde_json::Value::String(uri.to_string()),
                            serde_json::Value::String(nombre.clone()),
                        ]),
                    }),
                    data: None,
                });
            }
        }

        if lenses.is_empty() {
            Ok(None)
        } else {
            Ok(Some(lenses))
        }
    }
}

/// Inicia el servidor LSP
pub async fn iniciar_lsp() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend::nuevo(client));
    Server::new(stdin, stdout, socket).serve(service).await;
}
