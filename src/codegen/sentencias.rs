//! Sentencias — compilar_sentencia, bucles, condicionales, con_executor, hilos
use super::*;
impl Codegen {
    pub(crate) fn compilar_sentencia(
        &mut self,
        sentencia: &Sentencia,
        builder: &mut FunctionBuilder,
        variables: &mut HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        _func_span: &Span,  // Span de la funci├│n padre para contexto
    ) -> Result<(), ()> {
        match sentencia {
            Sentencia::Expresion(expr) => {
                let _ = self.compilar_expresion(expr, builder, variables)?;
            }
            Sentencia::DeclaracionVariable(decl) => {
                let tipo = decl.tipo.clone().unwrap_or_else(||
                    self.inferir_tipo(&decl.valor, variables)
                );
                // Resolver apodos de tipo (ej: apodo ID = Entero64 → Entero64)
                // para que el slot tenga el tamaño correcto y el literal se emita bien.
                let tipo = self.resolver_alias(&tipo);

                // R7.7 F2 — `un x = a + b` sin tipo explícito: el artículo `un` (incierto)
                // + aritmética → Option<T>. Sin esto, inferir_tipo del codegen devuelve
                // Entero32 (Binaria => Entero32 simplificado) y el slot es de 4 bytes.
                let tipo = if decl.articulo == crate::ast::Articulo::Un {
                    if decl.tipo.is_none() {
                        if let Expresion::Binaria(izq, op, _, _) = &decl.valor {
                            let es_aritmetica = matches!(
                                op,
                                OperadorBinario::Suma
                                    | OperadorBinario::Resta
                                    | OperadorBinario::Multiplicacion
                            );
                            if es_aritmetica {
                                let t_interno = self.inferir_tipo(izq, variables);
                                if matches!(self.resolver_alias(&t_interno),
                                    Tipo::Entero8 | Tipo::Entero16 | Tipo::Entero32
                                        | Tipo::Natural8 | Tipo::Natural16 | Tipo::Natural32)
                                {
                                    Tipo::Option(Box::new(t_interno))
                                } else {
                                    tipo
                                }
                            } else {
                                tipo
                            }
                        } else {
                            tipo
                        }
                    } else {
                        tipo
                    }
                } else {
                    tipo
                };
                
                // Arrays: stack slot grande
                let (slot, _tamano) = match &tipo {
                    Tipo::Array(tipo_elem, longitud) => {
                        let tamano_elem = self.tamano_tipo(tipo_elem);
                        let tamano_total = tamano_elem * (*longitud as u32);
                        let slot = builder.create_sized_stack_slot(
                            cranelift_codegen::ir::StackSlotData::new(
                                cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                                tamano_total,
                                0,
                            )
                        );
                        
                        // Inicializar array
                        let base_ptr = builder.ins().stack_addr(types::I64, slot, 0);
                        let tamano_elem_i64 = tamano_elem as i64;
                        
                        match &decl.valor {
                            Expresion::ArrayRelleno(elem, _, _) => {
                                // Caso "todos expr": inicializar todos con el mismo valor
                                let val = self.compilar_expresion(elem, builder, variables)?;
                                for i in 0..*longitud {
                                    let offset = (i as i64 * tamano_elem_i64) as i32;
                                    let elem_ptr = builder.ins().iadd_imm(base_ptr, offset as i64);
                                    builder.ins().store(
                                        cranelift_codegen::ir::MemFlags::new(),
                                        val,
                                        elem_ptr,
                                        0
                                    );
                                }
                            }
                            Expresion::LiteralArray(elementos, _) => {
                                // Inicializar con valores expl├¡citos
                                for (i, elem) in elementos.iter().enumerate() {
                                    let val = self.compilar_expresion(elem, builder, variables)?;
                                    let offset = (i as i64 * tamano_elem_i64) as i32;
                                    let elem_ptr = builder.ins().iadd_imm(base_ptr, offset as i64);
                                    builder.ins().store(
                                        cranelift_codegen::ir::MemFlags::new(),
                                        val,
                                        elem_ptr,
                                        0
                                    );
                                }
                            }
                            _ => {
                                self.errores.agregar(ErrorCompilador::nuevo(
                                    CategoriaError::Interno,
                                    21,
                                    decl.span.clone(),
                                    "Expresi├│n no v├ílida para inicializaci├│n de arreglo".to_string(),
                                ));
                            }
                        }
                        
                        (slot, tamano_total)
                    }
                    Tipo::Nombre(nombre_tipo) => {
                        let tamano = self.tamano_tipo(&tipo);
                        let slot = builder.create_sized_stack_slot(
                            cranelift_codegen::ir::StackSlotData::new(
                                cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                                tamano,
                                0,
                            )
                        );
                        
                        match &decl.valor {
                            Expresion::InicializacionStruct(_, campos, _) => {
                                let base_ptr = builder.ins().stack_addr(types::I64, slot, 0);
                                let layout = match self.structs.get(nombre_tipo) {
                                    Some(l) => l.clone(),
                                    None => {
                                        self.errores.agregar(ErrorCompilador::nuevo(
                                            CategoriaError::Interno,
                                            30,
                                            decl.span.clone(),
                                            format!("Struct '{}' no registrado", nombre_tipo),
                                        ));
                                        return Err(());
                                    }
                                };

                                // Fase 15B: bitfield struct
                                if layout.es_bitfield {
                                    let backing_type = match layout.tamano {
                                        1 => types::I8,
                                        2 => types::I16,
                                        4 => types::I32,
                                        _ => types::I64,
                                    };
                                    let cero = builder.ins().iconst(backing_type, 0);
                                    builder.ins().store(cranelift_codegen::ir::MemFlags::new(), cero, base_ptr, 0);
                                    for (nombre_campo, valor_expr) in campos {
                                        if let Some(&(bf_offset, bf_ancho)) = layout.bitfields.get(nombre_campo) {
                                            let val = self.compilar_expresion(valor_expr, builder, variables)?;
                                            let raw = builder.ins().load(backing_type, cranelift_codegen::ir::MemFlags::new(), base_ptr, 0);
                                            let cur_i32 = if backing_type != types::I32 {
                                                builder.ins().uextend(types::I32, raw)
                                            } else {
                                                raw
                                            };
                                            let uno = builder.ins().iconst(types::I32, 1);
                                            let ancho_val = builder.ins().iconst(types::I32, bf_ancho as i64);
                                            let field_mask = builder.ins().ishl(uno, ancho_val);
                                            let menos_uno = builder.ins().iconst(types::I32, -1);
                                            let field_mask = builder.ins().iadd(field_mask, menos_uno);
                                            let offset_val = builder.ins().iconst(types::I32, bf_offset as i64);
                                            let shifted_mask = builder.ins().ishl(field_mask, offset_val);
                                            let not_mask = builder.ins().bnot(shifted_mask);
                                            let cleared = builder.ins().band(cur_i32, not_mask);
                                            let valor_masked = builder.ins().band(val, field_mask);
                                            let valor_shifted = builder.ins().ishl(valor_masked, offset_val);
                                            let nuevo = builder.ins().bor(cleared, valor_shifted);
                                            let store_val = if backing_type != types::I32 {
                                                builder.ins().ireduce(backing_type, nuevo)
                                            } else {
                                                nuevo
                                            };
                                            builder.ins().store(cranelift_codegen::ir::MemFlags::new(), store_val, base_ptr, 0);
                                        }
                                    }
                                } else {
                                    for (nombre_campo, valor) in campos {
                                        let val = self.compilar_expresion(valor, builder, variables)?;
                                        let offset = match layout.offsets.get(nombre_campo) {
                                            Some(o) => *o as i64,
                                            None => {
                                                self.errores.agregar(ErrorCompilador::nuevo(
                                                    CategoriaError::Interno,
                                                    31,
                                                    decl.span.clone(),
                                                    format!("Campo '{}' no encontrado en '{}'", nombre_campo, nombre_tipo),
                                                ));
                                                return Err(());
                                            }
                                        };
                                        let campo_ptr = builder.ins().iadd_imm(base_ptr, offset);
                                        builder.ins().store(
                                            cranelift_codegen::ir::MemFlags::new(),
                                            val,
                                            campo_ptr,
                                            0,
                                        );
                                    }
                                }
                            }
                            Expresion::ConstructorEnum(enum_nombre, variante_nombre, argumentos, _) => {
                                let base_ptr = builder.ins().stack_addr(types::I64, slot, 0);
                                let layout = match self.enums.get(enum_nombre) {
                                    Some(l) => l.clone(),
                                    None => {
                                        self.errores.agregar(ErrorCompilador::nuevo(
                                            CategoriaError::Interno,
                                            50,
                                            decl.span.clone(),
                                            format!("Enum '{}' no registrado", enum_nombre),
                                        ));
                                        return Err(());
                                    }
                                };
                                
                                // Almacenar tag
                                let tag = *layout.variantes.get(variante_nombre).unwrap_or(&0);
                                let tag_val = builder.ins().iconst(types::I32, tag as i64);
                                builder.ins().store(
                                    cranelift_codegen::ir::MemFlags::new(),
                                    tag_val,
                                    base_ptr,
                                    0,
                                );
                                
                                // Almacenar datos si hay argumentos
                                if !argumentos.is_empty() {
                                    let datos_ptr = builder.ins().iadd_imm(base_ptr, layout.datos_offset as i64);
                                    let mut offset = 0i64;
                                    for arg in argumentos {
                                        let val = self.compilar_expresion(arg, builder, variables)?;
                                        let arg_ptr = builder.ins().iadd_imm(datos_ptr, offset);
                                        builder.ins().store(
                                            cranelift_codegen::ir::MemFlags::new(),
                                            val,
                                            arg_ptr,
                                            0,
                                        );
                                        offset += 4;
                                    }
                                }
                            }
                            _ => {
                                // R9.0.1/R9.0.2 — si el valor es una llamada que produce un
                                // puntero a struct (retorno de función de usuario con struct,
                                // o diccionario_obtener con valor struct), copiar el struct
                                // del ptr al slot de n en vez de stack_store (guardaría el ptr).
                                let es_llamada_struct = match &decl.valor {
                                    Expresion::Llamada(llamada) => {
                                        let ret_struct = self.declaraciones.get(&llamada.funcion)
                                            .and_then(|f| f.retorno.as_ref())
                                            .map(|r| self.tipo_es_struct(r).is_some())
                                            .unwrap_or(false);
                                        let dict_struct = llamada.funcion == "diccionario_obtener"
                                            && llamada.tipo_args.len() == 2
                                            && self.tipo_es_struct(&llamada.tipo_args[1]).is_some();
                                        ret_struct || dict_struct
                                    }
                                    _ => false,
                                };
                                // F-001: builtins que retornan colecciones (Vector, Diccionario,
                                  // Texto) devuelven un puntero I64 al descriptor en heap.
                                  // Hay que copiar los 24 bytes del descriptor, no guardar el puntero.
                                  let es_coleccion_builtin = matches!(
                                      tipo,
                                      Tipo::Vector(_) | Tipo::Diccionario(_, _) | Tipo::Conjunto(_) | Tipo::Texto
                                  );
                                if es_llamada_struct || es_coleccion_builtin {
                                    let base_ptr = builder.ins().stack_addr(types::I64, slot, 0);
                                    let src_ptr = self.compilar_expresion(&decl.valor, builder, variables)?;
                                    self.copiar_mem(base_ptr, src_ptr, tamano, builder);
                                } else {
                                    let valor = self.compilar_expresion(&decl.valor, builder, variables)?;
                                    builder.ins().stack_store(valor, slot, 0);
                                }
                            }
                        }
                        
                        (slot, tamano)
                    }
                    Tipo::Resultado(_, _) => {
                        // Resultado como valor I64 empaquetado (tag en low 32, data en high 32)
                        let tamano = self.tamano_tipo(&tipo);
                        let slot = builder.create_sized_stack_slot(
                            cranelift_codegen::ir::StackSlotData::new(
                                cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                                tamano,
                                0,
                            )
                        );
                        let valor = self.compilar_expresion(&decl.valor, builder, variables)?;
                        builder.ins().stack_store(valor, slot, 0);
                        (slot, tamano)
                    }
                    // R7.7 F2 — `un x = a + b`: Option<T> checked con None
                    // Algo(tag 0, data = valor) | Nada(tag 1). Empaqueta I64 como Resultado.
                    Tipo::Option(tipo_interno) => {
                        let tamano = self.tamano_tipo(&tipo);
                        let slot = builder.create_sized_stack_slot(
                            cranelift_codegen::ir::StackSlotData::new(
                                cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                                tamano,
                                0,
                            )
                        );
                        // Solo soportamos aritmética binaria checked: `un x = a + b`
                        match &decl.valor {
                            Expresion::Binaria(izq, op, der, span_bin) => {
                                let tipo_izq = self.inferir_tipo(izq, variables);
                                let tipo_der = self.inferir_tipo(der, variables);
                                let val_izq = self.compilar_lado_binaria(izq, &tipo_der, builder, variables)?;
                                let val_der = self.compilar_lado_binaria(der, &tipo_izq, builder, variables)?;
                                let val_r = self.compilar_operacion_binaria(*op, val_izq, val_der, builder)?;
                                let tipo_val = builder.func.dfg.value_type(val_izq);
                                let bits = tipo_val.bits() as i64;
                                let es_signed = matches!(
                                    self.resolver_alias(tipo_interno),
                                    Tipo::Entero8 | Tipo::Entero16 | Tipo::Entero32
                                );
                                use cranelift_codegen::ir::condcodes::IntCC;
                                let overflow = match op {
                                    OperadorBinario::Suma if es_signed => {
                                        let xor_a = builder.ins().bxor(val_izq, val_r);
                                        let xor_b = builder.ins().bxor(val_der, val_r);
                                        let and = builder.ins().band(xor_a, xor_b);
                                        let shift = builder.ins().iconst(tipo_val, bits - 1);
                                        let signo = builder.ins().sshr(and, shift);
                                        let cero = builder.ins().iconst(tipo_val, 0);
                                        builder.ins().icmp(IntCC::NotEqual, signo, cero)
                                    }
                                    OperadorBinario::Resta if es_signed => {
                                        let xor_ab = builder.ins().bxor(val_izq, val_der);
                                        let xor_ar = builder.ins().bxor(val_izq, val_r);
                                        let and = builder.ins().band(xor_ab, xor_ar);
                                        let shift = builder.ins().iconst(tipo_val, bits - 1);
                                        let signo = builder.ins().sshr(and, shift);
                                        let cero = builder.ins().iconst(tipo_val, 0);
                                        builder.ins().icmp(IntCC::NotEqual, signo, cero)
                                    }
                                    OperadorBinario::Multiplicacion if es_signed => {
                                        let hi = builder.ins().smulhi(val_izq, val_der);
                                        let shift = builder.ins().iconst(tipo_val, bits - 1);
                                        let ext = builder.ins().sshr(val_r, shift);
                                        builder.ins().icmp(IntCC::NotEqual, hi, ext)
                                    }
                                    OperadorBinario::Suma => {
                                        builder.ins().icmp(IntCC::UnsignedLessThan, val_r, val_izq)
                                    }
                                    OperadorBinario::Resta => {
                                        builder.ins().icmp(IntCC::UnsignedGreaterThan, val_r, val_izq)
                                    }
                                    OperadorBinario::Multiplicacion => {
                                        let hi = builder.ins().umulhi(val_izq, val_der);
                                        let cero = builder.ins().iconst(tipo_val, 0);
                                        builder.ins().icmp(IntCC::NotEqual, hi, cero)
                                    }
                                    _ => {
                                        self.errores.agregar(ErrorCompilador::nuevo(
                                            CategoriaError::Tipo,
                                            110, // T110: un requiere aritmética
                                            span_bin.clone(),
                                            format!("'un' (Option checked) solo aplica a suma, resta o multiplicación, no a '{:?}'", op),
                                        ));
                                        return Err(());
                                    }
                                };
                                // Empaquetar: Algo(tag 0, data = r) | Nada(tag 1)
                                let r_i64 = builder.ins().uextend(types::I64, val_r);
                                let shift32 = builder.ins().iconst(types::I64, 32);
                                let algo_packed = builder.ins().ishl(r_i64, shift32); // tag 0 implícito
                                let uno = builder.ins().iconst(types::I64, 1);
                                let nada_packed = uno; // tag 1, sin data
                                let valor = builder.ins().select(overflow, nada_packed, algo_packed);
                                builder.ins().stack_store(valor, slot, 0);
                                (slot, tamano)
                            }
                            _ => {
                                self.errores.agregar(ErrorCompilador::nuevo(
                                    CategoriaError::Tipo,
                                    111, // T111: un Option requiere aritmética binaria
                                    decl.span.clone(),
                                    "'un' (Option checked) requiere una operación aritmética binaria: un x = a + b".to_string(),
                                ));
                                return Err(());
                            }
                        }
                    }
                    _ => {
                        let tamano = self.tamano_tipo(&tipo);
                        let slot = builder.create_sized_stack_slot(
                            cranelift_codegen::ir::StackSlotData::new(
                                cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                                tamano,
                                0,
                            )
                        );
                        // Si el valor es un literal numérico y hay tipo declarado, emitir
                        // con el ancho correcto (ej: el x: Entero64 = 5, o apodo ID = Entero64)
                        let tipo_resuelto = self.resolver_alias(&tipo);
                        let valor = match (&decl.valor, &tipo_resuelto) {
                            (Expresion::Literal(lit), t) if self.es_tipo_numerico(t) => {
                                self.compilar_literal_con_tipo(lit, t, builder)?
                            }
                            (Expresion::Unaria(op, inner, _), t) if self.es_tipo_numerico(t) => {
                                if let Expresion::Literal(lit) = inner.as_ref() {
                                    let val = self.compilar_literal_con_tipo(lit, t, builder)?;
                                    self.compilar_operacion_unaria(*op, val, builder, &decl.span)?
                                } else {
                                    self.compilar_expresion(&decl.valor, builder, variables)?
                                }
                            }
                            _ => self.compilar_expresion(&decl.valor, builder, variables)?,
                        };
                        builder.ins().stack_store(valor, slot, 0);
                        (slot, tamano)
                    }
                };
                
                // R6: Drop automático — variable owned (el/los) de tipo heap → candidata a free
                // (registrar ANTES de mover `tipo` al insert)
                if matches!(decl.articulo, crate::ast::Articulo::El | crate::ast::Articulo::Los) {
                    self.registrar_heap(&decl.nombre, &tipo);
                }
                variables.insert(decl.nombre.clone(), (slot, tipo, decl.articulo));
            }
            Sentencia::Asignacion(asig) => {
                let valor = self.compilar_expresion(&asig.valor, builder, variables)?;
                
                match &asig.lugar {
                    crate::ast::Lugar::Identificador(nombre) => {
                        if let Some((slot, _tipo, _articulo)) = variables.get(nombre) {
                            builder.ins().stack_store(valor, *slot, 0);
                        } else {
                            self.errores.agregar(ErrorCompilador::nuevo(
                                CategoriaError::Interno,
                                15,
                                asig.span.clone(),
                                format!("Variable '{}' no encontrada para asignaci├│n", nombre),
                            ));
                        }
                    }
                    crate::ast::Lugar::Array(array_expr, indice_expr) => {
                        let array_val = self.compilar_expresion(array_expr, builder, variables)?;
                        let idx_val = self.compilar_expresion(indice_expr, builder, variables)?;
                        
                        let tipo_array = self.inferir_tipo(array_expr, variables);
                        let tamano_elem = match tipo_array {
                            Tipo::Array(ref t, _) => self.tamano_tipo(t) as i64,
                            _ => {
                                self.errores.agregar(ErrorCompilador::nuevo(
                                    CategoriaError::Interno,
                                    20,
                                    asig.span.clone(),
                                    "Asignaci├│n a arreglo en tipo no-arreglo".to_string(),
                                ));
                                return Err(());
                            }
                        };
                        
                        let idx_i64 = if builder.func.dfg.value_type(idx_val) == types::I32 {
                            builder.ins().sextend(types::I64, idx_val)
                        } else {
                            idx_val
                        };
                        
                        let offset = builder.ins().imul_imm(idx_i64, tamano_elem);
                        let elem_ptr = builder.ins().iadd(array_val, offset);
                        builder.ins().store(
                            cranelift_codegen::ir::MemFlags::new(),
                            valor,
                            elem_ptr,
                            0
                        );
                    }
                    // Fase 15B: bitfield write ÔÇö reg.campo = valor
                    crate::ast::Lugar::Campo(base_expr, nombre_campo) => {
                        let struct_ptr = self.compilar_expresion(base_expr, builder, variables)?;
                        let tipo_base = self.inferir_tipo(base_expr, variables);
                        // S003 fix: asignación a campo a través de referencia (&mut T).
                        // El identificador de tipo referencia ya carga el PUNTERO —
                        // resolver el tipo interno para el layout.
                        let nombre_struct = match &tipo_base {
                            Tipo::Nombre(n) => n.clone(),
                            Tipo::Referencia(inner) | Tipo::ReferenciaMut(inner) |
                            Tipo::ReferenciaConLifetime(_, inner) | Tipo::ReferenciaMutConLifetime(_, inner) |
                            Tipo::ReferenciaSelf(inner) | Tipo::ReferenciaMutSelf(inner) => {
                                match self.resolver_alias(inner) {
                                    Tipo::Nombre(n) => n,
                                    otro => {
                                        self.errores.agregar(ErrorCompilador::nuevo(
                                            CategoriaError::Interno, 32, asig.span.clone(),
                                            format!("Asignación a campo en tipo no-struct '{:?}'", otro),
                                        ));
                                        return Err(());
                                    }
                                }
                            }
                            _ => {
                                self.errores.agregar(ErrorCompilador::nuevo(
                                    CategoriaError::Interno, 32, asig.span.clone(),
                                    format!("Asignación a campo en tipo no-struct '{:?}'", tipo_base),
                                ));
                                return Err(());
                            }
                        };
                        let layout = self.structs.get(&nombre_struct).cloned().unwrap();
                        if layout.es_bitfield {
                            if let Some(&(bf_offset, bf_ancho)) = layout.bitfields.get(nombre_campo) {
                                let backing_type = match layout.tamano {
                                    1 => types::I8,
                                    2 => types::I16,
                                    4 => types::I32,
                                    _ => types::I64,
                                };
                                // Cargar entero de respaldo
                                let raw_val = builder.ins().load(backing_type, cranelift_codegen::ir::MemFlags::new(), struct_ptr, 0);
                                let val_i32 = if backing_type != types::I32 {
                                    builder.ins().uextend(types::I32, raw_val)
                                } else {
                                    raw_val
                                };
                                // mask = ((1 << ancho) - 1) << offset
                                let uno = builder.ins().iconst(types::I32, 1);
                                let ancho_val = builder.ins().iconst(types::I32, bf_ancho as i64);
                                let field_mask = builder.ins().ishl(uno, ancho_val);
                                let menos_uno = builder.ins().iconst(types::I32, -1);
                                let field_mask = builder.ins().iadd(field_mask, menos_uno);
                                let offset_val = builder.ins().iconst(types::I32, bf_offset as i64);
                                let shifted_mask = builder.ins().ishl(field_mask, offset_val);
                                // Limpiar bits: reg & ~shifted_mask
                                let not_mask = builder.ins().bnot(shifted_mask);
                                let cleared = builder.ins().band(val_i32, not_mask);
                                // Insertar valor: (valor & field_mask) << offset
                                let valor_masked = builder.ins().band(valor, field_mask);
                                let valor_shifted = builder.ins().ishl(valor_masked, offset_val);
                                let nuevo_val = builder.ins().bor(cleared, valor_shifted);
                                // Truncar y almacenar
                                let store_val = if backing_type != types::I32 {
                                    builder.ins().ireduce(backing_type, nuevo_val)
                                } else {
                                    nuevo_val
                                };
                                builder.ins().store(cranelift_codegen::ir::MemFlags::new(), store_val, struct_ptr, 0);
                            }
                        } else {
                            // Struct normal: store directo al offset del campo
                            if let Some(offset) = layout.offsets.get(nombre_campo) {
                                let campo_ptr = builder.ins().iadd_imm(struct_ptr, *offset as i64);
                                let tipo_campo = self.buscar_tipo_campo(&nombre_struct, nombre_campo);
                                let _cranelift_type = self.tipo_a_cranelift(&tipo_campo);
                                builder.ins().store(cranelift_codegen::ir::MemFlags::new(), valor, campo_ptr, 0);
                            }
                        }
                    }
                }
            }
            Sentencia::Retornar(expr, _span) => {
                // R6: Drop automático — si se retorna una variable heap owned, el caller
                // es dueño → quitar de vivas (no liberar aquí)
                if let Some(Expresion::Identificador(nombre, _)) = expr.as_ref() {
                    if self.heap_vivas.iter().any(|(n, _)| n == nombre) {
                        self.quitar_heap(nombre);
                    }
                }
                // R6: liberar el resto de variables heap vivas ANTES del return
                self.liberar_scope(0, builder, variables)?;
                if let Some(expr) = expr {
                    let val = self.compilar_expresion(expr, builder, variables)?;
                    // R9.0.1 — retorno de struct: copiar el struct (apuntado por val)
                    // al sret ptr y retornar void
                    if let Some(dest) = self.sret_destino {
                        let tipo_expr = self.inferir_tipo(expr, variables);
                        let tamano = self.tamano_tipo(&tipo_expr);
                        self.copiar_mem(dest, val, tamano, builder);
                        builder.ins().return_(&[]);
                        return Ok(());
                    }
                    // Si la expresi├│n accede a una variable de tipo Resultado o enum peque├▒o,
                    // el valor es un puntero al struct en stack ÔåÆ dereferenciar para retornar
                    if matches!(expr, Expresion::Identificador(_, _)) {
                        let tipo_expr = self.inferir_tipo(expr, variables);
                        if matches!(tipo_expr, Tipo::Resultado(_, _) | Tipo::Nombre(_)) && self.tamano_tipo(&tipo_expr) <= 8 {
                            let datos = builder.ins().load(types::I64, cranelift_codegen::ir::MemFlags::new(), val, 0);
                            builder.ins().return_(&[datos]);
                            return Ok(());
                        }
                    }
                    builder.ins().return_(&[val]);
                } else {
                    builder.ins().return_(&[]);
                }
            }
            // R7.7 — romper: salir del bucle más interno (jump al exit)
            Sentencia::Romper(span) => {
                match self.pila_bucles.last() {
                    Some((_, exit_block)) => {
                        builder.ins().jump(*exit_block, &[]);
                        // El código después de romper es inalcanzable; crear bloque
                        // huérfano para que el verifier no vea instrucciones muertas
                        let muerto = builder.create_block();
                        builder.switch_to_block(muerto);
                        builder.seal_block(muerto);
                    }
                    None => {
                        self.errores.agregar(ErrorCompilador::nuevo(
                            CategoriaError::Interno,
                            1,
                            span.clone(),
                            "'romper' fuera de bucle en codegen (semantic debería haberlo atrapado)".to_string(),
                        ));
                    }
                }
            }
            // R7.7 — continuar: saltar a la siguiente iteración (jump al epilogue → header)
            Sentencia::Continuar(span) => {
                match self.pila_bucles.last() {
                    Some((epilogue_block, _)) => {
                        builder.ins().jump(*epilogue_block, &[]);
                        // Código inalcanzable después de continuar
                        let muerto = builder.create_block();
                        builder.switch_to_block(muerto);
                        builder.seal_block(muerto);
                    }
                    None => {
                        self.errores.agregar(ErrorCompilador::nuevo(
                            CategoriaError::Interno,
                            1,
                            span.clone(),
                            "'continuar' fuera de bucle en codegen (semantic debería haberlo atrapado)".to_string(),
                        ));
                    }
                }
            }
            Sentencia::Condicional(cond) => {
                let then_block = builder.create_block();
                let else_block = builder.create_block();
                let merge_block = builder.create_block();

                // Compilar condici├│n
                let cond_val = self.compilar_expresion(&cond.condicion, builder, variables)?;
                
                // Branch condicional
                builder.ins().brif(cond_val, then_block, &[], else_block, &[]);

                match cond.modo {
                    crate::ast::ModoVerbal::Subjuntivo => {
                        // SUBJUNTIVO: condici├│n improbable ÔåÆ cold path
                        // Construir ELSE primero (hot path, en l├¡nea)
                        builder.switch_to_block(else_block);
                        builder.seal_block(else_block);
                        // R6: snapshot de scope de la rama (liberar heap declarado en ella)
                        let m_else = self.marcar_scope();
                        let mut else_terminado = false;
                        if let Some(ref bloque_sino) = cond.bloque_sino {
                            for sentencia in &bloque_sino.sentencias {
                                if matches!(sentencia, Sentencia::Retornar(_, _)) {
                                    else_terminado = true;
                                }
                                self.compilar_sentencia(sentencia, builder, variables, _func_span)?;
                            }
                        }
                        if !else_terminado {
                            self.liberar_scope(m_else, builder, variables)?;
                            builder.ins().jump(merge_block, &[]);
                        }

                        // Construir THEN despu├®s (cold path, fuera de l├¡nea)
                        builder.switch_to_block(then_block);
                        builder.seal_block(then_block);
                        // R6: snapshot de scope de la rama
                        let m_then = self.marcar_scope();
                        let mut then_terminado = false;
                        for sentencia in &cond.bloque_entonces.sentencias {
                            if matches!(sentencia, Sentencia::Retornar(_, _)) {
                                then_terminado = true;
                            }
                            self.compilar_sentencia(sentencia, builder, variables, _func_span)?;
                        }
                        if !then_terminado {
                            self.liberar_scope(m_then, builder, variables)?;
                            builder.ins().jump(merge_block, &[]);
                        }
                    }
                    _ => {
                        // INDICATIVO / ESTATIVO: flujo normal
                        // Construir THEN primero (hot path)
                        builder.switch_to_block(then_block);
                        builder.seal_block(then_block);
                        // R6: snapshot de scope de la rama
                        let m_then = self.marcar_scope();
                        let mut then_terminado = false;
                        for sentencia in &cond.bloque_entonces.sentencias {
                            if matches!(sentencia, Sentencia::Retornar(_, _)) {
                                then_terminado = true;
                            }
                            self.compilar_sentencia(sentencia, builder, variables, _func_span)?;
                        }
                        if !then_terminado {
                            self.liberar_scope(m_then, builder, variables)?;
                            builder.ins().jump(merge_block, &[]);
                        }

                        // Construir ELSE despu├®s
                        builder.switch_to_block(else_block);
                        builder.seal_block(else_block);
                        // R6: snapshot de scope de la rama
                        let m_else = self.marcar_scope();
                        let mut else_terminado = false;
                        if let Some(ref bloque_sino) = cond.bloque_sino {
                            for sentencia in &bloque_sino.sentencias {
                                if matches!(sentencia, Sentencia::Retornar(_, _)) {
                                    else_terminado = true;
                                }
                                self.compilar_sentencia(sentencia, builder, variables, _func_span)?;
                            }
                        }
                        if !else_terminado {
                            self.liberar_scope(m_else, builder, variables)?;
                            builder.ins().jump(merge_block, &[]);
                        }
                    }
                }

                // Bloque de uni├│n
                builder.switch_to_block(merge_block);
                builder.seal_block(merge_block);
            }
            Sentencia::BucleMientras(bucle) => {
                let header_block = builder.create_block();
                let body_block = builder.create_block();
                let epilogue_block = builder.create_block();
                let exit_block = builder.create_block();

                // R7.7: pila de bucles — romper → exit, continuar → epilogue (i++)
                self.pila_bucles.push((epilogue_block, exit_block));

                // Saltar al header inicialmente
                builder.ins().jump(header_block, &[]);

                // Header: evaluar condici├│n
                builder.switch_to_block(header_block);
                let cond_val = self.compilar_expresion(&bucle.condicion, builder, variables)?;
                builder.ins().brif(cond_val, body_block, &[], exit_block, &[]);
                // NO sellar header todav├¡a ÔÇö el body puede saltar de vuelta

                // Body: ejecutar sentencias y volver al header
                builder.switch_to_block(body_block);
                // R6: snapshot de scope — variables heap declaradas en el body se liberan
                // al final de CADA iteración (crítico para loops largos: sin esto, leak acumulado)
                let m_body = self.marcar_scope();
                let mut body_terminado = false;
                for sentencia in &bucle.bloque.sentencias {
                    if matches!(sentencia, Sentencia::Retornar(_, _)) {
                        body_terminado = true;
                    }
                    self.compilar_sentencia(sentencia, builder, variables, _func_span)?;
                }
                if !body_terminado {
                    // El epilogue libera el scope del body (evitar doble liberación)
                    builder.ins().jump(epilogue_block, &[]);
                }
                builder.seal_block(body_block);

                // Epilogue: punto de reunión — libera scope del body y salta al header
                // (destino de `continuar`; el body normal también pasa por aquí)
                builder.switch_to_block(epilogue_block);
                self.liberar_scope(m_body, builder, variables)?;
                builder.ins().jump(header_block, &[]);
                builder.seal_block(epilogue_block);

                // Ahora que todos los saltos al header est├ín declarados, sellarlo
                builder.seal_block(header_block);

                // Exit: continuar despu├®s del bucle (destino de `romper`)
                builder.switch_to_block(exit_block);
                builder.seal_block(exit_block);

                self.pila_bucles.pop();
            }
            Sentencia::BuclePara(bucle) => {
                // Detectar si el iterable es un rango
                if let Expresion::Rango(inicio_expr, fin_expr, inclusivo, _) = &bucle.iterable {
                    // === PARA SOBRE RANGO: para i en 0..10 { ... } ===
                    let header_block = builder.create_block();
                    let body_block = builder.create_block();
                    let epilogue_block = builder.create_block();
                    let exit_block = builder.create_block();

                    // R7.7: pila de bucles — romper → exit, continuar → epilogue (i++)
                    self.pila_bucles.push((epilogue_block, exit_block));

                    // Slot para la variable de iteraci├│n
                    let var_slot = builder.create_sized_stack_slot(
                        cranelift_codegen::ir::StackSlotData::new(
                            cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                            4, // I32
                            0,
                        )
                    );

                    // Compilar inicio y guardar en variable
                    let inicio_val = self.compilar_expresion(inicio_expr, builder, variables)?;
                    builder.ins().stack_store(inicio_val, var_slot, 0);

                    // Compilar fin y guardar en slot temporal
                    let fin_slot = builder.create_sized_stack_slot(
                        cranelift_codegen::ir::StackSlotData::new(
                            cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                            4,
                            0,
                        )
                    );
                    let fin_val = self.compilar_expresion(fin_expr, builder, variables)?;
                    builder.ins().stack_store(fin_val, fin_slot, 0);

                    // Registrar variable de iteraci├│n
                    let tipo_elem = self.inferir_tipo(inicio_expr, variables);
                    variables.insert(bucle.variable.clone(), (var_slot, tipo_elem, crate::ast::Articulo::La));

                    // Saltar al header
                    builder.ins().jump(header_block, &[]);

                    // Header: evaluar i < fin (o i <= fin si inclusivo)
                    builder.switch_to_block(header_block);
                    let cur_val = builder.ins().stack_load(types::I32, var_slot, 0);
                    let fin_loaded = builder.ins().stack_load(types::I32, fin_slot, 0);
                    let cc = if *inclusivo {
                        cranelift_codegen::ir::condcodes::IntCC::SignedLessThanOrEqual
                    } else {
                        cranelift_codegen::ir::condcodes::IntCC::SignedLessThan
                    };
                    let cond = builder.ins().icmp(cc, cur_val, fin_loaded);
                    builder.ins().brif(cond, body_block, &[], exit_block, &[]);

                    // Body: ejecutar bloque
                    builder.switch_to_block(body_block);
                    // R6: snapshot de scope del body (liberar heap por iteración)
                    let m_body = self.marcar_scope();
                    let mut body_terminado = false;
                    for sentencia in &bucle.bloque.sentencias {
                        if matches!(sentencia, Sentencia::Retornar(_, _)) {
                            body_terminado = true;
                        }
                        self.compilar_sentencia(sentencia, builder, variables, _func_span)?;
                    }

                    if !body_terminado {
                        // El epilogue libera el scope y hace i++ (evitar doble liberación)
                        builder.ins().jump(epilogue_block, &[]);
                    }
                    builder.seal_block(body_block);

                    // Epilogue: punto de reunión — libera scope, i++, vuelve al header
                    // (destino de `continuar`; el body normal también pasa por aquí)
                    builder.switch_to_block(epilogue_block);
                    self.liberar_scope(m_body, builder, variables)?;
                    // i = i + 1
                    let cur = builder.ins().stack_load(types::I32, var_slot, 0);
                    let uno = builder.ins().iconst(types::I32, 1);
                    let nuevo = builder.ins().iadd(cur, uno);
                    builder.ins().stack_store(nuevo, var_slot, 0);
                    builder.ins().jump(header_block, &[]);
                    builder.seal_block(epilogue_block);
                    builder.seal_block(header_block);

                    // Exit (destino de `romper`)
                    builder.switch_to_block(exit_block);
                    builder.seal_block(exit_block);

                    // Limpiar variable de iteraci├│n
                    variables.remove(&bucle.variable);
                    self.pila_bucles.pop();
                } else {
                    // === PARA SOBRE ARRAY (existente) ===
                    let header_block = builder.create_block();
                    let body_block = builder.create_block();
                    let epilogue_block = builder.create_block();
                    let exit_block = builder.create_block();

                    // R7.7: pila de bucles — romper → exit, continuar → epilogue (i++)
                    self.pila_bucles.push((epilogue_block, exit_block));

                    // Crear slot para ├¡ndice (i = 0)
                    let idx_slot = builder.create_sized_stack_slot(
                        cranelift_codegen::ir::StackSlotData::new(
                            cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                            4, // I32
                            0,
                        )
                    );
                    let cero = builder.ins().iconst(types::I32, 0);
                    builder.ins().stack_store(cero, idx_slot, 0);

                    // Compilar iterable (obtener puntero al array)
                    let array_ptr = self.compilar_expresion(&bucle.iterable, builder, variables)?;

                    // Obtener tipo y longitud del array
                    let tipo_iterable = self.inferir_tipo(&bucle.iterable, variables);
                    let (tipo_elem, longitud, tamano_elem) = match tipo_iterable {
                        Tipo::Array(ref t, n) => {
                            let tam = self.tamano_tipo(t);
                            ((*t).clone(), n as i64, tam as i64)
                        }
                        _ => {
                            self.errores.agregar(ErrorCompilador::nuevo(
                                CategoriaError::Interno,
                                40,
                                bucle.span.clone(),
                                "'para' requiere arreglo o rango en codegen".to_string(),
                            ));
                            return Err(());
                        }
                    };

                    // Crear slot para variable de iteraci├│n
                    let elem_slot = builder.create_sized_stack_slot(
                        cranelift_codegen::ir::StackSlotData::new(
                            cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                            tamano_elem as u32,
                            0,
                        )
                    );

                    // A├▒adir variables al entorno
                    variables.insert(bucle.variable.clone(), (elem_slot, (*tipo_elem).clone(), crate::ast::Articulo::La));
                    let idx_name = format!("__idx_{}", bucle.variable);
                    variables.insert(idx_name.clone(), (idx_slot, Tipo::Entero32, crate::ast::Articulo::La));

                    // Saltar al header
                    builder.ins().jump(header_block, &[]);

                    // Header: evaluar i < longitud
                    builder.switch_to_block(header_block);
                    let idx_val = builder.ins().stack_load(types::I32, idx_slot, 0);
                    let len_val = builder.ins().iconst(types::I32, longitud);
                    let cond = builder.ins().icmp(
                        cranelift_codegen::ir::condcodes::IntCC::SignedLessThan,
                        idx_val,
                        len_val,
                    );
                    builder.ins().brif(cond, body_block, &[], exit_block, &[]);

                    // Body: cargar elemento, ejecutar bloque, i++, volver
                    builder.switch_to_block(body_block);

                    // Calcular offset = i * tamano_elem
                    let idx_i64 = builder.ins().sextend(types::I64, idx_val);
                    let offset = builder.ins().imul_imm(idx_i64, tamano_elem);
                    let elem_ptr = builder.ins().iadd(array_ptr, offset);

                    // Cargar elemento y guardar en variable de iteraci├│n
                    let cranelift_type = self.tipo_a_cranelift(&tipo_elem);
                    let elem_val = builder.ins().load(
                        cranelift_type,
                        cranelift_codegen::ir::MemFlags::new(),
                        elem_ptr,
                        0,
                    );
                    builder.ins().stack_store(elem_val, elem_slot, 0);

                    // Ejecutar cuerpo
                    let mut body_terminado = false;
                    // R6: snapshot de scope del body (liberar heap por iteración)
                    let m_body = self.marcar_scope();
                    for sentencia in &bucle.bloque.sentencias {
                        if matches!(sentencia, Sentencia::Retornar(_, _)) {
                            body_terminado = true;
                        }
                        self.compilar_sentencia(sentencia, builder, variables, _func_span)?;
                    }

                    if !body_terminado {
                        // El epilogue libera el scope y hace i++ (evitar doble liberación)
                        builder.ins().jump(epilogue_block, &[]);
                    }
                    builder.seal_block(body_block);

                    // Epilogue: punto de reunión — libera scope, i++, vuelve al header
                    // (destino de `continuar`; el body normal también pasa por aquí)
                    builder.switch_to_block(epilogue_block);
                    self.liberar_scope(m_body, builder, variables)?;
                    // i = i + 1
                    let idx_val = builder.ins().stack_load(types::I32, idx_slot, 0);
                    let uno = builder.ins().iconst(types::I32, 1);
                    let nuevo_idx = builder.ins().iadd(idx_val, uno);
                    builder.ins().stack_store(nuevo_idx, idx_slot, 0);

                    builder.ins().jump(header_block, &[]);
                    builder.seal_block(epilogue_block);
                    builder.seal_block(header_block);

                    // Exit: continuar (destino de `romper`)
                    builder.switch_to_block(exit_block);
                    builder.seal_block(exit_block);

                    // Limpiar variables del bucle
                    variables.remove(&bucle.variable);
                    variables.remove(&idx_name);
                    self.pila_bucles.pop();
                }
            }
            Sentencia::Region { nombre: _, cuerpo, span: _ } => {
                // Regi├│n: nuevo scope l├®xico (arena allocation)
                // Guardar variables actuales para restaurar despu├®s
                let variables_antes: Vec<String> = variables.keys().cloned().collect();
                // R6: snapshot de scope — liberar heap declarado dentro de la regi├│n
                let m_region = self.marcar_scope();
                
                // Compilar cuerpo de la regi├│n
                for sentencia in cuerpo {
                    self.compilar_sentencia(sentencia, builder, variables, _func_span)?;
                }
                
                // R6: liberar heap del scope de la regi├│n
                self.liberar_scope(m_region, builder, variables)?;
                
                // Limpiar variables declaradas en la regi├│n (LIFO)
                let variables_despues: Vec<String> = variables.keys().cloned().collect();
                for var in &variables_despues {
                    if !variables_antes.contains(var) {
                        variables.remove(var);
                    }
                }
            }
            Sentencia::Seleccionar(seleccionar) => {
                // Desugar a cadena if/else con canal_intentar
                // seleccionar { c como v => { A }, _ => { D } }
                // ÔåÆ let __sel = canal_intentar(c); si __sel != MIN { v = __sel; A } sino { D }
                let bloque_fin = builder.create_block();
                self.compilar_seleccionar_cadena(
                    &seleccionar.ramas,
                    0,
                    builder,
                    variables,
                    bloque_fin,
                    _func_span,
                )?;
                builder.switch_to_block(bloque_fin);
                builder.seal_block(bloque_fin);
            }
            Sentencia::ConExecutor { hilos, cuerpo, span: _ } => {
                // con_executor(N) { body }
                // 1. Crear pool en heap
                // 2. Spawn N workers (__executor_worker)
                // 3. Compilar body (lanzar encola al pool)
                // 4. Esperar completitud + shutdown
                self.compilar_con_executor(hilos, cuerpo, builder, variables, _func_span)?;
            }
        }
        Ok(())
    }
    pub(crate)     fn compilar_seleccionar_cadena(
        &mut self,
        ramas: &[RamaSeleccionar],
        indice: usize,
        builder: &mut FunctionBuilder,
        variables: &mut HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        bloque_fin: cranelift_codegen::ir::Block,
        _func_span: &Span,
    ) -> Result<(), ()> {
        if indice >= ramas.len() {
            builder.ins().jump(bloque_fin, &[]);
            return Ok(());
        }

        let rama = &ramas[indice];

        if rama.variable.is_none() {
            for sentencia in &rama.cuerpo.sentencias {
                self.compilar_sentencia(sentencia, builder, variables, _func_span)?;
            }
            builder.ins().jump(bloque_fin, &[]);
            return Ok(());
        }

        // Usar builtin_canal_intentar (migrado a runtime) en vez de inline Win32 API
        let expr_canal = Expresion::Identificador(
            if let Expresion::Identificador(n, _) = &rama.canal {
                n.clone()
            } else {
                "_canal_temp".to_string()
            },
            rama.canal.span().clone(),
        );

        let resultado = self.builtin_canal_intentar(builder, variables, &[expr_canal.clone()])?;

        let sentinel = builder.ins().iconst(types::I32, -2147483648i64);
        let hay_dato = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::NotEqual, resultado, sentinel);

