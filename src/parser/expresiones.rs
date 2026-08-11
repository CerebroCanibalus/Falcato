use crate::ast::*;
use crate::lexer::Token;
use crate::span::Span;
use super::{ErrorSintaxis, ParserCursor};

/// Parsea una expresión completa con precedencia y span real
pub fn parse_expresion(cursor: &mut ParserCursor) -> Result<Expresion, ErrorSintaxis> {
    let mut izquierda = parse_expresion_precedencia(cursor, 0)?;

    // R7.7 F1 — Subjuntivo aritmético: `a + b fuese` → checked (Resultado<T, Entero32>)
    // Se aplica AQUÍ (entrada pública, NO recursiva): el check en parse_expresion_precedencia
    // envolvería el lado derecho (`a + Checked(b)`) en vez de la operación completa.
    // Desambiguación lingüística: el token SIGUIENTE al `fuese` decide el modo:
    //   ; ) } , EOF → checked aritmético ("la suma se desbordara...")
    //   { > < etc.  → subjuntivo de condición (lo maneja parse_condicional: si x fuese > 10)
    if cursor.actual() == Some(&Token::Fuese) {
        let siguiente = cursor.peek(1);
        let es_delimitador = matches!(
            siguiente,
            Some(Token::PuntoYComa)
                | Some(Token::ParenCierra)
                | Some(Token::LlaveCierra)
                | Some(Token::Coma)
                | None
        );
        if es_delimitador {
            let span = izquierda.span().clone();
            cursor.avanzar();
            izquierda = Expresion::Checked(Box::new(izquierda), span);
        }
    }

    // Rangos: expr..expr (exclusivo) o expr..=expr (inclusivo)
    // Precedencia más baja que todo lo demás
    match cursor.actual() {
        Some(Token::DoblePunto) => {
            let span_inicio = izquierda.span().clone();
            cursor.avanzar();
            let derecha = parse_expresion_precedencia(cursor, 0)?;
            let span = Span::combinar(&span_inicio, derecha.span());
            Ok(Expresion::Rango(Box::new(izquierda), Box::new(derecha), false, span))
        }
        Some(Token::PuntoPuntoIgual) => {
            let span_inicio = izquierda.span().clone();
            cursor.avanzar();
            let derecha = parse_expresion_precedencia(cursor, 0)?;
            let span = Span::combinar(&span_inicio, derecha.span());
            Ok(Expresion::Rango(Box::new(izquierda), Box::new(derecha), true, span))
        }
        _ => Ok(izquierda),
    }
}

/// Parser de expresiones con precedencia (Pratt parser)
/// Precedencia (mayor número = mayor precedencia):
/// 1: ||
/// 2: &&
/// 3: |  (bitwise OR)
/// 4: ^  (bitwise XOR)
/// 5: &  (bitwise AND)
/// 6: ==, !=
/// 7: <, >, <=, >=
/// 8: <<, >>, >>> (shifts)
/// 9: +, -
/// 10: *, /, %
/// 11: unario: -, !, ~
fn parse_expresion_precedencia(
    cursor: &mut ParserCursor,
    min_precedencia: u8,
) -> Result<Expresion, ErrorSintaxis> {
    let mut izquierda = parse_atom(cursor)?;
    
    // Aplicar postfix (acceso a array, acceso a campo) inmediatamente después del átomo
    izquierda = parse_postfix(cursor, izquierda)?;

    loop {
        let (op, precedencia) = match cursor.actual() {
            Some(Token::OLogico) => (OperadorBinario::O, 1),
            Some(Token::YLogico) => (OperadorBinario::Y, 2),
            Some(Token::Pipe) => (OperadorBinario::BitOr, 3),
            Some(Token::Caret) => (OperadorBinario::BitXor, 4),
            Some(Token::Ampersand) => (OperadorBinario::BitAnd, 5),
            Some(Token::IgualIgual) => (OperadorBinario::Igual, 6),
            Some(Token::Distinto) => (OperadorBinario::Distinto, 6),
            Some(Token::MenorQue) => (OperadorBinario::Menor, 7),
            Some(Token::MayorQue) => (OperadorBinario::Mayor, 7),
            Some(Token::MenorIgual) => (OperadorBinario::MenorIgual, 7),
            Some(Token::MayorIgual) => (OperadorBinario::MayorIgual, 7),
            Some(Token::DobleMenor) => (OperadorBinario::ShiftLeft, 8),
            Some(Token::DobleMayor) => (OperadorBinario::ShiftRight, 8),
            Some(Token::TripleMayor) => (OperadorBinario::ShiftRightLogico, 8),
            Some(Token::Mas) => (OperadorBinario::Suma, 9),
            Some(Token::Menos) => (OperadorBinario::Resta, 9),
            Some(Token::Asterisco) => (OperadorBinario::Multiplicacion, 10),
            Some(Token::Barra) => (OperadorBinario::Division, 10),
            Some(Token::Porcentaje) => (OperadorBinario::Modulo, 10),
            _ => break,
        };

        if precedencia < min_precedencia {
            break;
        }

        cursor.avanzar();
        let mut derecha = parse_expresion_precedencia(cursor, precedencia + 1)?;
        
        // Aplicar postfix al lado derecho también
        derecha = parse_postfix(cursor, derecha)?;
        
        let span = Span::combinar(izquierda.span(), derecha.span());
        
        izquierda = Expresion::Binaria(
            Box::new(izquierda),
            op,
            Box::new(derecha),
            span,
        );
    }

    Ok(izquierda)
}

