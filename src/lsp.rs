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

/// Información de una variable para hover/definition
#[derive(Debug, Clone)]
pub struct InfoVariableLsp {
    pub nombre: String,
    pub tipo: String,
    pub articulo: String,
    pub span_declaracion: Span,
}

/// Información de una función para hover/definition
#[derive(Debug, Clone)]
pub struct InfoFuncionLsp {
    pub nombre: String,
    pub retorno: Option<String>,
    pub parametros: Vec<String>,
    pub span_declaracion: Span,
}

/// Índice semántico de un documento
#[derive(Debug, Clone, Default)]
pub struct IndiceSemantico {
    pub variables: HashMap<String, InfoVariableLsp>,
    pub funciones: HashMap<String, InfoFuncionLsp>,
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

        self.funciones.insert(func.nombre.clone(), InfoFuncionLsp {
            nombre: func.nombre.clone(),
            retorno: func.retorno.as_ref().map(|t| format!("{:?}", t)),
            parametros: params,
            span_declaracion: func.span.clone(),
        });

        // Registrar parámetros como variables
        for param in &func.parametros {
            self.variables.insert(param.nombre.clone(), InfoVariableLsp {
                nombre: param.nombre.clone(),
                tipo: format!("{:?}", param.tipo),
                articulo: self.articulo_str(param.articulo).to_string(),
                span_declaracion: param.span.clone(),
            });
        }

        // Registrar variables del cuerpo
        for sentencia in &func.cuerpo.sentencias {
            self.indexar_sentencia(sentencia);
        }
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
            Expresion::Binaria(izq, _, der, span) => {
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
            Expresion::MetodoBitwise(receptor, _, args, _) => {
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
            let contenido = format!(
                "**{}** `\n{} {}: {}`\n\n---\n*Artículo*: `{}` → {}\n*Tipo*: `{}`",
                var.nombre,
                var.articulo,
                var.nombre,
                var.tipo,
                var.articulo,
                self.explicar_articulo(&var.articulo),
                var.tipo
            );

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
            let contenido = format!(
                "**fn** `{}({}) -> {}`\n\n---\n*Función de Falcato*",
                func.nombre,
                params,
                ret
            );

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
            "el" => "owned mutable (tú tienes el valor)",
            "la" => "borrowed immutable (prestado, solo lectura)",
            "un" => "optional (puede ser None)",
            "los" => "colección owned mutable",
            "las" => "colección borrowed immutable",
            _ => "desconocido",
        }
    }

    /// Lista de items para autocompletado
    fn items_autocompletado() -> Vec<CompletionItem> {
        let mut items = Vec::new();

        // Keywords
        let keywords = vec![
            ("función", "Declara una función"),
            ("retornar", "Retorna un valor"),
            ("si", "Condicional"),
            ("sino", "Rama alternativa"),
            ("mientras", "Bucle mientras"),
            ("para", "Bucle para (próximamente)"),
            ("inseguro", "Función FFI insegura"),
            ("estructural", "Define un struct (próximamente)"),
            ("enumeración", "Define un enum (próximamente)"),
            ("usar", "Importa un módulo (próximamente)"),
            ("módulo", "Define un módulo (próximamente)"),
        ];

        for (kw, doc) in keywords {
            items.push(CompletionItem {
                label: kw.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some(doc.to_string()),
                ..Default::default()
            });
        }

        // Artículos (ownership)
        let articulos = vec![
            ("el", "Owned mutable (tú tienes el valor, puedes modificarlo)"),
            ("la", "Borrowed immutable (prestado, solo lectura)"),
            ("un", "Optional (puede ser None o Some)"),
            ("los", "Colección owned mutable"),
            ("las", "Colección borrowed immutable"),
        ];

        for (art, doc) in articulos {
            items.push(CompletionItem {
                label: art.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some(doc.to_string()),
                ..Default::default()
            });
        }

        // Tipos primitivos
        let tipos = vec![
            "Entero8", "Entero16", "Entero32", "Entero64",
            "Natural8", "Natural16", "Natural32", "Natural64",
            "Flotante32", "Flotante64",
            "Booleano", "Caracter", "Palabra", "Texto", "Vacío",
        ];

        for t in tipos {
            items.push(CompletionItem {
                label: t.to_string(),
                kind: Some(CompletionItemKind::TYPE_PARAMETER),
                detail: Some(format!("Tipo primitivo {}", t)),
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

        items
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
                    ..Default::default()
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
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

    // === Autocompletado ===

    async fn completion(
        &self,
        _: CompletionParams,
    ) -> Result<Option<CompletionResponse>> {
        let items = Self::items_autocompletado();
        Ok(Some(CompletionResponse::Array(items)))
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

        // Buscar span de declaración
        let span = doc.indice.variables.get(&ident)
            .map(|v| v.span_declaracion.clone())
            .or_else(|| doc.indice.funciones.get(&ident)
                .map(|f| f.span_declaracion.clone()));

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
