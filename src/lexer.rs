use logos::Logos;
use crate::span::{Posicion, Span};
use std::sync::Arc;

/// Tokens del lenguaje Falcato
#[derive(Logos, Debug, Clone, PartialEq, Eq, Hash)]
#[logos(skip r"[ \t\r\n]+")]
#[logos(skip r"//[^\n]*")]
pub enum Token {
    // Keywords
    #[token("función")]
    #[token("funcion")]
    #[token("fn")]
    Funcion,
    
    #[token("retornar")]
    #[token("devolver")]
    Retornar,
    
    #[token("si")]
    Si,
    
    #[token("entonces")]
    Entonces,
    
    #[token("sino")]
    Sino,

    #[token("es")]
    Es,

    #[token("está")]
    Esta,

    #[token("fuese")]
    Fuese,
    
    #[token("mientras")]
    Mientras,
    
    #[token("para")]
    Para,
    
    #[token("en")]
    En,
    
    #[token("estructural")]
    Estructural,
    
    #[token("enumeración")]
    Enumeracion,
    
    #[token("usar")]
    Usar,
    
    #[token("módulo")]
    Modulo,
    
    #[token("todos")]
    Todos,

    #[token("inseguro")]
    Inseguro,
    
    #[token("mover")]
    Mover,
    
    #[token("copiar")]
    Copiar,
    
    #[token("prestar")]
    Prestar,
    
    #[token("verificado")]
    Verificado,
    
    #[token("estricto")]
    Estricto,
    
    #[token("mut")]
    Mut,
    
    #[token("como")]
    Como,
    
    #[token("tipo")]
    Tipo,

    #[token("región")]
    #[token("region")]
    Region,

    #[token("yo")]
    SelfKw,

    #[token("puro")]
    Puro,

    #[token("muta")]
    Muta,

    #[token("lee")]
    Lee,

    #[token("rasgo")]
    Rasgo,

    #[token("implementar")]
    Implementar,

    #[token("coincidir")]
    #[token("emparejar")]
    Coincidir,

    #[token("prueba")]
    Prueba,

    // Async (Fase 18)
    #[token("fut")]
    Fut,

    #[token("esperar")]
    Esperar,

    #[token("lanzar")]
    Lanzar,

    #[token("bloquear")]
    Bloquear,

    #[token("seleccionar")]
    Seleccionar,

    #[token("direccion_de")]
    DireccionDe,
    #[token("dir_de")]
    DirDe,

    #[token("con_executor")]
    ConExecutor,

    // Artículos (ownership)
    #[token("el")]
    ArticuloEl,
    
    #[token("la")]
    ArticuloLa,
    
    #[token("un")]
    ArticuloUn,
    
    #[token("los")]
    ArticuloLos,
    
    #[token("las")]
    ArticuloLas,

    // Tipos primitivos
    #[token("Entero8")]
    Entero8,
    
    #[token("Entero16")]
    Entero16,
    
    #[token("Entero32")]
    Entero32,
    
    #[token("Entero64")]
    Entero64,
    
    #[token("Natural8")]
    Natural8,
    
    #[token("Natural16")]
    Natural16,
    
    #[token("Natural32")]
    Natural32,
    
    #[token("Natural64")]
    Natural64,
    
    #[token("Flotante32")]
    Flotante32,
    
    #[token("Flotante64")]
    Flotante64,
    
    #[token("Booleano")]
    Booleano,
    
    #[token("Caracter")]
    Caracter,
    
    #[token("Palabra")]
    Palabra,
    
    #[token("Texto")]
    Texto,
    
    #[token("Resultado")]
    Resultado,
    
    #[token("Vacío")]
    Vacio,

    // Literales
    #[regex(r"[0-9]+", |lex| lex.slice().parse::<i64>().ok())]
    EnteroLiteral(Option<i64>),
    
    #[regex(r"[0-9]+\.[0-9]+", |lex| Some(lex.slice().to_string()))]
    FlotanteLiteral(Option<String>),
    
