//! # Sistema de paquetes (R8.1) — formato + CLI + resolver semver
//!
//! Ecosistema estilo crates.io pero **sin registry central** (R8 completo).
//! Esta fase define el contrato:
//!
//! - `falcato.toml`  — manifiesto del paquete (nombre, versión, deps, permisos)
//! - `falcato.lock`  — árbol de dependencias resuelto + hashes
//! - CLI: `falcato paquete init/new/add/actualizar`
//! - Resolver semver (>=1.0.0, <2.0.0, ~1.2.0)
//!
//! El transporte P2P (DHT/torrent) es R8.2 — aquí solo formato + resolución.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ============================================================
// Manifiesto falcato.toml
// ============================================================

/// Manifiesto completo de un paquete.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifiesto {
    #[serde(default)]
    pub paquete: InfoPaquete,
    #[serde(default)]
    pub dependencias: HashMap<String, String>,
    #[serde(default)]
    pub permisos: Permisos,
}

/// Información básica del paquete.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfoPaquete {
    pub nombre: String,
    pub version: String,
    #[serde(default)]
    pub descripcion: String,
    #[serde(default)]
    pub autor: String,
    #[serde(default)]
    pub licencia: String,
}

impl Default for InfoPaquete {
    fn default() -> Self {
        Self {
            nombre: String::new(),
            version: String::new(),
            descripcion: String::new(),
            autor: String::new(),
            licencia: String::new(),
        }
    }
}

/// Permisos declarados (Capa 4 de R8.3 — "tipos como permisos").
/// Buckets sencillos e intuitivos.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Permisos {
    #[serde(default)]
    pub red: bool,
    #[serde(default)]
    pub archivos: bool,
    #[serde(default)]
    pub procesos: bool,
    #[serde(default)]
    pub terminal: bool,
}

impl Manifiesto {
    /// Crea un manifiesto nuevo con valores por defecto.
    pub fn nuevo(nombre: &str, version: &str) -> Self {
        Self {
            paquete: InfoPaquete {
                nombre: nombre.to_string(),
                version: version.to_string(),
                ..Default::default()
            },
            dependencias: HashMap::new(),
            permisos: Permisos::default(),
        }
    }

    /// Lee y parsea un falcato.toml.
    pub fn desde_archivo(ruta: &Path) -> anyhow::Result<Self> {
        let contenido = std::fs::read_to_string(ruta)
            .map_err(|e| anyhow::anyhow!("No se pudo leer '{}': {}", ruta.display(), e))?;
        let m: Manifiesto = toml::from_str(&contenido)
            .map_err(|e| anyhow::anyhow!("Error parseando '{}': {}", ruta.display(), e))?;
        Ok(m)
    }

    /// Serializa a TOML.
    pub fn a_toml(&self) -> anyhow::Result<String> {
        Ok(toml::to_string_pretty(self)?)
    }

    /// Busca el falcato.toml en un directorio (o sube hasta la raíz).
    pub fn buscar_en(dir: &Path) -> Option<PathBuf> {
        let mut actual = Some(dir.to_path_buf());
        while let Some(d) = actual {
            let candidato = d.join("falcato.toml");
            if candidato.exists() {
                return Some(candidato);
            }
            actual = d.parent().map(|p| p.to_path_buf());
        }
        None
    }
}

// ============================================================
// Lock file falcato.lock
// ============================================================

/// Paquete resuelto en el lock.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaqueteResuelto {
    pub nombre: String,
    pub version: String,
    /// Hash del contenido (blake3/sha256, se llena en R8.2 al descargar)
    #[serde(default)]
    pub hash: String,
    /// Ruta local donde está instalado (caché)
    #[serde(default)]
    pub origen: String,
}

/// Lock file: árbol resuelto inmutable.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LockFile {
    #[serde(default)]
    pub paquetes: Vec<PaqueteResuelto>,
}

impl LockFile {
    pub fn desde_archivo(ruta: &Path) -> anyhow::Result<Self> {
        if !ruta.exists() {
            return Ok(Self::default());
        }
        let contenido = std::fs::read_to_string(ruta)
            .map_err(|e| anyhow::anyhow!("No se pudo leer '{}': {}", ruta.display(), e))?;
        Ok(toml::from_str(&contenido)?)
    }

    pub fn guardar(&self, ruta: &Path) -> anyhow::Result<()> {
        let contenido = toml::to_string_pretty(self)?;
        std::fs::write(ruta, contenido)?;
        Ok(())
    }
}

// ============================================================
// Resolución semver (subset suficiente)
// ============================================================

/// Representa una restricción de versión: `1.2.3`, `>=1.0`, `<2.0`, `~1.2`.
#[derive(Debug, Clone, PartialEq)]
pub struct Restriccion {
    pub mayor: Option<u64>,
    pub menor: Option<u64>,
    pub parche: Option<u64>,
    pub minimo: bool,
    pub max_mayor: Option<u64>,
}

