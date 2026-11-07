//! Manifiesto falcato.toml — IO + validación + escritura atómica.
//! Responsabilidad única: leer/escribir TOML, nada de resolver.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::error::ErrorFardo;
use super::modelo::{Dependencia, FardoId, InfoPaquete, Manifiesto, NombreFardo, Origen, Permisos, Version};
use super::validacion::{unir_ruta_segura, validar_nombre_fardo, validar_version_semver};

// ── Límites anti-DoS ──────────────────────────────────────
const MAX_FARDOS: usize = 100;
const MAX_TOML_BYTES: usize = 200_000; // 200KB suficiente, evita 10MB malicioso
const MAX_NOMBRE_BYTES: usize = 31;

// ── Estructuras TOML crudas ───────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RawManifiesto {
    paquete: RawPaquete,
    #[serde(default)]
    fardos: Option<BTreeMap<String, TomlDependencia>>,
    #[serde(default)]
    dependencias: Option<BTreeMap<String, TomlDependencia>>, // alias hispano
    #[serde(default)]
    permisos: Option<RawPermisos>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RawPaquete {
    nombre: String,
    version: String,
    #[serde(default)]
    descripcion: String,
    #[serde(default)]
    edicion: String,
    #[serde(default)]
    licencia: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum TomlDependencia {
    Simple(String),
    Detallada {
        #[serde(default)]
        version: Option<String>,
        #[serde(default)]
        ruta: Option<String>,
        #[serde(default)]
        origen: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct RawPermisos {
    #[serde(default)]
    red: bool,
    #[serde(default)]
    archivos: bool,
    #[serde(default)]
    procesos: bool,
    #[serde(default)]
    terminal: bool,
}

// ── Helpers ───────────────────────────────────────────────

fn escribir_atomico(ruta: &Path, contenido: &str) -> Result<(), ErrorFardo> {
    let dir = ruta.parent().unwrap_or_else(|| Path::new("."));
    // crear dir si no existe (nido nuevo)
    if !dir.exists() {
        std::fs::create_dir_all(dir).map_err(|e| ErrorFardo::Escritura { ruta: dir.to_path_buf(), detalle: e.to_string() })?;
    }
    let tmp = dir.join(format!(".{}.tmp", ruta.file_name().and_then(|s| s.to_str()).unwrap_or("falcato")));
    std::fs::write(&tmp, contenido).map_err(|e| ErrorFardo::Escritura { ruta: tmp.clone(), detalle: e.to_string() })?;
    std::fs::rename(&tmp, ruta).map_err(|e| ErrorFardo::Escritura { ruta: ruta.to_path_buf(), detalle: e.to_string() })?;
    Ok(())
}

fn raw_a_dependencia(nombre_str: &str, raw: TomlDependencia) -> Result<Dependencia, ErrorFardo> {
    let nombre = NombreFardo::nuevo(nombre_str)?;
    match raw {
        TomlDependencia::Simple(req) => {
            if req.trim().is_empty() {
                return Err(ErrorFardo::VersionInvalida { version: req });
            }
            // si empieza con . o / o contiene / o \ → tratar como ruta implícita (ergonomía)
            if req.contains('/') || req.contains('\\') || req.starts_with('.') {
                let p = PathBuf::from(&req);
                // validación léxica de escape sin tocar disco
                if p.is_absolute() {
                    return Err(ErrorFardo::RutaEscape { ruta: p, base: PathBuf::from(".") });
                }
                // requisito por defecto ^0.1.0 si solo es ruta
                let restriccion = super::modelo::Restriccion::parsear("^0.1.0")?;
                return Ok(Dependencia { nombre, requisito: restriccion, origen: Origen::Ruta(p) });
            }
            let restriccion = super::modelo::Restriccion::parsear(&req)?;
            // F1: origen ruta solo si se especifica ruta; si no, asumimos ruta vacía (se resolverá como error en fuentes/ruta)
            // Para mantener compat, si es Simple y no es ruta, lo dejamos como Ruta vacía con requisito
            // El resolver pedirá ruta explícita; aquí guardamos como Registro stub si no hay ruta
            // Decisión: Simple = requisito de registro (F2), pero en F1 si no hay ruta falla en resolver con mensaje claro
            Ok(Dependencia { nombre, requisito: restriccion, origen: Origen::Registro { pubkey: String::new(), version_req: req } })
        }
        TomlDependencia::Detallada { version, ruta, origen } => {
            if let Some(ruta_str) = ruta {
                let p = PathBuf::from(ruta_str.trim());
                if p.as_os_str().is_empty() {
                    return Err(ErrorFardo::RutaNoExiste { ruta: p });
                }
                if p.to_string_lossy().starts_with('/') {
                    return Err(ErrorFardo::RutaEscape { ruta: p.clone(), base: PathBuf::from(".") });
                }
                let req_str = version.or(origen).unwrap_or_else(|| "^0.1.0".to_string());
                let restriccion = super::modelo::Restriccion::parsear(&req_str)?;
                return Ok(Dependencia { nombre, requisito: restriccion, origen: Origen::Ruta(p) });
            }
            // sin ruta → registro
            let req_str = version.or(origen).unwrap_or_else(|| "^0.1.0".to_string());
            let restriccion = super::modelo::Restriccion::parsear(&req_str)?;
            Ok(Dependencia { nombre, requisito: restriccion, origen: Origen::Registro { pubkey: String::new(), version_req: req_str } })
        }
    }
}

fn dependencia_a_raw(dep: &Dependencia) -> TomlDependencia {
    match &dep.origen {
        Origen::Ruta(p) => TomlDependencia::Detallada {
            version: Some(dep.requisito.requisito.clone()),
            ruta: Some(p.display().to_string()),
            origen: None,
        },
        Origen::Registro { version_req, .. } => TomlDependencia::Simple(version_req.clone()),
    }
}

// ── API pública ───────────────────────────────────────────

impl Manifiesto {
    /// Crea manifiesto mínimo.
    #[must_use]
    pub fn nuevo(nombre: &str, version: &str) -> Self {
        Self {
            paquete: InfoPaquete {
                nombre: nombre.to_string(),
                version: version.to_string(),
                descripcion: String::new(),
                edicion: "2024".to_string(),
                licencia: "MIT".to_string(),
            },
            fardos: BTreeMap::new(),
            permisos: Permisos::default(),
        }
    }

    /// Busca falcato.toml hacia arriba desde `dir`.
    #[must_use]
    pub fn buscar_en(dir: &Path) -> Option<PathBuf> {
        let mut cur = Some(dir.to_path_buf());
        while let Some(d) = cur {
            let cand = d.join("falcato.toml");
            if cand.exists() {
                return Some(cand);
            }
            cur = d.parent().map(|p| p.to_path_buf());
        }
        None
    }

    /// Lee y valida desde archivo.
    pub fn desde_archivo(ruta: &Path) -> Result<Self, ErrorFardo> {
        let meta = std::fs::metadata(ruta).map_err(|_| ErrorFardo::ManifiestoNoEncontrado { ruta: ruta.to_path_buf() })?;
        if meta.len() as usize > MAX_TOML_BYTES {
            return Err(ErrorFardo::TomlInvalido { ruta: ruta.to_path_buf(), detalle: format!("TOML excede {} bytes", MAX_TOML_BYTES) });
        }
        let contenido = std::fs::read_to_string(ruta).map_err(|e| ErrorFardo::Lectura { ruta: ruta.to_path_buf(), detalle: e.to_string() })?;
        Self::desde_str(&contenido, ruta)
    }

    /// Parsea desde string (útil para tests y LSP).
    pub fn desde_str(contenido: &str, ruta: &Path) -> Result<Self, ErrorFardo> {
        if contenido.len() > MAX_TOML_BYTES {
            return Err(ErrorFardo::TomlInvalido { ruta: ruta.to_path_buf(), detalle: "TOML demasiado grande".to_string() });
        }
        let raw: RawManifiesto = toml::from_str(contenido).map_err(|e| ErrorFardo::TomlInvalido { ruta: ruta.to_path_buf(), detalle: e.to_string() })?;

        // validar paquete
        validar_nombre_fardo(&raw.paquete.nombre)?;
        validar_version_semver(&raw.paquete.version)?;
        if raw.paquete.nombre.len() > MAX_NOMBRE_BYTES {
            return Err(ErrorFardo::NombreInvalido { nombre: raw.paquete.nombre });
        }

        // unificar fardos + dependencias (alias)
        let mut mapa_raw = BTreeMap::new();
        if let Some(m) = raw.fardos {
            mapa_raw.extend(m);
        }
        if let Some(m) = raw.dependencias {
            for (k, v) in m {
                // si ya existe en fardos, dependencias tiene prioridad? mantenemos primero
                mapa_raw.entry(k).or_insert(v);
            }
        }
        if mapa_raw.len() > MAX_FARDOS {
            return Err(ErrorFardo::TomlInvalido { ruta: ruta.to_path_buf(), detalle: format!("demasiados fardos: {} > {}", mapa_raw.len(), MAX_FARDOS) });
        }

        let mut fardos = BTreeMap::new();
        for (k, v) in mapa_raw {
            if fardos.contains_key(&NombreFardo::nuevo(&k)?) {
                return Err(ErrorFardo::DependenciaDuplicada { nombre: k });
            }
            let dep = raw_a_dependencia(&k, v)?;
            fardos.insert(dep.nombre.clone(), dep);
        }

        let permisos = raw.permisos.map(|r| Permisos { red: r.red, archivos: r.archivos, procesos: r.procesos, terminal: r.terminal }).unwrap_or_default();

        Ok(Self {
            paquete: InfoPaquete {
                nombre: raw.paquete.nombre,
                version: raw.paquete.version,
                descripcion: raw.paquete.descripcion,
                edicion: if raw.paquete.edicion.is_empty() { "2024".to_string() } else { raw.paquete.edicion },
                licencia: raw.paquete.licencia,
            },
            fardos,
            permisos,
        })
    }

    /// Serializa a TOML estético.
    pub fn a_toml(&self) -> Result<String, ErrorFardo> {
        // construir raw
        let mut raw_fardos = BTreeMap::new();
        for (k, dep) in &self.fardos {
            raw_fardos.insert(k.as_str().to_string(), dependencia_a_raw(dep));
        }
        let raw = RawManifiesto {
            paquete: RawPaquete {
                nombre: self.paquete.nombre.clone(),
                version: self.paquete.version.clone(),
                descripcion: self.paquete.descripcion.clone(),
                edicion: self.paquete.edicion.clone(),
                licencia: self.paquete.licencia.clone(),
            },
            fardos: if raw_fardos.is_empty() { None } else { Some(raw_fardos) },
            dependencias: None,
            permisos: Some(RawPermisos { red: self.permisos.red, archivos: self.permisos.archivos, procesos: self.permisos.procesos, terminal: self.permisos.terminal }),
        };
        toml::to_string_pretty(&raw).map_err(|e| ErrorFardo::Interno { detalle: e.to_string() })
    }

    /// Guarda atómicamente en `ruta`.
    pub fn guardar(&self, ruta: &Path) -> Result<(), ErrorFardo> {
        let toml = self.a_toml()?;
        escribir_atomico(ruta, &toml)
    }

    /// Crea proyecto nuevo en `dir`.
    pub fn iniciar_proyecto(dir: &Path, nombre: Option<&str>) -> Result<PathBuf, ErrorFardo> {
        if !dir.exists() {
            std::fs::create_dir_all(dir).map_err(|e| ErrorFardo::Escritura { ruta: dir.to_path_buf(), detalle: e.to_string() })?;
        }
        let ruta_manifiesto = dir.join("falcato.toml");
        if ruta_manifiesto.exists() {
            return Err(ErrorFardo::Escritura { ruta: ruta_manifiesto, detalle: "ya existe falcato.toml".to_string() });
        }
        let nombre_paq = match nombre {
            Some(n) => n.to_string(),
            None => dir.file_name().and_then(|s| s.to_str()).map(|s| s.to_string()).unwrap_or_else(|| "nido".to_string()),
        };
        validar_nombre_fardo(&nombre_paq)?;
        let m = Self::nuevo(&nombre_paq, "0.1.0");
        m.guardar(&ruta_manifiesto)?;

        // esqueleto src/
        let src = dir.join("src");
        if !src.exists() {
            std::fs::create_dir_all(&src).map_err(|e| ErrorFardo::Escritura { ruta: src.clone(), detalle: e.to_string() })?;
            let lib = src.join("lib.fc");
            if !lib.exists() {
                let contenido = "función hola() -> Entero32 {\n    retornar 42;\n}\n";
                std::fs::write(&lib, contenido).map_err(|e| ErrorFardo::Escritura { ruta: lib, detalle: e.to_string() })?;
            }
        }
        // principal.fc si no existe
        let principal = dir.join("src").join("principal.fc");
        if !principal.exists() && !dir.join("principal.fc").exists() {
            let p = dir.join("principal.fc");
            if !p.exists() {
                let _ = std::fs::write(&p, "función principal() {\n    imprimir(\"¡Hola, bandada!\");\n}\n");
            }
        }
        Ok(ruta_manifiesto)
    }

    /// Añade dependencia con ruta (F1).
    pub fn agregar_ruta(&mut self, nombre: &str, ruta_relativa: &Path, requisito: &str) -> Result<(), ErrorFardo> {
        let n = NombreFardo::nuevo(nombre)?;
        if self.fardos.contains_key(&n) {
            return Err(ErrorFardo::DependenciaDuplicada { nombre: nombre.to_string() });
        }
        if self.fardos.len() >= MAX_FARDOS {
            return Err(ErrorFardo::TomlInvalido { ruta: PathBuf::from("falcato.toml"), detalle: "límite de fardos alcanzado".to_string() });
        }
        // validar requisito
        let restriccion = super::modelo::Restriccion::parsear(requisito)?;
        if ruta_relativa.as_os_str().is_empty() {
            return Err(ErrorFardo::RutaNoExiste { ruta: ruta_relativa.to_path_buf() });
        }
        // Rechazar POSIX "/etc" (inyección), permitir Windows "C:\"
        if ruta_relativa.to_string_lossy().starts_with('/') {
            return Err(ErrorFardo::RutaEscape { ruta: ruta_relativa.to_path_buf(), base: PathBuf::from(".") });
        }
        let dep = Dependencia { nombre: n.clone(), requisito: restriccion, origen: Origen::Ruta(ruta_relativa.to_path_buf()) };
        self.fardos.insert(n, dep);
        Ok(())
    }
}

// Re-export para tests
#[cfg(test)]
pub(crate) fn raw_a_dependencia_test(k: &str, v: TomlDependencia) -> Result<Dependencia, ErrorFardo> {
    raw_a_dependencia(k, v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn roundtrip_basico() {
        let mut m = Manifiesto::nuevo("mi-nido", "0.1.0");
        m.paquete.descripcion = "prueba".to_string();
        m.permisos.red = true;
        m.agregar_ruta("util", Path::new("../util"), "^0.1.0").unwrap();
        let toml = m.a_toml().unwrap();
        let m2 = Manifiesto::desde_str(&toml, Path::new("falcato.toml")).unwrap();
        assert_eq!(m2.paquete.nombre, "mi-nido");
        assert!(m2.permisos.red);
        assert_eq!(m2.fardos.len(), 1);
    }

    #[test]
    fn alias_dependencias() {
        let toml = r#"
[paquete]
nombre = "ab"
version = "0.1.0"

[dependencias]
viejo = "0.1.0"
"#;
        let m = Manifiesto::desde_str(toml, Path::new("falcato.toml")).unwrap();
        assert_eq!(m.fardos.len(), 1);
    }

    #[test]
    fn limite_fardos() {
        let mut toml = String::from("[paquete]\nnombre=\"a\"\nversion=\"0.1.0\"\n\n[fardos]\n");
        for i in 0..101 {
            toml.push_str(&format!("f{} = \"0.1.0\"\n", i));
        }
        let r = Manifiesto::desde_str(&toml, Path::new("falcato.toml"));
        assert!(r.is_err());
    }

    #[test]
    fn escritura_atomica() {
        let dir = tempdir().unwrap();
        let ruta = dir.path().join("falcato.toml");
        let m = Manifiesto::nuevo("atomico", "0.2.0");
        m.guardar(&ruta).unwrap();
        assert!(ruta.exists());
        let m2 = Manifiesto::desde_archivo(&ruta).unwrap();
        assert_eq!(m2.paquete.nombre, "atomico");
        // no queda .tmp
        assert!(!dir.path().join(".falcato.toml.tmp").exists());
    }

    #[test]
    fn iniciar_proyecto() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("mi-proyecto");
        Manifiesto::iniciar_proyecto(&p, None).unwrap();
        assert!(p.join("falcato.toml").exists());
        assert!(p.join("src").exists());
        // segunda vez falla
        assert!(Manifiesto::iniciar_proyecto(&p, None).is_err());
    }

    #[test]
    fn ruta_escape_rechazada() {
        let mut m = Manifiesto::nuevo("a", "0.1.0");
        assert!(m.agregar_ruta("mal", Path::new("/etc/passwd"), "0.1.0").is_err());
    }
}
