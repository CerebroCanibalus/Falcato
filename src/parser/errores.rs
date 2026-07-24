use crate::span::Span;
use crate::error::{CategoriaError, ErrorCompilador};

/// Error específico de parsing con código [S###]
#[derive(Debug, Clone)]
pub struct ErrorSintaxis {
    pub error: ErrorCompilador,
}

impl ErrorSintaxis {
    /// Crea un error de sintaxis genérico
    pub fn nuevo(codigo: u32, span: Span, mensaje: impl Into<String>) -> Self {
        Self {
            error: ErrorCompilador::nuevo(CategoriaError::Sintaxis, codigo, span, mensaje),
        }
    }

    /// Token inesperado encontrado (S001)
    pub fn token_inesperado(span: Span, esperado: &str, encontrado: Option<&str>) -> Self {
        let msg = match encontrado {
            Some(t) => format!("token inesperado: '{}', esperaba: '{}'", t, esperado),
            None => format!("fin de archivo inesperado, esperaba: '{}'", esperado),
        };
        Self::nuevo(1, span, msg)
            .con_sugerencia(format!("revisa la sintaxis cerca de esta posición"))
    }

    /// Fin de archivo inesperado (S002)
    pub fn fin_archivo_inesperado(span: Span) -> Self {
        Self::nuevo(2, span, "fin de archivo inesperado")
            .con_sugerencia("revisa que todas las llaves y paréntesis estén cerrados")
    }

    /// Se esperaba un token específico (S003)
    pub fn esperaba(span: Span, esperado: &str, encontrado: Option<&str>) -> Self {
        let msg = match encontrado {
            Some(t) => format!("esperaba '{}', encontrado: '{}'", esperado, t),
            None => format!("esperaba '{}', pero llegó al final del archivo", esperado),
        };
        Self::nuevo(3, span, msg)
    }

    /// Identificador esperado (S004)
    pub fn identificador_esperado(span: Span, contexto: &str) -> Self {
        Self::nuevo(4, span, format!("esperaba identificador para {}", contexto))
    }

    /// Expresión esperada (S005)
    pub fn expresion_esperada(span: Span) -> Self {
        Self::nuevo(5, span, "esperaba una expresión")
            .con_sugerencia("verifica que la expresión esté completa")
    }

    /// Tipo esperado (S006)
    pub fn tipo_esperado(span: Span) -> Self {
        Self::nuevo(6, span, "esperaba un tipo")
            .con_sugerencia("los tipos válidos son: Entero8-64, Natural8-64, Flotante32/64, Booleano, Caracter, Palabra, Texto, Vector<T>, Vacío")
    }

    /// Artículo esperado (S007)
    pub fn articulo_esperado(span: Span) -> Self {
        Self::nuevo(7, span, "esperaba artículo (el, la, un, los, las)")
            .con_sugerencia("recuerda: 'el'=mutable, 'la'=inmutable, 'un'=opcional")
    }

    /// Carácter/token inválido en el lexer (S008)
    pub fn token_invalido(span: Span, texto: &str) -> Self {
        Self::nuevo(8, span, format!("carácter no válido: '{}'", texto))
            .con_sugerencia("revisa que no haya símbolos extraños o caracteres no soportados")
    }

    pub fn con_sugerencia(mut self, sugerencia: impl Into<String>) -> Self {
        self.error = self.error.con_sugerencia(sugerencia);
        self
    }
}

impl From<ErrorSintaxis> for ErrorCompilador {
    fn from(e: ErrorSintaxis) -> Self {
        e.error
    }
}
