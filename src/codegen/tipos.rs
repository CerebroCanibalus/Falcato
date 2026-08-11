//! Tipos — tipo_a_cranelift, tamano_tipo, inferir_tipo, operaciones binarias/unarias

use super::*;

impl Codegen {
    pub(crate) fn compilar_operacion_binaria(
        &mut self,
        op: OperadorBinario,
        izq: cranelift_codegen::ir::Value,
        der: cranelift_codegen::ir::Value,
        builder: &mut FunctionBuilder,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
        
        // Detectar operandos flotantes: F32/F64 requieren instrucciones flotantes
        // (sdiv/srem/icmp con F64 = Verifier error — bug R7.6: división flotante nunca compiló)
        let es_flotante = matches!(
            builder.func.dfg.value_type(izq),
            types::F32 | types::F64
        );
        
        let val = match op {
            OperadorBinario::Suma => {
                if es_flotante { builder.ins().fadd(izq, der) } else { builder.ins().iadd(izq, der) }
            }
            OperadorBinario::Resta => {
                if es_flotante { builder.ins().fsub(izq, der) } else { builder.ins().isub(izq, der) }
            }
            OperadorBinario::Multiplicacion => {
                if es_flotante { builder.ins().fmul(izq, der) } else { builder.ins().imul(izq, der) }
            }
            OperadorBinario::Division => {
                if es_flotante { builder.ins().fdiv(izq, der) } else { builder.ins().sdiv(izq, der) }
            }
            OperadorBinario::Modulo => {
                if es_flotante {
                    // Cranelift 0.112 NO tiene frem nativo → emular: a - floor(a/b) * b
                    // (mod matemático, signo del divisor — spec R7.6)
                    let div = builder.ins().fdiv(izq, der);
                    let fl = builder.ins().floor(div);
                    let mul = builder.ins().fmul(fl, der);
                    builder.ins().fsub(izq, mul)
                } else {
                    builder.ins().srem(izq, der)
                }
            }
            OperadorBinario::Igual => {
                if es_flotante { builder.ins().fcmp(FloatCC::Equal, izq, der) } else { builder.ins().icmp(IntCC::Equal, izq, der) }
            }
            OperadorBinario::Distinto => {
                if es_flotante { builder.ins().fcmp(FloatCC::NotEqual, izq, der) } else { builder.ins().icmp(IntCC::NotEqual, izq, der) }
            }
            OperadorBinario::Menor => {
                if es_flotante { builder.ins().fcmp(FloatCC::LessThan, izq, der) } else { builder.ins().icmp(IntCC::SignedLessThan, izq, der) }
            }
            OperadorBinario::Mayor => {
                if es_flotante { builder.ins().fcmp(FloatCC::GreaterThan, izq, der) } else { builder.ins().icmp(IntCC::SignedGreaterThan, izq, der) }
            }
            OperadorBinario::MenorIgual => {
                if es_flotante { builder.ins().fcmp(FloatCC::LessThanOrEqual, izq, der) } else { builder.ins().icmp(IntCC::SignedLessThanOrEqual, izq, der) }
            }
            OperadorBinario::MayorIgual => {
                if es_flotante { builder.ins().fcmp(FloatCC::GreaterThanOrEqual, izq, der) } else { builder.ins().icmp(IntCC::SignedGreaterThanOrEqual, izq, der) }
            }
            OperadorBinario::Y => builder.ins().band(izq, der),
            OperadorBinario::O => builder.ins().bor(izq, der),
            // Bitwise
            OperadorBinario::BitAnd => builder.ins().band(izq, der),
            OperadorBinario::BitOr => builder.ins().bor(izq, der),
            OperadorBinario::BitXor => builder.ins().bxor(izq, der),
            OperadorBinario::ShiftLeft => builder.ins().ishl(izq, der),
            OperadorBinario::ShiftRight => builder.ins().sshr(izq, der),
            OperadorBinario::ShiftRightLogico => builder.ins().ushr(izq, der),
        };
        
        Ok(val)
    }
    
    pub(crate) fn compilar_operacion_unaria(
        &mut self,
        op: OperadorUnario,
        val: cranelift_codegen::ir::Value,
        builder: &mut FunctionBuilder,
        span: &Span,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let resultado = match op {
            OperadorUnario::Negacion => {
                // Negación aritmética: 0 - val (el cero usa el ancho del valor)
                let tipo_val = builder.func.dfg.value_type(val);
                let cero = builder.ins().iconst(tipo_val, 0);
                builder.ins().isub(cero, val)
            }
            OperadorUnario::NegacionLogica => {
                // NegaciÃƒÂ³n booleana: val XOR 1
                let uno = builder.ins().iconst(types::I8, 1);
                builder.ins().bxor(val, uno)
            }
            OperadorUnario::BitNot => {
                // Bitwise NOT: NOT val
                builder.ins().bnot(val)
            }
            _ => {
                self.errores.agregar(ErrorCompilador::nuevo(
                    CategoriaError::Interno,
                    8,
                    span.clone(),
                    "Operador unario no soportado".to_string(),
                ));
                return Err(());
            }
        };
        
        Ok(resultado)
    }

