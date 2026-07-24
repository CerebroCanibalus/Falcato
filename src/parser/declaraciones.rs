use crate::ast::*;
use crate::lexer::Token;
use crate::span::Span;
use super::{ErrorSintaxis, ParserCursor};
use super::tipos::{parse_articulo, parse_tipo};
use super::sentencias::parse_bloque;

/// Parsea una declaración top-level
pub fn parse_declaracion(cursor: &mut ParserCursor) -> Result<Declaracion, ErrorSintaxis> {
    match cursor.actual() {
        Some(Token::Inseguro) => parse_funcion(cursor),
        Some(Token::Funcion) => parse_funcion(cursor),
        Some(Token::Fut) => parse_funcion(cursor),
        Some(Token::Estructural) => parse_estructural(cursor),
        Some(Token::Enumeracion) => parse_enumeracion(cursor),
        Some(Token::Modulo) => parse_modulo(cursor),
        Some(Token::Usar) => parse_usar(cursor),
        Some(Token::Rasgo) => parse_rasgo(cursor),
        Some(Token::Implementar) => parse_impl(cursor),
        Some(Token::Prueba) => parse_prueba(cursor),
        // Visibilidad: `el función` o `la función`
        Some(Token::ArticuloEl) | Some(Token::ArticuloLa) => {
            let span_articulo = cursor.span_actual();
            let articulo = match cursor.actual() {
                Some(Token::ArticuloEl) => Some(Articulo::El),
                Some(Token::ArticuloLa) => Some(Articulo::La),
                _ => unreachable!(),
            };
            cursor.avanzar(); // consumir artículo
            
            match cursor.actual() {
                Some(Token::Funcion) => parse_funcion_con_visibilidad(cursor, articulo),
                _ => {
                    Err(ErrorSintaxis::nuevo(12, span_articulo, 
                        "el artículo 'el'/'la' solo puede preceder a 'función' en contexto de módulo"))
                }
            }
        },
        Some(_) => {
            let span = cursor.span_actual();
            let token = cursor.actual().map(|t| format!("{:?}", t)).unwrap_or_default();
            Err(ErrorSintaxis::nuevo(8, span, format!("token inesperado en declaración top-level: '{}'", token)))
        }
        None => {
            let span = cursor.span_actual();
            Err(ErrorSintaxis::fin_archivo_inesperado(span))
        }
    }
}

/// Parsea una función con artículo de visibilidad explícito.
/// Ej: `el función suma(...) { ... }` (pública)
///     `la función suma(...) { ... }` (privada)
/// El artículo ya fue consumido. El cursor apunta al token `función`/`funcion`/`fn`.
fn parse_funcion_con_visibilidad(cursor: &mut ParserCursor, articulo: Option<Articulo>) -> Result<Declaracion, ErrorSintaxis> {
    let mut func = parse_funcion(cursor)?;
    if let Declaracion::Funcion(ref mut f) = func {
        f.visibilidad = articulo;
    }
    Ok(func)
}