/// Parsea operadores postfix: expr[índice], expr.campo, expr?
fn parse_postfix(
    cursor: &mut ParserCursor,
    base: Expresion,
) -> Result<Expresion, ErrorSintaxis> {
    let mut resultado = base;
    
    loop {
        match cursor.actual() {
            Some(Token::CorcheteAbre) => {
                let span_inicio = resultado.span().clone();
                cursor.avanzar(); // [
                let indice = parse_expresion(cursor)?;
                cursor.esperar(Token::CorcheteCierra)?;
                let span_fin = cursor.span_actual();
                let span = Span::combinar(&span_inicio, &span_fin);
                resultado = Expresion::AccesoArray(Box::new(resultado), Box::new(indice), span);
            }
            Some(Token::Punto) => {
                let span_inicio = resultado.span().clone();
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
                // Fase 15A/15F: si sigue ParenAbre, es método: x.metodo(args)
                // General: funciona para bitwise, texto, vector, etc.
                if cursor.actual() == Some(&Token::ParenAbre) {
                    cursor.avanzar(); // (
                    let mut args = Vec::new();
                    if cursor.actual() != Some(&Token::ParenCierra) {
                        args.push(parse_expresion(cursor)?);
                        while cursor.actual() == Some(&Token::Coma) {
                            cursor.avanzar(); // ,
                            args.push(parse_expresion(cursor)?);
                        }
                    }
                    cursor.esperar(Token::ParenCierra)?;
                    let span = Span::combinar(&span_inicio, &cursor.span_actual());
                    resultado = Expresion::Metodo(Box::new(resultado), nombre_campo, args, span);
                } else {
                    let span = Span::combinar(&span_inicio, &cursor.span_actual());
                    resultado = Expresion::AccesoCampo(Box::new(resultado), nombre_campo, span);
                }
            }
            Some(Token::Interrogacion) => {
                let span_inicio = resultado.span().clone();
                cursor.avanzar(); // ?
                let span_fin = cursor.span_actual();
                let span = Span::combinar(&span_inicio, &span_fin);
                resultado = Expresion::Propagacion(Box::new(resultado), span);
            }
            _ => break,
        }
    }
    
    Ok(resultado)
}

