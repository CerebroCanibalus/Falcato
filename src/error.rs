use crate::span::Span;
use std::fmt;

/// Categorías de error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CategoriaError {
    Sintaxis,    // S001-S099
    Tipo,        // T001-T099
    Ownership,   // O001-O099
    FFI,         // C001-C099
    Modulos,     // M001-M099
    Interno,     // I001-I099
    Warning,     // W001-W099
}

impl CategoriaError {
    pub fn prefijo(self) -> &'static str {
        match self {
            CategoriaError::Sintaxis => "S",
            CategoriaError::Tipo => "T",
            CategoriaError::Ownership => "O",
            CategoriaError::FFI => "C",
            CategoriaError::Modulos => "M",
            CategoriaError::Interno => "I",
            CategoriaError::Warning => "W",
        }
    }
}

/// Error del compilador con código, span y mensaje
#[derive(Debug, Clone)]
pub struct ErrorCompilador {
    pub categoria: CategoriaError,
    pub codigo: u32,
    pub span: Span,
    pub mensaje: String,
    pub sugerencia: Option<String>,
}

impl ErrorCompilador {
    pub fn nuevo(
        categoria: CategoriaError,
        codigo: u32,
        span: Span,
        mensaje: impl Into<String>,
    ) -> Self {
        Self {
            categoria,
            codigo,
            span,
            mensaje: mensaje.into(),
            sugerencia: None,
        }
    }

    pub fn con_sugerencia(mut self, sugerencia: impl Into<String>) -> Self {
        self.sugerencia = Some(sugerencia.into());
        self
    }

    pub fn codigo_str(&self) -> String {
        format!("{}{:03}", self.categoria.prefijo(), self.codigo)
    }
}

impl fmt::Display for ErrorCompilador {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] {}:{}:{}: {}",
            self.codigo_str(),
            self.span.archivo,
            self.span.inicio.linea,
            self.span.inicio.columna,
            self.mensaje
        )?;

        if let Some(ref sug) = self.sugerencia {
            write!(f, "\n       │ sugerencia: {}", sug)?;
        }

        Ok(())
    }
}

impl std::error::Error for ErrorCompilador {}

/// Colección de errores
#[derive(Debug, Clone, Default)]
pub struct Errores {
    pub errores: Vec<ErrorCompilador>,
    /// Warnings: no bloquean la compilación, pero se reportan al usuario
    /// (categoría Warning W### o cualquier código que el compilador decida
    /// tratar como advertencia — p. ej. T031-T039: shadowing/duplicados).
    pub warnings: Vec<ErrorCompilador>,
}

impl Errores {
    pub fn nuevo() -> Self {
        Self::default()
    }

    pub fn agregar(&mut self, error: ErrorCompilador) {
        self.errores.push(error);
    }

    /// Añade un warning (no bloquea la compilación).
    pub fn agregar_warning(&mut self, warning: ErrorCompilador) {
        self.warnings.push(warning);
    }

    pub fn esta_vacio(&self) -> bool {
        self.errores.is_empty()
    }

    pub fn hay_errores(&self) -> bool {
        !self.errores.is_empty()
    }

    pub fn hay_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    pub fn cantidad(&self) -> usize {
        self.errores.len()
    }

    pub fn cantidad_warnings(&self) -> usize {
        self.warnings.len()
    }
}

impl fmt::Display for Errores {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for error in &self.errores {
            writeln!(f, "{}", error)?;
        }
        for warning in &self.warnings {
            writeln!(f, "{}", warning)?;
        }
        Ok(())
    }
}

/// Errores predefinidos (fábricas)
pub mod errores {
    use super::*;

    // Errores de sintaxis
    pub fn token_inesperado(span: Span, token: &str) -> ErrorCompilador {
        ErrorCompilador::nuevo(
            CategoriaError::Sintaxis,
            1,
            span,
            format!("token inesperado: '{}'", token),
        )
    }

    pub fn fin_archivo_inesperado(span: Span) -> ErrorCompilador {
        ErrorCompilador::nuevo(
            CategoriaError::Sintaxis,
            2,
            span,
            "fin de archivo inesperado",
        )
    }

    // Errores de tipo
    pub fn tipo_no_encontrado(span: Span, tipo: &str) -> ErrorCompilador {
        ErrorCompilador::nuevo(
            CategoriaError::Tipo,
            1,
            span,
            format!("no se encontró el tipo '{}'", tipo),
        )
        .con_sugerencia(format!("¿quisiste decir '{}'?", tipo))
    }

    // Errores de FFI
    pub fn funcion_ffi_no_encontrada(span: Span, nombre: &str) -> ErrorCompilador {
        ErrorCompilador::nuevo(
            CategoriaError::FFI,
            1,
            span,
            format!("función FFI '{}' no encontrada", nombre),
        )
    }
}