/// Parsea una declaración de función con span real
fn parse_funcion(cursor: &mut ParserCursor) -> Result<Declaracion, ErrorSintaxis> {
    let span_inicio = cursor.span_actual();
    
    // Detectar prefijo `fut` (async)
    let es_futuro = if let Some(Token::Fut) = cursor.actual() {
        cursor.avanzar();
        true
    } else {
        false
    };

    let es_insegura = if let Some(Token::Inseguro) = cursor.actual() {
        cursor.avanzar();
        true
    } else {
        false
    };

    cursor.esperar(Token::Funcion)?;

    let nombre = match cursor.actual() {
        Some(Token::Identificador(n)) => {
            let n = n.clone();
            cursor.avanzar();
            n
        }
        _ => {
            let span = cursor.span_actual();
            return Err(ErrorSintaxis::identificador_esperado(span, "nombre de función"));
        }
    };

    // Parsear parámetros genéricos opcionales: <T, N: Entero32>
    let parametros_genericos = if cursor.actual() == Some(&Token::MenorQue) {
        parse_parametros_genericos(cursor)?
    } else {
        Vec::new()
    };

    // Activar nombres de type params para que parse_tipo los reconozca
    let nombres_genericos_anteriores: Vec<String> = cursor.genericos.clone();
    cursor.genericos = parametros_genericos.iter()
        .filter(|g| g.tipo.is_none())
        .map(|g| g.nombre.clone())
        .collect();

    cursor.esperar(Token::ParenAbre)?;

    let parametros = if let Some(Token::ParenCierra) = cursor.actual() {
        Vec::new()
    } else {
        parse_parametros(cursor)?
    };

    cursor.esperar(Token::ParenCierra)?;

    let retorno = if let Some(Token::Flecha) = cursor.actual() {
        cursor.avanzar();
        Some(parse_tipo(cursor)?)
    } else {
        None
    };

    // Parsear anotación de efecto (puro, muta(campo), lee(campo))
    let efecto = match cursor.actual() {
        Some(Token::Puro) => {
            cursor.avanzar();
            crate::ast::Efecto::Puro
        }
        Some(Token::Muta) => {
            cursor.avanzar();
            cursor.esperar(Token::ParenAbre)?;
            let mut campos = Vec::new();
            while cursor.actual() != Some(&Token::ParenCierra) && !cursor.esta_vacio() {
                if let Some(Token::Identificador(campo)) = cursor.actual() {
                    campos.push(campo.clone());
                    cursor.avanzar();
                }
                if cursor.actual() == Some(&Token::Coma) {
                    cursor.avanzar();
                }
            }
            cursor.esperar(Token::ParenCierra)?;
            crate::ast::Efecto::Muta(campos)
        }
        Some(Token::Lee) => {
            cursor.avanzar();
            cursor.esperar(Token::ParenAbre)?;
            let mut campos = Vec::new();
            while cursor.actual() != Some(&Token::ParenCierra) && !cursor.esta_vacio() {
                if let Some(Token::Identificador(campo)) = cursor.actual() {
                    campos.push(campo.clone());
                    cursor.avanzar();
                }
                if cursor.actual() == Some(&Token::Coma) {
                    cursor.avanzar();
                }
            }
            cursor.esperar(Token::ParenCierra)?;
            crate::ast::Efecto::Lee(campos)
        }
        _ => crate::ast::Efecto::Conservador,
    };

    // Parsear nivel de verificación de ownership (borrow checker gradual)
    let nivel_verificacion = match cursor.actual() {
        Some(Token::Verificado) => {
            cursor.avanzar();
            crate::ast::NivelVerificacion::Verificado
        }
        Some(Token::Estricto) => {
            cursor.avanzar();
            crate::ast::NivelVerificacion::Estricto
        }
        _ => crate::ast::NivelVerificacion::Permisivo,
    };

    // Si es FFI (sin cuerpo), terminamos con punto y coma
    if es_insegura && cursor.actual() == Some(&Token::PuntoYComa) {
        cursor.avanzar();
        cursor.genericos = nombres_genericos_anteriores;
        let span = Span::combinar(
            &span_inicio, &cursor.span_actual()
        );
        return Ok(Declaracion::Funcion(FuncionDecl {
            nombre,
            parametros_genericos,
            parametros,
            retorno,
            cuerpo: Bloque { sentencias: Vec::new(), span: Span::vacio() },
            es_insegura,
            nivel_verificacion,
            efecto,
            visibilidad: None,
            es_futuro,
            span,
        }));
    }

    let cuerpo = parse_bloque(cursor)?;
    cursor.genericos = nombres_genericos_anteriores;
    let span = Span::combinar(
        &span_inicio, &cuerpo.span
    );

    Ok(Declaracion::Funcion(FuncionDecl {
        nombre,
        parametros_genericos,
        parametros,
        retorno,
        cuerpo,
        es_insegura,
        nivel_verificacion,
        efecto,
        visibilidad: None,
        es_futuro,
        span,
    }))
}

/// Parsea parámetros de función: articulo nombre: tipo [, ...]
fn parse_parametros(cursor: &mut ParserCursor) -> Result<Vec<Parametro>, ErrorSintaxis> {
    let mut params = Vec::new();

    loop {
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
                return Err(ErrorSintaxis::identificador_esperado(span, "parámetro"));
            }
        };

        cursor.esperar(Token::DosPuntos)?;
        let tipo = parse_tipo(cursor)?;
        
        let span = Span::combinar(
            &span_inicio, &cursor.span_actual()
        );

        params.push(Parametro {
            articulo,
            nombre,
            tipo,
            span,
        });

        if let Some(Token::Coma) = cursor.actual() {
            cursor.avanzar();
        } else {
            break;
        }
    }

    Ok(params)
}

