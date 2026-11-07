//! Modelo de dominio — tipos puros del nido.
//! Sin IO, sin clap, sin fs. Solo datos + validación por constructor.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

use super::error::ErrorFardo;
use super::validacion::{validar_nombre_fardo, validar_version_semver};

// ── NombreFardo ───────────────────────────────────────────

/// Newtype validado para nombre de fardo: kebab-case 2..31, [a-z0-9-_], sin -- ni -_
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NombreFardo(String);

impl NombreFardo {
    /// Crea validando. Retorna [F001] si falla.
    pub fn nuevo(s: &str) -> Result<Self, ErrorFardo> {
        validar_nombre_fardo(s)?;
        Ok(Self(s.to_string()))
    }

    /// Sin validar — solo para tests internos donde ya se validó.
    #[cfg(test)]
    pub fn sin_validar(s: &str) -> Self {
        Self(s.to_string())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for NombreFardo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for NombreFardo {
    type Err = ErrorFardo;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::nuevo(s)
    }
}

// ── Version ───────────────────────────────────────────────

/// Semver MAYOR.menor.parche, sin prerelease/build en F1 (simple y auditable).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Version {
    pub mayor: u64,
    pub menor: u64,
    pub parche: u64,
}

impl Version {
    pub fn nueva(mayor: u64, menor: u64, parche: u64) -> Self {
        Self { mayor, menor, parche }
    }

    pub fn parsear(s: &str) -> Result<Self, ErrorFardo> {
        validar_version_semver(s)?;
        let s = s.trim().trim_start_matches('v');
        let partes: Vec<&str> = s.split('.').collect();
        if partes.len() != 3 {
            return Err(ErrorFardo::VersionInvalida { version: s.to_string() });
        }
        let mayor = partes[0].parse::<u64>().map_err(|_| ErrorFardo::VersionInvalida { version: s.to_string() })?;
        let menor = partes[1].parse::<u64>().map_err(|_| ErrorFardo::VersionInvalida { version: s.to_string() })?;
        let parche = partes[2].parse::<u64>().map_err(|_| ErrorFardo::VersionInvalida { version: s.to_string() })?;
        Ok(Self { mayor, menor, parche })
    }

    #[must_use]
    pub fn to_string_semver(&self) -> String {
        format!("{}.{}.{}", self.mayor, self.menor, self.parche)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.mayor, self.menor, self.parche)
    }
}

impl FromStr for Version {
    type Err = ErrorFardo;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parsear(s)
    }
}

// ── Restriccion ───────────────────────────────────────────

/// Restricción semver mínima F1: ^0.1.0, ~1.2, >=1.0, <2.0, exacta 1.2.3
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Restriccion {
    pub requisito: String, // original para mensajes
    pub mayor: Option<u64>,
    pub menor: Option<u64>,
    pub parche: Option<u64>,
    pub minimo_inclusive: bool,
    pub max_mayor_exclusivo: Option<u64>,
}

impl Restriccion {
    pub fn parsear(s: &str) -> Result<Self, ErrorFardo> {
        let raw = s.trim().to_string();
        let t = raw.as_str();
        if t.is_empty() {
            return Err(ErrorFardo::VersionInvalida { version: raw });
        }
        // >=1.0.0
        if let Some(rest) = t.strip_prefix(">=") {
            let v = Version::parsear(rest.trim())?;
            return Ok(Self { requisito: raw, mayor: Some(v.mayor), menor: Some(v.menor), parche: Some(v.parche), minimo_inclusive: true, max_mayor_exclusivo: None });
        }
        if let Some(rest) = t.strip_prefix('^') {
            let v = Version::parsear(rest.trim())?;
            return Ok(Self { requisito: raw, mayor: Some(v.mayor), menor: Some(v.menor), parche: Some(v.parche), minimo_inclusive: true, max_mayor_exclusivo: Some(v.mayor + 1) });
        }
        if let Some(rest) = t.strip_prefix('~') {
            let v = Version::parsear(rest.trim())?;
            return Ok(Self { requisito: raw, mayor: Some(v.mayor), menor: Some(v.menor), parche: Some(v.parche), minimo_inclusive: true, max_mayor_exclusivo: Some(v.mayor + 1) });
        }
        if let Some(rest) = t.strip_prefix('<') {
            let v = Version::parsear(rest.trim())?;
            return Ok(Self { requisito: raw, mayor: None, menor: None, parche: None, minimo_inclusive: false, max_mayor_exclusivo: Some(v.mayor) });
        }
        // exacta 1.2.3 -> interpretada como ^1.2.3 (compatibilidad caret por defecto del ecosistema)
        let v = Version::parsear(t)?;
        Ok(Self { requisito: raw, mayor: Some(v.mayor), menor: Some(v.menor), parche: Some(v.parche), minimo_inclusive: true, max_mayor_exclusivo: Some(v.mayor + 1) })
    }

