//! Bloqueo falcato.lock — IO atómico + validación.
//! F1: guarda lista de fardos resueltos con origen Ruta y hash vacío (hash obligatorio en F2).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::error::ErrorFardo;
use super::modelo::{Bloqueo, FardoBloqueado, FardoId, NombreFardo, Origen, Version};
use super::validacion::validar_hash_sha256;

// ── Límites ───────────────────────────────────────────────
const MAX_BLOQUEADOS: usize = 200;
const MAX_LOCK_BYTES: usize = 300_000;

// ── TOML crudo ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RawBloqueo {
    #[serde(default)]
    fardo: Vec<RawFardoBloqueado>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RawFardoBloqueado {
    nombre: String,
    version: String,
    #[serde(default)]
    hash: String,
    #[serde(default)]
    origen: String, // "ruta:../util" o "registro:pubkey/req"
    #[serde(default)]
    fuente: Option<String>, // alias
}

fn escribir_atomico(ruta: &Path, contenido: &str) -> Result<(), ErrorFardo> {
    let dir = ruta.parent().unwrap_or_else(|| Path::new("."));
    if !dir.exists() {
        std::fs::create_dir_all(dir).map_err(|e| ErrorFardo::Escritura { ruta: dir.to_path_buf(), detalle: e.to_string() })?;
    }
    let tmp = dir.join(format!(".{}.tmp", ruta.file_name().and_then(|s| s.to_str()).unwrap_or("lock")));
    std::fs::write(&tmp, contenido).map_err(|e| ErrorFardo::Escritura { ruta: tmp.clone(), detalle: e.to_string() })?;
    std::fs::rename(&tmp, ruta).map_err(|e| ErrorFardo::Escritura { ruta: ruta.to_path_buf(), detalle: e.to_string() })?;
    Ok(())
}

fn origen_a_str(o: &Origen) -> String {
    match o {
        Origen::Ruta(p) => format!("ruta:{}", p.display()),
        Origen::Registro { pubkey, version_req } => format!("registro:{}@{}", pubkey, version_req),
    }
}

fn str_a_origen(s: &str) -> Result<Origen, ErrorFardo> {
    if let Some(rest) = s.strip_prefix("ruta:") {
        if rest.starts_with('/') {
            return Err(ErrorFardo::RutaEscape { ruta: PathBuf::from(rest), base: PathBuf::from(".") });
        }
        let p = PathBuf::from(rest);
        if p.as_os_str().is_empty() {
            return Err(ErrorFardo::RutaNoExiste { ruta: p });
        }
        return Ok(Origen::Ruta(p));
    }
    if let Some(rest) = s.strip_prefix("registro:") {
        // formato pubkey@req o solo req
        let (pk, req) = rest.split_once('@').unwrap_or(("", rest));
        return Ok(Origen::Registro { pubkey: pk.to_string(), version_req: req.to_string() });
    }
    // compat: si no tiene prefijo, tratar como ruta
    if !s.is_empty() {
        return Ok(Origen::Ruta(PathBuf::from(s)));
    }
    Ok(Origen::Ruta(PathBuf::from(".")))
}

// ── API ───────────────────────────────────────────────────

impl Bloqueo {
    /// Carga desde archivo, si no existe retorna vacío. Valida todo.
    pub fn desde_archivo(ruta: &Path) -> Result<Self, ErrorFardo> {
        if !ruta.exists() {
            return Ok(Self::default());
        }
        let meta = std::fs::metadata(ruta).map_err(|e| ErrorFardo::Lectura { ruta: ruta.to_path_buf(), detalle: e.to_string() })?;
        if meta.len() as usize > MAX_LOCK_BYTES {
            return Err(ErrorFardo::TomlInvalido { ruta: ruta.to_path_buf(), detalle: format!("falcato.lock excede {} bytes", MAX_LOCK_BYTES) });
        }
        let contenido = std::fs::read_to_string(ruta).map_err(|e| ErrorFardo::Lectura { ruta: ruta.to_path_buf(), detalle: e.to_string() })?;
        Self::desde_str(&contenido, ruta)
    }