/// Parsea parámetros genéricos: <T, N: Entero32, T que Comparable>
fn parse_parametros_genericos(cursor: &mut ParserCursor) -> Result<Vec<ParametroGenerico>, ErrorSintaxis> {
    cursor.esperar(Token::MenorQue)?;
    let mut params = Vec::new();

    while cursor.actual() != Some(&Token::MayorQue) && !cursor.esta_vacio() {
        let span_inicio = cursor.span_actual();

        let nombre = match cursor.actual() {
            Some(Token::Identificador(n)) => {
                let n = n.clone();
                cursor.avanzar();
                n
            }
            _ => {
                let span = cursor.span_actual();
                return Err(ErrorSintaxis::identificador_esperado(span, "parámetro genérico"));
            }
        };

        let tipo = if cursor.actual() == Some(&Token::DosPuntos) {
            cursor.avanzar();
            Some(parse_tipo(cursor)?)
        } else {
            None
        };

        let mut bounds = Vec::new();
        if let Some(Token::Identificador(ref s)) = cursor.actual() {
            if s == "que" {
                cursor.avanzar();
                loop {
                    match cursor.actual() {
                        Some(Token::Identificador(b)) => {
                            bounds.push(b.clone());
                            cursor.avanzar();
                        }
                        _ => break,
                    }
                    if cursor.actual() == Some(&Token::Coma) {
                        cursor.avanzar();
                        if cursor.actual() == Some(&Token::MayorQue) {
                            break;
                        }
                    } else {
                        break;
                    }
                }
            }
        }

        let span = Span::combinar(&span_inicio, &cursor.span_actual()
        );

        params.push(ParametroGenerico {
            nombre,
            tipo,
            bounds,
            span,
        });

        if cursor.actual() == Some(&Token::Coma) {
            cursor.avanzar();
        } else {
            break;
        }
    }

    cursor.esperar(Token::MayorQue)?;
    Ok(params)
}

/// Parsea una declaración de struct: estructural Nombre { campo: Tipo, ... }
fn parse_estructural(cursor: &mut ParserCursor) -> Result<Declaracion, ErrorSintaxis> {
    let span_inicio = cursor.span_actual();
    cursor.esperar(Token::Estructural)?;

    let nombre = match cursor.actual() {
        Some(Token::Identificador(n)) => {
            let n = n.clone();
            cursor.avanzar();
            n
        }
        _ => {
            let span = cursor.span_actual();
            return Err(ErrorSintaxis::identificador_esperado(span, "nombre de struct"));
        }
    };

    cursor.esperar(Token::LlaveAbre)?;

    let mut campos = Vec::new();
    let mut campos_bits = Vec::new();
    let mut offset_bits_actual: u32 = 0;

    while cursor.actual() != Some(&Token::LlaveCierra) && !cursor.esta_vacio() {
        let span_campo_inicio = cursor.span_actual();

        let nombre_campo = match cursor.actual() {
            Some(Token::Identificador(n)) => {
                let n = n.clone();
                cursor.avanzar();
                n
            }
            _ => {
                let span = cursor.span_actual();
                return Err(ErrorSintaxis::identificador_esperado(span, "nombre de campo"));
            }
        };

        // Fase 15B: `bits { campo: NaturalN, ... }` — bloque de campos de bits
        if nombre_campo == "bits" && cursor.actual() == Some(&Token::LlaveAbre) {
            cursor.avanzar(); // {
            while cursor.actual() != Some(&Token::LlaveCierra) && !cursor.esta_vacio() {
                let span_bit_inicio = cursor.span_actual();
                let nombre_bit = match cursor.actual() {
                    Some(Token::Identificador(n)) => {
                        let n = n.clone();
                        cursor.avanzar();
                        n
                    }
                    _ => {
                        let span = cursor.span_actual();
                        return Err(ErrorSintaxis::identificador_esperado(span, "nombre de campo de bits"));
                    }
                };
                cursor.esperar(Token::DosPuntos)?;
                // Parsear tipo NaturalN → extraer N como ancho de bits
                let tipo = parse_tipo(cursor)?;
                let ancho = match &tipo {
                    Tipo::Natural8 => 8,
                    Tipo::Natural16 => 16,
                    Tipo::Natural32 => 32,
                    Tipo::Natural64 => 64,
                    Tipo::Entero8 => 8,
                    Tipo::Entero16 => 16,
                    Tipo::Entero32 => 32,
                    Tipo::Entero64 => 64,
                    _ => {
                        let span = cursor.span_actual();
                        return Err(ErrorSintaxis::nuevo(
                            7,
                            span,
                            "Campos de bits requieren tipos Natural8/16/32/64 o Entero8/16/32/64",
                        ));
                    }
                };
                let span_bit = Span::combinar(&span_bit_inicio, &cursor.span_actual());
                campos_bits.push(CampoBits {
                    nombre: nombre_bit,
                    ancho_bits: ancho,
                    offset_bits: offset_bits_actual,
                    span: span_bit,
                });
                offset_bits_actual += ancho;

                if let Some(Token::Coma) = cursor.actual() {
                    cursor.avanzar();
                } else {
                    break;
                }
            }
            cursor.esperar(Token::LlaveCierra)?;
            // Después del bloque bits, puede haber coma o fin
            if let Some(Token::Coma) = cursor.actual() {
                cursor.avanzar();
            }
            continue;
        }

        cursor.esperar(Token::DosPuntos)?;
        let tipo = parse_tipo(cursor)?;

        let span_campo = Span::combinar(&span_campo_inicio, &cursor.span_actual());
        campos.push(Campo {
            nombre: nombre_campo,
            tipo,
            span: span_campo,
        });

        if let Some(Token::Coma) = cursor.actual() {
            cursor.avanzar();
        } else {
            break;
        }
    }

    cursor.esperar(Token::LlaveCierra)?;

    let span = Span::combinar(&span_inicio, &cursor.span_actual());
    Ok(Declaracion::Estructural(EstructuralDecl {
        nombre,
        campos,
        campos_bits,
        span,
    }))
}