/// Determina si a partir del token `<` actual hay una llamada genérica:
/// identificador < tipo [,< tipo]* > (
/// Usa balanceo de </> para soportar tipos anidados como Vector<Entero32>.
fn es_llamada_generica(cursor: &ParserCursor) -> bool {
    if cursor.actual() != Some(&Token::MenorQue) {
        return false;
    }
    
    let mut profundidad: i32 = 1;
    let mut i: usize = 1;
    
    while profundidad > 0 {
        match cursor.peek(i) {
            Some(Token::MenorQue) => profundidad += 1,
            Some(Token::MayorQue) => profundidad -= 1,
            Some(Token::Error) | None => return false,
            _ => {}
        }
        i += 1;
    }
    
    // i apunta al token siguiente al > de cierre
    cursor.peek(i) == Some(&Token::ParenAbre)
}

/// Parsea argumentos de tipo para una llamada genérica: <Tipo1, Tipo2>
/// El cursor debe apuntar a `<`; se consume hasta el `>` inclusive.
fn parse_tipo_args(cursor: &mut ParserCursor) -> Result<Vec<Tipo>, ErrorSintaxis> {
    use crate::parser::tipos::parse_tipo;
    
    cursor.esperar(Token::MenorQue)?;
    let mut args = Vec::new();
    
    while cursor.actual() != Some(&Token::MayorQue) && !cursor.esta_vacio() {
        args.push(parse_tipo(cursor)?);
        if let Some(Token::Coma) = cursor.actual() {
            cursor.avanzar();
        } else {
            break;
        }
    }
    
    cursor.esperar(Token::MayorQue)?;
    Ok(args)
}