    #[regex(r#""([^"\\]|\\.)*""#, |lex| {
        let slice = lex.slice();
        let inner = &slice[1..slice.len()-1];
        Some(desescapar_cadena(inner))
    })]
    PalabraLiteral(Option<String>),
    
    #[regex(r"'[^']'", |lex| {
        let slice = lex.slice();
        slice.chars().nth(1)
    })]
    CaracterLiteral(Option<char>),

    // Booleanos
    #[token("verdadero")]
    Verdadero,
    
    #[token("falso")]
    Falso,

    // Identificadores
    #[regex(r"[a-zA-ZáéíóúÁÉÍÓÚñÑ_][a-zA-ZáéíóúÁÉÍÓÚñÑ0-9_]*", |lex| lex.slice().to_string())]
    Identificador(String),

    // Símbolos
    #[token("(")]
    ParenAbre,
    
    #[token(")")]
    ParenCierra,
    
    #[token("{")]
    LlaveAbre,
    
    #[token("}")]
    LlaveCierra,
    
    #[token("[")]
    CorcheteAbre,
    
    #[token("]")]
    CorcheteCierra,
    
    #[token(",")]
    Coma,
    
    #[token(";")]
    PuntoYComa,
    
    #[token(":")]
    DosPuntos,
    
    #[token("=")]
    Igual,
    
    #[token("==")]
    IgualIgual,
    
    #[token("!=")]
    Distinto,
    
    #[token("+")]
    Mas,
    
    #[token("-")]
    Menos,
    
    #[token("*")]
    Asterisco,
    
    #[token("/")]
    Barra,
    
    #[token("%")]
    Porcentaje,
    
    #[token("&&")]
    YLogico,
    
    #[token("||")]
    OLogico,
    
    #[token("|")]
    Pipe,

    #[token("^")]
    Caret,

    #[token("~")]
    Tilde,

    #[token(">>>")]
    TripleMayor,

    #[token("<<")]
    DobleMenor,

    #[token(">>")]
    DobleMayor,

    #[token("&")]
    Ampersand,
    
    #[token("!")]
    Exclamacion,
    
    #[token("<=")]
    MenorIgual,
    
    #[token(">=")]
    MayorIgual,
    
    #[token("<")]
    MenorQue,
    
    #[token(">")]
    MayorQue,
    
    #[token("..=")]
    PuntoPuntoIgual,

    #[token("..")]
    DoblePunto,

    #[token(".")]
    Punto,
    
    #[token("->")]
    Flecha,

    #[token("=>")]
    FlechaGruesa,

    #[token("?")]
    Interrogacion,

    // Error (logos 0.13+ maneja errores automáticamente)
    Error,
}

/// Convierte secuencias de escape en su carácter real.
/// Soporta: \\n, \\t, \\r, \\\\, \\\", \\0, \\xNN
fn desescapar_cadena(entrada: &str) -> String {
    let mut resultado = String::with_capacity(entrada.len());
    let mut chars = entrada.chars().peekable();
    
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => resultado.push('\n'),
                Some('t') => resultado.push('\t'),
                Some('r') => resultado.push('\r'),
                Some('0') => resultado.push('\0'),
                Some('\\') => resultado.push('\\'),
                Some('"') => resultado.push('"'),
                Some('x') => {
                    // \\xNN: dos dígitos hexadecimales
                    let mut hex = String::with_capacity(2);
                    if let Some(h1) = chars.next() {
                        hex.push(h1);
                    }
                    if let Some(h2) = chars.next() {
                        hex.push(h2);
                    }
                    if hex.len() == 2 {
                        if let Ok(val) = u8::from_str_radix(&hex, 16) {
                            resultado.push(val as char);
                        } else {
                            // Hex inválido: conservar literal
                            resultado.push('\\');
                            resultado.push('x');
                            resultado.push_str(&hex);
                        }
                    } else {
                        resultado.push('\\');
                        resultado.push('x');
                        resultado.push_str(&hex);
                    }
                }
                Some(other) => {
                    // Escape desconocido: conservar el carácter tal cual
                    resultado.push(other);
                }
                None => {
                    // Backslash al final: conservar
                    resultado.push('\\');
                }
            }
        } else {
            resultado.push(c);
        }
    }
    
    resultado
}

/// Token con su span asociado
#[derive(Debug, Clone)]
pub struct TokenConSpan {
    pub token: Token,
    pub span: Span,
}

/// Lexer de Falcato
pub struct LexerFalcato {
    fuente: String,
    archivo: String,
}

impl LexerFalcato {
    pub fn nuevo(fuente: impl Into<String>, archivo: impl Into<String>) -> Self {
        Self {
            fuente: fuente.into(),
            archivo: archivo.into(),
        }
    }

