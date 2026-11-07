//! Formato visual cetrero — tablas para terminal.
//! F1: ASCII puro sin dependencias externas (comfy-table en F2 si hace falta).
//! Respeta Day-0: todo en español, sin colores forzados si NO_TTY.

use super::modelo::{Bloqueo, Manifiesto};
use super::resolver::Resolucion;

/// Renderiza `fardo lista` — tabla de fardos del nido.
#[must_use]
pub fn tabla_lista(manifiesto: &Manifiesto, resolucion: &Resolucion) -> String {
    let mut out = String::new();
    out.push_str("  ⠀⠀⠀⣏⡱ ⣏⡉ ⣏⡱ ⡇ ⣎⣱   FALCATO — Nido\n");
    out.push_str(&format!("  Nido: {}@{}\n", manifiesto.paquete.nombre, manifiesto.paquete.version));
    out.push_str(&format!("  Fardos: {}  •  Orden: {}\n", resolucion.orden.len(), resolucion.orden.iter().map(|id| id.nombre.as_str().to_string()).collect::<Vec<_>>().join(" → ")));
    out.push('\n');
    out.push_str("  ─────────────────────────────────────────────────────────\n");
    out.push_str("   NOMBRE              VERSIÓN   ORIGEN               ESTADO\n");
    out.push_str("  ─────────────────────────────────────────────────────────\n");
    if resolucion.orden.is_empty() {
        out.push_str("   (sin fardos)\n");
    } else {
        for id in &resolucion.orden {
            let dir = resolucion.directorios.get(id.nombre.as_str()).map(|p| p.display().to_string()).unwrap_or_else(|| "—".to_string());
            // truncar origen largo
            let origen_corto = if dir.len() > 22 { format!("…{}", &dir[dir.len()-21..]) } else { dir };
            out.push_str(&format!("   {:<18} {:<9} {:<20} ✓\n", id.nombre.as_str(), id.version.to_string(), origen_corto));
        }
    }
    out.push_str("  ─────────────────────────────────────────────────────────\n");
    out.push_str(&format!("  {} fardos • falcato lock: {} entradas\n", resolucion.orden.len(), resolucion.bloqueo.fardos.len()));
    out
}

/// Renderiza `fardo arbol` — árbol deduplicado.
#[must_use]
pub fn tabla_arbol(manifiesto: &Manifiesto, resolucion: &Resolucion) -> String {
    let mut out = String::new();
    out.push_str(&format!("  Nido {}@{}\n", manifiesto.paquete.nombre, manifiesto.paquete.version));

    // construir mapa nombre -> deps directas
    let mut hijos: std::collections::BTreeMap<String, Vec<String>> = std::collections::BTreeMap::new();
    // raíz
    let raiz_nombre = manifiesto.paquete.nombre.clone();
    hijos.insert(raiz_nombre.clone(), manifiesto.fardos.keys().map(|k| k.as_str().to_string()).collect());
    for (nombre, man) in &resolucion.manifiestos {
        if nombre == &raiz_nombre { continue; }
        hijos.insert(nombre.clone(), man.fardos.keys().map(|k| k.as_str().to_string()).collect());
    }

    // dfs para imprimir sin duplicar visitas (marcar visitados)
    let mut visitados = std::collections::HashSet::new();
    // ordenar hijos según orden topológico para estabilidad
    let orden_idx: std::collections::HashMap<String, usize> = resolucion.orden.iter().enumerate().map(|(i, id)| (id.nombre.as_str().to_string(), i)).collect();

    fn pintar(
        nombre: &str,
        hijos: &std::collections::BTreeMap<String, Vec<String>>,
        orden_idx: &std::collections::HashMap<String, usize>,
        visitados: &mut std::collections::HashSet<String>,
        prefijo: &str,
        es_ultimo: bool,
        out: &mut String,
        profundidad: usize,
    ) {
        if profundidad > 20 { return; } // anti-DoS
        let conector = if profundidad == 0 { "" } else if es_ultimo { "└─ " } else { "├─ " };
        let estado = if visitados.contains(nombre) { "✓ (compartido)" } else { "✓" };
        if profundidad > 0 {
            out.push_str(&format!("{}{}{} {}\n", prefijo, conector, nombre, estado));
        }
        if visitados.contains(nombre) {
            return;
        }
        visitados.insert(nombre.to_string());
        let mut kids = hijos.get(nombre).cloned().unwrap_or_default();
        kids.sort_by_key(|k| orden_idx.get(k).copied().unwrap_or(usize::MAX));
        let nuevo_prefijo = if profundidad == 0 { String::new() } else if es_ultimo { format!("{}   ", prefijo) } else { format!("{}│  ", prefijo) };
        for (i, hijo) in kids.iter().enumerate() {
            let ultimo = i == kids.len() - 1;
            pintar(hijo, hijos, orden_idx, visitados, &nuevo_prefijo, ultimo, out, profundidad + 1);
        }
    }

    pintar(&raiz_nombre, &hijos, &orden_idx, &mut visitados, "", true, &mut out, 0);

    // si no hay hijos, mostrar mensaje
    if hijos.get(&raiz_nombre).map(|v| v.is_empty()).unwrap_or(true) {
        out.push_str("  └─ (sin fardos)\n");
    }
    out.push('\n');
    out.push_str(&format!("  {} fardos • 0 duplicados\n", resolucion.orden.len()));
    out
}