/// Parsea un átomo: literal, identificador, llamada a función, paréntesis, unario, array literal, todos
fn parse_atom(cursor: &mut ParserCursor) -> Result<Expresion, ErrorSintaxis> {
    // Manejar operadores unarios y closures
    match cursor.actual() {
        // Closure: |params| cuerpo
        Some(Token::Pipe) => {
            let span_inicio = cursor.span_actual();
            cursor.avanzar(); // consumir |
            
            // Parsear parámetros: |x|, |x, y|, |x: Entero32|
            let mut params: Vec<(String, Option<Tipo>)> = Vec::new();
            while cursor.actual() != Some(&Token::Pipe) && !cursor.esta_vacio() {
                let nombre = match cursor.actual() {
                    Some(Token::Identificador(n)) => {
                        let n = n.clone();
                        cursor.avanzar();
                        n
                    }
                    _ => {
                        let span = cursor.span_actual();
                        return Err(ErrorSintaxis::identificador_esperado(span, "parámetro de closure"));
                    }
                };
                // Tipo opcional: x: Entero32
                let tipo = if cursor.actual() == Some(&Token::DosPuntos) {
                    cursor.avanzar();
                    Some(super::tipos::parse_tipo(cursor)?)
                } else {
                    None
                };
                params.push((nombre, tipo));
                if cursor.actual() == Some(&Token::Coma) {
                    cursor.avanzar();
                }
            }
            cursor.esperar(Token::Pipe)?; // consumir | de cierre
            
            // Parsear cuerpo: expresión o bloque
            let cuerpo = if cursor.actual() == Some(&Token::LlaveAbre) {
                // Bloque: |x| { ... } → wrap en expresión de bloque
                let _bloque = super::sentencias::parse_bloque(cursor)?;
                // Convertir bloque a expresión: usar la última expresión como valor
                // Por ahora, wrap como bloque con retorno implícito
                let span_bloque = Span::combinar(&span_inicio, &cursor.span_actual());
                Expresion::Closure(params, Box::new(Expresion::Literal(Literal::Entero(0, span_bloque.clone()))), span_bloque)
            } else {
                let cuerpo_expr = parse_expresion(cursor)?;
                let span = Span::combinar(&span_inicio, cuerpo_expr.span());
                Expresion::Closure(params, Box::new(cuerpo_expr), span)
            };
            
            return Ok(cuerpo);
        }
        Some(Token::Menos) => {
            let span_inicio = cursor.span_actual();
            cursor.avanzar();
            let expr = parse_atom(cursor)?;
            let span = Span::combinar(&span_inicio, expr.span());
            return Ok(Expresion::Unaria(
                OperadorUnario::Negacion,
                Box::new(expr),
                span,
            ));
        }
        Some(Token::Exclamacion) => {
            let span_inicio = cursor.span_actual();
            cursor.avanzar();
            let expr = parse_atom(cursor)?;
            let span = Span::combinar(&span_inicio, expr.span());
            return Ok(Expresion::Unaria(
                OperadorUnario::NegacionLogica,
                Box::new(expr),
                span,
            ));
        }
        Some(Token::Tilde) => {
            let span_inicio = cursor.span_actual();
            cursor.avanzar();
            let expr = parse_atom(cursor)?;
            let span = Span::combinar(&span_inicio, expr.span());
            return Ok(Expresion::Unaria(
                OperadorUnario::BitNot,
                Box::new(expr),
                span,
            ));
        }
        Some(Token::Ampersand) => {
            let span_inicio = cursor.span_actual();
            cursor.avanzar(); // &
            
            // Verificar si es &mut
            if cursor.actual() == Some(&Token::Mut) {
                cursor.avanzar(); // mut
                let expr = parse_expresion(cursor)?;
                let span = Span::combinar(&span_inicio, expr.span());
                return Ok(Expresion::Unaria(
                    OperadorUnario::ReferenciaMut,
                    Box::new(expr),
                    span,
                ));
            } else {
                let expr = parse_expresion(cursor)?;
                let span = Span::combinar(&span_inicio, expr.span());
                return Ok(Expresion::Unaria(
                    OperadorUnario::Referencia,
                    Box::new(expr),
                    span,
                ));
            }
        }
        Some(Token::Asterisco) => {
            // *expr — dereferencia
            let span_inicio = cursor.span_actual();
            cursor.avanzar(); // *
            let expr = parse_expresion(cursor)?;
            let span = Span::combinar(&span_inicio, expr.span());
            return Ok(Expresion::Unaria(
                OperadorUnario::Desreferencia,
                Box::new(expr),
                span,
            ));
        }
        // Async (Fase 18): esperar expr
        Some(Token::Esperar) => {
            let span_inicio = cursor.span_actual();
            cursor.avanzar(); // esperar
            let expr = parse_expresion(cursor)?;
            let span = Span::combinar(&span_inicio, expr.span());
            return Ok(Expresion::Esperar(Box::new(expr), span));
        }
        // Async (Fase 18): lanzar expr
        Some(Token::Lanzar) => {
            let span_inicio = cursor.span_actual();
            cursor.avanzar(); // lanzar
            let expr = parse_expresion(cursor)?;
            let span = Span::combinar(&span_inicio, expr.span());
            return Ok(Expresion::Lanzar(Box::new(expr), span));
        }
        // Async (Fase 18): bloquear(expr)
        Some(Token::Bloquear) => {
            let span_inicio = cursor.span_actual();
            cursor.avanzar(); // bloquear
            cursor.esperar(Token::ParenAbre)?;
            let expr = parse_expresion(cursor)?;
            cursor.esperar(Token::ParenCierra)?;
            let span = Span::combinar(&span_inicio, expr.span());
            return Ok(Expresion::Bloquear(Box::new(expr), span));
        }
        // GUI (Fase GUI-1): direccion_de(funcion) / dir_de(funcion)
        Some(Token::DireccionDe) | Some(Token::DirDe) => {
            let span_inicio = cursor.span_actual();
            cursor.avanzar(); // direccion_de / dir_de
            cursor.esperar(Token::ParenAbre)?;
            let nombre = match cursor.actual() {
                Some(Token::Identificador(n)) => {
                    let n = n.clone();
                    cursor.avanzar();
                    n
                }
                _ => {
                    let span = cursor.span_actual();
                    return Err(ErrorSintaxis::identificador_esperado(span, "nombre de función después de 'direccion_de('"));
                }
            };
            cursor.esperar(Token::ParenCierra)?;
            let span = Span::combinar(&span_inicio, &cursor.span_actual());
            return Ok(Expresion::DireccionDe(nombre, span));
        }
        _ => {}
    }

    match cursor.actual() {
        Some(Token::EnteroLiteral(Some(n))) => {
            let val = *n;
            let span = cursor.span_actual();
            cursor.avanzar();
            Ok(Expresion::Literal(Literal::Entero(val, span)))
        }
        Some(Token::FlotanteLiteral(Some(f))) => {
            let val = f.clone();
            let span = cursor.span_actual();
            cursor.avanzar();
            Ok(Expresion::Literal(Literal::Flotante(val.parse().unwrap_or(0.0), span)))
        }
        Some(Token::PalabraLiteral(Some(s))) => {
            let val = s.clone();
            let span = cursor.span_actual();
            cursor.avanzar();
            Ok(Expresion::Literal(Literal::Palabra(val, span)))
        }
        Some(Token::CaracterLiteral(Some(c))) => {
            let val = *c;
            let span = cursor.span_actual();
            cursor.avanzar();
            Ok(Expresion::Literal(Literal::Caracter(val, span)))
        }
        Some(Token::Verdadero) => {
            let span = cursor.span_actual();
            cursor.avanzar();
            Ok(Expresion::Literal(Literal::Booleano(true, span)))
        }
        Some(Token::Falso) => {
            let span = cursor.span_actual();
            cursor.avanzar();
            Ok(Expresion::Literal(Literal::Booleano(false, span)))
        }
        Some(Token::CorcheteAbre) => {
            // Literal array: [1, 2, 3]
            let span_inicio = cursor.span_actual();
            cursor.avanzar(); // [
            let mut elementos = Vec::new();
            
            while cursor.actual() != Some(&Token::CorcheteCierra) && !cursor.esta_vacio() {
                elementos.push(parse_expresion(cursor)?);
                if let Some(Token::Coma) = cursor.actual() {
                    cursor.avanzar();
                } else {
                    break;
                }
            }
            
            cursor.esperar(Token::CorcheteCierra)?;
            let span_fin = cursor.span_actual();
            let span = Span::combinar(&span_inicio, &span_fin);
            Ok(Expresion::LiteralArray(elementos, span))
        }
        Some(Token::LlaveAbre) => {
            // Bloque como expresión: { sentencia; expresión }
            let bloque = super::sentencias::parse_bloque(cursor)?;
            Ok(Expresion::Bloque(bloque))
        }
        Some(Token::Todos) => {
            // todos expr — inicialización de array con valor repetido
            let span_inicio = cursor.span_actual();
            cursor.avanzar(); // todos
            let valor = parse_expresion(cursor)?;
            let span = Span::combinar(&span_inicio, valor.span());
            Ok(Expresion::ArrayRelleno(Box::new(valor), 0, span))
        }
        Some(Token::Mover) => {
            // mover x [a destino] — transferencia de ownership
            let span_inicio = cursor.span_actual();
            cursor.avanzar(); // mover
            let nombre = match cursor.actual() {
                Some(Token::Identificador(n)) => {
                    let n = n.clone();
                    cursor.avanzar();
                    n
                }
                _ => {
                    let span = cursor.span_actual();
                    return Err(ErrorSintaxis::identificador_esperado(span, "variable después de 'mover'"));
                }
            };
            // Verificar si hay "a destino"
            let destino = if let Some(Token::Identificador(a)) = cursor.actual() {
                if a == "a" {
                    cursor.avanzar(); // a
                    Some(Box::new(parse_expresion(cursor)?))
                } else {
                    None
                }
            } else {
                None
            };
            let span = Span::combinar(&span_inicio, &cursor.span_actual());
            Ok(Expresion::Mover(nombre, destino, span))
        }
        Some(Token::Copiar) => {
            // copiar expr — clone explícito
            let span_inicio = cursor.span_actual();
            cursor.avanzar(); // copiar
            let expr = parse_expresion(cursor)?;
            let span = Span::combinar(&span_inicio, expr.span());
            Ok(Expresion::Copiar(Box::new(expr), span))
        }
        Some(Token::Resultado) => {
            // Constructor de enum Resultado: Resultado.Exito(...) o Resultado.Error(...)
            let span_inicio = cursor.span_actual();
            cursor.avanzar(); // Resultado
            
            if cursor.actual() == Some(&Token::Punto) {
                if let Some(Token::Identificador(v)) = cursor.peek(1) {
                    let variante = v.clone();
                    cursor.avanzar(); // .
                    cursor.avanzar(); // Identificador variante
                    
                    let mut argumentos = Vec::new();
                    if cursor.actual() == Some(&Token::ParenAbre) {
                        cursor.avanzar(); // (
                        while cursor.actual() != Some(&Token::ParenCierra) && !cursor.esta_vacio() {
                            argumentos.push(parse_expresion(cursor)?);
                            if let Some(Token::Coma) = cursor.actual() {
                                cursor.avanzar();
                            } else {
                                break;
                            }
                        }
                        cursor.esperar(Token::ParenCierra)?;
                    }
                    
                    let span_fin = cursor.span_actual();
                    let span = Span::combinar(&span_inicio, &span_fin);
                    return Ok(Expresion::ConstructorEnum("Resultado".to_string(), variante, argumentos, span));
                }
            }
            
            let span = cursor.span_actual();
            Err(ErrorSintaxis::nuevo(15, span, "esperaba '.Exito' o '.Error' después de 'Resultado'"))
        }
        Some(Token::Identificador(n)) => {
            let nombre = n.clone();
            let span_inicio = cursor.span_actual();
            cursor.avanzar();

            // Verificar si es constructor enum: Enum.Variante o Enum.Variante(args)
            // Heurística: si el nombre empieza con mayúscula, es constructor de enum
            // Si empieza con minúscula, es acceso a campo (se maneja en parse_postfix)
            let es_posible_enum = nombre.chars().next().map_or(false, |c| c.is_uppercase());
            
            if es_posible_enum && cursor.actual() == Some(&Token::Punto) {
                if let Some(Token::Identificador(v)) = cursor.peek(1) {
                    let variante = v.clone();
                    cursor.avanzar(); // .
                    cursor.avanzar(); // Identificador variante
                    
                    let mut argumentos = Vec::new();
                    if cursor.actual() == Some(&Token::ParenAbre) {
                        cursor.avanzar(); // (
                        while cursor.actual() != Some(&Token::ParenCierra) && !cursor.esta_vacio() {
                            argumentos.push(parse_expresion(cursor)?);
                            if let Some(Token::Coma) = cursor.actual() {
                                cursor.avanzar();
                            } else {
                                break;
                            }
                        }
                        cursor.esperar(Token::ParenCierra)?;
                    }
                    
                    let span_fin = cursor.span_actual();
                    let span = Span::combinar(&span_inicio, &span_fin);
                    return Ok(Expresion::ConstructorEnum(nombre, variante, argumentos, span));
                }
            }

            // Verificar si es ruta cualificada: modulo::funcion
            // Dos puntos consecutivos = ::
            if cursor.actual() == Some(&Token::DosPuntos) {
                cursor.avanzar(); // consumir primer :
                if let Some(Token::DosPuntos) = cursor.actual() {
                    cursor.avanzar(); // consumir segundo :
                    
                    // Caso: nombre::<T>(...) — llamada genérica directa (tamaño_de::<Entero32>())
                    if es_llamada_generica(cursor) {
                        let tipo_args = parse_tipo_args(cursor)?;
                        cursor.esperar(Token::ParenAbre)?;
                        let mut argumentos = Vec::new();
                        while cursor.actual() != Some(&Token::ParenCierra) && !cursor.esta_vacio() {
                            argumentos.push(parse_expresion(cursor)?);
                            if let Some(Token::Coma) = cursor.actual() {
                                cursor.avanzar();
                            }
                        }
                        cursor.esperar(Token::ParenCierra)?;
                        let span_fin = cursor.span_actual();
                        let span = Span::combinar(&span_inicio, &span_fin);
                        return Ok(Expresion::Llamada(Llamada {
                            funcion: nombre,
                            tipo_args,
                            argumentos,
                            span,
                        }));
                    }

                    let segundo = match cursor.actual() {
                        Some(Token::Identificador(s)) => {
                            let s = s.clone();
                            cursor.avanzar();
                            s
                        }
                        _ => {
                            let span = cursor.span_actual();
                            return Err(ErrorSintaxis::identificador_esperado(span, "nombre después de '::'"));
                        }
                    };

                    // Construir ruta: modulo::funcion
                    let ruta = format!("{}::{}", nombre, segundo);
                    let span_ruta = Span::combinar(&span_inicio, &cursor.span_actual());

                    // Si sigue <, es llamada genérica cualificada: modulo::funcion<T>(...)
                    if es_llamada_generica(cursor) {
                        let tipo_args = parse_tipo_args(cursor)?;
                        cursor.esperar(Token::ParenAbre)?;
                        let mut argumentos = Vec::new();
                        while cursor.actual() != Some(&Token::ParenCierra) && !cursor.esta_vacio() {
                            argumentos.push(parse_expresion(cursor)?);
                            if let Some(Token::Coma) = cursor.actual() {
                                cursor.avanzar();
                            }
                        }
                        cursor.esperar(Token::ParenCierra)?;
                        let span_fin = cursor.span_actual();
                        let span = Span::combinar(&span_inicio, &span_fin);
                        return Ok(Expresion::Llamada(Llamada {
                            funcion: ruta,
                            tipo_args,
                            argumentos,
                            span,
                        }));
                    }

                    // Si sigue (, es llamada a función cualificada
                    if let Some(Token::ParenAbre) = cursor.actual() {
                        cursor.avanzar();

                        let mut argumentos = Vec::new();
                        while cursor.actual() != Some(&Token::ParenCierra) && !cursor.esta_vacio() {
                            argumentos.push(parse_expresion(cursor)?);
                            if let Some(Token::Coma) = cursor.actual() {
                                cursor.avanzar();
                            }
                        }

                        cursor.esperar(Token::ParenCierra)?;
                        let span_fin = cursor.span_actual();
                        let span = Span::combinar(&span_inicio, &span_fin);

                        return Ok(Expresion::Llamada(Llamada {
                            funcion: ruta,
                            tipo_args: Vec::new(),
                            argumentos,
                            span,
                        }));
                    }

                    // Si no, es una referencia de ruta
                    return Ok(Expresion::Ruta(vec![nombre, segundo], span_ruta));
                }
            }

            // Verificar si es llamada a función genérica: funcion<T>(args)
            if es_llamada_generica(cursor) {
                let tipo_args = parse_tipo_args(cursor)?;
                cursor.esperar(Token::ParenAbre)?;

                let mut argumentos = Vec::new();
                while cursor.actual() != Some(&Token::ParenCierra) && !cursor.esta_vacio() {
                    argumentos.push(parse_expresion(cursor)?);

                    if let Some(Token::Coma) = cursor.actual() {
                        cursor.avanzar();
                    }
                }

                cursor.esperar(Token::ParenCierra)?;
                let span_fin = cursor.span_actual();
                let span = Span::combinar(&span_inicio, &span_fin);

                return Ok(Expresion::Llamada(Llamada {
                    funcion: nombre,
                    tipo_args,
                    argumentos,
                    span,
                }));
            }

            // Verificar si es llamada a función (sin cualificar)
            if let Some(Token::ParenAbre) = cursor.actual() {
                cursor.avanzar();

                let mut argumentos = Vec::new();
                while cursor.actual() != Some(&Token::ParenCierra) && !cursor.esta_vacio() {
                    argumentos.push(parse_expresion(cursor)?);

                    if let Some(Token::Coma) = cursor.actual() {
                        cursor.avanzar();
                    }
                }

                cursor.esperar(Token::ParenCierra)?;
                let span_fin = cursor.span_actual();
                let span = Span::combinar(&span_inicio, &span_fin);

                Ok(Expresion::Llamada(Llamada {
                    funcion: nombre,
                    tipo_args: Vec::new(),
                    argumentos,
                    span,
                }))
            } else {
                Ok(Expresion::Identificador(nombre, span_inicio))
            }
        }
        Some(Token::ParenAbre) => {
            cursor.avanzar();
            let expr = parse_expresion(cursor)?;
            cursor.esperar(Token::ParenCierra)?;
            Ok(expr)
        }
        Some(Token::Coincidir) => {
            // coincidir sujeto { patron => expr, ... }
            let span_inicio = cursor.span_actual();
            cursor.avanzar(); // coincidir

            let sujeto = parse_expresion(cursor)?;

            cursor.esperar(Token::LlaveAbre)?;

            let mut brazos: Vec<crate::ast::BrazoMatch> = Vec::new();
            while cursor.actual() != Some(&Token::LlaveCierra) && !cursor.esta_vacio() {
                let span_brazo_inicio = cursor.span_actual();
                let patron = parse_patron(cursor)?;
                cursor.esperar(Token::FlechaGruesa)?; // =>
                let cuerpo = parse_expresion(cursor)?;
                let span_brazo = Span::combinar(&span_brazo_inicio, cuerpo.span());
                brazos.push(crate::ast::BrazoMatch { patron, cuerpo, span: span_brazo });

                // Coma opcional entre brazos
                if let Some(Token::Coma) = cursor.actual() {
                    cursor.avanzar();
                }
            }

            cursor.esperar(Token::LlaveCierra)?;
            let span = Span::combinar(&span_inicio, &cursor.span_actual());
            Ok(Expresion::Coincidir(Box::new(sujeto), brazos, span))
        }
        _ => {
            let span = cursor.span_actual();
            Err(ErrorSintaxis::expresion_esperada(span))
        }
    }
}