    pub fn tokenizar(&self) -> Vec<TokenConSpan> {
        let lexer = Token::lexer(&self.fuente);
        let archivo: std::sync::Arc<str> = std::sync::Arc::from(self.archivo.clone());
        
        lexer
            .spanned()
            .map(|(token_result, range)| {
                let token = match token_result {
                    Ok(t) => t,
                    Err(_) => {
                        // Token inválido: reportar como Error con span real
                        Token::Error
                    }
                };
                let inicio = self.offset_a_posicion(range.start);
                let fin = self.offset_a_posicion(range.end);
                let span = Span::nuevo(inicio, fin, Arc::clone(&archivo));
                TokenConSpan { token, span }
            })
            .collect()
    }

    fn offset_a_posicion(&self, offset: usize) -> Posicion {
        let mut linea = 1u32;
        let mut columna = 1u32;
        
        for (i, c) in self.fuente.char_indices() {
            if i >= offset {
                break;
            }
            if c == '\n' {
                linea += 1;
                columna = 1;
            } else {
                columna += 1;
            }
        }
        
        Posicion::nueva(linea, columna, offset as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer_hola_mundo() {
        let fuente = r#"función principal() {
    // Hola mundo
    retornar 0;
}"#;
        
        let lexer = LexerFalcato::nuevo(fuente, "test.fc");
        let tokens = lexer.tokenizar();
        
        assert!(tokens.len() > 0);
        assert!(matches!(tokens[0].token, Token::Funcion));
    }

    #[test]
    fn test_lexer_aritmetica() {
        let fuente = r#"el a: Entero32 = 10;
el c: Entero32 = a + b * 2;"#;
        let lexer = LexerFalcato::nuevo(fuente, "test.fc");
        let tokens = lexer.tokenizar();
        
        for (i, t) in tokens.iter().enumerate() {
            println!("{}: {:?}", i, t.token);
        }
        
        // Verificar que tenemos los tokens correctos
        assert!(matches!(tokens[0].token, Token::ArticuloEl));
        assert!(matches!(tokens[5].token, Token::EnteroLiteral(Some(10))));
        assert!(matches!(tokens[12].token, Token::Identificador(_)));
        assert!(matches!(tokens[13].token, Token::Mas));
        assert!(matches!(tokens[15].token, Token::Asterisco));
    }

    #[test]
    fn test_lexer_string() {
        let fuente = r#""Hola, mundo""#;
        let lexer = LexerFalcato::nuevo(fuente, "test.fc");
        let tokens = lexer.tokenizar();
        
        assert_eq!(tokens.len(), 1);
        assert!(matches!(
            &tokens[0].token,
            Token::PalabraLiteral(Some(s)) if s == "Hola, mundo"
        ));
    }

    #[test]
    fn test_lexer_string_escapes() {
        // Construir literal fuente con escapes de forma explícita
        let fuente = "\"".to_string()
            + "a\\nb\\tc\\\"d\\\\e\\x41"
            + "\"";
        let lexer = LexerFalcato::nuevo(fuente, "test.fc");
        let tokens = lexer.tokenizar();
        
        let esperado = "a\nb\tc\"d\\eA";
        assert_eq!(tokens.len(), 1);
        assert!(matches!(
            &tokens[0].token,
            Token::PalabraLiteral(Some(s)) if s == esperado
        ));
    }

    #[test]
    fn test_lexer_funcion_alias() {
        for kw in ["función", "funcion", "fn"] {
            let fuente = format!("{} principal() {{
    retornar 0;
}}", kw);
            let lexer = LexerFalcato::nuevo(&fuente, "test.fc");
            let tokens = lexer.tokenizar();
            assert!(matches!(tokens[0].token, Token::Funcion), "'{}' debería lexear como Funcion", kw);
        }
    }

    #[test]
    fn test_lexer_articulos() {
        let fuente = "el la un los las";
        let lexer = LexerFalcato::nuevo(fuente, "test.fc");
        let tokens = lexer.tokenizar();
        
        assert_eq!(tokens.len(), 5);
        assert!(matches!(tokens[0].token, Token::ArticuloEl));
        assert!(matches!(tokens[1].token, Token::ArticuloLa));
        assert!(matches!(tokens[2].token, Token::ArticuloUn));
        assert!(matches!(tokens[3].token, Token::ArticuloLos));
        assert!(matches!(tokens[4].token, Token::ArticuloLas));
    }
}
