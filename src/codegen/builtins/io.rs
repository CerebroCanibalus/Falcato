use crate::codegen::*;

impl Codegen {
    pub(crate) fn builtin_imprimir(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
        con_newline: bool,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        // Verificar si hay interpolaciÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â³n: literal con {variable}
        if let Expresion::Literal(Literal::Palabra(texto, _)) = &argumentos[0] {
            if texto.contains('{') {
                return self.builtin_imprimir_interpolado(builder, variables, texto, con_newline);
            }
        }

        // Inferir tipo del argumento para dispatch
        let tipo_arg = self.inferir_tipo(&argumentos[0], variables);

        match tipo_arg {
            Tipo::Texto => {
                // Texto: extraer ptr y usar puts/printf %s
                let desc = self.compilar_expresion(&argumentos[0], builder, variables)?;
                let ptr = self.cargar_campo_descriptor(builder, desc, Self::OFFSET_PTR);
                if con_newline {
                    let func_id = self.asegurar_funcion_c("puts", &[types::I64], Some(types::I32));
                    let func_ref = self.module.declare_func_in_func(func_id, builder.func);
                    builder.ins().call(func_ref, &[ptr]);
                    self.flush_stdout(builder);
                } else {
                    let fmt_ptr = self.crear_string_literal(builder, "%s");
                    let func_id = self.asegurar_funcion_c("printf", &[types::I64, types::I64], Some(types::I32));
                    let func_ref = self.module.declare_func_in_func(func_id, builder.func);
                    builder.ins().call(func_ref, &[fmt_ptr, ptr]);
                }
            }
            Tipo::Entero32 | Tipo::Entero64 | Tipo::Entero8 | Tipo::Natural8 | Tipo::Natural16 | Tipo::Natural32 | Tipo::Natural64 => {
                // Enteros: printf %d ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â en Windows x64, args variÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¡dicos se pasan como I64
                let val = self.compilar_expresion(&argumentos[0], builder, variables)?;
                // R7.6: %d solo lee 32 bits — Entero64/Natural64 necesitan %lld
                // (antes: el valor se truncaba al imprimir, p.ej. 3000000000 → -1294967296)
                let fmt = match tipo_arg {
                    Tipo::Entero64 | Tipo::Natural64 => {
                        if con_newline { "%lld\n" } else { "%lld" }
                    }
                    _ => {
                        if con_newline { "%d\n" } else { "%d" }
                    }
                };
                let fmt_ptr = self.crear_string_literal(builder, fmt);
                // Extender a I64 para passing variÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¡dico correcto en Windows x64
                let val_i64 = match tipo_arg {
                    Tipo::Entero8 | Tipo::Natural8 | Tipo::Booleano | Tipo::Caracter => {
                        builder.ins().uextend(types::I64, val)
                    }
                    Tipo::Entero16 | Tipo::Natural16 => {
                        builder.ins().uextend(types::I64, val)
                    }
                    Tipo::Entero32 | Tipo::Natural32 => {
                        builder.ins().sextend(types::I64, val)
                    }
                    _ => val, // Ya es I64
                };
                let func_id = self.asegurar_funcion_c("printf", &[types::I64, types::I64], Some(types::I32));
                let func_ref = self.module.declare_func_in_func(func_id, builder.func);
                builder.ins().call(func_ref, &[fmt_ptr, val_i64]);
                if con_newline {
                    self.flush_stdout(builder);
                }
            }
            Tipo::Booleano => {
                // Booleano: imprimir "verdadero"/"falso"
                let val = self.compilar_expresion(&argumentos[0], builder, variables)?;
                let val_i32 = builder.ins().uextend(types::I32, val);
                let cero = builder.ins().iconst(types::I32, 0);
                let es_falso = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::Equal, val_i32, cero);
                let bloque_true = builder.create_block();
                let bloque_false = builder.create_block();
                let bloque_fin = builder.create_block();
                builder.ins().brif(es_falso, bloque_false, &[], bloque_true, &[]);

                builder.switch_to_block(bloque_true);
                if con_newline {
                    let msg_true = self.crear_string_literal(builder, "verdadero");
                    let puts_id = self.asegurar_funcion_c("puts", &[types::I64], Some(types::I32));
                    let puts_ref = self.module.declare_func_in_func(puts_id, builder.func);
                    builder.ins().call(puts_ref, &[msg_true]);
                    self.flush_stdout(builder);
                } else {
                    let msg_true = self.crear_string_literal(builder, "verdadero");
                    let fmt_ptr = self.crear_string_literal(builder, "%s");
                    let printf_id = self.asegurar_funcion_c("printf", &[types::I64, types::I64], Some(types::I32));
                    let printf_ref = self.module.declare_func_in_func(printf_id, builder.func);
                    builder.ins().call(printf_ref, &[fmt_ptr, msg_true]);
                }
                builder.ins().jump(bloque_fin, &[]);
                builder.seal_block(bloque_true);

                builder.switch_to_block(bloque_false);
                if con_newline {
                    let msg_false = self.crear_string_literal(builder, "falso");
                    let puts_id2 = self.asegurar_funcion_c("puts", &[types::I64], Some(types::I32));
                    let puts_ref2 = self.module.declare_func_in_func(puts_id2, builder.func);
                    builder.ins().call(puts_ref2, &[msg_false]);
                    self.flush_stdout(builder);
                } else {
                    let msg_false = self.crear_string_literal(builder, "falso");
                    let fmt_ptr2 = self.crear_string_literal(builder, "%s");
                    let printf_id2 = self.asegurar_funcion_c("printf", &[types::I64, types::I64], Some(types::I32));
                    let printf_ref2 = self.module.declare_func_in_func(printf_id2, builder.func);
                    builder.ins().call(printf_ref2, &[fmt_ptr2, msg_false]);
                }
                builder.ins().jump(bloque_fin, &[]);
                builder.seal_block(bloque_false);

                builder.switch_to_block(bloque_fin);
                builder.seal_block(bloque_fin);
            }
            Tipo::Flotante32 | Tipo::Flotante64 => {
                // Floats: printf %.17g (round-trip exacto)
                // CR-7 fix: en Windows, doubles van por GPR en variádicas (bitcast I64).
                // En POSIX (macOS/Linux), doubles van por FP regs (pasar F64 directo).
                let val = self.compilar_expresion(&argumentos[0], builder, variables)?;
                let fmt = if con_newline { "%.17g\n" } else { "%.17g" };
                let fmt_ptr = self.crear_string_literal(builder, fmt);
                let func_id;
                let func_ref;
                let args;
                #[cfg(target_os = "windows")]
                {
                    // Windows x64 variadic: doubles se pasan como bit pattern en reg entero
                    let val_bits = builder.ins().bitcast(types::I64, cranelift_codegen::ir::MemFlags::new(), val);
                    func_id = self.asegurar_funcion_c("printf", &[types::I64, types::I64], Some(types::I32));
                    func_ref = self.module.declare_func_in_func(func_id, builder.func);
                    args = vec![fmt_ptr, val_bits];
                }
                #[cfg(not(target_os = "windows"))]
                {
                    // POSIX: doubles van por FP regs — pasar F64 directo
                    func_id = self.asegurar_funcion_c("printf", &[types::I64, types::F64], Some(types::I32));
                    func_ref = self.module.declare_func_in_func(func_id, builder.func);
                    args = vec![fmt_ptr, val];
                }
                builder.ins().call(func_ref, &args);
                if con_newline {
                    self.flush_stdout(builder);
                }
            }
            _ => {
                // Palabra u otro puntero: camino original
                let msg_ptr = self.compilar_expresion(&argumentos[0], builder, variables)?;
                if con_newline {
                    let func_id = self.asegurar_funcion_c("puts", &[types::I64], Some(types::I32));
                    let func_ref = self.module.declare_func_in_func(func_id, builder.func);
                    builder.ins().call(func_ref, &[msg_ptr]);
                    self.flush_stdout(builder);
                } else {
                    let fmt_ptr = self.crear_string_literal(builder, "%s");
                    let func_id = self.asegurar_funcion_c("printf", &[types::I64, types::I64], Some(types::I32));
                    let func_ref = self.module.declare_func_in_func(func_id, builder.func);
                    builder.ins().call(func_ref, &[fmt_ptr, msg_ptr]);
                }
            }
        }

