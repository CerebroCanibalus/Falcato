//! FuenteRegistro — stub F2. No implementado en F1, pero ya existe el archivo
//! para que el trait escale sin reestructurar.

use super::Fuente;
use crate::fardo::error::ErrorFardo;
use crate::fardo::modelo::{Dependencia, Manifiesto};
use std::path::Path;

pub struct FuenteRegistro;

impl Fuente for FuenteRegistro {
    fn nombre(&self) -> &'static str {
        "registro"
    }

    fn resolver(&self, _dep: &Dependencia, _base: &Path) -> Result<(Manifiesto, std::path::PathBuf), ErrorFardo> {
        Err(ErrorFardo::Interno { detalle: "FuenteRegistro no disponible en F1 (requiere DHT F2)".to_string() })
    }
}
