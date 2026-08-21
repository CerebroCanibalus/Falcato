//! # Log inteligente — sin spam
//!
//! Otros lenguajes te inundan: 10k líneas de `leak 24b` idénticas.
//! Falcato apila repetidos y resume:
//! ```
//! [MEM] doble_free en main.fc:12 — 5 veces (última en 0x1A3F)
//! [MEM] leak 24b Texto en vector_agregar (12 alocaciones, ver --depurar-memoria=3 para detalle)
//! ```

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

struct EstadoLog {
    ultimo_mensaje: Option<String>,
    repeticiones: usize,
    contadores: HashMap<String, usize>,
}

static LOG: OnceLock<Mutex<EstadoLog>> = OnceLock::new();

fn estado() -> &'static Mutex<EstadoLog> {
    LOG.get_or_init(|| Mutex::new(EstadoLog {
        ultimo_mensaje: None,
        repeticiones: 0,
        contadores: HashMap::new(),
    }))
}

/// Imprime con deduplicación apilada.
/// Si el mensaje es idéntico al anterior, no imprime — incrementa contador.
/// Si cambia, descarga el apilado como `— N veces` y luego imprime el nuevo.
 pub fn imprimir(mensaje: &str) {
    let mut s = estado().lock().unwrap();
    if let Some(ref ultimo) = s.ultimo_mensaje {
        if ultimo == mensaje {
            s.repeticiones += 1;
            *s.contadores.entry(mensaje.to_string()).or_insert(0) += 1;
            return; // apilar, no imprimir
        } else if s.repeticiones > 0 {
            // Descargar apilado
            let total = s.repeticiones + 1;
            eprintln!("{} — {} veces", ultimo, total);
            s.repeticiones = 0;
        }
    }
    eprintln!("{}", mensaje);
    s.ultimo_mensaje = Some(mensaje.to_string());
}

/// Variante sin apilar — para volcados hex donde cada línea es única.
 pub fn imprimir_directo(mensaje: &str) { eprintln!("{}", mensaje); }

/// Contador agregado por clave (para resumen al salir).
 pub fn incrementar(clave: &str) {
    let mut s = estado().lock().unwrap();
    *s.contadores.entry(clave.to_string()).or_insert(0) += 1;
}

/// Descarga todo al finalizar el programa (llamado desde atexit o al final de principal).
 pub fn flush() {
    let mut s = estado().lock().unwrap();
    if s.repeticiones > 0 {
        if let Some(ref ultimo) = s.ultimo_mensaje.clone() {
            eprintln!("{} — {} veces", ultimo, s.repeticiones + 1);
        }
    }
    if !s.contadores.is_empty() && s.contadores.len() > 1 {
        eprintln!("[MEM] resumen: {} tipos de eventos", s.contadores.len());
        for (k, v) in &s.contadores {
            if *v > 3 { eprintln!("  {}: {} veces", k, v); }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_apilado() {
        imprimir("[MEM] leak");
        imprimir("[MEM] leak");
        imprimir("[MEM] leak");
        imprimir("[MEM] otro"); // debe descargar "leak — 3 veces"
    }
}