/// Parsea una declaración de enumeración: enumeración Nombre { Variante, Variante(dato: Tipo), ... }
fn parse_enumeracion(cursor: &mut ParserCursor) -> Result<Declaracion, ErrorSintaxis> {
    let span_inicio = cursor.span_actual();
    cursor.esperar(Token::Enumeracion)?;

    let nombre = match cursor.actual() {
        Some(Token::Identificador(n)) => {
            let n = n.clone();
            cursor.avanzar();
            n
        }
        _ => {
            let span = cursor.span_actual();
            return Err(ErrorSintaxis::identificador_esperado(span, "nombre de enumeración"));
        }
    };

    // Parámetros genéricos opcionales: enumeración alguno<T> { ... }
    let parametros_genericos = if cursor.actual() == Some(&Token::MenorQue) {
        parse_parametros_genericos(cursor)?
    } else {
        Vec::new()
    };

    cursor.esperar(Token::LlaveAbre)?;

    let mut variantes = Vec::new();
    while cursor.actual() != Some(&Token::LlaveCierra) && !cursor.esta_vacio() {
        let span_variante_inicio = cursor.span_actual();

        let nombre_variante = match cursor.actual() {
            Some(Token::Identificador(n)) => {
                let n = n.clone();
                cursor.avanzar();
                n
            }
            _ => {
                let span = cursor.span_actual();
                return Err(ErrorSintaxis::identificador_esperado(span, "nombre de variante"));
            }
        };

        // Verificar si la variante tiene datos: Variante(dato: Tipo)
        let datos = if cursor.actual() == Some(&Token::ParenAbre) {
            cursor.avanzar(); // (
            let mut campos = Vec::new();
            
            while cursor.actual() != Some(&Token::ParenCierra) && !cursor.esta_vacio() {
                let nombre_campo = match cursor.actual() {
                    Some(Token::Identificador(n)) => {
                        let n = n.clone();
                        cursor.avanzar();
                        n
                    }
                    _ => {
                        let span = cursor.span_actual();
                        return Err(ErrorSintaxis::identificador_esperado(span, "nombre de campo en variante"));
                    }
                };

                cursor.esperar(Token::DosPuntos)?;
                let tipo = parse_tipo(cursor)?;
                campos.push((nombre_campo, tipo));

                if let Some(Token::Coma) = cursor.actual() {
                    cursor.avanzar();
                } else {
                    break;
                }
            }

            cursor.esperar(Token::ParenCierra)?;
            Some(campos)
        } else {
            None
        };

        let span_variante = Span::combinar(&span_variante_inicio, &cursor.span_actual());
        variantes.push(Variante {
            nombre: nombre_variante,
            datos,
            span: span_variante,
        });

        if let Some(Token::Coma) = cursor.actual() {
            cursor.avanzar();
        } else {
            break;
        }
    }

    cursor.esperar(Token::LlaveCierra)?;

    let span = Span::combinar(&span_inicio, &cursor.span_actual());
    Ok(Declaracion::Enumeracion(EnumeracionDecl {
        nombre,
        parametros_genericos,
        variantes,
        span,
    }))
}