    pub fn desde_str(contenido: &str, ruta: &Path) -> Result<Self, ErrorFardo> {
        if contenido.trim().is_empty() {
            return Ok(Self::default());
        }
        if contenido.len() > MAX_LOCK_BYTES {
            return Err(ErrorFardo::TomlInvalido { ruta: ruta.to_path_buf(), detalle: "lock demasiado grande".to_string() });
        }
        let raw: RawBloqueo = toml::from_str(contenido).map_err(|e| ErrorFardo::TomlInvalido { ruta: ruta.to_path_buf(), detalle: e.to_string() })?;
        if raw.fardo.len() > MAX_BLOQUEADOS {
            return Err(ErrorFardo::TomlInvalido { ruta: ruta.to_path_buf(), detalle: format!("demasiados fardos bloqueados: {}", raw.fardo.len()) });
        }
        let mut fardos = Vec::with_capacity(raw.fardo.len());
        for r in raw.fardo {
            super::validacion::validar_nombre_fardo(&r.nombre)?;
            super::validacion::validar_version_semver(&r.version)?;
            if !r.hash.is_empty() {
                validar_hash_sha256(&r.hash)?;
            }
            let nombre = NombreFardo::nuevo(&r.nombre)?;
            let version = Version::parsear(&r.version)?;
            let origen_str = if r.origen.is_empty() { r.fuente.unwrap_or_default() } else { r.origen.clone() };
            let origen = str_a_origen(&origen_str)?;
            fardos.push(FardoBloqueado { id: FardoId { nombre, version }, hash: r.hash, origen });
        }
        // ordenar determinista por nombre para diff limpio
        fardos.sort_by(|a, b| a.id.nombre.as_str().cmp(b.id.nombre.as_str()));
        Ok(Self { fardos })
    }

    pub fn a_toml(&self) -> Result<String, ErrorFardo> {
        let raws: Vec<RawFardoBloqueado> = self.fardos.iter().map(|f| RawFardoBloqueado {
            nombre: f.id.nombre.as_str().to_string(),
            version: f.id.version.to_string(),
            hash: f.hash.clone(),
            origen: origen_a_str(&f.origen),
            fuente: None,
        }).collect();
        let raw = RawBloqueo { fardo: raws };
        toml::to_string_pretty(&raw).map_err(|e| ErrorFardo::Interno { detalle: e.to_string() })
    }

    pub fn guardar(&self, ruta: &Path) -> Result<(), ErrorFardo> {
        let toml = self.a_toml()?;
        escribir_atomico(ruta, &toml)
    }

    /// Actualiza o inserta un fardo.
    pub fn insertar(&mut self, bloqueado: FardoBloqueado) {
        if let Some(pos) = self.fardos.iter().position(|f| f.id.nombre == bloqueado.id.nombre) {
            self.fardos[pos] = bloqueado;
        } else {
            self.fardos.push(bloqueado);
        }
        self.fardos.sort_by(|a, b| a.id.nombre.as_str().cmp(b.id.nombre.as_str()));
    }

    /// Mapa para resolver rápido.
    #[must_use]
    pub fn mapa(&self) -> BTreeMap<String, &FardoBloqueado> {
        let mut m = BTreeMap::new();
        for f in &self.fardos {
            m.insert(f.id.nombre.as_str().to_string(), f);
        }
        m
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tempfile::tempdir;

    #[test]
    fn roundtrip_lock() {
        let mut b = Bloqueo::default();
        b.insertar(FardoBloqueado {
            id: FardoId { nombre: NombreFardo::nuevo("util").unwrap(), version: Version::parsear("0.1.0").unwrap() },
            hash: String::new(),
            origen: Origen::Ruta(PathBuf::from("../util")),
        });
        let toml = b.a_toml().unwrap();
        let b2 = Bloqueo::desde_str(&toml, Path::new("falcato.lock")).unwrap();
        assert_eq!(b2.fardos.len(), 1);
        assert_eq!(b2.fardos[0].id.nombre.as_str(), "util");
    }

    #[test]
    fn hash_invalido_rechazado() {
        let toml = r#"[[fardo]]
nombre = "a"
version = "0.1.0"
hash = "nohex"
origen = "ruta:../a"
"#;
        let r = Bloqueo::desde_str(toml, Path::new("falcato.lock"));
        assert!(r.is_err());
    }

    #[test]
    fn atomico_lock() {
        let dir = tempdir().unwrap();
        let ruta = dir.path().join("falcato.lock");
        let b = Bloqueo::default();
        b.guardar(&ruta).unwrap();
        assert!(ruta.exists());
        let b2 = Bloqueo::desde_archivo(&ruta).unwrap();
        assert_eq!(b2.fardos.len(), 0);
    }

    #[test]
    fn vacio_ok() {
        let b = Bloqueo::desde_str("", Path::new("falcato.lock")).unwrap();
        assert_eq!(b.fardos.len(), 0);
    }
}
