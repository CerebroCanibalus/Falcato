//! Fuentes de fardos — trait para escalabilidad F1→F2→F3.
//! F1: solo FuenteRuta. F2 añade FuenteRegistro (DHT) sin tocar resolver.

pub mod registro;
pub mod ruta;

use std::path::{Path, PathBuf};

use super::error::ErrorFardo;
use super::modelo::{Dependencia, Manifiesto};

/// Fuente capaz de resolver una dependencia a su manifiesto.
pub trait Fuente: Send + Sync {
    fn nombre(&self) -> &'static str;
    fn resolver(&self, dep: &Dependencia, base: &Path) -> Result<(Manifiesto, PathBuf), ErrorFardo>;
}
