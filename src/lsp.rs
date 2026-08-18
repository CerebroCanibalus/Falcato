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
}

impl Backend {
    pub fn nuevo(client: Client) -> Self {
        Self {
            client,
            documentos: Arc::new(RwLock::new(HashMap::new())),
        }
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

        // 3. Construir índice semántico
        let indice = IndiceSemantico::desde_ast(&programa);

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
            ("tipo", "Keyword de tipo (contextual)", CompletionItemKind::KEYWORD),
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

        // Tipos — primitivos + compuestos
        let tipos = vec![
            ("Entero8", "Entero de 8 bits con signo"),
            ("Entero16", "Entero de 16 bits con signo"),
            ("Entero32", "Entero de 32 bits con signo"),
            ("Entero64", "Entero de 64 bits con signo"),
            ("Natural8", "Entero de 8 bits sin signo"),
            ("Natural16", "Entero de 16 bits sin signo"),
            ("Natural32", "Entero de 32 bits sin signo"),
            ("Natural64", "Entero de 64 bits sin signo"),
            ("Flotante32", "Flotante de 32 bits (f32)"),
            ("Flotante64", "Flotante de 64 bits (f64)"),
            ("Booleano", "Booleano: verdadero o falso"),
            ("Caracter", "Carácter Unicode de 32 bits"),
            ("Palabra", "String literal inmutable (&str)"),
            ("Texto", "String heap-allocado growable (24 bytes, ¡liberar!)"),
            ("Vacío", "Tipo unitario (sin valor)"),
            ("Vector", "Vector dinámico genérico (heap, ¡liberar!)"),
            ("Resultado", "Result<T,E> — Exito(valor) o Error(codigo)"),
            ("Fase", "Estado de oscilador: { valor: Real, inc: Real } — librerias/math.fc"),
            ("Real_preciso", "apodo de Real — libm (error < 1 ULP)"),
            ("Real_rapido", "apodo de Real — polinomio grado 5 (error ~2^-16)"),
            ("Real_aprox", "apodo de Real — tabla 256 + lerp (error ~2^-10)"),
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

        // Built-in functions (más comunes)
        let builtins = vec![
            ("imprimir", "(mensaje: T) -> Vacío", CompletionItemKind::FUNCTION),
            ("imprimir_linea", "(mensaje: T) -> Vacío", CompletionItemKind::FUNCTION),
            ("decir", "(mensaje: T) -> Vacío — alias de imprimir_linea", CompletionItemKind::FUNCTION),
            ("tamaño_de::<T>", "() -> Entero64 — sizeof comptime", CompletionItemKind::FUNCTION),
            ("dormir", "(ms: Entero32) -> Vacío — suspende hilo actual", CompletionItemKind::FUNCTION),
            ("abs", "(x: Entero32) -> Entero32", CompletionItemKind::FUNCTION),
            ("max", "(a: Entero32, b: Entero32) -> Entero32", CompletionItemKind::FUNCTION),
            ("min", "(a: Entero32, b: Entero32) -> Entero32", CompletionItemKind::FUNCTION),
            ("raiz", "(x: Flotante64) -> Flotante64 — sqrt()", CompletionItemKind::FUNCTION),
            ("potencia", "(base: Flotante64, exp: Flotante64) -> Flotante64 — pow()", CompletionItemKind::FUNCTION),
            ("texto_nuevo", "() -> Texto", CompletionItemKind::FUNCTION),
            ("texto_desde", "(s: Palabra) -> Texto", CompletionItemKind::FUNCTION),
            ("texto_agregar", "(texto: Texto, fragmento: Palabra) -> Vacío", CompletionItemKind::FUNCTION),
            ("texto_concatenar", "(a: Texto, b: Texto) -> Texto", CompletionItemKind::FUNCTION),
            ("texto_liberar", "(texto: Texto) -> Vacío", CompletionItemKind::FUNCTION),
            ("vector_nuevo", "<T>() -> Vector<T>", CompletionItemKind::FUNCTION),
            ("vector_agregar", "<T>(v: Vector<T>, val: T) -> Vacío", CompletionItemKind::FUNCTION),
            ("vector_liberar", "<T>(v: Vector<T>) -> Vacío", CompletionItemKind::FUNCTION),
            ("diccionario_nuevo", "<K,V>() -> Diccionario<K,V>", CompletionItemKind::FUNCTION),
            ("diccionario_insertar", "<K,V>(d: Diccionario<K,V>, k: K, v: V) -> Vacío", CompletionItemKind::FUNCTION),
            ("archivo_leer", "(ruta: Palabra) -> Texto", CompletionItemKind::FUNCTION),
            ("archivo_escribir", "(ruta: Palabra, contenido: Texto) -> Entero32", CompletionItemKind::FUNCTION),
            ("canal_nuevo", "(capacidad: Entero32) -> Canal", CompletionItemKind::FUNCTION),
            ("canal_enviar", "(canal: Canal, valor: Entero32) -> Vacío", CompletionItemKind::FUNCTION),
            ("canal_recibir", "(canal: Canal) -> Entero32", CompletionItemKind::FUNCTION),
            // Trigonometría — F1: libm (preciso)
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
            // Trigonometría — F4: precisión
            ("seno_preciso", "(x: Real) -> Real — seno preciso (libm)", CompletionItemKind::FUNCTION),
            ("coseno_preciso", "(x: Real) -> Real — coseno preciso (libm)", CompletionItemKind::FUNCTION),
            ("tangente_preciso", "(x: Real) -> Real — tangente precisa (libm)", CompletionItemKind::FUNCTION),
            ("exp_preciso", "(x: Real) -> Real — exp preciso (libm)", CompletionItemKind::FUNCTION),
            ("log_preciso", "(x: Real) -> Real — log preciso (libm)", CompletionItemKind::FUNCTION),
            // Trigonometría — F7: polinomios minimax
            ("seno_rapido", "(x: Real) -> Real — seno rápido (polinomio)", CompletionItemKind::FUNCTION),
            ("coseno_rapido", "(x: Real) -> Real — coseno rápido (polinomio)", CompletionItemKind::FUNCTION),
            ("seno_2pi", "(fase: Real) -> Real — seno(2π×fase), fase ∈ [0,1)", CompletionItemKind::FUNCTION),
            ("coseno_2pi", "(fase: Real) -> Real — coseno(2π×fase), fase ∈ [0,1)", CompletionItemKind::FUNCTION),
            ("exp_rapido", "(x: Real) -> Real — exp rápido (libm)", CompletionItemKind::FUNCTION),
            ("log_rapido", "(x: Real) -> Real — log rápido (libm)", CompletionItemKind::FUNCTION),
            ("seno_aprox", "(x: Real) -> Real — seno aproximado (placeholder)", CompletionItemKind::FUNCTION),
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
        _: InitializeParams,
    ) -> Result<InitializeResult> {
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
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
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
}

/// Inicia el servidor LSP
pub async fn iniciar_lsp() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend::nuevo(client));
    Server::new(stdin, stdout, socket).serve(service).await;
}
