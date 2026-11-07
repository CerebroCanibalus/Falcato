//! Validación de entrada — anti-inyección, anti-escape, anti-DoS.
//! Todo lo que viene de TOML/CLI/usuario pasa por aquí.

use std::path::{Path, PathBuf};

use super::error::ErrorFardo;

/// Valida nombre de fardo: kebab-case 2..31, [a-z0-9-_], empieza letra, sin -- ni -_ ni _-
pub fn validar_nombre_fardo(s: &str) -> Result<(), ErrorFardo> {
    if s.len() < 2 || s.len() > 31 {
        return Err(ErrorFardo::NombreInvalido { nombre: s.to_string() });
    }
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {},
        _ => return Err(ErrorFardo::NombreInvalido { nombre: s.to_string() }),
    }
    if s.contains("--") || s.contains("__") || s.contains("-_") || s.contains("_-") {
        return Err(ErrorFardo::NombreInvalido { nombre: s.to_string() });
    }
    if !s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_') {
        return Err(ErrorFardo::NombreInvalido { nombre: s.to_string() });
    }
    if s.ends_with('-') || s.ends_with('_') {
        return Err(ErrorFardo::NombreInvalido { nombre: s.to_string() });
    }
    Ok(())
}

/// Valida semver simple MAYOR.menor.parche (tres enteros).
pub fn validar_version_semver(s: &str) -> Result<(), ErrorFardo> {
    let s = s.trim().trim_start_matches('v');
    if s.is_empty() {
        return Err(ErrorFardo::VersionInvalida { version: s.to_string() });
    }
    let partes: Vec<&str> = s.split('.').collect();
    if partes.len() != 3 {
        return Err(ErrorFardo::VersionInvalida { version: s.to_string() });
    }
    for p in partes {
        if p.is_empty() || !p.chars().all(|c| c.is_ascii_digit()) {
            return Err(ErrorFardo::VersionInvalida { version: s.to_string() });
        }
        // evita overflow parse posterior: max 10 dígitos es suficiente
        if p.len() > 10 {
            return Err(ErrorFardo::VersionInvalida { version: s.to_string() });
        }
        p.parse::<u64>().map_err(|_| ErrorFardo::VersionInvalida { version: s.to_string() })?;
    }
    Ok(())
}

/// Valida hash sha256 hex 64
pub fn validar_hash_sha256(s: &str) -> Result<(), ErrorFardo> {
    if s.len() != 64 || !s.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(ErrorFardo::HashInvalido { hash: s.to_string() });
    }
    Ok(())
}

/// Canonicaliza y verifica que `candidate` está dentro de `base`.
/// Mitiga ZipSlip / RutaEscape [F004][F013].
pub fn ruta_segura(candidate: &Path, base: &Path) -> Result<PathBuf, ErrorFardo> {
    let base_canon = base.canonicalize().map_err(|_| ErrorFardo::RutaNoExiste { ruta: base.to_path_buf() })?;
    let cand_canon = candidate.canonicalize().map_err(|_| ErrorFardo::RutaNoExiste { ruta: candidate.to_path_buf() })?;
    if !cand_canon.starts_with(&base_canon) {
        return Err(ErrorFardo::RutaEscape { ruta: cand_canon, base: base_canon });
    }
    Ok(cand_canon)
}

/// Une `base + ruta_relativa` y verifica escape, sin requerir que exista (para escritura atómica).
/// Usa `base.canonicalize` + `join` + `normalize` léxico.
pub fn unir_ruta_segura(base: &Path, relativa: &Path) -> Result<PathBuf, ErrorFardo> {
    if relativa.is_absolute() {
        return Err(ErrorFardo::RutaEscape { ruta: relativa.to_path_buf(), base: base.to_path_buf() });
    }
    for comp in relativa.components() {
        if let std::path::Component::ParentDir = comp {
            return Err(ErrorFardo::RutaEscape { ruta: relativa.to_path_buf(), base: base.to_path_buf() });
        }
    }
    let base_canon = base.canonicalize().unwrap_or_else(|_| base.to_path_buf());
    let joined = base_canon.join(relativa);
    // normalización léxica para detectar ../ sin tocar disco
    let normalized = normalize_lexico(&joined);
    if !normalized.starts_with(&base_canon) {
        return Err(ErrorFardo::RutaEscape { ruta: normalized, base: base_canon });
    }
    Ok(joined)
}

fn normalize_lexico(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in p.components() {
        match comp {
            std::path::Component::ParentDir => { out.pop(); },
            std::path::Component::CurDir => {},
            c => out.push(c.as_os_str()),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn ruta_escape_parent() {
        let base = std::env::temp_dir();
        let res = unir_ruta_segura(&base, Path::new("../escape"));
        assert!(res.is_err());
    }

    #[test]
    fn ruta_absoluta_rechazada() {
        let base = std::env::temp_dir();
        let res = unir_ruta_segura(&base, Path::new("/etc/passwd"));
        assert!(res.is_err());
    }

    #[test]
    fn hash_valido() {
        assert!(validar_hash_sha256(&"a".repeat(64)).is_ok());
        assert!(validar_hash_sha256("abc").is_err());
    }
}
