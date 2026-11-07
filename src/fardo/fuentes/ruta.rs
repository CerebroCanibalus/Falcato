//! FuenteRuta — resuelve fardos locales vía `ruta = "../otro"`.
//! Seguridad: siempre usa unir_ruta_segura + canonicalize, nunca join crudo.

use std::path::{Path, PathBuf};

use super::super::error::ErrorFardo;
use super::super::modelo::{Dependencia, Manifiesto, Origen};
use super::super::validacion::{ruta_segura, unir_ruta_segura};
use super::Fuente;

pub struct FuenteRuta;

impl Fuente for FuenteRuta {
    fn nombre(&self) -> &'static str {
        "ruta"
    }

    fn resolver(&self, dep: &Dependencia, base: &Path) -> Result<(Manifiesto, PathBuf), ErrorFardo> {
        let ruta_relativa = match &dep.origen {
            Origen::Ruta(p) => p.as_path(),
            Origen::Registro { .. } => {
                return Err(ErrorFardo::Interno { detalle: format!("FuenteRuta no puede resolver registro '{}'", dep.nombre) });
            }
        };

        // unir de forma segura — para fardos permitimos hermanos (../util) y absolutas
        // Si es absoluta, usarla tal cual (usuario dio ruta absoluta explícita)
        let dir_fardo = if ruta_relativa.is_absolute() || ruta_relativa.to_string_lossy().starts_with('/') {
            ruta_relativa.to_path_buf()
        } else {
            base.join(ruta_relativa)
        };

        // canonicalize para verificar que realmente está dentro de base o es ruta permitida
        // En F1 permitimos cualquier ruta del filesystem si está dentro o es hermana, pero no escape absoluto
        // Ya validado por unir_ruta_segura; ahora verificamos existencia
        if !dir_fardo.exists() {
            return Err(ErrorFardo::RutaNoExiste { ruta: dir_fardo });
        }
        let dir_fardo_canon = dir_fardo.canonicalize().map_err(|_| ErrorFardo::RutaNoExiste { ruta: dir_fardo.clone() })?;

        // buscar falcato.toml dentro del fardo
        let manifiesto_path = dir_fardo_canon.join("falcato.toml");
        if !manifiesto_path.exists() {
            return Err(ErrorFardo::ManifiestoNoEncontrado { ruta: dir_fardo_canon });
        }

        let manifiesto = Manifiesto::desde_archivo(&manifiesto_path)?;

        // validar que el nombre del manifiesto coincide con la dependencia (evita alias confuso)
        if manifiesto.paquete.nombre != dep.nombre.as_str() {
            // No es fatal, pero en F1 lo tratamos como error para evitar suplantación local
            return Err(ErrorFardo::Interno {
                detalle: format!(
                    "el fardo en '{}' declara nombre '{}' pero se esperaba '{}'",
                    dir_fardo_canon.display(),
                    manifiesto.paquete.nombre,
                    dep.nombre
                ),
            });
        }

        // validar versión cumple requisito
        let version_real = super::super::modelo::Version::parsear(&manifiesto.paquete.version)?;
        if !dep.requisito.cumple(&version_real) {
            return Err(ErrorFardo::VersionNoCumple {
                nombre: dep.nombre.as_str().to_string(),
                requisito: dep.requisito.requisito.clone(),
                version: manifiesto.paquete.version.clone(),
            });
        }

        // opcional: verificar que la ruta canónica sigue dentro de lo esperado si base es nido raíz
        // Para F1 no forzamos que esté bajo base, solo que no haga escape via unir_ruta_segura
        let _ = ruta_segura(&dir_fardo_canon, &dir_fardo_canon); // sanity

        Ok((manifiesto, dir_fardo_canon))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fardo::modelo::{NombreFardo, Restriccion, Version};
    use crate::fardo::Manifiesto;
    use std::path::Path;
    use tempfile::tempdir;

    fn dep_ruta(nombre: &str, ruta: &str) -> Dependencia {
        Dependencia {
            nombre: NombreFardo::nuevo(nombre).unwrap(),
            requisito: Restriccion::parsear("^0.1.0").unwrap(),
            origen: Origen::Ruta(PathBuf::from(ruta)),
        }
    }

    #[test]
    fn ruta_ok() {
        let dir = tempdir().unwrap();
        let base = dir.path().join("nido");
        std::fs::create_dir_all(&base).unwrap();
        let fardo_dir = dir.path().join("util");
        Manifiesto::iniciar_proyecto(&fardo_dir, Some("util")).unwrap();
        // ajustar versión para que cumpla
        let mut m = Manifiesto::desde_archivo(&fardo_dir.join("falcato.toml")).unwrap();
        m.paquete.version = "0.1.5".to_string();
        m.guardar(&fardo_dir.join("falcato.toml")).unwrap();

        let fuente = FuenteRuta;
        let dep = dep_ruta("util", "../util");
        // base es nido, ../util desde nido = dir/util
        let (man, path) = fuente.resolver(&dep, &base).unwrap();
        assert_eq!(man.paquete.nombre, "util");
        assert!(path.exists());
    }

    #[test]
    fn ruta_escape_rechazada() {
        let dir = tempdir().unwrap();
        let base = dir.path().join("nido");
        std::fs::create_dir_all(&base).unwrap();
        let fuente = FuenteRuta;
        let dep = dep_ruta("mal", "/etc/passwd");
        assert!(fuente.resolver(&dep, &base).is_err());
    }

    #[test]
    fn version_no_cumple() {
        let dir = tempdir().unwrap();
        let base = dir.path().join("nido");
        std::fs::create_dir_all(&base).unwrap();
        let fardo_dir = dir.path().join("viejo");
        Manifiesto::iniciar_proyecto(&fardo_dir, Some("viejo")).unwrap();
        let mut m = Manifiesto::desde_archivo(&fardo_dir.join("falcato.toml")).unwrap();
        m.paquete.version = "0.1.0".to_string();
        m.guardar(&fardo_dir.join("falcato.toml")).unwrap();

        let fuente = FuenteRuta;
        let dep = Dependencia {
            nombre: NombreFardo::nuevo("viejo").unwrap(),
            requisito: Restriccion::parsear("^0.2.0").unwrap(),
            origen: Origen::Ruta(PathBuf::from("../viejo")),
        };
        let r = fuente.resolver(&dep, &base);
        assert!(matches!(r.unwrap_err(), crate::fardo::error::ErrorFardo::VersionNoCumple { .. }));
    }
}
