//! Análisis de futuros para transformación state machine (Fase 18E)
//!
//! Transforma `fut función` en state machines poll-based:
//! - Cada `esperar` es un punto de suspensión (state transition)
//! - Variables live-across-await se almacenan en un struct heap
//! - `__poll_N(ptr) -> i64` ejecuta el segmento correspondiente al state
//! - `__init_N(args) -> ptr` aloca e inicializa el state machine

use crate::ast::*;
use crate::span::Span;
use std::collections::HashSet;

/// Un punto de suspensión (await) dentro de un futuro
#[derive(Debug, Clone)]
pub struct PuntoSuspension {
    /// Índice de la sentencia donde ocurre el esperar
    pub indice_sentencia: usize,
    /// La expresión interna del esperar (ej: dormir(100))
    pub expresion: Box<Expresion>,
    /// Span para errores
    pub span: Span,
}

/// Una variable que vive a través de un punto de suspensión
#[derive(Debug, Clone)]
pub struct VarLiveAcross {
    pub nombre: String,
    pub tipo: Tipo,
    /// Offset en el struct del state machine (se calcula después)
    pub offset: u32,
}

/// Resultado del análisis de un futuro
#[derive(Debug, Clone)]
pub struct AnalisisFuturo {
    /// Nombre de la función original
    pub nombre: String,
    /// Número de estados (puntos de suspensión + 1)
    pub num_estados: usize,
    /// Puntos de suspensión en orden
    pub suspensiones: Vec<PuntoSuspension>,
    /// Variables que deben vivir en el struct (live across await)
    pub vars_struct: Vec<VarLiveAcross>,
    /// Parámetros de la función (también van en el struct)
    pub parametros: Vec<Parametro>,
    /// Tipo de retorno
    pub retorno: Option<Tipo>,
    /// Sentencias segmentadas por estado
    /// segmentos[i] = sentencias del estado i (antes de la suspensión i)
    pub segmentos: Vec<Vec<Sentencia>>,
    /// Span de la función
    pub span: Span,
}

/// Analiza una función futura y produce el plan de transformación state machine
pub fn analizar_futuro(func: &FuncionDecl) -> AnalisisFuturo {
    let mut suspensiones = Vec::new();
    let mut segmentos: Vec<Vec<Sentencia>> = vec![Vec::new()];

    // 1. Recorrer sentencias, encontrar puntos de suspensión
    for (i, sent) in func.cuerpo.sentencias.iter().enumerate() {
        if let Some((expr, span)) = extraer_esperar(sent) {
            suspensiones.push(PuntoSuspension {
                indice_sentencia: i,
                expresion: expr,
                span,
            });
            // Nuevo segmento después de esta suspensión
            segmentos.push(Vec::new());
        } else {
            // Agregar al segmento actual
            let ultimo = segmentos.len() - 1;
            segmentos[ultimo].push(sent.clone());
        }
    }

    let num_estados = suspensiones.len() + 1;

    // 2. Análisis de liveness: variables definidas antes de un await
    //    y usadas después de él deben ir en el struct
    let vars_struct = analizar_liveness(&func.cuerpo.sentencias, &suspensiones, &func.parametros);

    AnalisisFuturo {
        nombre: func.nombre.clone(),
        num_estados,
        suspensiones,
        vars_struct,
        parametros: func.parametros.clone(),
        retorno: func.retorno.clone(),
        segmentos,
        span: func.span.clone(),
    }
}

