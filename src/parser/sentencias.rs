use crate::ast::*;
use crate::lexer::Token;
use crate::span::Span;
use super::{ErrorSintaxis, ParserCursor};
use super::tipos::parse_articulo;
use super::expresiones::parse_expresion;

/// Parsea una sentencia con span real
pub fn parse_sentencia(cursor: &mut ParserCursor) -> Result<Sentencia, ErrorSintaxis> {
    match cursor.actual() {
        Some(Token::Retornar) => parse_retornar(cursor),
        Some(Token::Si) => parse_condicional(cursor),
        Some(Token::Mientras) => parse_bucle_mientras(cursor),
        Some(Token::Para) => parse_bucle_para(cursor),
        Some(Token::Region) => parse_region(cursor),
        Some(Token::Seleccionar) => parse_seleccionar(cursor),
        Some(Token::ConExecutor) => parse_con_executor(cursor),
        Some(Token::ArticuloEl) |
        Some(Token::ArticuloLa) |
        Some(Token::ArticuloUn) |
        Some(Token::ArticuloLos) |
        Some(Token::ArticuloLas) => parse_declaracion_variable(cursor),
        Some(Token::Identificador(_)) => {
            // Verificar si es asignación (ident = expr) o expresión
            if es_asignacion(cursor) {
                parse_asignacion(cursor)
            } else {
                let expr = parse_expresion(cursor)?;
                cursor.esperar(Token::PuntoYComa)?;
                Ok(Sentencia::Expresion(expr))
            }
        }
        _ => {
            let expr = parse_expresion(cursor)?;
            cursor.esperar(Token::PuntoYComa)?;
            Ok(Sentencia::Expresion(expr))
        }
    }
}

/// Verifica si el token actual es identificador seguido de `=` o `[...] =` o `.campo =`
fn es_asignacion(cursor: &ParserCursor) -> bool {
    if let Some(Token::Identificador(_)) = cursor.actual() {
        // Buscar '=' saltando posibles accesos a array [expr] o .campo
        let mut offset = 1;
        if let Some(Token::CorcheteAbre) = cursor.peek(offset) {
            offset += 1;
            // Saltar contenido hasta encontrar CorcheteCierra
            let mut profundidad = 1;
            while profundidad > 0 {
                match cursor.peek(offset) {
                    Some(Token::CorcheteAbre) => profundidad += 1,
                    Some(Token::CorcheteCierra) => profundidad -= 1,
                    Some(_) => {}
                    None => return false,
                }
                offset += 1;
            }
        } else if let Some(Token::Punto) = cursor.peek(offset) {
            // ident.campo = ...
            offset += 1; // .
            if let Some(Token::Identificador(_)) = cursor.peek(offset) {
                offset += 1; // campo
            } else {
                return false;
            }
        }
        if let Some(Token::Igual) = cursor.peek(offset) {
            return true;
        }
    }
    false
}

/// Parsea una asignación: identificador = expresion; o identificador[expr] = expr; o ident.campo = expr;
fn parse_asignacion(cursor: &mut ParserCursor) -> Result<Sentencia, ErrorSintaxis> {
    let span_inicio = cursor.span_actual();
    
    // Parsear lado izquierdo (puede ser ident, array[índice], o ident.campo)
    let base = match cursor.actual() {
        Some(Token::Identificador(n)) => {
            let n = n.clone();
            cursor.avanzar();
            Expresion::Identificador(n, span_inicio.clone())
        }
        _ => {
            let span = cursor.span_actual();
            return Err(ErrorSintaxis::identificador_esperado(span, "asignación"));
        }
    };
    
    // Verificar si es acceso a array: base[expr] = ...
    // o acceso a campo: base.campo = ... (Fase 15B: bitfield write)
    let lugar = if let Some(Token::CorcheteAbre) = cursor.actual() {
        cursor.avanzar(); // [
        let indice = parse_expresion(cursor)?;
        cursor.esperar(Token::CorcheteCierra)?;
        Lugar::Array(Box::new(base), Box::new(indice))
    } else if let Some(Token::Punto) = cursor.actual() {
        cursor.avanzar(); // .
        let nombre_campo = match cursor.actual() {
            Some(Token::Identificador(n)) => {
                let n = n.clone();
                cursor.avanzar();
                n
            }
            _ => {
                let span = cursor.span_actual();
                return Err(ErrorSintaxis::identificador_esperado(span, "nombre de campo después de '.'"));
            }
        };
        Lugar::Campo(Box::new(base), nombre_campo)
    } else {
        match base {
            Expresion::Identificador(nombre, _) => Lugar::Identificador(nombre),
            _ => unreachable!(),
        }
    };

    cursor.esperar(Token::Igual)?;
    let valor = parse_expresion(cursor)?;
    cursor.esperar(Token::PuntoYComa)?;
    
    let span = Span::combinar(
        &span_inicio, &cursor.span_actual()
    );

    Ok(Sentencia::Asignacion(Asignacion {
        lugar,
        valor,
        span,
    }))
}