/// Parsea un bloque módulo: `módulo nombre { ... }`
fn parse_modulo(cursor: &mut ParserCursor) -> Result<Declaracion, ErrorSintaxis> {
    let span_inicio = cursor.span_actual();
    cursor.avanzar(); // consumir `módulo`

    let nombre = match cursor.actual() {
        Some(Token::Identificador(n)) => {
            let n = n.clone();
            cursor.avanzar();
            n
        }
        _ => {
            let span = cursor.span_actual();
            return Err(ErrorSintaxis::identificador_esperado(span, "nombre de módulo"));
        }
    };

    cursor.esperar(Token::LlaveAbre)?;

    let mut contenido = Vec::new();
    while let Some(token) = cursor.actual() {
        match token {
            Token::LlaveCierra => {
                cursor.avanzar();
                break;
            }
            _ => {
                contenido.push(parse_declaracion(cursor)?);
            }
        }
    }

    let span = Span::combinar(&span_inicio, &cursor.span_actual());
    Ok(Declaracion::Modulo(ModuloDecl {
        nombre,
        contenido,
        span,
    }))
}

/// Parsea una declaración de uso: `usar ruta::simbolo;`
fn parse_usar(cursor: &mut ParserCursor) -> Result<Declaracion, ErrorSintaxis> {
    let span_inicio = cursor.span_actual();
    cursor.avanzar(); // consumir `usar`

    let mut ruta = Vec::new();

    // Leer identificadores separados por ::
    loop {
        match cursor.actual() {
            Some(Token::Identificador(nombre)) => {
                ruta.push(nombre.clone());
                cursor.avanzar();
            }
            Some(Token::Asterisco) => {
                // `usar modulo::*`
                ruta.push("*".to_string());
                cursor.avanzar();
                break;
            }
            _ => {
                let span = cursor.span_actual();
                return Err(ErrorSintaxis::identificador_esperado(span, "nombre de módulo en 'usar'"));
            }
        }

        // Siguiente token: `::` o `;`
        match cursor.actual() {
            Some(Token::DosPuntos) => {
                cursor.avanzar();
                // Esperar otro `:`
                if let Some(Token::DosPuntos) = cursor.actual() {
                    cursor.avanzar();
                } else {
                    let span = cursor.span_actual();
                    return Err(ErrorSintaxis::nuevo(13, span, 
                        "se esperaba '::' después del nombre del módulo"));
                }
            }
            _ => break,
        }
    }

    cursor.esperar(Token::PuntoYComa)?;

    let span = Span::combinar(&span_inicio, &cursor.span_actual());
    Ok(Declaracion::Usar(UsarDecl {
        ruta,
        span,
    }))
}

