//! Cache local del nido — hash de verificación + .o futuro.
//! F1: cache incremental de resolución (hash de falcato.toml) para <100ms.
//! F3: cache .o content-addressed P2P.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use super::error::ErrorFardo;

/// Directorio de cache local: <nido>/.falcato/
#[must_use]
pub fn dir_cache(nido: &Path) -> PathBuf {
    nido.join(".falcato")
}

/// Ruta del índice de cache de resolución.
#[must_use]
pub fn ruta_cache_resolucion(nido: &Path) -> PathBuf {
    dir_cache(nido).join("cache_resolucion.bin")
}

/// Hash estable de un string (fácil, sin sha2 para F1 local).
#[must_use]
pub fn hash_str(s: &str) -> String {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Hash de un manifiesto serializado (para invalidar cache).
#[must_use]
pub fn hash_manifiesto_toml(toml: &str) -> String {
    hash_str(toml)
}

/// Lee cache si existe y es válida (hash coincide).
pub fn leer_cache_resolucion(nido: &Path, hash_actual: &str) -> Option<String> {
    let ruta = ruta_cache_resolucion(nido);
    let contenido = std::fs::read_to_string(&ruta).ok()?;
    let mut lineas = contenido.lines();
    let hash_guardado = lineas.next()?;
    if hash_guardado != hash_actual {
        return None;
    }
    Some(lineas.collect::<Vec<_>>().join("\n"))
}

/// Escribe cache atómicamente.
pub fn escribir_cache_resolucion(nido: &Path, hash_actual: &str, datos: &str) -> Result<(), ErrorFardo> {
    let dir = dir_cache(nido);
    std::fs::create_dir_all(&dir).map_err(|e| ErrorFardo::Escritura { ruta: dir.clone(), detalle: e.to_string() })?;
    let ruta = ruta_cache_resolucion(nido);
    let tmp = dir.join(".cache_resolucion.tmp");
    let contenido = format!("{}\n{}", hash_actual, datos);
    std::fs::write(&tmp, contenido).map_err(|e| ErrorFardo::Escritura { ruta: tmp.clone(), detalle: e.to_string() })?;
    std::fs::rename(&tmp, &ruta).map_err(|e| ErrorFardo::Escritura { ruta: ruta.clone(), detalle: e.to_string() })?;
    Ok(())
}

/// Limpia cache del nido.
pub fn limpiar_cache(nido: &Path) -> Result<(), ErrorFardo> {
    let ruta = ruta_cache_resolucion(nido);
    if ruta.exists() {
        std::fs::remove_file(&ruta).map_err(|e| ErrorFardo::Escritura { ruta, detalle: e.to_string() })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn hash_estable() {
        assert_eq!(hash_str("hola"), hash_str("hola"));
        assert_ne!(hash_str("hola"), hash_str("adios"));
    }

    #[test]
    fn cache_roundtrip() {
        let dir = tempdir().unwrap();
        let nido = dir.path().join("nido");
        std::fs::create_dir_all(&nido).unwrap();
        let toml = "[paquete]\nnombre=\"a\"\nversion=\"0.1.0\"";
        let h = hash_manifiesto_toml(toml);
        escribir_cache_resolucion(&nido, &h, "orden: a,b").unwrap();
        assert_eq!(leer_cache_resolucion(&nido, &h).unwrap(), "orden: a,b");
        // hash distinto → miss
        assert!(leer_cache_resolucion(&nido, "0000000000000000").is_none());
        limpiar_cache(&nido).unwrap();
        assert!(!ruta_cache_resolucion(&nido).exists());
    }
}
