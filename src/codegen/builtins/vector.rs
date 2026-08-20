use crate::codegen::*;

impl Codegen {
    pub(crate) fn builtin_vector_nuevo(
        &mut self,
        builder: &mut FunctionBuilder,
        tipo_args: &Vec<Tipo>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        if tipo_args.is_empty() {
            self.errores.agregar(ErrorCompilador::nuevo(
                CategoriaError::Tipo,
                81,
                Span::vacio(),
                "vector_nuevo requiere un tipo genÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â©rico".to_string(),
            ));
            return Err(());
        }
        let _tipo_t = &tipo_args[0];
        Ok(self.descriptor_nuevo(builder))
    }

    pub(crate) fn builtin_vector_agregar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
        tipo_args: &Vec<Tipo>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        if tipo_args.is_empty() {
            self.errores.agregar(ErrorCompilador::nuevo(
                CategoriaError::Tipo,
                81,
                Span::vacio(),
                "vector_agregar requiere un tipo genÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â©rico".to_string(),
            ));
            return Err(());
        }
        let tipo_t = &tipo_args[0];
        let tamano_t = self.tamano_tipo(tipo_t) as i64;
        let _cranelift_t = self.tipo_a_cranelift(tipo_t);

        let desc = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let val = self.compilar_expresion(&argumentos[1], builder, variables)?;

        let data = self.cargar_campo_descriptor(builder, desc, Self::OFFSET_PTR);
        let len = self.cargar_campo_descriptor(builder, desc, Self::OFFSET_LEN);
        let cap = self.cargar_campo_descriptor(builder, desc, Self::OFFSET_CAP);

        let necesita_realloc = builder.ins().icmp(
            cranelift_codegen::ir::condcodes::IntCC::SignedGreaterThanOrEqual,
            len,
            cap,
        );

        let then_block = builder.create_block();
        let merge_block = builder.create_block();
        let data_var = self.nueva_variable();
        let cap_var = self.nueva_variable();
        builder.declare_var(data_var, types::I64);
        builder.declare_var(cap_var, types::I64);
        builder.def_var(data_var, data);
        builder.def_var(cap_var, cap);

        builder.ins().brif(necesita_realloc, then_block, &[], merge_block, &[]);

        // then
        builder.switch_to_block(then_block);
        let cero = builder.ins().iconst(types::I64, 0);
        let cap_actual = builder.use_var(cap_var);
        let es_cero = builder.ins().icmp(
            cranelift_codegen::ir::condcodes::IntCC::Equal,
            cap_actual,
            cero,
        );
        let if_cero = builder.create_block();
        let if_no_cero = builder.create_block();
        let merge_cap = builder.create_block();
        builder.ins().brif(es_cero, if_cero, &[], if_no_cero, &[]);

        // cap == 0: alloc 4 elementos
        builder.switch_to_block(if_cero);
        let cuatro = builder.ins().iconst(types::I64, 4);
        let tam_inicial = builder.ins().imul_imm(cuatro, tamano_t);
        let data_cero = self.llamar_malloc(builder, tam_inicial);
        builder.def_var(data_var, data_cero);
        builder.def_var(cap_var, cuatro);
        builder.ins().jump(merge_cap, &[]);
        builder.seal_block(if_cero);

        // cap > 0: realloc 2*cap
        builder.switch_to_block(if_no_cero);
        let dos = builder.ins().iconst(types::I64, 2);
        let cap_previa = builder.use_var(cap_var);
        let new_cap = builder.ins().imul(dos, cap_previa);
        let new_size = builder.ins().imul_imm(new_cap, tamano_t);
        let data_previo = builder.use_var(data_var);
        let data_realloc = self.llamar_realloc(builder, data_previo, new_size);
        builder.def_var(data_var, data_realloc);
        builder.def_var(cap_var, new_cap);
        builder.ins().jump(merge_cap, &[]);
        builder.seal_block(if_no_cero);

        builder.switch_to_block(merge_cap);
        builder.seal_block(merge_cap);
        builder.ins().jump(merge_block, &[]);
        builder.seal_block(then_block);

        // merge
        builder.switch_to_block(merge_block);
        builder.seal_block(merge_block);
        let data_final = builder.use_var(data_var);
        let cap_final = builder.use_var(cap_var);