        Ok(builder.ins().iconst(types::I64, 0))
    }

    /// 3.1a — flush stdout tras imprimir_linea.
    /// Llama a fflush(NULL) para forzar salida inmediata aunque el programa
    /// crashee después (evita perder prints de debug por buffering).
    pub(crate) fn flush_stdout(&mut self, builder: &mut FunctionBuilder) {
        let zero = builder.ins().iconst(types::I64, 0);
        let func_id = self.asegurar_funcion_c("fflush", &[types::I64], Some(types::I32));
        let func_ref = self.module.declare_func_in_func(func_id, builder.func);
        builder.ins().call(func_ref, &[zero]);
    }

    pub(crate) fn builtin_afirmar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
        span: &crate::span::Span,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        if argumentos.is_empty() {
            self.errores.agregar(ErrorCompilador::nuevo(
                CategoriaError::Tipo,
                75,
                span.clone(),
                "'afirmar' requiere un argumento booleano".to_string(),
            ));
            return Err(());
        }

        let cond = self.compilar_expresion(&argumentos[0], builder, variables)?;

        // Si condiciÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â³n es falsa ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ imprimir error y terminar
        let bloque_fallo = builder.create_block();
        let bloque_ok = builder.create_block();

        // Extender condiciÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â³n a I32 para comparaciÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â³n segura
        let cond_i32 = builder.ins().uextend(types::I32, cond);
        let cero = builder.ins().iconst(types::I32, 0);
        let es_falso = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::Equal, cond_i32, cero);
        builder.ins().brif(es_falso, bloque_fallo, &[], bloque_ok, &[]);

        // Bloque fallo: puts("  FALLO: afirmaciÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â³n fallida") + ExitProcess(1)
        builder.switch_to_block(bloque_fallo);
        builder.seal_block(bloque_fallo);

        let msg = self.crear_string_literal(builder, "  FALLO: afirmaciÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â³n fallida");
        let puts_id = self.asegurar_funcion_c("puts", &[types::I64], Some(types::I32));
        let puts_ref = self.module.declare_func_in_func(puts_id, builder.func);
        builder.ins().call(puts_ref, &[msg]);

        let uno = builder.ins().iconst(types::I32, 1);
        self.platform_call_void("exit_process", builder, &[uno]);
        builder.ins().trap(cranelift_codegen::ir::TrapCode::UnreachableCodeReached);

        // Bloque OK: continuar
        builder.switch_to_block(bloque_ok);
        builder.seal_block(bloque_ok);

        Ok(builder.ins().iconst(types::I32, 0))
    }

    pub(crate) fn builtin_dormir(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        if argumentos.is_empty() {
            return Ok(builder.ins().iconst(types::I32, 0));
        }

        let ms_val = self.compilar_expresion(&argumentos[0], builder, variables)?;

        // Sleep(DWORD ms) ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â kernel32.dll, Windows x64 fastcall
        // DWORD es u32. Si el valor ya es I32, usar directo; si es I64, truncar.
        let ms_i32 = if builder.func.dfg.value_type(ms_val) == types::I64 {
            builder.ins().ireduce(types::I32, ms_val)
        } else {
            ms_val
        };

        self.platform_call_void("sleep", builder, &[ms_i32]);

        Ok(builder.ins().iconst(types::I32, 0))
    }

    pub(crate) fn builtin_imprimir_interpolado(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        texto: &str,
        con_newline: bool,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        // Parsear interpolaciÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â³n: dividir en segmentos literales y variables
        let mut segmentos: Vec<(bool, String)> = Vec::new(); // (es_variable, contenido)
        let mut literal_actual = String::new();
        let mut chars = texto.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '{' {
                if !literal_actual.is_empty() {
                    segmentos.push((false, literal_actual.clone()));
                    literal_actual.clear();
                }
                let mut nombre = String::new();
                while let Some(&ch) = chars.peek() {
                    if ch == '}' { chars.next(); break; }
                    nombre.push(ch);
                    chars.next();
                }
                segmentos.push((true, nombre));
            } else {
                literal_actual.push(c);
            }
        }
        if !literal_actual.is_empty() {
            segmentos.push((false, literal_actual));
        }

        // Imprimir cada segmento
        for (es_var, contenido) in &segmentos {
            if *es_var {
                // Variable (o acceso a campo args.nombre): imprimir según su tipo
                if contenido.contains('.') {
                    // Acceso a campo: compilar la expresión AccesoCampo real.
                    // Soporta un nivel: args.nombre (base.campo).
                    let partes: Vec<&str> = contenido.split('.').collect();
                    if partes.len() == 2 {
                        let base = partes[0].to_string();
                        let campo = partes[1].to_string();
                        let expr_campo = Expresion::AccesoCampo(
                            Box::new(Expresion::Identificador(base.clone(), Span::vacio())),
                            campo.clone(),
                            Span::vacio(),
                        );
                        // Compilar la expresión completa → devuelve el valor del campo cargado
                        let val = self.compilar_expresion(&expr_campo, builder, variables)?;
                        let tipo = self.inferir_tipo(&expr_campo, variables);
                        self.imprimir_valor_interpolado(builder, variables, val, &tipo, contenido)?;
                    }
                } else if let Some((slot, tipo, _)) = variables.get(contenido) {
                    let slot = *slot;
                    let tipo = self.resolver_alias(tipo);
                    let (fmt_str, val) = match tipo {
                        Tipo::Entero8 => {
                            let v = builder.ins().stack_load(types::I8, slot, 0);
                            let ext = builder.ins().sextend(types::I64, v);
                            ("%d\0", ext)
                        }
                        Tipo::Entero16 => {
                            let v = builder.ins().stack_load(types::I16, slot, 0);
                            let ext = builder.ins().sextend(types::I64, v);
                            ("%d\0", ext)
                        }
                        Tipo::Entero32 => {
                            let v = builder.ins().stack_load(types::I32, slot, 0);
                            let ext = builder.ins().sextend(types::I64, v);
                            ("%d\0", ext)
                        }
                        Tipo::Entero64 => {
                            let v = builder.ins().stack_load(types::I64, slot, 0);
                            ("%lld\0", v)
                        }
                        Tipo::Natural8 => {
                            let v = builder.ins().stack_load(types::I8, slot, 0);
                            let ext = builder.ins().uextend(types::I64, v);
                            ("%u\0", ext)
                        }
                        Tipo::Natural16 => {
                            let v = builder.ins().stack_load(types::I16, slot, 0);
                            let ext = builder.ins().uextend(types::I64, v);
                            ("%u\0", ext)
                        }
                        Tipo::Natural32 => {
                            let v = builder.ins().stack_load(types::I32, slot, 0);
                            let ext = builder.ins().uextend(types::I64, v);
                            ("%u\0", ext)
                        }
                        Tipo::Natural64 => {
                            let v = builder.ins().stack_load(types::I64, slot, 0);
                            ("%llu\0", v)
                        }
                        Tipo::Flotante32 => {
                            // Cargar F32 y promover a F64 (el slot es de 4 bytes)
                            let v32 = builder.ins().stack_load(types::F32, slot, 0);
                            let v = builder.ins().fpromote(types::F64, v32);
                            // %.17g (round-trip exacto) espera un double; la firma Cranelift usa I64,
                            // así que pasamos los bits del F64 como I64 (bitcast).
                            let bits = builder.ins().bitcast(types::I64, cranelift_codegen::ir::MemFlags::new(), v);
                            ("%.17g\0", bits)
                        }
                        Tipo::Flotante64 => {
                            let v = builder.ins().stack_load(types::F64, slot, 0);
                            // %.17g (round-trip exacto); la firma Cranelift usa I64,
                            // así que pasamos los bits del F64 como I64 (bitcast).
                            let bits = builder.ins().bitcast(types::I64, cranelift_codegen::ir::MemFlags::new(), v);
                            ("%.17g\0", bits)
                        }
                        Tipo::Booleano => {
                            let v = builder.ins().stack_load(types::I8, slot, 0);
                            let ext = builder.ins().uextend(types::I64, v);
                            ("%d\0", ext)
                        }
                        Tipo::Caracter => {
                            let v = builder.ins().stack_load(types::I8, slot, 0);
                            let ext = builder.ins().uextend(types::I64, v);
                            ("%c\0", ext)
                        }
                        Tipo::Texto => {
                            // Texto = puntero a descriptor {ptr, len, cap}.
                            // Cargar descriptor y luego el ptr de datos (offset 0).
                            let desc = builder.ins().stack_load(types::I64, slot, 0);
                            let v = builder.ins().load(types::I64, cranelift_codegen::ir::MemFlags::new(), desc, Self::OFFSET_PTR);
                            ("%s\0", v)
                        }
                        _ => {
                            let v = builder.ins().stack_load(types::I64, slot, 0);
                            ("%s\0", v)
                        }
                    };
                    let fmt_ptr = self.crear_string_literal(builder, fmt_str);
                    let func_id = self.asegurar_funcion_c("printf", &[types::I64, types::I64], Some(types::I32));
                    let func_ref = self.module.declare_func_in_func(func_id, builder.func);
                    builder.ins().call(func_ref, &[fmt_ptr, val]);
                }
            } else {
                // Literal: imprimir con printf("%s", literal)
                let mut bytes = contenido.as_bytes().to_vec();
                bytes.push(0);
                let ptr = self.crear_string_literal_bytes(builder, &bytes);
                let fmt_ptr = self.crear_string_literal(builder, "%s\0");
                let func_id = self.asegurar_funcion_c("printf", &[types::I64, types::I64], Some(types::I32));
                let func_ref = self.module.declare_func_in_func(func_id, builder.func);
                builder.ins().call(func_ref, &[fmt_ptr, ptr]);
            }
        }

        // Newline final si es imprimir_linea
        if con_newline {
            let nl_ptr = self.crear_string_literal(builder, "\n\0");
            let func_id = self.asegurar_funcion_c("printf", &[types::I64, types::I64], Some(types::I32));
            let func_ref = self.module.declare_func_in_func(func_id, builder.func);
            let fmt_ptr = self.crear_string_literal(builder, "%s\0");
            builder.ins().call(func_ref, &[fmt_ptr, nl_ptr]);
            self.flush_stdout(builder);
        }

        Ok(builder.ins().iconst(types::I64, 0))
    }

    pub(crate) fn imprimir_valor_interpolado(
        &mut self,
        builder: &mut FunctionBuilder,
        _variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        val: cranelift_codegen::ir::Value,
        tipo: &Tipo,
        _contenido: &str,
    ) -> Result<(), ()> {
        let tipo = self.resolver_alias(tipo);
        let (fmt_str, val_out) = match tipo {
            Tipo::Entero8 | Tipo::Caracter => {
                let ext = builder.ins().sextend(types::I64, val);
                ("%d\0", ext)
            }
            Tipo::Entero16 => {
                let ext = builder.ins().sextend(types::I64, val);
                ("%d\0", ext)
            }
            Tipo::Entero32 => {
                let ext = builder.ins().sextend(types::I64, val);
                ("%d\0", ext)
            }
            Tipo::Entero64 => ("%lld\0", val),
            Tipo::Natural8 | Tipo::Natural16 | Tipo::Natural32 | Tipo::Booleano => {
                let ext = builder.ins().uextend(types::I64, val);
                ("%u\0", ext)
            }
            Tipo::Natural64 => ("%llu\0", val),
            Tipo::Flotante32 => {
                let f = builder.ins().fpromote(types::F64, val);
                let bits = builder.ins().bitcast(types::I64, cranelift_codegen::ir::MemFlags::new(), f);
                ("%f\0", bits)
            }
            Tipo::Flotante64 => {
                let bits = builder.ins().bitcast(types::I64, cranelift_codegen::ir::MemFlags::new(), val);
                ("%f\0", bits)
            }
            Tipo::Texto => {
                // val es puntero al descriptor {ptr, len, cap} → extraer ptr de datos
                let v = builder.ins().load(types::I64, cranelift_codegen::ir::MemFlags::new(), val, Self::OFFSET_PTR);
                ("%s\0", v)
            }
            _ => ("%s\0", val),
        };
        let fmt_ptr = self.crear_string_literal(builder, fmt_str);
        let func_id = self.asegurar_funcion_c("printf", &[types::I64, types::I64], Some(types::I32));
        let func_ref = self.module.declare_func_in_func(func_id, builder.func);
        builder.ins().call(func_ref, &[fmt_ptr, val_out]);
        Ok(())
    }

    pub(crate) fn crear_string_literal(&mut self, builder: &mut FunctionBuilder, s: &str) -> cranelift_codegen::ir::Value {
        if let Some(data_id) = self.strings_internados.get(s) {
            let global = self.module.declare_data_in_func(*data_id, builder.func);
            return builder.ins().global_value(types::I64, global);
        }
        self.contador_strings += 1;
        let data_id = self.module.declare_data(
            &format!("str_lit_{}", self.contador_strings),
            Linkage::Preemptible, // Preemptible: visible para el linker en Mach-O (fix CR-3 macOS)
            false,
            false,
        ).unwrap();
        let mut bytes = s.as_bytes().to_vec();
        bytes.push(0); // null terminator para compatibilidad C
        let mut desc = cranelift_module::DataDescription::new();
        desc.define(bytes.into_boxed_slice());
        self.module.define_data(data_id, &desc).unwrap();
        self.strings_internados.insert(s.to_string(), data_id);
        let global = self.module.declare_data_in_func(data_id, builder.func);
        builder.ins().global_value(types::I64, global)
    }

    pub(crate) fn crear_string_literal_bytes(&mut self, builder: &mut FunctionBuilder, bytes: &[u8]) -> cranelift_codegen::ir::Value {
        self.contador_strings += 1;
        let data_id = self.module.declare_data(
            &format!("str_bytes_{}", self.contador_strings),
            Linkage::Preemptible, // Preemptible: visible para el linker en Mach-O (fix CR-3 macOS)
            false,
            false,
        ).unwrap();
        let mut desc = cranelift_module::DataDescription::new();
        desc.define(bytes.to_vec().into_boxed_slice());
        self.module.define_data(data_id, &desc).unwrap();
        let global = self.module.declare_data_in_func(data_id, builder.func);
        builder.ins().global_value(types::I64, global)
    }

}