/// Renderiza `fardo buscar` mock (F1 sin DHT, solo local).
#[must_use]
pub fn tabla_buscar_mock(termino: &str, resultados: &[(String, String, String)]) -> String {
    let mut out = String::new();
    out.push_str("  ⠀⠀⠀⣏⡱ ⣏⡉ ⣏⡱ ⡇ ⣎⣱   FALCATO — Bandada (mock local)\n");
    out.push_str(&format!("  Buscando \"{}\"... {} fardos\n", termino, resultados.len()));
    out.push_str("  ─────────────────────────────────────────────────────────\n");
    out.push_str("   NOMBRE              VERSIÓN   ORIGEN\n");
    out.push_str("  ─────────────────────────────────────────────────────────\n");
    for (nombre, version, origen) in resultados {
        out.push_str(&format!("   {:<18} {:<9} {}\n", nombre, version, origen));
    }
    out.push_str("  ─────────────────────────────────────────────────────────\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fardo::modelo::{Manifiesto, Version, NombreFardo, Bloqueo, FardoBloqueado, FardoId, Origen};
    use crate::fardo::resolver::Resolucion;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    fn resolucion_dummy(nombres: Vec<&str>) -> Resolucion {
        let orden = nombres.iter().map(|n| FardoId { nombre: NombreFardo::nuevo(n).unwrap(), version: Version::parsear("0.1.0").unwrap() }).collect();
        let mut directorios = BTreeMap::new();
        for n in &nombres {
            directorios.insert(n.to_string(), PathBuf::from(format!("/tmp/{}", n)));
        }
        Resolucion { orden, manifiestos: BTreeMap::new(), directorios, bloqueo: Bloqueo::default() }
    }

    #[test]
    fn lista_vacia() {
        let m = Manifiesto::nuevo("mi-nido", "0.1.0");
        let r = resolucion_dummy(vec![]);
        let s = tabla_lista(&m, &r);
        assert!(s.contains("(sin fardos)"));
    }

    #[test]
    fn lista_con_fardos() {
        let m = Manifiesto::nuevo("app", "0.1.0");
        let r = resolucion_dummy(vec!["util", "base"]);
        let s = tabla_lista(&m, &r);
        assert!(s.contains("util"));
        assert!(s.contains("base"));
    }

    #[test]
    fn arbol_simple() {
        let mut m = Manifiesto::nuevo("app", "0.1.0");
        m.fardos.insert(NombreFardo::nuevo("util").unwrap(), crate::fardo::modelo::Dependencia {
            nombre: NombreFardo::nuevo("util").unwrap(),
            requisito: crate::fardo::modelo::Restriccion::parsear("^0.1.0").unwrap(),
            origen: Origen::Ruta(PathBuf::from("../util")),
        });
        let mut res = resolucion_dummy(vec!["util", "app"]);
        res.manifiestos.insert("app".to_string(), m.clone());
        let s = tabla_arbol(&m, &res);
        assert!(s.contains("app"));
        assert!(s.contains("util"));
    }
}