    /// Resuelve un alias de tipo recursivamente hasta el tipo base.
    /// Ej: alias Entero = Entero32 → resolver_alias(Entero) = Entero32.
    pub(crate) fn resolver_alias(&self, tipo: &Tipo) -> Tipo {
        match tipo {
            Tipo::Nombre(nombre) => {
                if let Some(alias) = self.aliases.get(nombre) {
                    self.resolver_alias(alias)
                } else {
                    tipo.clone()
                }
            }
            Tipo::Vector(inner) => Tipo::Vector(Box::new(self.resolver_alias(inner))),
            Tipo::Array(inner, n) => Tipo::Array(Box::new(self.resolver_alias(inner)), *n),
            Tipo::Puntero(inner) => Tipo::Puntero(Box::new(self.resolver_alias(inner))),
            Tipo::Referencia(inner) => Tipo::Referencia(Box::new(self.resolver_alias(inner))),
            Tipo::ReferenciaMut(inner) => Tipo::ReferenciaMut(Box::new(self.resolver_alias(inner))),
            _ => tipo.clone(),
        }
    }

    pub(crate) fn tipo_a_cranelift(
        &self,
        tipo: &Tipo,
    ) -> cranelift_codegen::ir::Type {
        let tipo = self.resolver_alias(tipo);
        match &tipo {
            Tipo::Entero8 |
            Tipo::Natural8 => types::I8,
            Tipo::Entero16 |
            Tipo::Natural16 => types::I16,
            Tipo::Entero32 |
            Tipo::Natural32 => types::I32,
            Tipo::Entero64 |
            Tipo::Natural64 => types::I64,
            Tipo::Flotante32 => types::F32,
            Tipo::Flotante64 => types::F64,
            Tipo::Booleano => types::I8,
            Tipo::Caracter => types::I8,
            Tipo::Palabra => types::I64,
            Tipo::Texto => types::I64, // Puntero
            Tipo::Vacio => types::I8,
            Tipo::Puntero(_) => types::I64,
            Tipo::Referencia(_) => types::I64,
            Tipo::ReferenciaMut(_) => types::I64,
            Tipo::ReferenciaConLifetime(_, _) => types::I64,
            Tipo::ReferenciaMutConLifetime(_, _) => types::I64,
            Tipo::ReferenciaSelf(_) => types::I64,
            Tipo::ReferenciaMutSelf(_) => types::I64,
            Tipo::Array(_, _) => types::I64, // Puntero
            Tipo::ArrayGenerico(_, _) => types::I64,
            Tipo::Vector(_) => types::I64, // Puntero
            Tipo::Diccionario(_, _) => types::I64, // Puntero
            Tipo::Conjunto(_) => types::I64, // Puntero
            Tipo::Resultado(_, _) => types::I64, // Puntero
            Tipo::Generico(n) => panic!("No se puede compilar tipo genÃƒÂ©rico '{}' sin concretar", n),
            Tipo::Nombre(n) => panic!("No se puede compilar tipo Nombre '{}' sin resolver (Ã‚Â¿olvidaste importarlo?)", n),
            Tipo::NombreGenerico(n, _) => panic!("Tipo NombreGenerico '{}' no se pudo resolver (Ã‚Â¿olvidaste concretar genÃƒÂ©ricos?)", n),
        }
    }

    pub(crate) fn tamano_tipo(
        &self,
        tipo: &Tipo,
    ) -> u32 {
        let tipo = self.resolver_alias(tipo);
        match &tipo {
            Tipo::Entero8 |
            Tipo::Natural8 |
            Tipo::Booleano |
            Tipo::Caracter => 1,
            Tipo::Entero16 |
            Tipo::Natural16 => 2,
            Tipo::Entero32 |
            Tipo::Natural32 |
            Tipo::Flotante32 => 4,
            Tipo::Entero64 |
            Tipo::Natural64 |
            Tipo::Flotante64 |
            Tipo::Palabra |
            Tipo::Texto |
            Tipo::Vector(_) |
            Tipo::Diccionario(_, _) |
            Tipo::Conjunto(_) |
            Tipo::Resultado(_, _) |
            Tipo::Puntero(_) |
            Tipo::Referencia(_) |
            Tipo::ReferenciaMut(_) |
            Tipo::ReferenciaConLifetime(_, _) |
            Tipo::ReferenciaMutConLifetime(_, _) |
            Tipo::ReferenciaSelf(_) |
            Tipo::ReferenciaMutSelf(_) => 8,
            Tipo::Array(tipo_elem, longitud) => self.tamano_tipo(tipo_elem) * (*longitud as u32),
            Tipo::ArrayGenerico(tipo_elem, _) => {
                // En monomorfizaciÃƒÂ³n, esto se reemplaza por Array con tamaÃƒÂ±o conocido
                // Por ahora, retornar tamaÃƒÂ±o del elemento como fallback
                self.tamano_tipo(tipo_elem)
            }
            Tipo::Vacio => 4,
            Tipo::Nombre(nombre) => {
                // Buscar en structs o enums
                if let Some(layout) = self.structs.get(nombre) {
                    layout.tamano
                } else if let Some(layout) = self.enums.get(nombre) {
                    layout.tamano
                } else {
                    4
                }
            }
            Tipo::Generico(_) => 4, // Se resuelve en monomorfizaciÃƒÂ³n
            Tipo::NombreGenerico(_, _) => 4, // Se resuelve en monomorfizaciÃƒÂ³n
        }
    }