impl Restriccion {
    /// Parsea una restricción semver simple.
    pub fn parsear(s: &str) -> Self {
        let s = s.trim();
        // Rangos con operadores
        if s.starts_with(">=") {
            let v = parse_version(&s[2..]);
            return Self { mayor: v.0, menor: v.1, parche: v.2, minimo: true, max_mayor: None };
        }
        if s.starts_with("^") {
            let v = parse_version(&s[1..]);
            return Self { mayor: v.0, menor: v.1, parche: v.2, minimo: true, max_mayor: v.0.map(|m| m + 1) };
        }
        if s.starts_with("~") {
            let v = parse_version(&s[1..]);
            return Self { mayor: v.0, menor: v.1, parche: v.2, minimo: true, max_mayor: v.0.map(|m| m + 1) };
        }
        // "<2.0"
        if s.starts_with('<') {
            let v = parse_version(&s[1..]);
            return Self { mayor: None, menor: None, parche: None, minimo: false, max_mayor: v.0 };
        }
        // Versión exacta o "1.2" (compatible con 1.x)
        let v = parse_version(s);
        if v.1.is_some() {
            Self { mayor: v.0, menor: v.1, parche: v.2, minimo: true, max_mayor: v.0.map(|m| m + 1) }
        } else {
            Self { mayor: v.0, menor: v.1, parche: v.2, minimo: true, max_mayor: None }
        }
    }

    /// ¿La versión dada cumple la restricción?
    pub fn cumple(&self, version: &str) -> bool {
        let (m, mn, p) = parse_version(version);
        let m = m.unwrap_or(0);
        let mn = mn.unwrap_or(0);
        let p = p.unwrap_or(0);
        if let Some(req_mayor) = self.mayor {
            if m != req_mayor {
                return false;
            }
        }
        if let Some(req_menor) = self.menor {
            if mn < req_menor {
                return false;
            }
            // Si el menor es igual, el parche solo importa cuando también es igual el menor
            if mn == req_menor {
                if let Some(req_parche) = self.parche {
                    if p < req_parche {
                        return false;
                    }
                }
            }
        }
        if let Some(max) = self.max_mayor {
            if m >= max {
                return false;
            }
        }
        true
    }
}

fn parse_version(s: &str) -> (Option<u64>, Option<u64>, Option<u64>) {
    let s = s.trim().trim_start_matches('v');
    let partes: Vec<&str> = s.split('.').collect();
    let mayor = partes.get(0).and_then(|p| p.parse().ok());
    let menor = partes.get(1).and_then(|p| p.parse().ok());
    let parche = partes.get(2).and_then(|p| p.parse().ok());
    (mayor, menor, parche)
}

/// Compara dos versiones: >0 si a es mayor, 0 si igual, <0 si menor.
pub fn comparar_versiones(a: &str, b: &str) -> i32 {
    let (ma, mna, pa) = parse_version(a);
    let (mb, mnb, pb) = parse_version(b);
    let ma = ma.unwrap_or(0);
    let mb = mb.unwrap_or(0);
    if ma != mb { return if ma > mb { 1 } else { -1 }; }
    let mna = mna.unwrap_or(0);
    let mnb = mnb.unwrap_or(0);
    if mna != mnb { return if mna > mnb { 1 } else { -1 }; }
    let pa = pa.unwrap_or(0);
    let pb = pb.unwrap_or(0);
    if pa != pb { return if pa > pb { 1 } else { -1 }; }
    0
}

// ============================================================
// CLI helpers
// ============================================================

/// Crea un proyecto nuevo con falcato.toml.
pub fn iniciar_proyecto(dir: &Path, nombre: Option<&str>) -> anyhow::Result<()> {
    if !dir.exists() {
        std::fs::create_dir_all(dir)?;
    }
    let ruta_manifiesto = dir.join("falcato.toml");
    if ruta_manifiesto.exists() {
        anyhow::bail!("Ya existe falcato.toml en {}", dir.display());
    }
    let nombre_paq = match nombre {
        Some(n) => n.to_string(),
        None => dir.file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "paquete".to_string()),
    };
    let m = Manifiesto::nuevo(&nombre_paq, "0.1.0");
    std::fs::write(&ruta_manifiesto, m.a_toml()?)?;
    // Estructura estándar
    let src = dir.join("src");
    if !src.exists() {
        std::fs::create_dir_all(&src)?;
        let lib = src.join("lib.fc");
        if !lib.exists() {
            std::fs::write(&lib, "función hola() -> Entero32 {\n    retornar 42;\n}\n")?;
        }
    }
    // Lock vacío
    let lock = LockFile::default();
    lock.guardar(&dir.join("falcato.lock"))?;
    Ok(())
}