/// Parsea un patrón de match:
/// - Literal entero: 0, 1, 42
/// - Wildcard: _
/// - Variante de enum: Estado.Activo, Resultado.Exito(x)
fn parse_patron(cursor: &mut ParserCursor) -> Result<crate::ast::PatronMatch, ErrorSintaxis> {
    let span = cursor.span_actual();
    match cursor.actual() {
        // Wildcard: _
        Some(Token::Identificador(n)) if n == "_" => {
            cursor.avanzar();
            Ok(crate::ast::PatronMatch::Comodin(span))
        }
        // Literal entero
        Some(Token::EnteroLiteral(Some(val))) => {
            let val = *val;
            let span = cursor.span_actual();
            cursor.avanzar();
            Ok(crate::ast::PatronMatch::Literal(crate::ast::Literal::Entero(val, span)))
        }
        // Variante de enum: Nombre.Variante o Nombre.Variante(binding)
        Some(Token::Identificador(nombre)) => {
            let nombre = nombre.clone();
            let span_inicio = cursor.span_actual();
            cursor.avanzar();

            if cursor.actual() == Some(&Token::Punto) {
                cursor.avanzar(); // .
                let variante = match cursor.actual() {
                    Some(Token::Identificador(v)) => {
                        let v = v.clone();
                        cursor.avanzar();
                        v
                    }
                    _ => {
                        let span = cursor.span_actual();
                        return Err(ErrorSintaxis::identificador_esperado(span, "nombre de variante después de '.'"));
                    }
                };

                // Binding opcional: Variante(x)
                let binding = if cursor.actual() == Some(&Token::ParenAbre) {
                    cursor.avanzar(); // (
                    let b = match cursor.actual() {
                        Some(Token::Identificador(b)) => {
                            let b = b.clone();
                            cursor.avanzar();
                            b
                        }
                        _ => {
                            let span = cursor.span_actual();
                            return Err(ErrorSintaxis::identificador_esperado(span, "nombre de binding en patrón"));
                        }
                    };
                    cursor.esperar(Token::ParenCierra)?;
                    Some(b)
                } else {
                    None
                };

                let span_fin = cursor.span_actual();
                let span = Span::combinar(&span_inicio, &span_fin);
                Ok(crate::ast::PatronMatch::VarianteEnum(nombre, variante, binding, span))
            } else {
                // Identificador simple como binding (variable que captura cualquier valor)
                // Lo tratamos como comodin con nombre — por ahora, comodin
                Ok(crate::ast::PatronMatch::Comodin(span_inicio))
            }
        }
        _ => {
            Err(ErrorSintaxis::nuevo(16, span, "esperaba patrón de match (literal, Enum.Variante, o _)"))
        }
    }
}