    pub(crate) fn inferir_tipo(
        &self,
        expr: &Expresion,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
    ) -> Tipo {
        match expr {
            Expresion::Literal(lit) => {
                match lit {
                    Literal::Entero(_, _) => Tipo::Entero32,
                    Literal::Flotante(_, _) => Tipo::Flotante64,
                    Literal::Palabra(_, _) => Tipo::Palabra,
                    Literal::Caracter(_, _) => Tipo::Caracter,
                    Literal::Booleano(_, _) => Tipo::Booleano,
                }
            }
            Expresion::Identificador(nombre, _) => {
                variables.get(nombre)
                    .map(|(_, tipo, _)| self.resolver_alias(tipo))
                    .unwrap_or(Tipo::Entero32)
            }
            Expresion::AccesoArray(array, _, _) => {
                let tipo_array = self.inferir_tipo(array, variables);
                match tipo_array {
                    Tipo::Array(t, _) => *t,
                    _ => Tipo::Entero32,
                }
            }
            Expresion::AccesoCampo(base, nombre_campo, _) => {
                let tipo_base = self.inferir_tipo(base, variables);
                if let Tipo::Nombre(nombre_struct) = tipo_base {
                    self.buscar_tipo_campo(&nombre_struct, nombre_campo)
                } else {
                    Tipo::Entero32
                }
            }
            Expresion::Binaria(_, _, _, _) => Tipo::Entero32, // Simplificado
            Expresion::ConstructorEnum(enum_nombre, _, _, _) => {
                // Para enums genÃƒÂ©ricos como Resultado, necesitamos inferir los tipos
                if enum_nombre == "Resultado" {
                    // Por defecto, asumir Entero32 para ambos parÃƒÂ¡metros
                    Tipo::Resultado(Box::new(Tipo::Entero32), Box::new(Tipo::Entero32))
                } else {
                    Tipo::Nombre(enum_nombre.clone())
                }
            }
            Expresion::Llamada(llamada) => {
                // Inferir tipo segÃƒÂºn la funciÃƒÂ³n conocida
                // Bug fix: sin esto, toda llamada caÃƒÂ­a al default Entero32
                match llamada.funcion.as_str() {
                    "como_entero64" | "texto_a_puntero" | "direccion_de" | "dir_de" => {
                        Tipo::Entero64
                    }
                    "texto_nuevo" | "texto_desde" | "texto_concatenar" | "texto_subtexto" => {
                        Tipo::Texto
                    }
                    "archivo_leer" => Tipo::Texto,
                    "vector_nuevo" => Tipo::Vector(Box::new(Tipo::Entero32)),
                    "canal_nuevo" => Tipo::Entero64,
                    "tcp_vincular" | "tcp_aceptar" => Tipo::Entero64,
                    "abs" | "max" | "min" | "texto_longitud" | "texto_comparar" | "archivo_escribir" => {
                        Tipo::Entero32
                    }
                    "tamano_de" => Tipo::Entero32,
                    "raiz" | "potencia" => Tipo::Flotante64,
                    "archivo_existe" | "texto_obtener_byte" => Tipo::Entero8,
                    _ => {
                        // Para built-ins no listados o funciones de usuario, verificar
                        // si es inseguro FFI (no tenemos firma en codegen, asumir Entero64
                        // por ser el tipo de puntero mÃƒÂ¡s comÃƒÂºn en FFI)
                        if llamada.funcion.starts_with("fc_") {
                            Tipo::Entero64  // funciones del trampolÃƒÂ­n C retornan punteros
                        } else {
                            Tipo::Entero32  // default: Entero32 por compatibilidad
                        }
                    }
                }
            }
            Expresion::DireccionDe(_, _) => Tipo::Entero64,
            _ => Tipo::Entero32,
        }
    }

    pub(crate) fn inferir_tipo_rango(&self, inicio: &Expresion, variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>) -> Tipo {
        self.inferir_tipo(inicio, variables)
    }

    /// ¿Es un tipo numérico (entero o flotante)?
    pub(crate) fn es_tipo_numerico(&self, tipo: &Tipo) -> bool {
        matches!(tipo,
            Tipo::Entero8 | Tipo::Entero16 | Tipo::Entero32 | Tipo::Entero64 |
            Tipo::Natural8 | Tipo::Natural16 | Tipo::Natural32 | Tipo::Natural64 |
            Tipo::Flotante32 | Tipo::Flotante64
        )
    }
}
