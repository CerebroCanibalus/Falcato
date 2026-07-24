use crate::ast::*;
use crate::span::Span;
use crate::lexer::{Token, TokenConSpan};

pub mod errores;
pub mod tipos;
pub mod expresiones;
pub mod sentencias;
pub mod declaraciones;

pub use errores::ErrorSintaxis;

/// Cursor de parsing: itera sobre tokens con spans y permite lookahead
pub struct ParserCursor {
    tokens: Vec<TokenConSpan>,
    posicion: usize,
    /// Nombres de parámetros genéricos de tipo activos en el scope actual
    pub genericos: Vec<String>,
}

impl ParserCursor {
    pub fn nuevo(tokens: Vec<TokenConSpan>) -> Self {
        Self { tokens, posicion: 0, genericos: Vec::new() }
    }

    pub fn esta_vacio(&self) -> bool {
        self.posicion >= self.tokens.len()
    }

    /// Token actual (sin span)
    pub fn actual(&self) -> Option<&Token> {
        self.tokens.get(self.posicion).map(|t| &t.token)
    }

    /// Token actual con span
    pub fn actual_con_span(&self) -> Option<&TokenConSpan> {
        self.tokens.get(self.posicion)
    }

    /// Avanza al siguiente token
    pub fn avanzar(&mut self) {
        self.posicion += 1;
    }

    /// Mira el siguiente token sin avanzar
    pub fn peek(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.posicion + offset).map(|t| &t.token)
    }

    /// Espera un token específico y avanza; si no coincide, error
    pub fn esperar(&mut self, token: Token) -> Result<(), ErrorSintaxis> {
        match self.actual() {
            Some(t) if *t == token => {
                self.avanzar();
                Ok(())
            }
            Some(t) => {
                let span = self.span_actual();
                Err(ErrorSintaxis::esperaba(span, &format!("{:?}", token), Some(&format!("{:?}", t))))
            }
            None => {
                let span = self.span_actual();
                Err(ErrorSintaxis::esperaba(span, &format!("{:?}", token), None))
            }
        }
    }

    /// Span del token actual; si no hay token, devuelve span vacío
    pub fn span_actual(&self) -> Span {
        self.actual_con_span()
            .map(|t| t.span.clone())
            .unwrap_or_else(Span::vacio)
    }

    /// Span desde una posición anterior hasta el token actual
    pub fn span_desde(&self, inicio_pos: usize) -> Span {
        let inicio = self.tokens.get(inicio_pos)
            .map(|t| t.span.clone())
            .unwrap_or_else(Span::vacio);
        let fin = self.tokens.get(self.posicion.saturating_sub(1))
            .map(|t| t.span.clone())
            .unwrap_or_else(Span::vacio);
        Span::combinar(&inicio, &fin)
    }

    /// Posición actual en el stream de tokens
    pub fn posicion(&self) -> usize {
        self.posicion
    }

    /// Salta tokens hasta encontrar uno de sincronización (usado en recovery)
    pub fn sincronizar(&mut self, tokens_sinc: &[Token]) {
        while !self.esta_vacio() {
            if let Some(t) = self.actual() {
                if tokens_sinc.contains(t) {
                    return;
                }
            }
            self.avanzar();
        }
    }
}

/// Parser público de Falcato
pub struct ParserFalcato;

impl ParserFalcato {
    pub fn parse(tokens: Vec<TokenConSpan>) -> Result<Programa, Vec<ErrorSintaxis>> {
        let mut errores = Vec::new();

        // Primera pasada: detectar errores léxicos (Token::Error)
        for tcs in &tokens {
            if let Token::Error = tcs.token {
                let span = tcs.span.clone();
                errores.push(ErrorSintaxis::token_invalido(span, "carácter inválido"));
            }
        }

        // Si hay errores léxicos, abortamos (no tiene sentido parsear tokens inválidos)
        if !errores.is_empty() {
            return Err(errores);
        }

        let mut cursor = ParserCursor::nuevo(tokens);
        let mut declaraciones = Vec::new();

        while !cursor.esta_vacio() {
            match declaraciones::parse_declaracion(&mut cursor) {
                Ok(decl) => declaraciones.push(decl),
                Err(e) => {
                    errores.push(e);
                    // Recovery: sincronizar hasta siguiente declaración
                    cursor.sincronizar(&[
                        Token::Funcion,
                        Token::Inseguro,
                        Token::Estructural,
                        Token::Enumeracion,
                        Token::Modulo,
                        Token::Usar,
                    ]);
                }
            }
        }

        if !errores.is_empty() {
            return Err(errores);
        }

        Ok(Programa {
            declaraciones,
            span: Span::vacio(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::LexerFalcato;

    fn parse_fuente(fuente: &str) -> Result<Programa, Vec<ErrorSintaxis>> {
        let lexer = LexerFalcato::nuevo(fuente, "test.fc");
        let tokens = lexer.tokenizar();
        ParserFalcato::parse(tokens)
    }

    #[test]
    fn test_parse_funcion_simple() {
        let fuente = r#"función principal() {
    retornar 0;
}"#;
        let resultado = parse_fuente(fuente);
        assert!(resultado.is_ok(), "Error: {:?}", resultado.err());
        let programa = resultado.unwrap();
        assert_eq!(programa.declaraciones.len(), 1);
    }

    #[test]
    fn test_parse_expresion_aritmetica() {
        let fuente = r#"función principal() -> Entero32 {
    el c: Entero32 = a + b * 2;
    retornar c;
}"#;
        let resultado = parse_fuente(fuente);
        assert!(resultado.is_ok(), "Error: {:?}", resultado.err());
    }

    #[test]
    fn test_parse_condicional() {
        let fuente = r#"función principal() -> Entero32 {
    el x: Entero32 = 10;
    si x > 5 {
        retornar 100;
    } sino {
        retornar 0;
    }
}"#;
        let resultado = parse_fuente(fuente);
        assert!(resultado.is_ok(), "Error: {:?}", resultado.err());
        
        let programa = resultado.unwrap();
        if let Declaracion::Funcion(func) = &programa.declaraciones[0] {
            if let Sentencia::Condicional(cond) = &func.cuerpo.sentencias[1] {
                assert!(cond.bloque_sino.is_some());
            } else {
                panic!("Esperaba condicional");
            }
        }
    }

    #[test]
    fn test_parse_bucle_mientras() {
        let fuente = r#"función principal() -> Entero32 {
    el i: Entero32 = 0;
    mientras i < 10 {
        i = i + 1;
    }
    retornar i;
}"#;
        let resultado = parse_fuente(fuente);
        assert!(resultado.is_ok(), "Error: {:?}", resultado.err());
    }