/// Parsea una declaración de rasgo: `rasgo Nombre { función ...; función ...; }`
fn parse_rasgo(cursor: &mut ParserCursor) -> Result<Declaracion, ErrorSintaxis> {
    let span_inicio = cursor.span_actual();
    cursor.avanzar(); // consumir `rasgo`
    
    let nombre = match cursor.actual() {
        Some(Token::Identificador(n)) => {
            let n = n.clone();
            cursor.avanzar();
            n
        }
        _ => {
            return Err(ErrorSintaxis::nuevo(70, cursor.span_actual(), 
                "Se esperaba nombre del rasgo".to_string()));
        }
    };
    
    cursor.esperar(Token::LlaveAbre)?;
    
    let mut metodos = Vec::new();
    while cursor.actual() != Some(&Token::LlaveCierra) && !cursor.esta_vacio() {
        match cursor.actual() {
            Some(Token::Funcion) => {
                let span_metodo = cursor.span_actual();
                cursor.avanzar();
                
                let nombre_met = match cursor.actual() {
                    Some(Token::Identificador(n)) => {
                        let n = n.clone();
                        cursor.avanzar();
                        n
                    }
                    _ => return Err(ErrorSintaxis::nuevo(71, cursor.span_actual(),
                        "Se esperaba nombre del método".to_string())),
                };
                
                cursor.esperar(Token::ParenAbre)?;
                let params = if let Some(Token::ParenCierra) = cursor.actual() {
                    Vec::new()
                } else {
                    parse_parametros(cursor)?
                };
                cursor.esperar(Token::ParenCierra)?;
                
                let ret = if let Some(Token::Flecha) = cursor.actual() {
                    cursor.avanzar();
                    Some(parse_tipo(cursor)?)
                } else {
                    None
                };
                
                cursor.esperar(Token::PuntoYComa)?;
                
                metodos.push(crate::ast::FirmaMetodo {
                    nombre: nombre_met,
                    parametros: params,
                    retorno: ret,
                    span: span_metodo,
                });
            }
            _ => {
                return Err(ErrorSintaxis::nuevo(72, cursor.span_actual(),
                    "Dentro de rasgo solo se permiten funciones".to_string()));
            }
        }
    }
    
    cursor.esperar(Token::LlaveCierra)?;
    let span = Span::combinar(&span_inicio, &cursor.span_actual());
    
    Ok(Declaracion::Rasgo(RasgoDecl {
        nombre,
        metodos,
        span,
    }))
}

/// Parsea una impl de rasgo: `implementar Rasgo para Tipo { función ... { ... } ... }`
fn parse_impl(cursor: &mut ParserCursor) -> Result<Declaracion, ErrorSintaxis> {
    let span_inicio = cursor.span_actual();
    cursor.avanzar(); // consumir `implementar`
    
    let rasgo = match cursor.actual() {
        Some(Token::Identificador(n)) => {
            let n = n.clone();
            cursor.avanzar();
            n
        }
        _ => return Err(ErrorSintaxis::nuevo(73, cursor.span_actual(),
            "Se esperaba nombre del rasgo".to_string())),
    };
    
    // `para` (keyword)
    cursor.esperar(Token::Para)?;
    
    let tipo = parse_tipo(cursor)?;
    
    cursor.esperar(Token::LlaveAbre)?;
    
    let mut metodos = Vec::new();
    while cursor.actual() != Some(&Token::LlaveCierra) && !cursor.esta_vacio() {
        match cursor.actual() {
            Some(Token::Funcion) => {
                let func = parse_funcion(cursor)?;
                if let Declaracion::Funcion(f) = func {
                    metodos.push(f);
                }
            }
            _ => break,
        }
    }
    
    cursor.esperar(Token::LlaveCierra)?;
    let span = Span::combinar(&span_inicio, &cursor.span_actual());
    
    Ok(Declaracion::Implementacion(ImplDecl {
        rasgo,
        tipo,
        metodos,
        span,
    }))
}

/// Parsea: prueba "nombre" { ... }
fn parse_prueba(cursor: &mut ParserCursor) -> Result<Declaracion, ErrorSintaxis> {
    let span_inicio = cursor.span_actual();
    cursor.esperar(Token::Prueba)?;

    // Nombre de la prueba (string literal)
    let nombre = match cursor.actual() {
        Some(Token::PalabraLiteral(Some(s))) => {
            let s = s.clone();
            cursor.avanzar();
            s
        }
        _ => {
            let span = cursor.span_actual();
            return Err(ErrorSintaxis::nuevo(17, span, "esperaba nombre de prueba entre comillas: prueba \"nombre\" { ... }"));
        }
    };

    // Bloque de la prueba
    let bloque = super::sentencias::parse_bloque(cursor)?;
    let span = Span::combinar(&span_inicio, &cursor.span_actual());

    Ok(Declaracion::Prueba(PruebaDecl {
        nombre,
        bloque,
        span,
    }))
}