        let bloque_hay = builder.create_block();
        let bloque_sig = builder.create_block();

        builder.ins().brif(hay_dato, bloque_hay, &[], bloque_sig, &[]);

        // Hay dato: bindear variable y ejecutar cuerpo
        builder.switch_to_block(bloque_hay);
        builder.seal_block(bloque_hay);

        if let Some(ref var_nombre) = rama.variable {
            let slot = builder.create_sized_stack_slot(cranelift_codegen::ir::StackSlotData::new(
                cranelift_codegen::ir::StackSlotKind::ExplicitSlot, 4, 0,
            ));
            builder.ins().stack_store(resultado, slot, 0);
            variables.insert(var_nombre.clone(), (slot, Tipo::Entero32, crate::ast::Articulo::La));
        }

        for sentencia in &rama.cuerpo.sentencias {
            self.compilar_sentencia(sentencia, builder, variables, _func_span)?;
        }

        if let Some(ref var_nombre) = rama.variable {
            variables.remove(var_nombre);
        }

        builder.ins().jump(bloque_fin, &[]);

        builder.switch_to_block(bloque_sig);
        builder.seal_block(bloque_sig);
        self.compilar_seleccionar_cadena(ramas, indice + 1, builder, variables, bloque_fin, _func_span)?;

        Ok(())
    }

    /// con_executor(N) { body } ÔÇö thread pool con work queue
    /// Pool layout (heap):
    ///   0: HANDLE mutex | 8: HANDLE semaphore | 16: HANDLE done_event
    ///   24: i64* worker_handles | 32: i32 head | 36: i32 tail
    ///   40: i32 count | 44: i32 capacity | 48: i32 shutdown
    pub(crate) fn compilar_con_executor(
        &mut self,
        hilos_expr: &Expresion,
        cuerpo: &[Sentencia],
        builder: &mut FunctionBuilder,
        variables: &mut HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        _func_span: &Span,
    ) -> Result<(), ()> {
        let num_workers = self.compilar_expresion(hilos_expr, builder, variables)?;
        let num_workers_i32 = if builder.func.dfg.value_type(num_workers) == types::I64 {
            builder.ins().ireduce(types::I32, num_workers)
        } else {
            num_workers
        };
        let queue_cap = builder.ins().iconst(types::I32, 256);

        // falcato_executor_new(num_threads: i32, queue_capacity: i32) -> ptr
        let fn_new = self.asegurar_funcion_c("falcato_executor_new", &[types::I32, types::I32], Some(types::I64));
        let ref_new = self.module.declare_func_in_func(fn_new, builder.func);
        let call_new = builder.ins().call(ref_new, &[num_workers_i32, queue_cap]);
        let pool_ptr = builder.inst_results(call_new)[0];

        // Guardar pool_ptr en variable __executor_pool y compilar body
        let slot_pool = builder.create_sized_stack_slot(cranelift_codegen::ir::StackSlotData::new(
            cranelift_codegen::ir::StackSlotKind::ExplicitSlot, 8, 0,
        ));
        builder.ins().stack_store(pool_ptr, slot_pool, 0);
        let pool_var_name = format!("__executor_pool_{}", self.contador_closures);
        self.contador_closures += 1;
        variables.insert(pool_var_name.clone(), (slot_pool, Tipo::Entero64, crate::ast::Articulo::La));
        self.executor_pool_var = Some(pool_var_name.clone());

        for sentencia in cuerpo {
            self.compilar_sentencia(sentencia, builder, variables, _func_span)?;
        }

        self.executor_pool_var = None;
        variables.remove(&pool_var_name);

        // falcato_executor_close(exec) → shutdown + join + cleanup
        let fn_close = self.asegurar_funcion_c("falcato_executor_close", &[types::I64], None);
        let ref_close = self.module.declare_func_in_func(fn_close, builder.func);
        builder.ins().call(ref_close, &[pool_ptr]);

        Ok(())
    }

    pub(crate) fn compilar_lanzar_hilo(
        &mut self,
        llamada: &Llamada,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        // Generar nombre ├║nico para el wrapper
        let nombre_wrapper = format!("__hilo_{}", self.contador_closures);
        self.contador_closures += 1;

        // Evaluar argumentos y guardarlos en un buffer heap (malloc)
        // Layout del buffer: cada arg ocupa 8 bytes (alineado a i64)
        let num_args = llamada.argumentos.len();
        let buffer_size = (num_args * 8) as i64;

        // malloc(buffer_size)
        let malloc_id = self.asegurar_funcion_c("malloc", &[types::I64], Some(types::I64));
        let malloc_ref = self.module.declare_func_in_func(malloc_id, builder.func);
        let size_val = builder.ins().iconst(types::I64, buffer_size.max(8));
        let call_malloc = builder.ins().call(malloc_ref, &[size_val]);
        let buffer_ptr = builder.inst_results(call_malloc)[0];

        // Guardar cada argumento en el buffer (offset = i * 8)
        for (i, arg) in llamada.argumentos.iter().enumerate() {
            let arg_val = self.compilar_expresion(arg, builder, variables)?;
            let offset = (i * 8) as i32;
            // Si el valor es I32, extender a I64 para almacenamiento uniforme
            let arg_i64 = if builder.func.dfg.value_type(arg_val) == types::I32 {
                builder.ins().sextend(types::I64, arg_val)
            } else if builder.func.dfg.value_type(arg_val) == types::I8 {
                builder.ins().sextend(types::I64, arg_val)
            } else {
                arg_val
            };
            builder.ins().store(cranelift_codegen::ir::MemFlags::new(), arg_i64, buffer_ptr, offset);
        }

        // Declarar el wrapper como funci├│n externa (se compilar├í despu├®s)
        let mut sig_wrapper = Signature::new(self.call_conv_default());
        sig_wrapper.params.push(AbiParam::new(types::I64)); // LPVOID lpParameter
        sig_wrapper.returns.push(AbiParam::new(types::I32)); // DWORD retorno

        let wrapper_id = self.module.declare_function(&nombre_wrapper, Linkage::Local, &sig_wrapper)
            .map_err(|_| ())?;
        let wrapper_ref = self.module.declare_func_in_func(wrapper_id, builder.func);

        // Registrar el hilo pendiente para compilaci├│n diferida (con FuncId ya declarada)
        // Guardar tipos Cranelift de cada argumento para desempacar correctamente
        let arg_types: Vec<cranelift_codegen::ir::Type> = llamada.argumentos.iter()
            .map(|arg| {
                let tipo = self.inferir_tipo(arg, variables);
                self.tipo_a_cranelift(&tipo)
            })
            .collect();
        self.hilos_pendientes.push(HiloPendiente {
            nombre: nombre_wrapper.clone(),
            llamada: llamada.clone(),
            func_id_module: wrapper_id,
            arg_types,
        });

        // Obtener puntero a la funci├│n wrapper (func_addr)
        let wrapper_addr = builder.ins().func_addr(types::I64, wrapper_ref);

        // Si hay executor activo, encolar al pool v├¡a runtime
        if let Some(ref pool_var) = self.executor_pool_var {
            if let Some(&(pool_slot, _, _)) = variables.get(pool_var) {
                let pool_ptr = builder.ins().stack_load(types::I64, pool_slot, 0);

                // falcato_executor_submit(exec, task_fn, arg) -> i32
                let submit_fn = self.asegurar_funcion_c(
                    "falcato_executor_submit",
                    &[types::I64, types::I64, types::I64],
                    Some(types::I32),
                );
                let submit_ref = self.module.declare_func_in_func(submit_fn, builder.func);
                builder.ins().call(submit_ref, &[pool_ptr, wrapper_addr, buffer_ptr]);

                return Ok(builder.ins().iconst(types::I64, 0));
            }
        }

        // Fallback: falcato_thread_run(fn_ptr, arg) -> handle
        let thread_run_fn = self.asegurar_funcion_c(
            "falcato_thread_run",
            &[types::I64, types::I64],
            Some(types::I64),
        );
        let thread_run_ref = self.module.declare_func_in_func(thread_run_fn, builder.func);
        builder.ins().call(thread_run_ref, &[wrapper_addr, buffer_ptr]);

        Ok(builder.ins().iconst(types::I64, 0))
    }

    pub(crate) fn compilar_hilos_pendientes(&mut self) {
        let hilos = std::mem::take(&mut self.hilos_pendientes);

        for hilo in hilos {
            // Usar el FuncId ya declarado en compilar_lanzar_hilo
            let func_id = hilo.func_id_module;

            let mut sig = Signature::new(self.call_conv_default());
            sig.params.push(AbiParam::new(types::I64)); // buffer_ptr
            sig.returns.push(AbiParam::new(types::I32)); // DWORD

            let mut ctx = self.module.make_context();
            ctx.func.signature = sig;
            let mut func_ctx = FunctionBuilderContext::new();

            {
                let mut builder = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);
                let entry_block = builder.create_block();
                builder.append_block_params_for_function_params(entry_block);
                builder.switch_to_block(entry_block);
                builder.seal_block(entry_block);

                let buffer_ptr = builder.block_params(entry_block)[0];

                let nombre_target = hilo.llamada.funcion.clone();
                let num_args = hilo.llamada.argumentos.len();

                // Verificar si el target es un futuro (existe __poll_NOMBRE)
                let nombre_poll = format!("__poll_{}", nombre_target);
                let nombre_init = format!("__init_{}", nombre_target);
                let es_futuro = self.funciones.contains_key(&nombre_poll) && self.funciones.contains_key(&nombre_init);

                if es_futuro {
                    // Futuro: __init(args) + poll loop + free
                    let init_id = *self.funciones.get(&nombre_init).unwrap();
                    let poll_id = *self.funciones.get(&nombre_poll).unwrap();
                    let init_ref = self.module.declare_func_in_func(init_id, builder.func);
                    let poll_ref = self.module.declare_func_in_func(poll_id, builder.func);

                    // Desempacar args del buffer y llamar __init(args...)
                    let mut args = Vec::new();
                    for i in 0..num_args {
                        let offset = (i * 8) as i32;
                        let arg_i64 = builder.ins().load(types::I64, cranelift_codegen::ir::MemFlags::new(), buffer_ptr, offset);
                        let target_type = hilo.arg_types.get(i).copied().unwrap_or(types::I32);
                        let arg_val = if target_type == types::I64 {
                            arg_i64
                        } else if target_type == types::I32 {
                            builder.ins().ireduce(types::I32, arg_i64)
                        } else if target_type == types::I8 {
                            builder.ins().ireduce(types::I8, arg_i64)
                        } else {
                            arg_i64
                        };
                        args.push(arg_val);
                    }

                    let init_call = builder.ins().call(init_ref, &args);
                    let fut_ptr = builder.inst_results(init_call)[0];

                    // Poll loop: while __poll(fut_ptr) == 0 { Sleep(1); }
                    let sleep_id = self.asegurar_funcion_c("Sleep", &[types::I32], None);
                    let sleep_ref = self.module.declare_func_in_func(sleep_id, builder.func);

                    let bloque_check = builder.create_block();
                    let bloque_sleep = builder.create_block();
                    let bloque_done = builder.create_block();

                    builder.ins().jump(bloque_check, &[]);

                    // Check: poll(fut_ptr) == 0?
                    builder.switch_to_block(bloque_check);
                    let poll_call = builder.ins().call(poll_ref, &[fut_ptr]);
                    let poll_result = builder.inst_results(poll_call)[0];
                    let cero64 = builder.ins().iconst(types::I64, 0);
                    let es_pending = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::Equal, poll_result, cero64);
                    builder.ins().brif(es_pending, bloque_sleep, &[], bloque_done, &[]);

                    // Sleep(1) + jump back
                    builder.switch_to_block(bloque_sleep);
                    let uno32 = builder.ins().iconst(types::I32, 1);
                    builder.ins().call(sleep_ref, &[uno32]);
                    builder.ins().jump(bloque_check, &[]);
                    builder.seal_block(bloque_sleep);

                    // Done: sellar check (2 predecesores: entry + sleep)
                    builder.seal_block(bloque_check);
                    builder.switch_to_block(bloque_done);
                    builder.seal_block(bloque_done);

                    // free(fut_ptr)
                    let free_id = self.asegurar_funcion_c("free", &[types::I64], None);
                    let free_ref = self.module.declare_func_in_func(free_id, builder.func);
                    builder.ins().call(free_ref, &[fut_ptr]);
                } else if let Some(target_id) = self.funciones.get(&nombre_target).copied() {
                    // Funci├│n normal: llamada directa
                    let target_ref = self.module.declare_func_in_func(target_id, builder.func);

                    let mut args = Vec::new();
                    for i in 0..num_args {
                        let offset = (i * 8) as i32;
                        let arg_i64 = builder.ins().load(types::I64, cranelift_codegen::ir::MemFlags::new(), buffer_ptr, offset);
                        let target_type = hilo.arg_types.get(i).copied().unwrap_or(types::I32);
                        let arg_val = if target_type == types::I64 {
                            arg_i64
                        } else if target_type == types::I32 {
                            builder.ins().ireduce(types::I32, arg_i64)
                        } else if target_type == types::I8 {
                            builder.ins().ireduce(types::I8, arg_i64)
                        } else {
                            arg_i64
                        };
                        args.push(arg_val);
                    }

                    builder.ins().call(target_ref, &args);
                }

                // free(buffer_ptr) ÔÇö liberar el buffer de argumentos
                let free_id = self.asegurar_funcion_c("free", &[types::I64], None);
                let free_ref = self.module.declare_func_in_func(free_id, builder.func);
                builder.ins().call(free_ref, &[buffer_ptr]);

                // return 0
                let cero = builder.ins().iconst(types::I32, 0);
                builder.ins().return_(&[cero]);
            }

            if let Err(_) = self.definir_funcion(func_id, &mut ctx, &hilo.nombre) {
                // Error silencioso en MVP
            }
        }
    }

}