        // Guardar valor en data + len * tamano_t
        let offset = builder.ins().imul_imm(len, tamano_t);
        let addr = builder.ins().iadd(data_final, offset);
        builder.ins().store(cranelift_codegen::ir::MemFlags::new(), val, addr, 0);

        // len++
        let new_len = builder.ins().iadd_imm(len, 1);
        self.guardar_campo_descriptor(builder, desc, Self::OFFSET_PTR, data_final);
        self.guardar_campo_descriptor(builder, desc, Self::OFFSET_LEN, new_len);
        self.guardar_campo_descriptor(builder, desc, Self::OFFSET_CAP, cap_final);

        Ok(builder.ins().iconst(types::I32, 0))
    }

    pub(crate) fn builtin_vector_obtener(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
        tipo_args: &Vec<Tipo>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        if tipo_args.is_empty() {
            self.errores.agregar(ErrorCompilador::nuevo(
                CategoriaError::Tipo,
                81,
                Span::vacio(),
                "vector_obtener requiere un tipo genÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â©rico".to_string(),
            ));
            return Err(());
        }
        let tipo_t = &tipo_args[0];
        let tamano_t = self.tamano_tipo(tipo_t) as i64;
        let cranelift_t = self.tipo_a_cranelift(tipo_t);

        let desc = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let idx = self.compilar_expresion(&argumentos[1], builder, variables)?;
        // El índice siempre es Entero32 → extender a I64 para el cálculo de offset
        // (el offset se suma a un puntero I64). Bug preexistente: la condición
        // anterior dependía del tipo del ELEMENTO, no del índice → con Texto
        // (I64) no convertía y `iadd(I64, I32)` rompía el verifier.
        let idx_i64 = builder.ins().sextend(types::I64, idx);

        // Bounds check (R7.6): si idx >= len → devolver 0 definido (no UB).
        // Spec: acceso fuera de rango devuelve 0 en lugar de leer memoria basura.
        let len = self.cargar_campo_descriptor(builder, desc, Self::OFFSET_LEN);
        let en_rango = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::UnsignedLessThan, idx_i64, len);
        let ok_block = builder.create_block();
        let fuera_block = builder.create_block();
        let merge_block = builder.create_block();
        builder.append_block_param(merge_block, cranelift_t);
        builder.ins().brif(en_rango, ok_block, &[], fuera_block, &[]);
        builder.seal_block(ok_block);
        builder.seal_block(fuera_block);

        // Dentro de rango: leer el elemento
        builder.switch_to_block(ok_block);
        let data = self.cargar_campo_descriptor(builder, desc, Self::OFFSET_PTR);
        let offset = builder.ins().imul_imm(idx_i64, tamano_t);
        let addr = builder.ins().iadd(data, offset);
        let valor = builder.ins().load(cranelift_t, cranelift_codegen::ir::MemFlags::new(), addr, 0);
        builder.ins().jump(merge_block, &[valor]);

        // Fuera de rango: devolver 0 (definido, no UB)
        builder.switch_to_block(fuera_block);
        let cero = match cranelift_t {
            types::F32 => builder.ins().f32const(0.0),
            types::F64 => builder.ins().f64const(0.0),
            _ => builder.ins().iconst(cranelift_t, 0),
        };
        builder.ins().jump(merge_block, &[cero]);

        builder.seal_block(merge_block);
        builder.switch_to_block(merge_block);
        Ok(builder.block_params(merge_block)[0])
    }

    pub(crate) fn builtin_vector_longitud(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let len = self.cargar_campo_descriptor(builder, desc, Self::OFFSET_LEN);
        Ok(builder.ins().ireduce(types::I32, len))
    }

    pub(crate) fn builtin_vector_liberar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let data = self.cargar_campo_descriptor(builder, desc, Self::OFFSET_PTR);
        self.llamar_free(builder, data);
        self.llamar_free(builder, desc);
        Ok(builder.ins().iconst(types::I32, 0))
    }

    // ============================================================
    // Diccionario<K, V> ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â implementaciÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â³n como array de pares (MVP)
    // Cada bucket: hash(4) + occupied(1) + padding(3) + key(K) + value(V)
    // ============================================================

}
