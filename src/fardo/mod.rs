//! Fardo — ecosistema P2P de Falcato (Plan Canónico v1.1)
//! Fase 1: Nido Local — sin red, solo `path`.
//! Arquitectura limpia, responsabilidades separadas, escalable a F2/F3.

pub mod bloqueo;
pub mod error;
pub mod manifiesto;
pub mod modelo;
pub mod validacion;

// Re-exports de dominio para el resto del compilador
pub use error::ErrorFardo;
pub use modelo::{Bloqueo, Dependencia, FardoBloqueado, FardoId, Manifiesto, NombreFardo, Origen, Version, Valoracion};
pub use validacion::{ruta_segura, unir_ruta_segura, validar_hash_sha256, validar_nombre_fardo, validar_version_semver};

pub mod cache;
pub mod formato;
pub mod fuentes;
pub mod resolver;
