use std::sync::Arc;

/// Ubicación en el código fuente: línea, columna, offset
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Posicion {
    pub linea: u32,
    pub columna: u32,
    pub offset: u32,
}

impl Posicion {
    pub fn nueva(linea: u32, columna: u32, offset: u32) -> Self {
        Self {
            linea,
            columna,
            offset,
        }
    }
}

/// Span: rango de texto en el código fuente con referencia al archivo
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Span {
    pub inicio: Posicion,
    pub fin: Posicion,
    pub archivo: Arc<str>,
}

impl Span {
    pub fn nuevo(inicio: Posicion, fin: Posicion, archivo: impl Into<Arc<str>>) -> Self {
        Self {
            inicio,
            fin,
            archivo: archivo.into(),
        }
    }

    /// Span vacío para testing
    pub fn vacio() -> Self {
        Self {
            inicio: Posicion::nueva(0, 0, 0),
            fin: Posicion::nueva(0, 0, 0),
            archivo: Arc::from("<test>"),
        }
    }

    /// Combina dos spans: inicio del primero, fin del segundo
    pub fn combinar(a: &Span, b: &Span) -> Self {
        Self {
            inicio: a.inicio,
            fin: b.fin,
            archivo: Arc::clone(&a.archivo),
        }
    }
}

impl Default for Span {
    fn default() -> Self {
        Self::vacio()
    }
}
