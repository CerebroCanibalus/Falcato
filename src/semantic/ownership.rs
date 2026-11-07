//! Análisis de ownership, borrowing y moves

use super::*;

impl AnalizadorSemantico {
    /// Verifica si un artículo es mutable
    pub(crate) fn es_mutable(&self, articulo: Articulo) -> bool {
        // el = owned mutable, la = borrowed immutable
        // un = optional (mutable by default)
        // los = shared ownership (mutable, reference-counted)
        // las = shared borrowed (inmutable, solo lectura)
        matches!(articulo, Articulo::El | Articulo::Un | Articulo::Los)
    }

    /// Convierte un artículo a string para mensajes de error
    pub(crate) fn articulo_a_str(&self, articulo: Articulo) -> &'static str {
        match articulo {
            Articulo::El => "el",
            Articulo::La => "la",
            Articulo::Un => "un",
            Articulo::Los => "los",
            Articulo::Las => "las",
        }
    }
}
