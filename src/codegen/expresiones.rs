use super::*;
impl Codegen {
    pub(crate) fn compilar_expresion(
        &mut self,
        expr: &Expresion,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        match expr {
            Expresion::LiteralArray(elementos, _span) => {
                // Arrays literales: creamos stack slot grande y llenamos
                if elementos.is_empty() {
                    return Ok(builder.ins().iconst(types::I64, 0)); // null pointer
                }
                
                let tipo_elem = self.inferir_tipo(&elementos[0], variables);
                let tamano_elem = self.tamano_tipo(&tipo_elem) as i64;
                let longitud = elementos.len() as i64;
                let tamano_total = (tamano_elem * longitud) as u32;
                
                let slot = builder.create_sized_stack_slot(
                    cranelift_codegen::ir::StackSlotData::new(
                        cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                        tamano_total,
                        0,
                    )
                );
                
                // Obtener direcci├│n base del array
                let base_ptr = builder.ins().stack_addr(types::I64, slot, 0);
                
                // Almacenar cada elemento
                for (i, elem) in elementos.iter().enumerate() {
                    let val = self.compilar_expresion(elem, builder, variables)?;
                    let offset = (i as i64 * tamano_elem) as i32;
                    let elem_ptr = builder.ins().iadd_imm(base_ptr, offset as i64);
                    builder.ins().store(
                        cranelift_codegen::ir::MemFlags::new(),
                        val,
                        elem_ptr,
                        0
                    );
                }
                
                Ok(base_ptr)
            }
            Expresion::AccesoArray(array, indice, span) => {
                let tipo_array = self.inferir_tipo(array, variables);
                
                // Texto[i] ÔåÆ builtin texto_obtener_byte
                // Texto[inicio..fin] ÔåÆ builtin texto_subtexto
                if tipo_array == Tipo::Texto {
                    // Verificar si el ├¡ndice es un rango (slicing)
                    if let Expresion::Rango(inicio, fin, _inclusivo, _) = indice.as_ref() {
                        let llamada = Llamada {
                            funcion: "texto_subtexto".to_string(),
                            tipo_args: vec![],
                            argumentos: vec![
                                array.as_ref().clone(),
                                *inicio.clone(),
                                *fin.clone(),
                            ],
                            span: span.clone(),
                        };
                        return self.compilar_llamada(&llamada, builder, variables);
                    } else {
                        let llamada = Llamada {
                            funcion: "texto_obtener_byte".to_string(),
                            tipo_args: vec![],
                            argumentos: vec![
                                array.as_ref().clone(),
                                indice.as_ref().clone(),
                            ],
                            span: span.clone(),
                        };
                        return self.compilar_llamada(&llamada, builder, variables);
                    }
                }
                
                // Vector<T>[i] ÔåÆ builtin vector_obtener
                if let Tipo::Vector(_) = &tipo_array {
                    let llamada = Llamada {
                        funcion: "vector_obtener".to_string(),
                        tipo_args: vec![],
                        argumentos: vec![
                            array.as_ref().clone(),
                            indice.as_ref().clone(),
                        ],
                        span: span.clone(),
                    };
                    return self.compilar_llamada(&llamada, builder, variables);
                }
                
                let array_val = self.compilar_expresion(array, builder, variables)?;
                let idx_val = self.compilar_expresion(indice, builder, variables)?;
                
                let (tipo_elem, tamano_elem) = match tipo_array {
                    Tipo::Array(ref t, _) => {
                        let tam = self.tamano_tipo(t);
                        (t.clone(), tam as i64)
                    }
                    _ => {
                        self.errores.agregar(ErrorCompilador::nuevo(
                            CategoriaError::Interno,
                            20,
                            span.clone(),
                            "Acceso a arreglo en tipo no-arreglo".to_string(),
                        ));
                        return Err(());
                    }
                };
                
                // Asegurar que ├¡ndice sea I64 para aritm├®tica de punteros
                let idx_i64 = if builder.func.dfg.value_type(idx_val) == types::I32 {
                    builder.ins().sextend(types::I64, idx_val)
                } else {
                    idx_val
                };
                
                // Calcular offset = ├¡ndice * tama├▒o_elemento
                let offset = builder.ins().imul_imm(idx_i64, tamano_elem);
                
                // Calcular direcci├│n = array_ptr + offset
                let elem_ptr = builder.ins().iadd(array_val, offset);
                
                // Cargar elemento
                let cranelift_type = self.tipo_a_cranelift(&tipo_elem);
                let val = builder.ins().load(
                    cranelift_type,
                    cranelift_codegen::ir::MemFlags::new(),
                    elem_ptr,
                    0
                );
                Ok(val)
            }
            Expresion::Literal(lit) => {
                match lit {
                    Literal::Entero(n, _span) => {
                        Ok(builder.ins().iconst(types::I32, *n as i64))
                    }
                    Literal::Palabra(s, _span) => {
                        // Internado (R7.6): mismo contenido → mismo global → mismo
                        // puntero. Requisito para Diccionario<Palabra, V> (comparación
                        // de claves por puntero) y reduce el tamaño del binario.
                        if let Some(data_id) = self.strings_internados.get(s) {
                            let global = self.module.declare_data_in_func(*data_id, builder.func);
                            return Ok(builder.ins().global_value(types::I64, global));
                        }
                        self.contador_strings += 1;
                        let data_id = self.module.declare_data(
                            &format!("str_{}_{}", self.contador_strings, s.len()),
                            Linkage::Local,
                            false,
                            false,
                        ).map_err(|_| ())?;
                        
                        // Escribir datos incluyendo terminador nulo para compatibilidad C
                        let mut bytes = s.as_bytes().to_vec();
                        bytes.push(0);
                        let mut desc = cranelift_module::DataDescription::new();
                        desc.define(bytes.into_boxed_slice());
                        self.module.define_data(data_id, &desc)
                            .map_err(|_| ())?;
                        self.strings_internados.insert(s.to_string(), data_id);
                        
                        // Crear puntero al string
                        let global = self.module.declare_data_in_func(data_id, builder.func);
                        let ptr = builder.ins().global_value(types::I64, global);
                        Ok(ptr)
                    }
                    Literal::Flotante(n, _span) => {
                        Ok(builder.ins().f64const(*n))
                    }
                    Literal::Booleano(v, _span) => {
                        let val = if *v { 1i64 } else { 0i64 };
                        Ok(builder.ins().iconst(types::I8, val))
                    }
                    _ => {
                        self.errores.agregar(ErrorCompilador::nuevo(
                            CategoriaError::Interno,
                            5,
                            lit.span().clone(),
                            "Literal no soportado".to_string(),
                        ));
                        Err(())
                    }
                }
            }
            Expresion::Identificador(nombre, span) => {
                let (slot, tipo, _articulo) = match variables.get(nombre) {
                    Some(v) => v.clone(),
                    None => {
                        self.errores.agregar(ErrorCompilador::nuevo(
                            CategoriaError::Interno,
                            6,
                            span.clone(),
                            format!("Variable '{}' no encontrada", nombre),
                        ));
                        return Err(());
                    }
                };
                
                // Si es array, struct o enum con datos, devolvemos puntero (direcci├│n base)
                // Resolver apodos primero (ej: apodo ID = Entero64 → Entero64)
                let tipo_resuelto = self.resolver_alias(&tipo);
                let val = if matches!(tipo_resuelto, Tipo::Array(_, _) | Tipo::Nombre(_) | Tipo::Resultado(_, _) | Tipo::Option(_)) {
                    builder.ins().stack_addr(types::I64, slot, 0)
                } else {
                    builder.ins().stack_load(
                        self.tipo_a_cranelift(&tipo_resuelto),
                        slot,
                        0,
                    )
                };
                Ok(val)
            }
            Expresion::Binaria(izq, op, der, span) => {
                // Texto + Texto ÔåÆ concatenaci├│n via builtin
                if *op == OperadorBinario::Suma {
                    let tipo_izq = self.inferir_tipo(izq, variables);
                    if tipo_izq == Tipo::Texto {
                        let llamada = Llamada {
                            funcion: "texto_concatenar".to_string(),
                            tipo_args: vec![],
                            argumentos: vec![izq.as_ref().clone(), der.as_ref().clone()],
                            span: span.clone(),
                        };
                        return self.compilar_llamada(&llamada, builder, variables);
                    }
                }
                // Adaptar literales numéricos al tipo del otro operando
                // (ej: x: Entero64 + 1 → el 1 se emite como I64)
                let tipo_izq = self.inferir_tipo(izq, variables);
                let tipo_der = self.inferir_tipo(der, variables);
                let val_izq = self.compilar_lado_binaria(izq, &tipo_der, builder, variables)?;
                let val_der = self.compilar_lado_binaria(der, &tipo_izq, builder, variables)?;
                self.compilar_operacion_binaria(*op, val_izq, val_der, builder)
            }
            // R7.7 F1 — Subjuntivo aritmético: `a + b fuese` → checked (Resultado<T, Entero32>)
            // Empaqueta tag+data en I64 (layout enum pequeño): tag low 32, data high 32.
            // Exito(tag 0, data = resultado) | Error(tag 1, data = 1 = desbordamiento)
            Expresion::Checked(inner, span) => {
                let (izq, op, der, span_bin) = match inner.as_ref() {
                    Expresion::Binaria(i, o, d, s) => (i, o, d, s),
                    _ => {
                        self.errores.agregar(ErrorCompilador::nuevo(
                            CategoriaError::Interno,
                            1,
                            span.clone(),
                            "Checked sin operación binaria interna".to_string(),
                        ));
                        return Err(());
                    }
                };
                let tipo_izq = self.inferir_tipo(izq, variables);
                let tipo_der = self.inferir_tipo(der, variables);
                let val_izq = self.compilar_lado_binaria(izq, &tipo_der, builder, variables)?;
                let val_der = self.compilar_lado_binaria(der, &tipo_izq, builder, variables)?;
                // El resultado wrap (módulo 2ⁿ) — igual que sin `fuese`
                let val_r = self.compilar_operacion_binaria(*op, val_izq, val_der, builder)?;
                let tipo_val = builder.func.dfg.value_type(val_izq);
                let bits = tipo_val.bits() as i64;
                // ¿Signed (Entero*) o unsigned (Natural*)?
                let es_signed = matches!(
                    self.resolver_alias(&tipo_izq),
                    Tipo::Entero8 | Tipo::Entero16 | Tipo::Entero32
                );
                use cranelift_codegen::ir::condcodes::IntCC;
                // Detectar desbordamiento:
                //   Suma signed:   ((a ^ r) & (b ^ r)) < 0   → signos de a y b iguales, r distinto
                //   Resta signed:  ((a ^ b) & (a ^ r)) < 0   → signos de a y b distintos, r distinto de a
                //   Mul signed:    smulhi(a,b) != sext(r)    → mitad alta no es extensión de signo
                //   Suma unsigned: r < a                     → carry
                //   Resta unsigned: r > a                    → borrow
                //   Mul unsigned:  umulhi(a,b) != 0          → bits altos no vacíos
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
                            CategoriaError::Interno,
                            1,
                            span_bin.clone(),
                            format!("Checked con operación no aritmética: {:?}", op),
                        ));
                        return Err(());
                    }
                };
                // Empaquetar: Exito(tag 0) | Error(tag 1, data = 1)
                let r_i64 = builder.ins().uextend(types::I64, val_r);
                let shift32 = builder.ins().iconst(types::I64, 32);
                let exito_packed = builder.ins().ishl(r_i64, shift32); // tag 0 implícito
                let uno = builder.ins().iconst(types::I64, 1);
                let error_data = builder.ins().ishl(uno, shift32); // 1 << 32
                let error_packed = builder.ins().bor(error_data, uno); // | tag 1
                Ok(builder.ins().select(overflow, error_packed, exito_packed))
            }
            Expresion::Unaria(op, expr, span) => {
                // Manejar referencias de forma especial (necesitan puntero al stack slot)
                match op {
                    OperadorUnario::Referencia | OperadorUnario::ReferenciaMut => {
                        // &x o &mut x: obtener puntero al stack slot
                        if let Expresion::Identificador(nombre, _) = expr.as_ref() {
                            if let Some((slot, _tipo, _articulo)) = variables.get(nombre) {
                                let ptr = builder.ins().stack_addr(types::I64, *slot, 0);
                                return Ok(ptr);
                            }
                        }
                        // &punto.x o &mut punto.x: obtener puntero al campo
                        if let Expresion::AccesoCampo(base, campo, _) = expr.as_ref() {
                            if let Expresion::Identificador(nombre, _) = base.as_ref() {
                                if let Some((slot, tipo, _articulo)) = variables.get(nombre) {
                                    // Obtener el offset del campo
                                    if let Tipo::Nombre(nombre_struct) = tipo {
                                        if let Some(layout) = self.structs.get(nombre_struct) {
                                            if let Some(offset) = layout.offsets.get(campo) {
                                                let base_ptr = builder.ins().stack_addr(types::I64, *slot, 0);
                                                let campo_ptr = builder.ins().iadd_imm(base_ptr, *offset as i64);
                                                return Ok(campo_ptr);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        // Si no es un identificador o acceso a campo, compilar la expresi├│n y retornar su direcci├│n
                        // (esto es una simplificaci├│n, no funciona para expresiones complejas)
                        let val = self.compilar_expresion(expr, builder, variables)?;
                        Ok(val)
                    }
                    OperadorUnario::Desreferencia => {
                        // *expr: cargar valor desde puntero
                        let ptr = self.compilar_expresion(expr, builder, variables)?;
                        // Asumimos I32 por ahora; en v2 inferir tipo desde el contexto
                        let val = builder.ins().load(
                            types::I32,
                            cranelift_codegen::ir::MemFlags::new(),
                            ptr,
                            0,
                        );
                        Ok(val)
                    }
                    _ => {
                        let val = self.compilar_expresion(expr, builder, variables)?;
                        self.compilar_operacion_unaria(*op, val, builder, span)
                    }
                }
            }
            Expresion::Llamada(llamada) => {
                self.compilar_llamada(llamada, builder, variables)
            }
            Expresion::InicializacionStruct(nombre, campos, span) => {
                let layout = match self.structs.get(nombre) {
                    Some(l) => l.clone(),
                    None => {
                        self.errores.agregar(ErrorCompilador::nuevo(
                            CategoriaError::Interno,
                            30,
                            span.clone(),
                            format!("Struct '{}' no registrado en codegen", nombre),
                        ));
                        return Err(());
                    }
                };

                // Crear stack slot para el struct
                let slot = builder.create_sized_stack_slot(
                    cranelift_codegen::ir::StackSlotData::new(
                        cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                        layout.tamano,
                        0,
                    )
                );

                let base_ptr = builder.ins().stack_addr(types::I64, slot, 0);

                // Fase 15B: bitfield struct ÔÇö inicializar entero de respaldo
                if layout.es_bitfield {
                    let backing_type = match layout.tamano {
                        1 => types::I8,
                        2 => types::I16,
                        4 => types::I32,
                        _ => types::I64,
                    };
                    // Inicializar a 0
                    let cero = builder.ins().iconst(backing_type, 0);
                    builder.ins().store(cranelift_codegen::ir::MemFlags::new(), cero, base_ptr, 0);

                    // Escribir cada campo con shift+mask
                    for (nombre_campo, valor_expr) in campos {
                        if let Some(&(bf_offset, bf_ancho)) = layout.bitfields.get(nombre_campo) {
                            let val = self.compilar_expresion(valor_expr, builder, variables)?;
                            // Cargar entero actual
                            let raw = builder.ins().load(backing_type, cranelift_codegen::ir::MemFlags::new(), base_ptr, 0);
                            let cur_i32 = if backing_type != types::I32 {
                                builder.ins().uextend(types::I32, raw)
                            } else {
                                raw
                            };
                            // mask = (1 << ancho) - 1
                            let uno = builder.ins().iconst(types::I32, 1);
                            let ancho_val = builder.ins().iconst(types::I32, bf_ancho as i64);
                            let field_mask = builder.ins().ishl(uno, ancho_val);
                            let menos_uno = builder.ins().iconst(types::I32, -1);
                            let field_mask = builder.ins().iadd(field_mask, menos_uno);
                            let offset_val = builder.ins().iconst(types::I32, bf_offset as i64);
                            let shifted_mask = builder.ins().ishl(field_mask, offset_val);
                            // Limpiar + insertar
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
                    return Ok(base_ptr);
                }

                // Struct normal: almacenar cada campo
                for (nombre_campo, valor) in campos {
                    let val = self.compilar_expresion(valor, builder, variables)?;
                    let offset = match layout.offsets.get(nombre_campo) {
                        Some(o) => *o as i64,
                        None => {
                            self.errores.agregar(ErrorCompilador::nuevo(
                                CategoriaError::Interno,
                                31,
                                span.clone(),
                                format!("Campo '{}' no encontrado en layout de '{}'", nombre_campo, nombre),
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

                Ok(base_ptr)
            }
            Expresion::AccesoCampo(expr, nombre_campo, span) => {
                let struct_ptr = self.compilar_expresion(expr, builder, variables)?;
                let tipo_expr = self.inferir_tipo(expr, variables);

                let nombre_struct = match tipo_expr {
                    Tipo::Nombre(n) => n,
                    _ => {
                        self.errores.agregar(ErrorCompilador::nuevo(
                            CategoriaError::Interno,
                            32,
                            span.clone(),
                            format!("Acceso a campo en tipo no-struct '{:?}'", tipo_expr),
                        ));
                        return Err(());
                    }
                };

                let layout = match self.structs.get(&nombre_struct) {
                    Some(l) => l.clone(),
                    None => {
                        self.errores.agregar(ErrorCompilador::nuevo(
                            CategoriaError::Interno,
                            30,
                            span.clone(),
                            format!("Struct '{}' no registrado en codegen", nombre_struct),
                        ));
                        return Err(());
                    }
                };

                // Fase 15B: bitfield read ÔåÆ (val >> offset) & mask
                if layout.es_bitfield {
                    if let Some(&(bf_offset, bf_ancho)) = layout.bitfields.get(nombre_campo) {
                        // Cargar el entero de respaldo
                        let backing_type = match layout.tamano {
                            1 => types::I8,
                            2 => types::I16,
                            4 => types::I32,
                            _ => types::I64,
                        };
                        let raw_val = builder.ins().load(
                            backing_type,
                            cranelift_codegen::ir::MemFlags::new(),
                            struct_ptr,
                            0,
                        );
                        // Extender a I32 para operaciones
                        let val_i32 = if backing_type != types::I32 {
                            builder.ins().uextend(types::I32, raw_val)
                        } else {
                            raw_val
                        };
                        // (val >> offset) & ((1 << ancho) - 1)
                        let offset_val = builder.ins().iconst(types::I32, bf_offset as i64);
                        let shifted = builder.ins().ushr(val_i32, offset_val);
                        let uno = builder.ins().iconst(types::I32, 1);
                        let ancho_val = builder.ins().iconst(types::I32, bf_ancho as i64);
                        let mask = builder.ins().ishl(uno, ancho_val);
                        let menos_uno = builder.ins().iconst(types::I32, -1);
                        let mask_final = builder.ins().iadd(mask, menos_uno);
                        let resultado = builder.ins().band(shifted, mask_final);
                        return Ok(resultado);
                    }
                }

                let offset = match layout.offsets.get(nombre_campo) {
                    Some(o) => *o as i64,
                    None => {
                        self.errores.agregar(ErrorCompilador::nuevo(
                            CategoriaError::Interno,
                            31,
                            span.clone(),
                            format!("Campo '{}' no encontrado en layout de '{}'", nombre_campo, nombre_struct),
                        ));
                        return Err(());
                    }
                };

                let campo_ptr = builder.ins().iadd_imm(struct_ptr, offset);

                // Inferir tipo del campo para saber c├│mo cargar
                let tipo_campo = self.buscar_tipo_campo(&nombre_struct, nombre_campo);
                let cranelift_type = self.tipo_a_cranelift(&tipo_campo);
                let val = builder.ins().load(
                    cranelift_type,
                    cranelift_codegen::ir::MemFlags::new(),
                    campo_ptr,
                    0,
                );
                Ok(val)
            }
            Expresion::ArrayRelleno(_, _, span) => {
                self.errores.agregar(ErrorCompilador::nuevo(
                    CategoriaError::Interno,
                    22,
                    span.clone(),
                    "'todos' solo puede usarse en inicializaci├│n de variable con tipo expl├¡cito".to_string(),
                ));
                Err(())
            }
            Expresion::ConstructorEnum(enum_nombre, variante_nombre, argumentos, span) => {
                let layout = match self.enums.get(enum_nombre) {
                    Some(l) => l.clone(),
                    None => {
                        self.errores.agregar(ErrorCompilador::nuevo(
                            CategoriaError::Interno,
                            50,
                            span.clone(),
                            format!("Enum '{}' no registrado en codegen", enum_nombre),
                        ));
                        return Err(());
                    }
                };

                // Para enums peque├▒os (Ôëñ 8 bytes): empaquetar tag+data en I64
                // Layout little-endian: bytes 0-3 = tag (low 32), bytes 4-7 = data (high 32)
                // Esto coincide con el layout de struct (tag en offset 0, data en offset 4)
                // As├¡ EsVariante, Propagacion e Identificador funcionan sin cambios
                if layout.tamano <= 8 {
                    let tag = *layout.variantes.get(variante_nombre).unwrap_or(&0);
                    let tag_iconst = builder.ins().iconst(types::I32, tag as i64);
                    let tag_ext = builder.ins().uextend(types::I64, tag_iconst);
                    
                    if !argumentos.is_empty() {
                        let data_val = self.compilar_expresion(&argumentos[0], builder, variables)?;
                        let data_i64 = builder.ins().uextend(types::I64, data_val);
                        // Shift data to occupy high bytes: data << (datos_offset * 8)
                        let shift_bits = (layout.datos_offset * 8) as i64;
                        if shift_bits > 0 {
                            let shift_val = builder.ins().iconst(types::I64, shift_bits);
                            let data_shifted = builder.ins().ishl(data_i64, shift_val);
                            let packed = builder.ins().bor(tag_ext, data_shifted);
                            Ok(packed)
                        } else {
                            Ok(builder.ins().bor(tag_ext, data_i64))
                        }
                    } else {
                        Ok(tag_ext)
                    }
                } else {
                    // Para enums grandes: mantener stack slot + puntero
                    let slot = builder.create_sized_stack_slot(
                        cranelift_codegen::ir::StackSlotData::new(
                            cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                            layout.tamano,
                            0,
                        )
                    );

                    let base_ptr = builder.ins().stack_addr(types::I64, slot, 0);

                    let tag = *layout.variantes.get(variante_nombre).unwrap_or(&0);
                    let tag_val = builder.ins().iconst(types::I32, tag as i64);
                    builder.ins().store(
                        cranelift_codegen::ir::MemFlags::new(),
                        tag_val,
                        base_ptr,
                        0,
                    );

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

                    Ok(base_ptr)
                }
            }
            Expresion::EsVariante(expr, enum_nombre, variante_nombre, _binding, span) => {
                let layout = match self.enums.get(enum_nombre) {
                    Some(l) => l.clone(),
                    None => {
                        self.errores.agregar(ErrorCompilador::nuevo(
                            CategoriaError::Interno,
                            50,
                            span.clone(),
                            format!("Enum '{}' no registrado en codegen", enum_nombre),
                        ));
                        return Err(());
                    }
                };

                let enum_ptr = self.compilar_expresion(expr, builder, variables)?;
                
                // Cargar tag (I32 en offset 0)
                let tag_val = builder.ins().load(
                    types::I32,
                    cranelift_codegen::ir::MemFlags::new(),
                    enum_ptr,
                    0,
                );

                let tag_esperado = *layout.variantes.get(variante_nombre).unwrap_or(&0);
                let esperado_val = builder.ins().iconst(types::I32, tag_esperado as i64);
                
                let resultado = builder.ins().icmp(
                    cranelift_codegen::ir::condcodes::IntCC::Equal,
                    tag_val,
                    esperado_val,
                );

                Ok(resultado)
            }
            Expresion::Ruta(path, _span) => {
                // Ruta cualificada sin llamada (ej: pasar funci├│n como valor)
                // Por ahora: error, ya que no soportamos funciones como valores
                eprintln!("[Falcato] Error: ruta '{}' no es una expresi├│n v├ílida sin llamada",
                    path.join("::"));
                Err(())
            }
            Expresion::Propagacion(expr, _span) => {
                // Operador ?: propaga errores
                // Si la expresi├│n es Resultado.Error, retorna inmediatamente
                // Si es Resultado.Exito, extrae el valor
                
                // Por ahora: implementaci├│n simplificada
                // Extrae el valor del campo de datos (asumiendo Exito)
                // TODO: Implementar early return real con CFG restructuring
                
                let enum_ptr = self.compilar_expresion(expr, builder, variables)?;
                
                // Cargar el valor del campo de datos (offset 4, despu├®s del tag)
                let datos_ptr = builder.ins().iadd_imm(enum_ptr, 4);
                let valor = builder.ins().load(
                    types::I32,
                    cranelift_codegen::ir::MemFlags::new(),
                    datos_ptr,
                    0,
                );
                
                Ok(valor)
            }
            Expresion::Mover(nombre, _destino, span) => {
                // TODO: Implementar transferencia de ownership
                // Por ahora: compilar como identificador (sin verificaci├│n)
                let (slot, tipo, _articulo) = match variables.get(nombre) {
                    Some(v) => v.clone(),
                    None => {
                        self.errores.agregar(ErrorCompilador::nuevo(
                            CategoriaError::Interno,
                            52,
                            span.clone(),
                            format!("Variable '{}' no encontrada en 'mover'", nombre),
                        ));
                        return Err(());
                    }
                };
                let val = if matches!(tipo, Tipo::Array(_, _) | Tipo::Nombre(_) | Tipo::Resultado(_, _)) {
                    builder.ins().stack_addr(types::I64, slot, 0)
                } else {
                    builder.ins().stack_load(self.tipo_a_cranelift(&tipo), slot, 0)
                };
                Ok(val)
            }
            Expresion::Copiar(expr, _span) => {
                // TODO: Implementar clonaci├│n profunda
                // Por ahora: compilar la expresi├│n interna (copia superficial)
                self.compilar_expresion(expr, builder, variables)
            }
            Expresion::Rango(_, _, _, span) => {
                // Los rangos solo son v├ílidos dentro de 'para'
                self.errores.agregar(ErrorCompilador::nuevo(
                    CategoriaError::Tipo,
                    41,
                    span.clone(),
                    "Los rangos (..) solo pueden usarse dentro de un bucle 'para'".to_string(),
                ));
                Err(())
            }
            Expresion::Closure(params, cuerpo, _span) => {
                // Generar nombre ├║nico para la funci├│n an├│nima
                self.contador_closures += 1;
                let nombre_closure = format!("__closure_{}", self.contador_closures);

                // Inferir tipos de par├ímetros (default Entero32 si no se especifica)
                let params_tipos: Vec<(String, Tipo)> = params.iter().map(|(n, t)| {
                    (n.clone(), t.clone().unwrap_or(Tipo::Entero32))
                }).collect();

                // Detectar capturas: variables usadas en el cuerpo que no son params
                let mut capturas: Vec<(String, Tipo)> = Vec::new();
                for (nombre_var, (_, tipo_var, _)) in variables.iter() {
                    if !params_tipos.iter().any(|(pn, _)| pn == nombre_var) {
                        if self.expresion_usa_variable(cuerpo, nombre_var) {
                            capturas.push((nombre_var.clone(), tipo_var.clone()));
                        }
                    }
                }

                // Firma: SIEMPRE env_ptr como primer par├ímetro (simplifica llamadas)
                let mut sig = Signature::new(self.call_conv_default());
                sig.params.push(AbiParam::new(types::I64)); // env_ptr (0 si no hay capturas)
                for (_, tipo) in &params_tipos {
                    sig.params.push(AbiParam::new(self.tipo_a_cranelift(tipo)));
                }
                let tipo_retorno = self.inferir_tipo(cuerpo, variables);
                sig.returns.push(AbiParam::new(self.tipo_a_cranelift(&tipo_retorno)));

                // Declarar la funci├│n closure en el m├│dulo
                let func_id = self.module.declare_function(
                    &nombre_closure,
                    Linkage::Local,
                    &sig,
                ).map_err(|_| ())?;

                self.funciones.insert(nombre_closure.clone(), func_id);

                // Guardar para compilaci├│n diferida
                self.closures_pendientes.push(ClosurePendiente {
                    nombre: nombre_closure.clone(),
                    params: params_tipos,
                    cuerpo: *cuerpo.clone(),
                    capturas: capturas.clone(),
                    retorno: tipo_retorno,
                });

                // Obtener function pointer
                let func_ref = self.module.declare_func_in_func(func_id, builder.func);
                let fn_ptr = builder.ins().func_addr(types::I64, func_ref);

                // Crear closure object: 16 bytes {fn_ptr: I64, env_ptr: I64}
                let closure_slot = builder.create_sized_stack_slot(
                    cranelift_codegen::ir::StackSlotData::new(
                        cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                        16, // fn_ptr (8) + env_ptr (8)
                        0,
                    )
                );
                let closure_base = builder.ins().stack_addr(types::I64, closure_slot, 0);

                // Guardar fn_ptr en offset 0
                builder.ins().store(cranelift_codegen::ir::MemFlags::new(), fn_ptr, closure_base, 0);

                // Crear env struct si hay capturas
                if !capturas.is_empty() {
                    // Env struct: array de punteros a las variables capturadas (8 bytes cada uno)
                    let env_size = (capturas.len() * 8) as u32;
                    let env_slot = builder.create_sized_stack_slot(
                        cranelift_codegen::ir::StackSlotData::new(
                            cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                            env_size,
                            0,
                        )
                    );
                    let env_base = builder.ins().stack_addr(types::I64, env_slot, 0);

                    // Guardar punteros a cada variable capturada
                    for (i, (nombre_cap, _)) in capturas.iter().enumerate() {
                        if let Some((cap_slot, _, _)) = variables.get(nombre_cap) {
                            let cap_addr = builder.ins().stack_addr(types::I64, *cap_slot, 0);
                            let offset = (i * 8) as i32;
                            builder.ins().store(cranelift_codegen::ir::MemFlags::new(), cap_addr, env_base, offset);
                        }
                    }

                    // Guardar env_ptr en offset 8 del closure object
                    builder.ins().store(cranelift_codegen::ir::MemFlags::new(), env_base, closure_base, 8);
                } else {
                    // Sin capturas: env_ptr = 0
                    let cero = builder.ins().iconst(types::I64, 0);
                    builder.ins().store(cranelift_codegen::ir::MemFlags::new(), cero, closure_base, 8);
                }

                // Retornar puntero al closure object
                Ok(closure_base)
            }
            Expresion::Coincidir(sujeto, brazos, _span) => {
                // Compilar sujeto
                let val_sujeto = self.compilar_expresion(sujeto, builder, variables)?;
                let tipo_sujeto = self.inferir_tipo(sujeto, variables);
                let cranelift_tipo = self.tipo_a_cranelift(&tipo_sujeto);

                // Slot para el resultado del match
                let _resultado_slot = builder.create_sized_stack_slot(
                    cranelift_codegen::ir::StackSlotData::new(
                        cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                        cranelift_tipo.bytes(),
                        0,
                    )
                );

                let bloque_fin = builder.create_block();
                builder.append_block_param(bloque_fin, cranelift_tipo);

                for brazo in brazos {
                    let bloque_brazo = builder.create_block();
                    let bloque_siguiente = builder.create_block();

                    match &brazo.patron {
                        crate::ast::PatronMatch::Comodin(_) => {
                            // Wildcard: siempre matchea, saltar directo al brazo
                            builder.ins().jump(bloque_brazo, &[]);
                        }
                        crate::ast::PatronMatch::Literal(lit) => {
                            // Comparar sujeto con literal
                            let val_lit = self.compilar_literal(lit, builder)?;
                            let cmp = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::Equal, val_sujeto, val_lit);
                            builder.ins().brif(cmp, bloque_brazo, &[], bloque_siguiente, &[]);
                        }
                        crate::ast::PatronMatch::VarianteEnum(enum_nombre, variante, binding, _span_pat) => {
                            // Para enums: comparar tag (primer campo I32)
                            // El sujeto es un puntero al struct del enum
                            let tag_offset = 0i32;
                            let tag_val = builder.ins().load(types::I32, cranelift_codegen::ir::MemFlags::new(), val_sujeto, tag_offset);

                            // Obtener ├¡ndice de la variante
                            let tag_idx = self.indice_variante_enum(enum_nombre, variante).unwrap_or(0) as i64;
                            let tag_const = builder.ins().iconst(types::I32, tag_idx);
                            let cmp = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::Equal, tag_val, tag_const);

                            // Si hay binding, necesitamos pasar el dato al bloque del brazo
                            if let Some(_nombre_binding) = binding {
                                // Cargar dato del enum (offset 8, despu├®s del tag + padding)
                                let dato_val = builder.ins().load(types::I64, cranelift_codegen::ir::MemFlags::new(), val_sujeto, 8);
                                builder.ins().brif(cmp, bloque_brazo, &[dato_val], bloque_siguiente, &[]);
                            } else {
                                builder.ins().brif(cmp, bloque_brazo, &[], bloque_siguiente, &[]);
                            }
                        }
                    }

                    // Bloque del brazo: compilar cuerpo
                    builder.switch_to_block(bloque_brazo);
                    builder.seal_block(bloque_brazo);

                    // Si hay binding, declararlo como variable
                    if let crate::ast::PatronMatch::VarianteEnum(_, _, Some(nombre_binding), _) = &brazo.patron {
                        let dato_param = builder.block_params(bloque_brazo)[0];
                        let binding_slot = builder.create_sized_stack_slot(
                            cranelift_codegen::ir::StackSlotData::new(
                                cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                                8,
                                0,
                            )
                        );
                        builder.ins().stack_store(dato_param, binding_slot, 0);
                        let mut vars_con_binding = variables.clone();
                        vars_con_binding.insert(nombre_binding.clone(), (binding_slot, Tipo::Entero64, crate::ast::Articulo::La));
                        let val_cuerpo = self.compilar_expresion(&brazo.cuerpo, builder, &vars_con_binding)?;
                        builder.ins().jump(bloque_fin, &[val_cuerpo]);
                    } else {
                        let val_cuerpo = self.compilar_expresion(&brazo.cuerpo, builder, variables)?;
                        builder.ins().jump(bloque_fin, &[val_cuerpo]);
                    }

                    // Bloque siguiente (para el pr├│ximo brazo)
                    builder.switch_to_block(bloque_siguiente);
                    builder.seal_block(bloque_siguiente);
                }

                // Despu├®s de todos los brazos, saltar al fin (caso no-exhaustivo: valor default 0)
                let default_val = builder.ins().iconst(cranelift_tipo, 0);
                builder.ins().jump(bloque_fin, &[default_val]);

                // Bloque fin: recibir el resultado
                builder.switch_to_block(bloque_fin);
                builder.seal_block(bloque_fin);
                let resultado = builder.block_params(bloque_fin)[0];
                Ok(resultado)
            }

            // Async (Fase 18A): esperar expr ÔÇö MVP: compila la expresi├│n interna
            // TODO: poll loop + waker cuando el runtime est├® listo
            Expresion::Esperar(expr_interno, _span) => {
                self.compilar_expresion(expr_interno, builder, variables)
            }

            // Async (Fase 18A): lanzar expr ÔÇö MVP: crea thread real del OS
            Expresion::Lanzar(expr_interno, _span) => {
                if let Expresion::Llamada(llamada) = expr_interno.as_ref() {
                    self.compilar_lanzar_hilo(llamada, builder, variables)
                } else {
                    // Fallback: compilar inline (secuencial)
                    self.compilar_expresion(expr_interno, builder, variables)
                }
            }

            // GUI (Fase GUI-1): direccion_de(funcion) ÔÇö obtiene direcci├│n de funci├│n
            Expresion::DireccionDe(nombre_funcion, _span) => {
                // Buscar la funci├│n en el mapa de funciones declaradas
                let func_id = self.funciones.get(nombre_funcion)
                    .ok_or(())?;
                let func_ref = self.module.declare_func_in_func(*func_id, builder.func);
                let ptr = builder.ins().func_addr(types::I64, func_ref);
                Ok(ptr)
            }

            // Bloque como expresi├│n: compilar sentencias, retornar valor de la ├║ltima
            Expresion::Bloque(bloque) => {
                let mut ultimo_valor = None;
                for sentencia in &bloque.sentencias {
                    match sentencia {
                        Sentencia::Expresion(expr) => {
                            ultimo_valor = Some(self.compilar_expresion(expr, builder, variables)?);
                        }
                        Sentencia::Retornar(Some(expr), _) => {
                            ultimo_valor = Some(self.compilar_expresion(expr, builder, variables)?);
                            break;
                        }
                        _ => {
                            // Variables declaradas en un bloque-expresi├│n no se propagan
                            let mut vars_locales = variables.clone();
                            self.compilar_sentencia(sentencia, builder, &mut vars_locales, &bloque.span)?;
                        }
                    }
                }
                match ultimo_valor {
                    Some(val) => Ok(val),
                    None => Ok(builder.ins().iconst(types::I32, 0)),
                }
            }

            // Async (Fase 18A): bloquear(expr) ÔÇö MVP: compila la expresi├│n interna
            // TODO: bridge syncÔåÆasync con runtime
            Expresion::Bloquear(expr_interno, _span) => {
                self.compilar_expresion(expr_interno, builder, variables)
            }

            // Fase 15A: m├®todos bitwise en enteros
            Expresion::Metodo(receptor, nombre, args, _span) => {
                self.compilar_metodo(receptor, nombre, args, builder, variables)
            }
        }
    }
    /// Compila un lado de una operación binaria, adaptando literales numéricos
    /// al tipo del otro operando (ej: x: Entero64 + 1 → el 1 se emite como I64).
    pub(crate) fn compilar_lado_binaria(
        &mut self,
        expr: &Expresion,
        tipo_otro: &Tipo,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        match expr {
            Expresion::Literal(lit) if self.es_tipo_numerico(tipo_otro) => {
                self.compilar_literal_con_tipo(lit, tipo_otro, builder)
            }
            Expresion::Unaria(op, inner, span) if self.es_tipo_numerico(tipo_otro) => {
                if let Expresion::Literal(lit) = inner.as_ref() {
                    let val = self.compilar_literal_con_tipo(lit, tipo_otro, builder)?;
                    self.compilar_operacion_unaria(*op, val, builder, span)
                } else {
                    self.compilar_expresion(expr, builder, variables)
                }
            }
            _ => self.compilar_expresion(expr, builder, variables),
        }
    }

    pub(crate) fn compilar_literal(&mut self, lit: &Literal, builder: &mut FunctionBuilder) -> Result<cranelift_codegen::ir::Value, ()> {
        match lit {
            Literal::Entero(n, _) => Ok(builder.ins().iconst(types::I32, *n as i64)),
            Literal::Booleano(b, _) => Ok(builder.ins().iconst(types::I8, if *b { 1 } else { 0 })),
            Literal::Caracter(c, _) => Ok(builder.ins().iconst(types::I32, *c as i64)),
            Literal::Flotante(f, _) => Ok(builder.ins().f64const(*f)),
            Literal::Palabra(_, _) => {
                // Strings en patrones no soportados por ahora
                Ok(builder.ins().iconst(types::I64, 0))
            }
        }
    }

    /// Compila un literal respetando el tipo declarado (ej: `el x: Entero64 = 5`).
    /// Los literales enteros se infieren como I32; si el tipo esperado es más ancho,
    /// se emite con el ancho correcto para evitar basura en los bytes altos.
    pub(crate) fn compilar_literal_con_tipo(
        &mut self,
        lit: &Literal,
        tipo_esperado: &Tipo,
        builder: &mut FunctionBuilder,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        match lit {
            Literal::Entero(n, _) => {
                let ct = self.tipo_a_cranelift(tipo_esperado);
                Ok(builder.ins().iconst(ct, *n as i64))
            }
            Literal::Flotante(f, _) => {
                let ct = self.tipo_a_cranelift(tipo_esperado);
                if ct == types::F32 {
                    Ok(builder.ins().f32const(*f as f32))
                } else {
                    Ok(builder.ins().f64const(*f))
                }
            }
            _ => self.compilar_literal(lit, builder),
        }
    }
    pub(crate) fn indice_variante_enum(&self, enum_nombre: &str, variante: &str) -> Option<u32> {
        self.enums.get(enum_nombre).and_then(|layout| layout.variantes.get(variante).copied())
    }
    pub(crate) fn expresion_usa_variable(&self, expr: &Expresion, nombre: &str) -> bool {
        match expr {
            Expresion::Identificador(n, _) => n == nombre,
            Expresion::Binaria(izq, _, der, _) => {
                self.expresion_usa_variable(izq, nombre) || self.expresion_usa_variable(der, nombre)
            }
            Expresion::Unaria(_, inner, _) => self.expresion_usa_variable(inner, nombre),
            Expresion::Llamada(llamada) => {
                llamada.argumentos.iter().any(|a| self.expresion_usa_variable(a, nombre))
            }
            Expresion::AccesoArray(base, idx, _) => {
                self.expresion_usa_variable(base, nombre) || self.expresion_usa_variable(idx, nombre)
            }
            Expresion::AccesoCampo(base, _, _) => self.expresion_usa_variable(base, nombre),
            Expresion::Rango(inicio, fin, _, _) => {
                self.expresion_usa_variable(inicio, nombre) || self.expresion_usa_variable(fin, nombre)
            }
            Expresion::Closure(params, cuerpo, _) => {
                // No contar si el closure shadowea la variable
                if params.iter().any(|(pn, _)| pn == nombre) {
                    false
                } else {
                    self.expresion_usa_variable(cuerpo, nombre)
                }
            }
            _ => false,
        }
    }
    pub(crate) fn buscar_tipo_campo(&self, nombre_struct: &str, nombre_campo: &str) -> Tipo {
        match self.structs.get(nombre_struct) {
            Some(layout) => {
                layout.tipos.get(nombre_campo).cloned().unwrap_or(Tipo::Entero32)
            }
            None => Tipo::Entero32,
        }
    }
    pub(crate) fn compilar_llamada(
        &mut self,
        llamada: &Llamada,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        // R6: Drop automático — análisis de moves y liberaciones manuales.
        // Esto decide qué variables heap NO se liberan al final del scope.
        if llamada.funcion.ends_with("_liberar") {
            // x.liberar() → liberación manual: la variable ya no necesita free automático
            if let Some(Expresion::Identificador(nombre, _)) = llamada.argumentos.first() {
                self.quitar_heap(nombre);
            }
        } else if !self.es_llamada_builtin(llamada) {
            // Función de usuario: parámetro con artículo `el`/`los` → mueve la variable
            for (i, arg) in llamada.argumentos.iter().enumerate() {
                if let Expresion::Identificador(nombre, _) = arg {
                    if self.parametro_mueve(&llamada.funcion, i)
                        && self.heap_vivas.iter().any(|(n, _)| n == nombre) {
                        self.quitar_heap(nombre);
                    }
                }
            }
        }

        // Verificar si es llamada a funci├│n built-in (Texto / Vector<T>)
        if self.es_llamada_builtin(llamada) {
            return self.compilar_llamada_builtin(llamada, builder, variables);
        }

        // Verificar si es llamada a funci├│n gen├®rica
        if self.funciones_genericas.contains_key(&llamada.funcion) {
            return self.compilar_llamada_generica(llamada, builder, variables);
        }

        let func_id = match self.funciones.get(&llamada.funcion).copied() {
            Some(func_id) => func_id,
            None => {
                // Verificar si es una llamada indirecta (variable con closure object)
                if let Some((slot, _tipo, _)) = variables.get(&llamada.funcion) {
                    let slot = *slot;
                    // Cargar puntero al closure object desde la variable
                    let closure_ptr = builder.ins().stack_load(types::I64, slot, 0);

                    // Cargar fn_ptr (offset 0) y env_ptr (offset 8) del closure object
                    let fn_ptr = builder.ins().load(types::I64, cranelift_codegen::ir::MemFlags::new(), closure_ptr, 0);
                    let env_ptr = builder.ins().load(types::I64, cranelift_codegen::ir::MemFlags::new(), closure_ptr, 8);

                    // Compilar argumentos
                    let mut args = vec![env_ptr]; // env_ptr siempre primero
                    for arg in &llamada.argumentos {
                        let val = self.compilar_expresion(arg, builder, variables)?;
                        args.push(val);
                    }

                    // Crear firma para la llamada indirecta
                    let mut sig = Signature::new(self.call_conv_default());
                    sig.params.push(AbiParam::new(types::I64)); // env_ptr
                    for _ in &llamada.argumentos {
                        sig.params.push(AbiParam::new(types::I32)); // default I32
                    }
                    sig.returns.push(AbiParam::new(types::I32)); // default retorno I32

                    let sig_ref = builder.import_signature(sig);
                    let call = builder.ins().call_indirect(sig_ref, fn_ptr, &args);
                    let result = builder.inst_results(call);
                    if result.is_empty() {
                        return Ok(builder.ins().iconst(types::I32, 0));
                    } else {
                        return Ok(result[0]);
                    }
                }

                self.errores.agregar(ErrorCompilador::nuevo(
                    CategoriaError::FFI,
                    1,
                    llamada.span.clone(),
                    format!("Funci├│n '{}' no encontrada", llamada.funcion),
                ));
                return Err(());
            }
        };
        
        let func_ref = self.module.declare_func_in_func(func_id, builder.func);

        // R9.0.1 — si la función retorna un struct, el caller aloca un slot temporal
        // y lo pasa como primer argumento oculto (sret). Devuelve el ptr del slot.
        let ret_es_struct = self.declaraciones.get(&llamada.funcion)
            .and_then(|f| f.retorno.as_ref())
            .map(|r| self.tipo_es_struct(r).is_some())
            .unwrap_or(false);

        let mut args = Vec::new();
        let mut slot_sret: Option<cranelift_codegen::ir::StackSlot> = None;
        if ret_es_struct {
            let layout = self.declaraciones.get(&llamada.funcion)
                .and_then(|f| f.retorno.as_ref())
                .and_then(|r| self.tipo_es_struct(r))
                .expect("retorno struct debe tener layout");
            let slot = builder.create_sized_stack_slot(
                cranelift_codegen::ir::StackSlotData::new(
                    cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                    layout.tamano,
                    0,
                )
            );
            slot_sret = Some(slot);
            args.push(builder.ins().stack_addr(types::I64, slot, 0));
        }
        for arg in &llamada.argumentos {
            let val = self.compilar_expresion(arg, builder, variables)?;
            args.push(val);
        }

        let call = builder.ins().call(func_ref, &args);
        let results = builder.inst_results(call);

        if ret_es_struct {
            Ok(builder.ins().stack_addr(types::I64, slot_sret.unwrap(), 0))
        } else if results.is_empty() {
            Ok(builder.ins().iconst(types::I32, 0))
        } else {
            Ok(results[0])
        }
    }
    pub(crate) fn es_llamada_builtin(&self, llamada: &Llamada) -> bool {
        matches!(llamada.funcion.as_str(),
            "imprimir" | "imprimir_linea" | "decir" | "tamaño_de" | "afirmar" |
            "texto_nuevo" | "texto_desde" | "texto_agregar" | "texto_longitud" | "texto_tam" | "texto_liberar" |
            "texto_concatenar" | "texto_subtexto" | "texto_comparar" | "texto_obtener_byte" |
            "texto_a_entero" | "texto_a_natural" | "texto_a_flotante" | "texto_a_booleano" |
            "archivo_leer" | "archivo_escribir" | "archivo_existe" |
            "abs" | "max" | "min" | "raiz" | "potencia" |
            // Trigonometría — F1: libm (preciso)
            "seno" | "coseno" | "tangente" | "arcseno" | "arccoseno" | "arctangente" | "arctangente2" |
            "senoh" | "cosenoh" | "tangenteh" |
            "exp" | "log" | "log10" | "piso" | "techo" | "fabs" | "fmod" |
            // Trigonometría precisa (F1) — libm
            "seno_preciso" | "coseno_preciso" | "tangente_preciso" | "exp_preciso" | "log_preciso" |
            // Trigonometría rápida (F2/F3) — math.rs
            "seno_rapido" | "coseno_rapido" | "seno_2pi" | "coseno_2pi" |
            "exp_rapido" | "log_rapido" | "seno_aprox" |
            "vector_nuevo" | "vector_agregar" | "vector_obtener" | "vector_longitud" | "vector_tam" | "vector_liberar" |
            "dormir" |
            "diccionario_nuevo" | "diccionario_insertar" | "diccionario_obtener" |
            "diccionario_existe" | "diccionario_eliminar" | "diccionario_longitud" | "diccionario_liberar" |
            "conjunto_nuevo" | "conjunto_insertar" | "conjunto_contiene" |
            "conjunto_eliminar" | "conjunto_longitud" | "conjunto_liberar" |
            "tcp_vincular" | "tcp_aceptar" | "tcp_leer" | "tcp_escribir" | "tcp_cerrar" |
            "canal_nuevo" | "canal_enviar" | "canal_recibir" | "canal_cerrar" | "canal_intentar" |
            "proceso_crear" | "proceso_esperar" | "proceso_leer_salida_completa" | "proceso_cerrar" |
            "proceso_crear_con_pipes" | "proceso_escribir" | "proceso_leer_salida_chunk" |
            "proceso_leer_error_chunk" | "proceso_cerrar_entrada" | "proceso_listo_para_leer" |
            "proceso_cerrar_bidireccional" |
            "tcp_conectar" | "dns_resolver" | "tcp_establecer_timeout" | "tcp_datos_disponibles" |
            "terminal_modo_raw" | "terminal_leer_tecla" |
            "entrada_leer" |
            "argumentos" |
            "fecha_unix" | "fecha_ms" |
            "dht_nuevo" | "dht_publicar" | "dht_consultar" | "dht_bootstrap" | "dht_cerrar" |
            "cancelar" |
            "texto_a_puntero" |
            "como_entero64" | "como_entero32" |
            "como_entero8" | "como_entero16" |
            "como_flotante32" | "como_flotante64"
        )
    }
    pub(crate) fn compilar_llamada_builtin(
        &mut self,
        llamada: &Llamada,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        match llamada.funcion.as_str() {
            "imprimir" => self.builtin_imprimir(builder, variables, &llamada.argumentos, false),
            "imprimir_linea" | "decir" => self.builtin_imprimir(builder, variables, &llamada.argumentos, true),
            "tamaño_de" => self.builtin_tamano_de(builder, &llamada.tipo_args),
            "afirmar" => self.builtin_afirmar(builder, variables, &llamada.argumentos, &llamada.span),
            "texto_nuevo" => self.builtin_texto_nuevo(builder),
            "texto_desde" => self.builtin_texto_desde(builder, variables, &llamada.argumentos),
            "texto_agregar" => self.builtin_texto_agregar(builder, variables, &llamada.argumentos),
            "texto_longitud" | "texto_tam" => self.builtin_texto_longitud(builder, variables, &llamada.argumentos),
            "texto_liberar" => self.builtin_texto_liberar(builder, variables, &llamada.argumentos),
            "texto_concatenar" => self.builtin_texto_concatenar(builder, variables, &llamada.argumentos),
            "texto_subtexto" => self.builtin_texto_subtexto(builder, variables, &llamada.argumentos),
            "texto_comparar" => self.builtin_texto_comparar(builder, variables, &llamada.argumentos),
            "texto_obtener_byte" => self.builtin_texto_obtener_byte(builder, variables, &llamada.argumentos),
            "texto_a_entero" => self.builtin_texto_a_entero(builder, variables, &llamada.argumentos),
            "texto_a_natural" => self.builtin_texto_a_natural(builder, variables, &llamada.argumentos),
            "texto_a_flotante" => self.builtin_texto_a_flotante(builder, variables, &llamada.argumentos),
            "texto_a_booleano" => self.builtin_texto_a_booleano(builder, variables, &llamada.argumentos),
            "texto_a_puntero" => self.builtin_texto_a_puntero(builder, variables, &llamada.argumentos),
            "como_entero64" => self.builtin_como_entero64(builder, variables, &llamada.argumentos),
            "como_entero32" => self.builtin_como_entero32(builder, variables, &llamada.argumentos),
            "como_entero8" => self.builtin_conversion_numerica(&llamada.argumentos, Tipo::Entero8, builder, variables),
            "como_entero16" => self.builtin_conversion_numerica(&llamada.argumentos, Tipo::Entero16, builder, variables),
            "como_flotante32" => self.builtin_conversion_numerica(&llamada.argumentos, Tipo::Flotante32, builder, variables),
            "como_flotante64" => self.builtin_conversion_numerica(&llamada.argumentos, Tipo::Flotante64, builder, variables),
            "archivo_leer" => self.builtin_archivo_leer(builder, variables, &llamada.argumentos),
            "archivo_escribir" => self.builtin_archivo_escribir(builder, variables, &llamada.argumentos),
            "archivo_existe" => self.builtin_archivo_existe(builder, variables, &llamada.argumentos),
            "abs" => self.builtin_abs(builder, variables, &llamada.argumentos),
            "max" => self.builtin_max(builder, variables, &llamada.argumentos),
            "min" => self.builtin_min(builder, variables, &llamada.argumentos),
            "raiz" => self.builtin_raiz(builder, variables, &llamada.argumentos),
            "potencia" => self.builtin_potencia(builder, variables, &llamada.argumentos),
            // Trigonometría — F1: libm (preciso)
            "seno" => self.builtin_seno(builder, variables, &llamada.argumentos),
            "coseno" => self.builtin_coseno(builder, variables, &llamada.argumentos),
            "tangente" => self.builtin_tangente(builder, variables, &llamada.argumentos),
            "arcseno" => self.builtin_arcseno(builder, variables, &llamada.argumentos),
            "arccoseno" => self.builtin_arccoseno(builder, variables, &llamada.argumentos),
            "arctangente" => self.builtin_arctangente(builder, variables, &llamada.argumentos),
            "arctangente2" => self.builtin_arctangente2(builder, variables, &llamada.argumentos),
            "senoh" => self.builtin_seno_hiperbolico(builder, variables, &llamada.argumentos),
            "cosenoh" => self.builtin_coseno_hiperbolico(builder, variables, &llamada.argumentos),
            "tangenteh" => self.builtin_tangente_hiperbolica(builder, variables, &llamada.argumentos),
            "exp" => self.builtin_exponencial(builder, variables, &llamada.argumentos),
            "log" => self.builtin_logaritmo(builder, variables, &llamada.argumentos),
            "log10" => self.builtin_logaritmo10(builder, variables, &llamada.argumentos),
            "piso" => self.builtin_piso(builder, variables, &llamada.argumentos),
            "techo" => self.builtin_techo(builder, variables, &llamada.argumentos),
            "fabs" => self.builtin_valor_absoluto(builder, variables, &llamada.argumentos),
            "fmod" => self.builtin_modulo_flotante(builder, variables, &llamada.argumentos),
            // Trigonometría precisa (F1) — libm
            "seno_preciso" => self.builtin_seno(builder, variables, &llamada.argumentos),
            "coseno_preciso" => self.builtin_coseno(builder, variables, &llamada.argumentos),
            "tangente_preciso" => self.builtin_tangente(builder, variables, &llamada.argumentos),
            "exp_preciso" => self.builtin_exponencial(builder, variables, &llamada.argumentos),
            "log_preciso" => self.builtin_logaritmo(builder, variables, &llamada.argumentos),
            // Trigonometría rápida (F2/F3) — math.rs
            "seno_rapido" => self.builtin_seno_rapido(builder, variables, &llamada.argumentos),
            "coseno_rapido" => self.builtin_coseno_rapido(builder, variables, &llamada.argumentos),
            "seno_2pi" => self.builtin_seno_2pi(builder, variables, &llamada.argumentos),
            "coseno_2pi" => self.builtin_coseno_2pi(builder, variables, &llamada.argumentos),
            "exp_rapido" => self.builtin_exponencial_rapido(builder, variables, &llamada.argumentos),
            "log_rapido" => self.builtin_logaritmo_rapido(builder, variables, &llamada.argumentos),
            "seno_aprox" => self.builtin_seno_aprox(builder, variables, &llamada.argumentos),
            "vector_nuevo" => self.builtin_vector_nuevo(builder, &llamada.tipo_args),
            "vector_agregar" => self.builtin_vector_agregar(builder, variables, &llamada.argumentos, &llamada.tipo_args),
            "vector_obtener" => self.builtin_vector_obtener(builder, variables, &llamada.argumentos, &llamada.tipo_args),
            "vector_longitud" | "vector_tam" => self.builtin_vector_longitud(builder, variables, &llamada.argumentos),
            "vector_liberar" => self.builtin_vector_liberar(builder, variables, &llamada.argumentos),
            "dormir" => self.builtin_dormir(builder, variables, &llamada.argumentos),
            "tcp_vincular" => self.builtin_tcp_vincular(builder, variables, &llamada.argumentos),
            "tcp_aceptar" => self.builtin_tcp_aceptar(builder, variables, &llamada.argumentos),
            "tcp_leer" => self.builtin_tcp_leer(builder, variables, &llamada.argumentos),
            "tcp_escribir" => self.builtin_tcp_escribir(builder, variables, &llamada.argumentos),
            "tcp_cerrar" => self.builtin_tcp_cerrar(builder, variables, &llamada.argumentos),
            "canal_nuevo" => self.builtin_canal_nuevo(builder, variables, &llamada.argumentos),
            "canal_enviar" => self.builtin_canal_enviar(builder, variables, &llamada.argumentos),
            "canal_recibir" => self.builtin_canal_recibir(builder, variables, &llamada.argumentos),
            "canal_cerrar" => self.builtin_canal_cerrar(builder, variables, &llamada.argumentos),
            "cancelar" => self.builtin_cancelar(builder, variables),
            "canal_intentar" => self.builtin_canal_intentar(builder, variables, &llamada.argumentos),
            "proceso_crear" => self.builtin_proceso_crear(builder, variables, &llamada.argumentos),
            "proceso_esperar" => self.builtin_proceso_esperar(builder, variables, &llamada.argumentos),
            "proceso_leer_salida_completa" => self.builtin_proceso_leer_salida(builder, variables, &llamada.argumentos),
            "proceso_cerrar" => self.builtin_proceso_cerrar(builder, variables, &llamada.argumentos),
            "proceso_crear_con_pipes" => self.builtin_proceso_crear_con_pipes(builder, variables, &llamada.argumentos),
            "proceso_escribir" => self.builtin_proceso_escribir(builder, variables, &llamada.argumentos),
            "proceso_leer_salida_chunk" => self.builtin_proceso_leer_salida_chunk(builder, variables, &llamada.argumentos),
            "proceso_leer_error_chunk" => self.builtin_proceso_leer_error_chunk(builder, variables, &llamada.argumentos),
            "proceso_cerrar_entrada" => self.builtin_proceso_cerrar_entrada(builder, variables, &llamada.argumentos),
            "proceso_listo_para_leer" => self.builtin_proceso_listo_para_leer(builder, variables, &llamada.argumentos),
            "proceso_cerrar_bidireccional" => self.builtin_proceso_cerrar_bidireccional(builder, variables, &llamada.argumentos),
            "tcp_conectar" => self.builtin_tcp_conectar(builder, variables, &llamada.argumentos),
            "dns_resolver" => self.builtin_dns_resolver(builder, variables, &llamada.argumentos),
            "tcp_establecer_timeout" => self.builtin_tcp_establecer_timeout(builder, variables, &llamada.argumentos),
            "tcp_datos_disponibles" => self.builtin_tcp_datos_disponibles(builder, variables, &llamada.argumentos),
            "terminal_modo_raw" => self.builtin_terminal_modo_raw(builder, variables, &llamada.argumentos),
            "terminal_leer_tecla" => self.builtin_terminal_leer_tecla(builder, variables, &llamada.argumentos),
            "entrada_leer" => self.builtin_entrada_leer(builder, variables, &llamada.argumentos),
"argumentos" => self.builtin_argumentos(builder, variables, &llamada.argumentos),
            "fecha_unix" => self.builtin_fecha_unix(builder, variables, &llamada.argumentos),
            "fecha_ms" => self.builtin_fecha_ms(builder, variables, &llamada.argumentos),
            "dht_nuevo" => self.builtin_dht_nuevo(builder, variables, &llamada.argumentos),
            "dht_publicar" => self.builtin_dht_publicar(builder, variables, &llamada.argumentos),
            "dht_consultar" => self.builtin_dht_consultar(builder, variables, &llamada.argumentos),
            "dht_bootstrap" => self.builtin_dht_bootstrap(builder, variables, &llamada.argumentos),
            "dht_cerrar" => self.builtin_dht_cerrar(builder, variables, &llamada.argumentos),
            "diccionario_nuevo" => self.builtin_diccionario_nuevo(builder, &llamada.tipo_args),
            "diccionario_insertar" => self.builtin_diccionario_insertar(builder, variables, &llamada.argumentos, &llamada.tipo_args),
            "diccionario_obtener" => self.builtin_diccionario_obtener(builder, variables, &llamada.argumentos, &llamada.tipo_args),
            "diccionario_existe" => self.builtin_diccionario_existe(builder, variables, &llamada.argumentos, &llamada.tipo_args),
            "diccionario_eliminar" => self.builtin_diccionario_eliminar(builder, variables, &llamada.argumentos, &llamada.tipo_args),
            "diccionario_longitud" => self.builtin_diccionario_longitud(builder, variables, &llamada.argumentos),
            "diccionario_liberar" => self.builtin_diccionario_liberar(builder, variables, &llamada.argumentos, &llamada.tipo_args),
            "conjunto_nuevo" => self.builtin_conjunto_nuevo(builder, &llamada.tipo_args),
            "conjunto_insertar" => self.builtin_conjunto_insertar(builder, variables, &llamada.argumentos, &llamada.tipo_args),
            "conjunto_contiene" => self.builtin_conjunto_contiene(builder, variables, &llamada.argumentos, &llamada.tipo_args),
            "conjunto_eliminar" => self.builtin_conjunto_eliminar(builder, variables, &llamada.argumentos, &llamada.tipo_args),
            "conjunto_longitud" => self.builtin_conjunto_longitud(builder, variables, &llamada.argumentos),
            "conjunto_liberar" => self.builtin_conjunto_liberar(builder, variables, &llamada.argumentos, &llamada.tipo_args),
            _ => {
                self.errores.agregar(ErrorCompilador::nuevo(
                    CategoriaError::Interno,
                    80,
                    llamada.span.clone(),
                    format!("Funci├│n built-in '{}' no implementada", llamada.funcion),
                ));
                Err(())
            }
        }
    }
}