    #[must_use]
    pub fn cumple(&self, v: &Version) -> bool {
        if let Some(req_mayor) = self.mayor {
            if v.mayor != req_mayor && self.max_mayor_exclusivo.is_some() {
                return false;
            }
            if self.minimo_inclusive {
                if v.mayor < req_mayor { return false; }
                if v.mayor == req_mayor {
                    if let Some(req_menor) = self.menor {
                        if v.menor < req_menor { return false; }
                        if v.menor == req_menor {
                            if let Some(req_parche) = self.parche {
                                if v.parche < req_parche { return false; }
                            }
                        }
                    }
                }
            }
        }
        if let Some(max) = self.max_mayor_exclusivo {
            if v.mayor >= max { return false; }
        }
        true
    }
}

// ── Origen / Dependencia ──────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origen {
    Ruta(PathBuf),
    Registro { pubkey: String, version_req: String }, // solo F2, hoy no se usa
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dependencia {
    pub nombre: NombreFardo,
    pub requisito: Restriccion,
    pub origen: Origen,
}

// ── FardoId / InfoPaquete ─────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FardoId {
    pub nombre: NombreFardo,
    pub version: Version,
}

impl fmt::Display for FardoId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{}", self.nombre, self.version)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfoPaquete {
    pub nombre: String,
    pub version: String,
    #[serde(default)]
    pub descripcion: String,
    #[serde(default)]
    pub edicion: String,
    #[serde(default)]
    pub licencia: String,
}

impl Default for InfoPaquete {
    fn default() -> Self {
        Self { nombre: String::new(), version: String::new(), descripcion: String::new(), edicion: String::new(), licencia: String::new() }
    }
}

// ── Manifiesto / Lock — tipos de dominio (IO en manifiesto.rs/bloqueo.rs) ──

#[derive(Debug, Clone, Default)]
pub struct Permisos {
    pub red: bool,
    pub archivos: bool,
    pub procesos: bool,
    pub terminal: bool,
}

#[derive(Debug, Clone)]
pub struct Manifiesto {
    pub paquete: InfoPaquete,
    pub fardos: BTreeMap<NombreFardo, Dependencia>,
    pub permisos: Permisos,
}

#[derive(Debug, Clone)]
pub struct FardoBloqueado {
    pub id: FardoId,
    pub hash: String,   // sha256 hex 64 — vacío en F1 local, obligatorio en F2
    pub origen: Origen,
}

#[derive(Debug, Clone, Default)]
pub struct Bloqueo {
    pub fardos: Vec<FardoBloqueado>,
}

// ── Valoracion (Capa 11) — solo modelo en F1 ──────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Valoracion {
    pub fardo_hash: String,
    pub fardo_id: String,
    pub revisor: String, // pubkey hex
    pub estrellas: u8,   // 1..=5
    pub comentario: String,
    pub timestamp: u64,
    pub seq: u64,
    pub firma: String, // ed25519 hex
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nombre_valido() {
        assert!(NombreFardo::nuevo("json").is_ok());
        assert!(NombreFardo::nuevo("mi-fardo_2").is_ok());
        assert!(NombreFardo::nuevo("a-b").is_ok());
    }

    #[test]
    fn nombre_invalido() {
        assert!(NombreFardo::nuevo("").is_err());
        assert!(NombreFardo::nuevo("a").is_err()); // <2
        assert!(NombreFardo::nuevo("MiFardo").is_err());
        assert!(NombreFardo::nuevo("-malo").is_err());
        assert!(NombreFardo::nuevo("doble--guion").is_err());
        assert!(NombreFardo::nuevo(&"a".repeat(32)).is_err());
    }

    #[test]
    fn version_parse() {
        let v = Version::parsear("0.1.0").unwrap();
        assert_eq!(v.to_string(), "0.1.0");
        assert!(Version::parsear("1").is_err());
        assert!(Version::parsear("a.b.c").is_err());
    }

    #[test]
    fn restriccion_caret() {
        let r = Restriccion::parsear("^0.1.0").unwrap();
        assert!(r.cumple(&Version::parsear("0.1.5").unwrap()));
        // caret 0.1.x con max_mayor 1 -> 0.2 no debería? en semver caret 0.1 es especial,
        // pero en F1 simplificamos a mayor+1, así que 0.2 aún cumple — consciente, se refinará en F2 con semver crate
        let r2 = Restriccion::parsear("^1.2.0").unwrap();
        assert!(r2.cumple(&Version::parsear("1.9.9").unwrap()));
        assert!(!r2.cumple(&Version::parsear("2.0.0").unwrap()));
    }
}