/// Parsea un retorno: retornar [expresion];
fn parse_retornar(cursor: &mut ParserCursor) -> Result<Sentencia, ErrorSintaxis> {
    let span_inicio = cursor.span_actual();
    cursor.esperar(Token::Retornar)?;

    let expr = if cursor.actual() != Some(&Token::PuntoYComa) {
        Some(parse_expresion(cursor)?)
    } else {
        None
    };

    cursor.esperar(Token::PuntoYComa)?;
    let span = Span::combinar(&span_inicio, &cursor.span_actual()
    );

    Ok(Sentencia::Retornar(expr, span))
}

/// Parsea un condicional con soporte para ser/estar y subjuntivo:
/// si expr { ... }                     — indicativo
/// si expr es expr { ... }             — identidad
/// si expr está expr { ... }           — estado
/// si expr fuese { ... }               — subjuntivo
/// si expr fuese es expr { ... }       — identidad subjuntiva
fn parse_condicional(cursor: &mut ParserCursor) -> Result<Sentencia, ErrorSintaxis> {
    let span_inicio = cursor.span_actual();
    cursor.esperar(Token::Si)?;

    // Parsear expresión base de la condición
    let mut expr_izq = parse_expresion(cursor)?;
    let mut modo = ModoVerbal::Indicativo;

    // Verificar si hay 'fuese' (subjuntivo)
    if cursor.actual() == Some(&Token::Fuese) {
        cursor.avanzar();
        modo = ModoVerbal::Subjuntivo;
    }

    // Verificar si hay 'es' o 'está' (ser/estar)
    let condicion = match cursor.actual() {
        Some(Token::Es) => {
            cursor.avanzar();
            
            // Verificar si es pattern matching de enum: Estado.Activo [como variable]
            // También soporta Resultado.Exito (keyword como nombre de enum)
            let enum_nombre_opt = match cursor.actual() {
                Some(Token::Identificador(n)) => Some(n.clone()),
                Some(Token::Resultado) => Some("Resultado".to_string()),
                _ => None,
            };
            
            if let (Some(enum_nombre), Some(Token::Punto), Some(Token::Identificador(variante_nombre))) = 
                (enum_nombre_opt, cursor.peek(1), cursor.peek(2)) {
                let variante_nombre = variante_nombre.clone();
                cursor.avanzar(); // enum
                cursor.avanzar(); // .
                cursor.avanzar(); // variante
                
                // Verificar si hay binding: como variable
                let binding = if cursor.actual() == Some(&Token::Como) {
                    cursor.avanzar(); // como
                    match cursor.actual() {
                        Some(Token::Identificador(n)) => {
                            let n = n.clone();
                            cursor.avanzar();
                            Some(n)
                        }
                        _ => {
                            let span = cursor.span_actual();
                            return Err(ErrorSintaxis::identificador_esperado(span, "nombre de variable después de 'como'"));
                        }
                    }
                } else {
                    None
                };
                
                let span = Span::combinar(expr_izq.span(), &cursor.span_actual());
                Expresion::EsVariante(Box::new(expr_izq), enum_nombre, variante_nombre, binding, span)
            } else {
                let expr_der = parse_expresion(cursor)?;
                let span = Span::combinar(expr_izq.span(), expr_der.span());
                Expresion::Binaria(Box::new(expr_izq), OperadorBinario::Igual, Box::new(expr_der), span)
            }
        }
        Some(Token::Esta) => {
            cursor.avanzar();
            modo = ModoVerbal::Estativo;
            let condicion = if cursor.actual() == Some(&Token::LlaveAbre) {
                // "está" sin RHS: check de truthiness (non-zero / not null)
                // El valor se pasa directamente a brif en codegen
                expr_izq
            } else {
                let expr_der = parse_expresion(cursor)?;
                let span = Span::combinar(expr_izq.span(), expr_der.span());
                Expresion::Binaria(Box::new(expr_izq), OperadorBinario::Igual, Box::new(expr_der), span)
            };
            condicion
        }
        _ => {
            // Condición simple sin ser/estar
            expr_izq
        }
    };

    let bloque_entonces = parse_bloque(cursor)?;

    let bloque_sino = if cursor.actual() == Some(&Token::Sino) {
        cursor.avanzar();
        Some(parse_bloque(cursor)?)
    } else {
        None
    };

    let span_fin = cursor.span_actual();
    let span = Span::combinar(&span_inicio, &span_fin);

    Ok(Sentencia::Condicional(Condicional {
        condicion,
        bloque_entonces,
        bloque_sino,
        modo,
        span,
    }))
}

