//! Errores de fardos — códigos [F001..F099]
//! Todo en español, sin unwrap, con thiserror.

use thiserror::Error;
use std::path::PathBuf;

/// Errores del sistema de fardos.
#[derive(Debug, Error)]
pub enum ErrorFardo {
    #[error("[F001] nombre de fardo inválido '{nombre}': debe ser kebab-case 2..31 chars, empieza con letra, solo [a-z0-9-_], sin '--' ni '-_'")]
    NombreInvalido { nombre: String },

    #[error("[F002] versión inválida '{version}': debe ser semver MAYOR.menor.parche (ej: 0.1.0)")]
    VersionInvalida { version: String },

    #[error("[F003] no se encontró falcato.toml en '{ruta}' ni en padres")]
    ManifiestoNoEncontrado { ruta: PathBuf },

    #[error("[F004] ruta '{ruta}' escapa del nido '{base}': canonicalize fuera de base")]
    RutaEscape { ruta: PathBuf, base: PathBuf },

    #[error("[F005] ruta '{ruta}' no existe o no es directorio")]
    RutaNoExiste { ruta: PathBuf },

    #[error("[F006] no se pudo leer '{ruta}': {detalle}")]
    Lectura { ruta: PathBuf, detalle: String },

    #[error("[F007] no se pudo escribir '{ruta}': {detalle}")]
    Escritura { ruta: PathBuf, detalle: String },

    #[error("[F008] TOML inválido en '{ruta}': {detalle}")]
    TomlInvalido { ruta: PathBuf, detalle: String },

    #[error("[F009] dependencia duplicada '{nombre}'")]
    DependenciaDuplicada { nombre: String },

    #[error("[F010] ciclo detectado: {ciclo}")]
    Ciclo { ciclo: String },

    #[error("[F011] versión '{version}' no cumple requisito '{requisito}' para '{nombre}'")]
    VersionNoCumple { nombre: String, requisito: String, version: String },

    #[error("[F012] posible zip bomb: ratio sospechoso en '{ruta}'")]
    ZipBomb { ruta: PathBuf },

    #[error("[F013] ruta fuera del nido al extraer: '{ruta}'")]
    ZipSlip { ruta: PathBuf },

    #[error("[F014] hash inválido '{hash}': debe ser hex 64 (sha256)")]
    HashInvalido { hash: String },

    #[error("[F020] error interno de fardo: {detalle}")]
    Interno { detalle: String },
}

impl ErrorFardo {
    /// Código sin corchetes, ej "F001".
    #[must_use]
    pub fn codigo(&self) -> &'static str {
        match self {
            Self::NombreInvalido { .. } => "F001",
            Self::VersionInvalida { .. } => "F002",
            Self::ManifiestoNoEncontrado { .. } => "F003",
            Self::RutaEscape { .. } => "F004",
            Self::RutaNoExiste { .. } => "F005",
            Self::Lectura { .. } => "F006",
            Self::Escritura { .. } => "F007",
            Self::TomlInvalido { .. } => "F008",
            Self::DependenciaDuplicada { .. } => "F009",
            Self::Ciclo { .. } => "F010",
            Self::VersionNoCumple { .. } => "F011",
            Self::ZipBomb { .. } => "F012",
            Self::ZipSlip { .. } => "F013",
            Self::HashInvalido { .. } => "F014",
            Self::Interno { .. } => "F020",
        }
    }
}