/// Añade una dependencia al manifiesto (sin resolver ni descargar aún).
pub fn agregar_dependencia(dir: &Path, nombre: &str, restriccion: &str) -> anyhow::Result<()> {
    let ruta = Manifiesto::buscar_en(dir)
        .ok_or_else(|| anyhow::anyhow!("No se encontró falcato.toml en {} o padres", dir.display()))?;
    let mut m = Manifiesto::desde_archivo(&ruta)?;
    if m.paquetes_restringidos().contains_key(nombre) {
        anyhow::bail!("El paquete '{}' ya está en dependencias", nombre);
    }
    m.dependencias.insert(nombre.to_string(), restriccion.to_string());
    std::fs::write(&ruta, m.a_toml()?)?;
    println!("[Falcato] Dependencia añadida: {} = \"{}\"", nombre, restriccion);
    println!("[Falcato] (R8.2 pendiente: descarga P2P por DHT/torrent)");
    Ok(())
}

impl Manifiesto {
    fn paquetes_restringidos(&self) -> &HashMap<String, String> {
        &self.dependencias
    }
}

/// Resuelve las dependencias del manifiesto contra una lista de versiones
/// disponibles. Devuelve el lock actualizado.
///
/// `versiones_disponibles`: nombre → lista de versiones (de R8.2 vendrá de la DHT;
/// por ahora de un índice local o caché).
pub fn resolver_dependencias(
    m: &Manifiesto,
    versiones_disponibles: &HashMap<String, Vec<String>>,
) -> anyhow::Result<Vec<PaqueteResuelto>> {
    let mut resueltos: Vec<PaqueteResuelto> = Vec::new();

    for (nombre, restriccion_str) in &m.dependencias {
        let restriccion = Restriccion::parsear(restriccion_str);
        let disponibles = versiones_disponibles.get(nombre);
        let version_ok = disponibles
            .map(|vs| {
                let mut mejores: Vec<&String> = vs.iter()
                    .filter(|v| restriccion.cumple(v))
                    .collect();
                mejores.sort_by(|a, b| comparar_versiones(b, a).cmp(&0));
                mejores.first().map(|s| (*s).clone())
            })
            .flatten();

        match version_ok {
            Some(v) => {
                resueltos.push(PaqueteResuelto {
                    nombre: nombre.clone(),
                    version: v,
                    hash: String::new(), // se llena en R8.2
                    origen: String::new(),
                });
            }
            None => {
                anyhow::bail!(
                    "No se encontró versión de '{}' que cumpla '{}' (disponibles: {:?})",
                    nombre, restriccion_str, disponibles
                );
            }
        }
    }
    Ok(resueltos)
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_restriccion_exacta() {
        let r = Restriccion::parsear("1.2.3");
        assert!(r.cumple("1.2.3"));
        assert!(r.cumple("1.9.0")); // compatible con 1.x
        assert!(!r.cumple("2.0.0"));
        assert!(!r.cumple("0.9.0"));
    }

    #[test]
    fn test_restriccion_caret() {
        let r = Restriccion::parsear("^1.2.0");
        assert!(r.cumple("1.2.0"));
        assert!(r.cumple("1.9.9"));
        assert!(!r.cumple("2.0.0"));
        assert!(!r.cumple("1.1.9"));
    }

    #[test]
    fn test_restriccion_mayor_igual() {
        let r = Restriccion::parsear(">=2.0");
        assert!(r.cumple("2.5.0"));
        assert!(!r.cumple("1.9.0"));
    }

    #[test]
    fn test_restriccion_menor() {
        let r = Restriccion::parsear("<3.0");
        assert!(r.cumple("2.9.9"));
        assert!(!r.cumple("3.0.0"));
    }

    #[test]
    fn test_comparar() {
        assert!(comparar_versiones("2.0.0", "1.9.0") > 0);
        assert!(comparar_versiones("1.2.0", "1.2.1") < 0);
        assert_eq!(comparar_versiones("1.2.3", "1.2.3"), 0);
    }

    #[test]
    fn test_resolver() {
        let mut m = Manifiesto::nuevo("test", "0.1.0");
        m.dependencias.insert("json".to_string(), "^1.0".to_string());
        let mut disponibles = HashMap::new();
        disponibles.insert("json".to_string(), vec!["0.9.0".to_string(), "1.0.0".to_string(), "1.5.0".to_string(), "2.0.0".to_string()]);

        let resueltos = resolver_dependencias(&m, &disponibles).unwrap();
        assert_eq!(resueltos.len(), 1);
        assert_eq!(resueltos[0].nombre, "json");
        assert_eq!(resueltos[0].version, "1.5.0"); // la mejor versión que cumple
    }

    #[test]
    fn test_manifiesto_roundtrip() {
        let mut m = Manifiesto::nuevo("mi_lib", "1.0.0");
        m.dependencias.insert("texto_util".to_string(), "0.2.0".to_string());
        m.permisos.red = true;
        let toml = m.a_toml().unwrap();
        let m2: Manifiesto = toml::from_str(&toml).unwrap();
        assert_eq!(m2.paquete.nombre, "mi_lib");
        assert!(m2.permisos.red);
        assert_eq!(m2.dependencias.get("texto_util").unwrap(), "0.2.0");
    }
}
