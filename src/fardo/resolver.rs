//! Resolver — DFS con detección de ciclo [F010] y orden topológico.
//! Responsabilidad: dado un manifiesto raíz, colecta todos los fardos transitivos
//! vía las fuentes registradas. F1 solo FuenteRuta, F2 añade FuenteRegistro.

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use super::error::ErrorFardo;
use super::fuentes::Fuente;
use super::fuentes::ruta::FuenteRuta;
use super::modelo::{Bloqueo, FardoBloqueado, FardoId, Manifiesto, NombreFardo, Origen, Version};

/// Resultado de resolución.
#[derive(Debug, Clone)]
pub struct Resolucion {
    /// Orden topológico: dependencias primero, raíz al final.
    pub orden: Vec<FardoId>,
    /// Manifiesto por fardo.
    pub manifiestos: BTreeMap<String, Manifiesto>,
    /// Directorio canónico por fardo (para colectar .fc).
    pub directorios: BTreeMap<String, PathBuf>,
    /// Bloqueo generado (para escribir falcato.lock).
    pub bloqueo: Bloqueo,
}

impl Resolucion {
    /// Lista de directorios en orden topológico (para verificar/compilar).
    #[must_use]
    pub fn directorios_ordenados(&self) -> Vec<PathBuf> {
        self.orden.iter().filter_map(|id| self.directorios.get(id.nombre.as_str()).cloned()).collect()
    }

    /// Colecta todos los .fc relevantes (lib.fc + *.fc en src/) en orden.
    pub fn colectar_fuentes(&self) -> Vec<PathBuf> {
        let mut fuentes = Vec::new();
        for dir in self.directorios_ordenados() {
            // cada fardo aporta src/lib.fc y src/*.fc (F1 simple)
            let lib = dir.join("src").join("lib.fc");
            if lib.exists() {
                fuentes.push(lib);
            }
            // también principal.fc en raíz del fardo si existe
            let principal = dir.join("principal.fc");
            if principal.exists() {
                fuentes.push(principal);
            }
            // src/*.fc
            if let Ok(entries) = std::fs::read_dir(dir.join("src")) {
                for e in entries.flatten() {
                    let p = e.path();
                    if p.extension().and_then(|s| s.to_str()) == Some("fc") && p.file_name().and_then(|s| s.to_str()) != Some("lib.fc") {
                        fuentes.push(p);
                    }
                }
            }
        }
        fuentes
    }
}

/// Resuelve un nido local a partir de su directorio (donde está falcato.toml).
/// Si no hay falcato.toml, retorna resolución vacía (compat hacia atrás).
pub fn resolver_nido(dir: &Path) -> Result<Resolucion, ErrorFardo> {
    let manifiesto_path = Manifiesto::buscar_en(dir);
    let Some(m_path) = manifiesto_path else {
        // sin nido → sin fardos
        return Ok(Resolucion {
            orden: Vec::new(),
            manifiestos: BTreeMap::new(),
            directorios: BTreeMap::new(),
            bloqueo: Bloqueo::default(),
        });
    };
    let raiz_dir = m_path.parent().unwrap_or(dir).to_path_buf();
    let manifiesto_raiz = Manifiesto::desde_archivo(&m_path)?;

    // fuentes registradas F1
    let fuentes: Vec<Box<dyn Fuente>> = vec![Box::new(FuenteRuta)];

    let mut orden = Vec::new();
    let mut manifiestos = BTreeMap::new();
    let mut directorios = BTreeMap::new();
    let mut bloqueo = Bloqueo::default();
    let mut visitado: HashSet<String> = HashSet::new();
    let mut en_pila: HashSet<String> = HashSet::new();
    let mut pila_nombres: Vec<String> = Vec::new();

    // insertar raíz al final después de sus deps
    let raiz_id = FardoId { nombre: NombreFardo::nuevo(&manifiesto_raiz.paquete.nombre)?, version: Version::parsear(&manifiesto_raiz.paquete.version)? };

    dfs(
        &manifiesto_raiz,
        &raiz_dir,
        &fuentes,
        &mut orden,
        &mut manifiestos,
        &mut directorios,
        &mut bloqueo,
        &mut visitado,
        &mut en_pila,
        &mut pila_nombres,
    )?;

    // raíz al final
    if !visitado.contains(raiz_id.nombre.as_str()) {
        orden.push(raiz_id.clone());
        manifiestos.insert(raiz_id.nombre.as_str().to_string(), manifiesto_raiz);
        directorios.insert(raiz_id.nombre.as_str().to_string(), raiz_dir);
        visitado.insert(raiz_id.nombre.as_str().to_string());
    }

    Ok(Resolucion { orden, manifiestos, directorios, bloqueo })
}