/// Extrae la expresión de un `esperar` si la sentencia lo contiene
fn extraer_esperar(sent: &Sentencia) -> Option<(Box<Expresion>, Span)> {
    match sent {
        // `esperar expr;` como sentencia de expresión
        Sentencia::Expresion(expr) => {
            if let Expresion::Esperar(inner, sp) = expr {
                Some((inner.clone(), sp.clone()))
            } else {
                None
            }
        }
        // `el x: T = esperar expr;`
        Sentencia::DeclaracionVariable(decl) => {
            if let Expresion::Esperar(inner, sp) = &decl.valor {
                Some((inner.clone(), sp.clone()))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Análisis de liveness simplificado:
/// Una variable debe ir en el struct si:
/// - Se define ANTES de un punto de suspensión Y
/// - Se usa DESPUÉS de ese punto de suspensión
///
/// Los parámetros siempre van en el struct (pueden usarse en cualquier estado).
fn analizar_liveness(
    sentencias: &[Sentencia],
    suspensiones: &[PuntoSuspension],
    parametros: &[Parametro],
) -> Vec<VarLiveAcross> {
    if suspensiones.is_empty() {
        return Vec::new();
    }

    let mut resultado: Vec<VarLiveAcross> = Vec::new();
    let mut vars_en_struct: HashSet<String> = HashSet::new();

    // Los parámetros siempre van en el struct
    for param in parametros {
        if vars_en_struct.insert(param.nombre.clone()) {
            resultado.push(VarLiveAcross {
                nombre: param.nombre.clone(),
                tipo: param.tipo.clone(),
                offset: 0, // se calcula después
            });
        }
    }

    // Para cada punto de suspensión, encontrar variables definidas antes y usadas después
    for susp in suspensiones {
        // Variables definidas antes de esta suspensión
        let defs_antes = recolectar_defs(&sentencias[..susp.indice_sentencia]);

        // Variables usadas después de esta suspensión
        let usos_despues = recolectar_usos(&sentencias[susp.indice_sentencia + 1..]);

        // Intersección: vars que cruzan el await
        for nombre in &defs_antes {
            if usos_despues.contains(nombre) && vars_en_struct.insert(nombre.clone()) {
                // Buscar tipo de la variable
                let tipo = buscar_tipo_var(nombre, sentencias, parametros);
                resultado.push(VarLiveAcross {
                    nombre: nombre.clone(),
                    tipo,
                    offset: 0,
                });
            }
        }
    }

    // Calcular offsets (layout C: i32=4 bytes, i64=8 bytes, alineación natural)
    calcular_offsets(&mut resultado);

    resultado
}

/// Recolecta nombres de variables definidas en un slice de sentencias
fn recolectar_defs(sentencias: &[Sentencia]) -> HashSet<String> {
    let mut defs = HashSet::new();
    for sent in sentencias {
        if let Sentencia::DeclaracionVariable(decl) = sent {
            defs.insert(decl.nombre.clone());
        }
    }
    defs
}

/// Recolecta nombres de variables usadas en un slice de sentencias
fn recolectar_usos(sentencias: &[Sentencia]) -> HashSet<String> {
    let mut usos = HashSet::new();
    for sent in sentencias {
        recolectar_usos_sentencia(sent, &mut usos);
    }
    usos
}

fn recolectar_usos_sentencia(sent: &Sentencia, usos: &mut HashSet<String>) {
    match sent {
        Sentencia::Expresion(expr) => {
            recolectar_usos_expr(expr, usos);
        }
        Sentencia::DeclaracionVariable(decl) => {
            recolectar_usos_expr(&decl.valor, usos);
        }
        Sentencia::Retornar(expr, _) => {
            if let Some(e) = expr {
                recolectar_usos_expr(e, usos);
            }
        }
        Sentencia::Asignacion(asig) => {
            recolectar_usos_expr(&asig.valor, usos);
            // También el lugar si es array
            if let Lugar::Array(base, idx) = &asig.lugar {
                recolectar_usos_expr(base, usos);
                recolectar_usos_expr(idx, usos);
            }
        }
        Sentencia::Condicional(cond) => {
            recolectar_usos_expr(&cond.condicion, usos);
            for s in &cond.bloque_entonces.sentencias {
                recolectar_usos_sentencia(s, usos);
            }
            if let Some(bloque) = &cond.bloque_sino {
                for s in &bloque.sentencias {
                    recolectar_usos_sentencia(s, usos);
                }
            }
        }
        Sentencia::BucleMientras(bucle) => {
            recolectar_usos_expr(&bucle.condicion, usos);
            for s in &bucle.bloque.sentencias {
                recolectar_usos_sentencia(s, usos);
            }
        }
        Sentencia::BuclePara(bucle) => {
            recolectar_usos_expr(&bucle.iterable, usos);
            for s in &bucle.bloque.sentencias {
                recolectar_usos_sentencia(s, usos);
            }
        }
        Sentencia::Region { cuerpo, .. } => {
            for s in cuerpo {
                recolectar_usos_sentencia(s, usos);
            }
        }
        Sentencia::ConExecutor { cuerpo, hilos, .. } => {
            recolectar_usos_expr(hilos, usos);
            for s in cuerpo {
                recolectar_usos_sentencia(s, usos);
            }
        }
        Sentencia::Seleccionar(sel) => {
            for rama in &sel.ramas {
                recolectar_usos_expr(&rama.canal, usos);
                for s in &rama.cuerpo.sentencias {
                    recolectar_usos_sentencia(s, usos);
                }
            }
        }
    }
}

fn recolectar_usos_expr(expr: &Expresion, usos: &mut HashSet<String>) {
    match expr {
        Expresion::Identificador(nombre, _) => {
            usos.insert(nombre.clone());
        }
        Expresion::Binaria(izq, _, der, _) => {
            recolectar_usos_expr(izq, usos);
            recolectar_usos_expr(der, usos);
        }
        Expresion::Unaria(_, operando, _) => {
            recolectar_usos_expr(operando, usos);
        }
        Expresion::Llamada(llamada) => {
            for arg in &llamada.argumentos {
                recolectar_usos_expr(arg, usos);
            }
        }
        Expresion::Esperar(inner, _) => {
            recolectar_usos_expr(inner, usos);
        }
        Expresion::Lanzar(inner, _) => {
            recolectar_usos_expr(inner, usos);
        }
        Expresion::Bloquear(inner, _) => {
            recolectar_usos_expr(inner, usos);
        }
        Expresion::DireccionDe(_, _) => {}  // referencia a función, sin variables
        Expresion::AccesoArray(base, indice, _) => {
            recolectar_usos_expr(base, usos);
            recolectar_usos_expr(indice, usos);
        }
        Expresion::AccesoCampo(objeto, _, _) => {
            recolectar_usos_expr(objeto, usos);
        }
        Expresion::LiteralArray(exprs, _) => {
            for e in exprs {
                recolectar_usos_expr(e, usos);
            }
        }
        Expresion::ArrayRelleno(expr, _, _) => {
            recolectar_usos_expr(expr, usos);
        }
        Expresion::InicializacionStruct(_, campos, _) => {
            for (_, e) in campos {
                recolectar_usos_expr(e, usos);
            }
        }
        Expresion::ConstructorEnum(_, _, args, _) => {
            for arg in args {
                recolectar_usos_expr(arg, usos);
            }
        }
        Expresion::EsVariante(expr, _, _, _, _) => {
            recolectar_usos_expr(expr, usos);
        }
        Expresion::Propagacion(expr, _) => {
            recolectar_usos_expr(expr, usos);
        }
        Expresion::Mover(_, destino, _) => {
            if let Some(d) = destino {
                recolectar_usos_expr(d, usos);
            }
        }
        Expresion::Copiar(expr, _) => {
            recolectar_usos_expr(expr, usos);
        }
        Expresion::Rango(inicio, fin, _, _) => {
            recolectar_usos_expr(inicio, usos);
            recolectar_usos_expr(fin, usos);
        }
        Expresion::Closure(_, cuerpo, _) => {
            recolectar_usos_expr(cuerpo, usos);
        }
        Expresion::Coincidir(sujeto, brazos, _) => {
            recolectar_usos_expr(sujeto, usos);
            for brazo in brazos {
                recolectar_usos_expr(&brazo.cuerpo, usos);
            }
        }
        Expresion::Metodo(receptor, _, args, _) => {
            recolectar_usos_expr(receptor, usos);
            for arg in args {
                recolectar_usos_expr(arg, usos);
            }
        }
        Expresion::Bloque(bloque) => {
            for sentencia in &bloque.sentencias {
                if let Sentencia::Expresion(expr) = sentencia {
                    recolectar_usos_expr(expr, usos);
                }
            }
        }
        Expresion::Literal(_) | Expresion::Ruta(_, _) => {}
    }
}

/// Busca el tipo de una variable en las sentencias o parámetros
fn buscar_tipo_var(nombre: &str, sentencias: &[Sentencia], parametros: &[Parametro]) -> Tipo {
    // Buscar en parámetros
    for param in parametros {
        if param.nombre == nombre {
            return param.tipo.clone();
        }
    }
    // Buscar en declaraciones de variables
    for sent in sentencias {
        if let Sentencia::DeclaracionVariable(decl) = sent {
            if decl.nombre == nombre {
                if let Some(ref tipo) = decl.tipo {
                    return tipo.clone();
                }
            }
        }
    }
    // Fallback: Entero32
    Tipo::Entero32
}

/// Calcula offsets con alineación C (i32=4, i64=8)
fn calcular_offsets(vars: &mut [VarLiveAcross]) {
    let mut offset: u32 = 0;
    for var in vars.iter_mut() {
        let (tamano, alineacion) = tamano_tipo(&var.tipo);
        // Alinear
        if offset % alineacion != 0 {
            offset += alineacion - (offset % alineacion);
        }
        var.offset = offset;
        offset += tamano;
    }
}

/// Tamaño y alineación de un tipo para layout del struct
pub fn tamano_tipo(tipo: &Tipo) -> (u32, u32) {
    match tipo {
        Tipo::Entero8 | Tipo::Natural8 => (1, 1),
        Tipo::Entero16 | Tipo::Natural16 => (2, 2),
        Tipo::Entero32 | Tipo::Natural32 | Tipo::Booleano | Tipo::Caracter => (4, 4),
        Tipo::Entero64 | Tipo::Natural64 => (8, 8),
        Tipo::Flotante32 => (4, 4),
        Tipo::Flotante64 => (8, 8),
        Tipo::Palabra | Tipo::Texto => (8, 8), // punteros
        Tipo::Referencia(_) | Tipo::ReferenciaMut(_) => (8, 8),
        Tipo::ReferenciaConLifetime(_, _) | Tipo::ReferenciaMutConLifetime(_, _) => (8, 8),
        Tipo::ReferenciaSelf(_) | Tipo::ReferenciaMutSelf(_) => (8, 8),
        Tipo::Puntero(_) => (8, 8),
        Tipo::Array(inner, len) => {
            let (t, a) = tamano_tipo(inner);
            (t * (*len as u32), a)
        }
        Tipo::Vector(_) => (24, 8), // {ptr, len, cap}
        _ => (8, 8),
    }
}

/// Tamaño total del struct del state machine (incluye campo `state: i32` al inicio)
pub fn tamano_struct_futuro(analisis: &AnalisisFuturo) -> u32 {
    // Layout: state(i32) + vars... + deadline(i64)
    let mut offset: u32 = 4; // state field
    for var in &analisis.vars_struct {
        let (tamano, alineacion) = tamano_tipo(&var.tipo);
        if offset % alineacion != 0 {
            offset += alineacion - (offset % alineacion);
        }
        offset += tamano;
    }
    // deadline: i64 para timers (siempre presente)
    if offset % 8 != 0 {
        offset += 8 - (offset % 8);
    }
    offset += 8; // deadline field
    offset
}

/// Offset del campo `deadline` en el struct del futuro
pub fn offset_deadline(analisis: &AnalisisFuturo) -> u32 {
    let mut offset: u32 = 4; // state
    for var in &analisis.vars_struct {
        let (tamano, alineacion) = tamano_tipo(&var.tipo);
        if offset % alineacion != 0 {
            offset += alineacion - (offset % alineacion);
        }
        offset += tamano;
    }
    if offset % 8 != 0 {
        offset += 8 - (offset % 8);
    }
    offset
}

/// Offset de una variable específica en el struct del futuro
pub fn offset_var(analisis: &AnalisisFuturo, nombre: &str) -> Option<u32> {
    // state está en offset 0
    let mut offset: u32 = 4;
    for var in &analisis.vars_struct {
        let (tamano, alineacion) = tamano_tipo(&var.tipo);
        if offset % alineacion != 0 {
            offset += alineacion - (offset % alineacion);
        }
        if var.nombre == nombre {
            return Some(offset);
        }
        offset += tamano;
    }
    None
}
