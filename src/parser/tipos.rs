use crate::ast::{Articulo, Tipo};
use crate::lexer::Token;
use super::{ErrorSintaxis, ParserCursor};

/// Parsea un artículo: el, la, un, los, las
pub fn parse_articulo(cursor: &mut ParserCursor) -> Result<Articulo, ErrorSintaxis> {
    match cursor.actual() {
        Some(Token::ArticuloEl) => { cursor.avanzar(); Ok(Articulo::El) }
        Some(Token::ArticuloLa) => { cursor.avanzar(); Ok(Articulo::La) }
        Some(Token::ArticuloUn) => { cursor.avanzar(); Ok(Articulo::Un) }
        Some(Token::ArticuloLos) => { cursor.avanzar(); Ok(Articulo::Los) }
        Some(Token::ArticuloLas) => { cursor.avanzar(); Ok(Articulo::Las) }
        _ => {
            let span = cursor.span_actual();
            Err(ErrorSintaxis::articulo_esperado(span))
        }
    }
}

/// Parsea un tipo primitivo o identificador de tipo
pub fn parse_tipo(cursor: &mut ParserCursor) -> Result<Tipo, ErrorSintaxis> {
    let span = cursor.span_actual();
    match cursor.actual() {
        // Referencias: &T, &mut T, &nombre T, &mut nombre T, &self T, &mut self T
        Some(Token::Ampersand) => {
            cursor.avanzar(); // &
            
            // Verificar si es &mut
            if cursor.actual() == Some(&Token::Mut) {
                cursor.avanzar(); // mut
                
                // Verificar si es &mut self T (self-referential)
                if cursor.actual() == Some(&Token::SelfKw) {
                    cursor.avanzar(); // self
                    let tipo = parse_tipo(cursor)?;
                    return Ok(Tipo::ReferenciaMutSelf(Box::new(tipo)));
                }
                
                // Verificar si es &mut nombre T (lifetime léxico)
                // Heurística: si el siguiente token es identificador seguido de otro tipo, es lifetime
                if let Some(Token::Identificador(nombre)) = cursor.actual() {
                    let nombre = nombre.clone();
                    // Verificar si hay otro tipo después (lifetime léxico)
                    if let Some(Token::Identificador(_)) | Some(Token::Entero8) | Some(Token::Entero16) | 
                       Some(Token::Entero32) | Some(Token::Entero64) | Some(Token::Texto) | 
                       Some(Token::Booleano) | Some(Token::Ampersand) = cursor.peek(1) {
                        cursor.avanzar(); // nombre (lifetime)
                        let tipo = parse_tipo(cursor)?;
                        return Ok(Tipo::ReferenciaMutConLifetime(nombre, Box::new(tipo)));
                    }
                }
                
                let tipo = parse_tipo(cursor)?;
                Ok(Tipo::ReferenciaMut(Box::new(tipo)))
            } else {
                // Verificar si es &self T (self-referential)
                if cursor.actual() == Some(&Token::SelfKw) {
                    cursor.avanzar(); // self
                    let tipo = parse_tipo(cursor)?;
                    return Ok(Tipo::ReferenciaSelf(Box::new(tipo)));
                }
                
                // Verificar si es &nombre T (lifetime léxico)
                if let Some(Token::Identificador(nombre)) = cursor.actual() {
                    let nombre = nombre.clone();
                    // Verificar si hay otro tipo después (lifetime léxico)
                    if let Some(Token::Identificador(_)) | Some(Token::Entero8) | Some(Token::Entero16) | 
                       Some(Token::Entero32) | Some(Token::Entero64) | Some(Token::Texto) | 
                       Some(Token::Booleano) | Some(Token::Ampersand) = cursor.peek(1) {
                        cursor.avanzar(); // nombre (lifetime)
                        let tipo = parse_tipo(cursor)?;
                        return Ok(Tipo::ReferenciaConLifetime(nombre, Box::new(tipo)));
                    }
                }
                
                let tipo = parse_tipo(cursor)?;
                Ok(Tipo::Referencia(Box::new(tipo)))
            }
        }
        Some(Token::Entero8) => { cursor.avanzar(); Ok(Tipo::Entero8) }
        Some(Token::Entero16) => { cursor.avanzar(); Ok(Tipo::Entero16) }
        Some(Token::Entero32) => { cursor.avanzar(); Ok(Tipo::Entero32) }
        Some(Token::Entero64) => { cursor.avanzar(); Ok(Tipo::Entero64) }
        Some(Token::Natural8) => { cursor.avanzar(); Ok(Tipo::Natural8) }
        Some(Token::Natural16) => { cursor.avanzar(); Ok(Tipo::Natural16) }
        Some(Token::Natural32) => { cursor.avanzar(); Ok(Tipo::Natural32) }
        Some(Token::Natural64) => { cursor.avanzar(); Ok(Tipo::Natural64) }
        Some(Token::Flotante32) => { cursor.avanzar(); Ok(Tipo::Flotante32) }
        Some(Token::Flotante64) => { cursor.avanzar(); Ok(Tipo::Flotante64) }
        Some(Token::Booleano) => { cursor.avanzar(); Ok(Tipo::Booleano) }
        Some(Token::Caracter) => { cursor.avanzar(); Ok(Tipo::Caracter) }
        Some(Token::Palabra) => { cursor.avanzar(); Ok(Tipo::Palabra) }
        Some(Token::Texto) => { cursor.avanzar(); Ok(Tipo::Texto) }
        Some(Token::Resultado) => {
            cursor.avanzar();
            // Resultado<T, E>
            cursor.esperar(Token::MenorQue)?;
            let tipo_exito = parse_tipo(cursor)?;
            cursor.esperar(Token::Coma)?;
            let tipo_error = parse_tipo(cursor)?;
            cursor.esperar(Token::MayorQue)?;
            Ok(Tipo::Resultado(Box::new(tipo_exito), Box::new(tipo_error)))
        }
        Some(Token::Vacio) => { cursor.avanzar(); Ok(Tipo::Vacio) }
        Some(Token::Identificador(n)) => {
            let nombre = n.clone();
            cursor.avanzar();
            
            // Si el identificador es un parámetro genérico de tipo activo, lo parseamos como tal
            if cursor.genericos.contains(&nombre) {
                // Verificar si es tipo genérico instanciado: Nombre<Tipo1, Tipo2>
                if cursor.actual() == Some(&Token::MenorQue) {
                    cursor.avanzar(); // <
                    let mut argumentos = Vec::new();
                    
                    while cursor.actual() != Some(&Token::MayorQue) && !cursor.esta_vacio() {
                        argumentos.push(parse_tipo(cursor)?);
                        if let Some(Token::Coma) = cursor.actual() {
                            cursor.avanzar();
                        } else {
                            break;
                        }
                    }
                    
                    cursor.esperar(Token::MayorQue)?;
                    return Ok(Tipo::NombreGenerico(nombre, argumentos));
                }
                return Ok(Tipo::Generico(nombre));
            }
            
            // Verificar si es tipo genérico instanciado: Nombre<Tipo1, Tipo2>
            if cursor.actual() == Some(&Token::MenorQue) {
                cursor.avanzar(); // <
                let mut argumentos = Vec::new();
                
                while cursor.actual() != Some(&Token::MayorQue) && !cursor.esta_vacio() {
                    argumentos.push(parse_tipo(cursor)?);
                    if let Some(Token::Coma) = cursor.actual() {
                        cursor.avanzar();
                    } else {
                        break;
                    }
                }
                
                cursor.esperar(Token::MayorQue)?;
                
                if nombre == "Vector" && argumentos.len() == 1 {
                    return Ok(Tipo::Vector(Box::new(argumentos.into_iter().next().unwrap())));
                }
                
                Ok(Tipo::NombreGenerico(nombre, argumentos))
            } else {
                Ok(Tipo::Nombre(nombre))
            }
        }
        Some(Token::CorcheteAbre) => {
            cursor.avanzar(); // [
            let tipo = parse_tipo(cursor)?;
            cursor.esperar(Token::PuntoYComa)?;
            match cursor.actual() {
                Some(Token::EnteroLiteral(Some(n))) => {
                    let n = *n as usize;
                    cursor.avanzar();
                    cursor.esperar(Token::CorcheteCierra)?;
                    Ok(Tipo::Array(Box::new(tipo), n))
                }
                Some(Token::Identificador(n)) => {
                    // Parámetro genérico const en tamaño de array (ej: [Entero32; N])
                    let nombre = n.clone();
                    cursor.avanzar();
                    cursor.esperar(Token::CorcheteCierra)?;
                    Ok(Tipo::ArrayGenerico(Box::new(tipo), nombre))
                }
                _ => Err(ErrorSintaxis::nuevo(14, cursor.span_actual(), "esperaba longitud del array")),
            }
        }
        _ => Err(ErrorSintaxis::tipo_esperado(span)),
    }
}