/// Parsea un bucle mientras: mientras expresion { bloque }
fn parse_bucle_mientras(cursor: &mut ParserCursor) -> Result<Sentencia, ErrorSintaxis> {
    let span_inicio = cursor.span_actual();
    cursor.esperar(Token::Mientras)?;

    let condicion = parse_expresion(cursor)?;
    let bloque = parse_bloque(cursor)?;
    
    let span_fin = cursor.span_actual();
    let span = Span::combinar(&span_inicio, &span_fin
    );

    Ok(Sentencia::BucleMientras(BucleMientras {
        condicion,
        bloque,
        span,
    }))
}

/// Parsea un bucle para: para identificador en expresion { bloque }
fn parse_bucle_para(cursor: &mut ParserCursor) -> Result<Sentencia, ErrorSintaxis> {
    let span_inicio = cursor.span_actual();
    cursor.esperar(Token::Para)?;

    let variable = match cursor.actual() {
        Some(Token::Identificador(n)) => {
            let n = n.clone();
            cursor.avanzar();
            n
        }
        _ => {
            let span = cursor.span_actual();
            return Err(ErrorSintaxis::identificador_esperado(span, "variable de iteración"));
        }
    };

    cursor.esperar(Token::En)?;
    let iterable = parse_expresion(cursor)?;
    let bloque = parse_bloque(cursor)?;
    
    let span_fin = cursor.span_actual();
    let span = Span::combinar(&span_inicio, &span_fin);

    Ok(Sentencia::BuclePara(BuclePara {
        variable,
        iterable,
        bloque,
        span,
    }))
}

/// Parsea una declaración de variable: articulo nombre [: tipo] = expresion;
fn parse_declaracion_variable(cursor: &mut ParserCursor) -> Result<Sentencia, ErrorSintaxis> {
    let span_inicio = cursor.span_actual();
    let articulo = parse_articulo(cursor)?;

    let nombre = match cursor.actual() {
        Some(Token::Identificador(n)) => {
            let n = n.clone();
            cursor.avanzar();
            n
        }
        _ => {
            let span = cursor.span_actual();
            return Err(ErrorSintaxis::identificador_esperado(span, "variable"));
        }
    };

    let tipo = if let Some(Token::DosPuntos) = cursor.actual() {
        cursor.avanzar();
        Some(super::tipos::parse_tipo(cursor)?)
    } else {
        None
    };

    cursor.esperar(Token::Igual)?;
    
    // Verificar si es inicialización de struct: Nombre { campo: valor, ... }
    let valor = if let Some(Token::Identificador(n)) = cursor.actual() {
        if cursor.peek(1) == Some(&Token::LlaveAbre) {
            let nombre_struct = n.clone();
            let span_inicio = cursor.span_actual();
            cursor.avanzar(); // Nombre struct
            cursor.avanzar(); // {
            
            let mut campos = Vec::new();
            while cursor.actual() != Some(&Token::LlaveCierra) && !cursor.esta_vacio() {
                let nombre_campo = match cursor.actual() {
                    Some(Token::Identificador(cn)) => {
                        let cn = cn.clone();
                        cursor.avanzar();
                        cn
                    }
                    _ => {
                        let span = cursor.span_actual();
                        return Err(ErrorSintaxis::identificador_esperado(span, "nombre de campo en inicialización de struct"));
                    }
                };
                
                cursor.esperar(Token::DosPuntos)?;
                let val = parse_expresion(cursor)?;
                campos.push((nombre_campo, val));
                
                if let Some(Token::Coma) = cursor.actual() {
                    cursor.avanzar();
                } else {
                    break;
                }
            }
            
            cursor.esperar(Token::LlaveCierra)?;
            let span_fin = cursor.span_actual();
            let span = Span::combinar(&span_inicio, &span_fin);
            Expresion::InicializacionStruct(nombre_struct, campos, span)
        } else {
            parse_expresion(cursor)?
        }
    } else {
        parse_expresion(cursor)?
    };
    
    cursor.esperar(Token::PuntoYComa)?;
    
    let span = Span::combinar(&span_inicio, &cursor.span_actual()
    );

    Ok(Sentencia::DeclaracionVariable(DeclaracionVariable {
        articulo,
        nombre,
        tipo,
        valor,
        span,
    }))
}