    #[test]
    fn test_parse_ffi_puts() {
        let fuente = r#"inseguro función puts(el mensaje: Palabra);

función principal() {
    puts("¡Hola, Falcato!");
    retornar 0;
}"#;
        let resultado = parse_fuente(fuente);
        assert!(resultado.is_ok(), "Error: {:?}", resultado.err());
    }

    #[test]
    fn test_parse_asignacion() {
        let fuente = r#"función principal() -> Entero32 {
    el x: Entero32 = 10;
    x = 20;
    retornar x;
}"#;
        let resultado = parse_fuente(fuente);
        assert!(resultado.is_ok(), "Error: {:?}", resultado.err());
    }

    #[test]
    fn test_error_token_inesperado() {
        let fuente = r#"función principal() {
    retornar + ;
}"#;
        let resultado = parse_fuente(fuente);
        assert!(resultado.is_err(), "Debería fallar con token inesperado");
    }

    #[test]
    fn test_parse_enumeracion() {
        let fuente = r#"enumeración Estado {
    Activo,
    Inactivo,
    Pausado
}

función principal() -> Entero32 {
    el estado: Estado = Estado.Activo;
    si estado es Estado.Activo {
        retornar 1;
    }
    retornar 0;
}"#;
        let resultado = parse_fuente(fuente);
        assert!(resultado.is_ok(), "Error: {:?}", resultado.err());
    }

    #[test]
    fn test_parse_enum_con_datos() {
        let fuente = r#"enumeración MiResultado {
    Exito(valor: Entero32),
    Error(codigo: Entero32)
}

función principal() -> Entero32 {
    el r: MiResultado = MiResultado.Exito(42);
    retornar 0;
}"#;
        let resultado = parse_fuente(fuente);
        assert!(resultado.is_ok(), "Error: {:?}", resultado.err());
    }

    #[test]
    fn test_parse_const_generics() {
        let fuente = r#"función longitud<N: Entero32>(los nums: [Entero32; N]) -> Entero32 { retornar 0; }"#;
        let resultado = parse_fuente(fuente);
        assert!(resultado.is_ok(), "Error: {:?}", resultado.err());
        
        let programa = resultado.unwrap();
        assert_eq!(programa.declaraciones.len(), 1);
        
        if let Declaracion::Funcion(func) = &programa.declaraciones[0] {
            assert_eq!(func.nombre, "longitud");
            assert_eq!(func.parametros_genericos.len(), 1);
            assert_eq!(func.parametros_genericos[0].nombre, "N");
            assert!(func.parametros_genericos[0].tipo.is_some());
            assert_eq!(func.parametros_genericos[0].bounds.len(), 0);
        } else {
            panic!("Esperaba declaración de función");
        }
    }

    #[test]
    fn test_parse_que_bounds() {
        let fuente = r#"función máximo<T que Comparable>(el a: T, el b: T) -> T {
    si a > b {
        retornar a;
    } sino {
        retornar b;
    }
}"#;
        let resultado = parse_fuente(fuente);
        assert!(resultado.is_ok(), "Error: {:?}", resultado.err());
        
        let programa = resultado.unwrap();
        if let Declaracion::Funcion(func) = &programa.declaraciones[0] {
            assert_eq!(func.parametros_genericos.len(), 1);
            assert_eq!(func.parametros_genericos[0].nombre, "T");
            assert!(func.parametros_genericos[0].tipo.is_none());
            assert_eq!(func.parametros_genericos[0].bounds, vec!["Comparable"]);
        } else {
            panic!("Esperaba declaración de función");
        }
    }
}