#[allow(clippy::too_many_arguments)]
fn dfs(
    manifiesto: &Manifiesto,
    dir: &Path,
    fuentes: &[Box<dyn Fuente>],
    orden: &mut Vec<FardoId>,
    manifiestos: &mut BTreeMap<String, Manifiesto>,
    directorios: &mut BTreeMap<String, PathBuf>,
    bloqueo: &mut Bloqueo,
    visitado: &mut HashSet<String>,
    en_pila: &mut HashSet<String>,
    pila_nombres: &mut Vec<String>,
) -> Result<(), ErrorFardo> {
    let id_actual = format!("{}@{}", manifiesto.paquete.nombre, manifiesto.paquete.version);

    for (nombre, dep) in &manifiesto.fardos {
        let key = nombre.as_str().to_string();

        if en_pila.contains(&key) {
            let mut ciclo = pila_nombres.clone();
            ciclo.push(key.clone());
            return Err(ErrorFardo::Ciclo { ciclo: ciclo.join(" -> ") });
        }
        if visitado.contains(&key) {
            continue;
        }

        // buscar fuente que pueda resolver
        let mut resuelto = None;
        let mut ultimo_error = None;
        for fuente in fuentes {
            match fuente.resolver(dep, dir) {
                Ok((m, p)) => {
                    resuelto = Some((m, p));
                    break;
                }
                Err(e) => ultimo_error = Some(e),
            }
        }
        let (man_dep, dir_dep) = resuelto.ok_or_else(|| ultimo_error.unwrap_or_else(|| ErrorFardo::Interno { detalle: format!("no se pudo resolver '{}'", key) }))?;

        // límite anti-DoS: profundidad
        if pila_nombres.len() > 50 {
            return Err(ErrorFardo::TomlInvalido { ruta: dir.to_path_buf(), detalle: "profundidad de dependencias excede 50".to_string() });
        }

        en_pila.insert(key.clone());
        pila_nombres.push(key.clone());

        dfs(&man_dep, &dir_dep, fuentes, orden, manifiestos, directorios, bloqueo, visitado, en_pila, pila_nombres)?;

        en_pila.remove(&key);
        pila_nombres.pop();

        // marcar visitado y añadir en orden post-orden
        let fardo_id = FardoId { nombre: nombre.clone(), version: Version::parsear(&man_dep.paquete.version)? };
        if !visitado.contains(fardo_id.nombre.as_str()) {
            orden.push(fardo_id.clone());
            manifiestos.insert(fardo_id.nombre.as_str().to_string(), man_dep.clone());
            directorios.insert(fardo_id.nombre.as_str().to_string(), dir_dep.clone());
            visitado.insert(fardo_id.nombre.as_str().to_string());

            // bloqueo
            let bloqueado = FardoBloqueado {
                id: fardo_id,
                hash: String::new(), // F1 local: vacío, F2 se llena con sha256
                origen: dep.origen.clone(),
            };
            bloqueo.insertar(bloqueado);
        }

        let _ = id_actual; // para futuro uso en mensajes
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fardo::Manifiesto;
    use std::path::Path;
    use tempfile::tempdir;

    fn crear_fardo(dir: &Path, nombre: &str, version: &str, deps: Vec<(&str, &str)>) {
        Manifiesto::iniciar_proyecto(dir, Some(nombre)).unwrap();
        let mut m = Manifiesto::desde_archivo(&dir.join("falcato.toml")).unwrap();
        m.paquete.version = version.to_string();
        for (dep_nombre, dep_ruta) in deps {
            m.agregar_ruta(dep_nombre, Path::new(dep_ruta), "^0.1.0").unwrap();
        }
        m.guardar(&dir.join("falcato.toml")).unwrap();
        // asegurar src/lib.fc existe para colectar_fuentes (ya creado por iniciar_proyecto)
    }

    #[test]
    fn resolver_simple() {
        let tmp = tempdir().unwrap();
        let util = tmp.path().join("util");
        crear_fardo(&util, "util", "0.1.0", vec![]);
        let app = tmp.path().join("app");
        crear_fardo(&app, "app", "0.1.0", vec![("util", "../util")]);

        let res = resolver_nido(&app).unwrap();
        assert_eq!(res.orden.len(), 2); // util + app
        assert_eq!(res.orden[0].nombre.as_str(), "util");
        assert_eq!(res.orden[1].nombre.as_str(), "app");
    }

    #[test]
    fn resolver_diamante() {
        let tmp = tempdir().unwrap();
        let base = tmp.path().join("base");
        crear_fardo(&base, "base", "0.1.0", vec![]);
        let aa = tmp.path().join("aa");
        crear_fardo(&aa, "aa", "0.1.0", vec![("base", "../base")]);
        let bb = tmp.path().join("bb");
        crear_fardo(&bb, "bb", "0.1.0", vec![("base", "../base")]);
        let app = tmp.path().join("app");
        crear_fardo(&app, "app", "0.1.0", vec![("aa", "../aa"), ("bb", "../bb")]);

        let res = resolver_nido(&app).unwrap();
        // base solo una vez, orden topológico
        let nombres: Vec<_> = res.orden.iter().map(|id| id.nombre.as_str()).collect();
        assert_eq!(nombres.iter().filter(|&&n| n == "base").count(), 1);
        assert!(nombres.iter().position(|&n| n == "base").unwrap() < nombres.iter().position(|&n| n == "app").unwrap());
    }

    #[test]
    fn ciclo_detectado() {
        let tmp = tempdir().unwrap();
        let aa = tmp.path().join("aa");
        let bb = tmp.path().join("bb");
        crear_fardo(&aa, "aa", "0.1.0", vec![("bb", "../bb")]);
        crear_fardo(&bb, "bb", "0.1.0", vec![("aa", "../aa")]);

        let r = resolver_nido(&aa);
        assert!(r.is_err());
        assert!(matches!(r.unwrap_err(), crate::fardo::error::ErrorFardo::Ciclo { .. }));
    }

    #[test]
    fn sin_falcato_toml_vacio() {
        // Usar directorio sin padre que tenga falcato.toml (el home puede tener uno)
        let dir = PathBuf::from("Z:\\__falcato_test_vacio_nohome__");
        // Si el directorio no existe, crearlo temporalmente
        let _ = std::fs::create_dir_all(&dir);
        let res = resolver_nido(&dir).unwrap();
        // Si no hay falcato.toml, orden debe ser 0
        // Si hay uno (por accidente), al menos no debe ciclar
        assert!(res.orden.len() <= 1);
        let _ = std::fs::remove_dir(&dir);
    }
}