/// Parsea un bloque: { sentencias }
pub fn parse_bloque(cursor: &mut ParserCursor) -> Result<Bloque, ErrorSintaxis> {
    let span_inicio = cursor.span_actual();
    cursor.esperar(Token::LlaveAbre)?;

    let mut sentencias = Vec::new();

    while cursor.actual() != Some(&Token::LlaveCierra) && !cursor.esta_vacio() {
        sentencias.push(parse_sentencia(cursor)?);
    }

    cursor.esperar(Token::LlaveCierra)?;
    
    let span = Span::combinar(
        &span_inicio, &cursor.span_actual()
    );

    Ok(Bloque {
        sentencias,
        span,
    })
}

/// Parsea una región: región nombre { sentencias }
fn parse_region(cursor: &mut ParserCursor) -> Result<Sentencia, ErrorSintaxis> {
    let span_inicio = cursor.span_actual();
    cursor.esperar(Token::Region)?;
    
    // Nombre de la región (identificador)
    let nombre = match cursor.actual() {
        Some(Token::Identificador(n)) => {
            let n = n.clone();
            cursor.avanzar();
            n
        }
        _ => {
            return Err(ErrorSintaxis::nuevo(
                50,
                cursor.span_actual(),
                "Se esperaba un nombre para la región (ej: región mi_región { ... })".to_string(),
            ));
        }
    };
    
    // Bloque de la región
    let bloque = parse_bloque(cursor)?;
    
    let span = Span::combinar(&span_inicio, &bloque.span);
    
    Ok(Sentencia::Region {
        nombre,
        cuerpo: bloque.sentencias,
        span,
    })
}

/// Parsea seleccionar { canal como v => { ... }, _ => { ... } }
fn parse_seleccionar(cursor: &mut ParserCursor) -> Result<Sentencia, ErrorSintaxis> {
    let span_inicio = cursor.span_actual();
    cursor.esperar(Token::Seleccionar)?;
    cursor.esperar(Token::LlaveAbre)?;

    let mut ramas = Vec::new();

    while cursor.actual() != Some(&Token::LlaveCierra) {
        let span_rama = cursor.span_actual();

        // Rama default: _ => { ... }
        let (canal, variable) = match cursor.actual() {
            Some(Token::Identificador(n)) if n == "_" => {
                cursor.avanzar();
                (Expresion::Literal(Literal::Entero(0, span_rama.clone())), None)
            }
            _ => {
                // canal como variable => { ... }
                let canal_expr = parse_expresion(cursor)?;
                cursor.esperar(Token::Como)?;
                let var_nombre = match cursor.actual() {
                    Some(Token::Identificador(n)) => {
                        let n = n.clone();
                        cursor.avanzar();
                        n
                    }
                    _ => {
                        return Err(ErrorSintaxis::nuevo(
                            51,
                            cursor.span_actual(),
                            "Se esperaba un nombre de variable después de 'como' en seleccionar".to_string(),
                        ));
                    }
                };
                (canal_expr, Some(var_nombre))
            }
        };

        cursor.esperar(Token::FlechaGruesa)?;
        let cuerpo = parse_bloque(cursor)?;
        let span = Span::combinar(&span_rama, &cuerpo.span);

        ramas.push(RamaSeleccionar {
            canal,
            variable,
            cuerpo,
            span,
        });

        // Coma opcional entre ramas
        if cursor.actual() == Some(&Token::Coma) {
            cursor.avanzar();
        }
    }

    cursor.esperar(Token::LlaveCierra)?;
    let span = Span::combinar(&span_inicio, &cursor.span_actual());

    Ok(Sentencia::Seleccionar(Seleccionar { ramas, span }))
}

/// Parsea con_executor(N) { sentencias }
fn parse_con_executor(cursor: &mut ParserCursor) -> Result<Sentencia, ErrorSintaxis> {
    let span_inicio = cursor.span_actual();
    cursor.esperar(Token::ConExecutor)?;
    cursor.esperar(Token::ParenAbre)?;
    let hilos = parse_expresion(cursor)?;
    cursor.esperar(Token::ParenCierra)?;
    let bloque = parse_bloque(cursor)?;
    let span = Span::combinar(&span_inicio, &bloque.span);

    Ok(Sentencia::ConExecutor {
        hilos,
        cuerpo: bloque.sentencias,
        span,
    })
}
