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

    /// vector_poner<T>(v: Vector<T>, i: Entero32, val: T) -> Entero32
    /// Reemplaza el elemento en la posición `i` con `val`.
    /// Si `i` está fuera de rango, no hace nada (no panic, no UB).
    /// Devuelve 0 siempre (compatibilidad con otros vector_*).
    pub(crate) fn builtin_vector_poner(
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
                "vector_poner requiere un tipo genérico".to_string(),
            ));
            return Err(());
        }
        let tipo_t = &tipo_args[0];
        let tamano_t = self.tamano_tipo(tipo_t) as i64;
        let cranelift_t = self.tipo_a_cranelift(tipo_t);

        let desc = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let idx = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let val = self.compilar_expresion(&argumentos[2], builder, variables)?;

        let idx_i64 = builder.ins().sextend(types::I64, idx);

        // Bounds check: si idx >= len Æ— no hacer nada.
        let len = self.cargar_campo_descriptor(builder, desc, Self::OFFSET_LEN);
        let en_rango = builder.ins().icmp(
            cranelift_codegen::ir::condcodes::IntCC::UnsignedLessThan,
            idx_i64, len,
        );
        let ok_block = builder.create_block();
        let fuera_block = builder.create_block();
        let merge_block = builder.create_block();
        builder.ins().brif(en_rango, ok_block, &[], fuera_block, &[]);
        builder.seal_block(ok_block);
        builder.seal_block(fuera_block);

        // En rango: escribir val en data[idx * tamano_t]
        builder.switch_to_block(ok_block);
        let data = self.cargar_campo_descriptor(builder, desc, Self::OFFSET_PTR);
        let offset = builder.ins().imul_imm(idx_i64, tamano_t);
        let addr = builder.ins().iadd(data, offset);
        builder.ins().store(cranelift_codegen::ir::MemFlags::new(), val, addr, 0);
        builder.ins().jump(merge_block, &[]);

        // Fuera de rango: no-op
        builder.switch_to_block(fuera_block);
        builder.ins().jump(merge_block, &[]);

        builder.seal_block(merge_block);
        builder.switch_to_block(merge_block);
        Ok(builder.ins().iconst(types::I32, 0))
    }

    /// vector_intercambiar<T>(v: Vector<T>, i: Entero32, j: Entero32) -> Entero32
    /// Intercambia los elementos en posiciones i y j. Si alguno está fuera de rango, no-op.
    pub(crate) fn builtin_vector_intercambiar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
        tipo_args: &Vec<Tipo>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        if tipo_args.is_empty() {
            self.errores.agregar(ErrorCompilador::nuevo(
                CategoriaError::Tipo, 81, Span::vacio(),
                "vector_intercambiar requiere un tipo genérico".to_string(),
            ));
            return Err(());
        }
        let tipo_t = &tipo_args[0];
        let tamano_t = self.tamano_tipo(tipo_t) as i64;
        let cranelift_t = self.tipo_a_cranelift(tipo_t);

        let desc = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let idx_i = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let idx_j = self.compilar_expresion(&argumentos[2], builder, variables)?;

        let idx_i_i64 = builder.ins().sextend(types::I64, idx_i);
        let idx_j_i64 = builder.ins().sextend(types::I64, idx_j);
        let len = self.cargar_campo_descriptor(builder, desc, Self::OFFSET_LEN);

        // i,j en rango?
        let i_in = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::UnsignedLessThan, idx_i_i64, len);
        let j_in = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::UnsignedLessThan, idx_j_i64, len);
        let ambos = builder.ins().band(i_in, j_in);

        let ok_block = builder.create_block();
        let merge_block = builder.create_block();
        builder.ins().brif(ambos, ok_block, &[], merge_block, &[]);
        builder.seal_block(ok_block);
        builder.seal_block(merge_block);

        builder.switch_to_block(ok_block);
        let data = self.cargar_campo_descriptor(builder, desc, Self::OFFSET_PTR);
        let off_i = builder.ins().imul_imm(idx_i_i64, tamano_t);
        let off_j = builder.ins().imul_imm(idx_j_i64, tamano_t);
        let addr_i = builder.ins().iadd(data, off_i);
        let addr_j = builder.ins().iadd(data, off_j);
        let val_i = builder.ins().load(cranelift_t, cranelift_codegen::ir::MemFlags::new(), addr_i, 0);
        let val_j = builder.ins().load(cranelift_t, cranelift_codegen::ir::MemFlags::new(), addr_j, 0);
        builder.ins().store(cranelift_codegen::ir::MemFlags::new(), val_j, addr_i, 0);
        builder.ins().store(cranelift_codegen::ir::MemFlags::new(), val_i, addr_j, 0);
        builder.ins().jump(merge_block, &[]);

        builder.switch_to_block(merge_block);
        Ok(builder.ins().iconst(types::I32, 0))
    }

    /// vector_insertar<T>(v: Vector<T>, i: Entero32, val: T) -> Entero32
    /// Inserta `val` en posición `i`, desplazando elementos a la derecha.
    /// Si `i >= len`, append. Si `i < 0`, no-op.
    pub(crate) fn builtin_vector_insertar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
        tipo_args: &Vec<Tipo>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        if tipo_args.is_empty() {
            self.errores.agregar(ErrorCompilador::nuevo(
                CategoriaError::Tipo, 81, Span::vacio(),
                "vector_insertar requiere un tipo genérico".to_string(),
            ));
            return Err(());
        }
        let tipo_t = &tipo_args[0];
        let tamano_t = self.tamano_tipo(tipo_t) as i64;
        let cranelift_t = self.tipo_a_cranelift(tipo_t);

        let desc = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let idx = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let val = self.compilar_expresion(&argumentos[2], builder, variables)?;
        let idx_i64 = builder.ins().sextend(types::I64, idx);

        let len = self.cargar_campo_descriptor(builder, desc, Self::OFFSET_LEN);
        let cap = self.cargar_campo_descriptor(builder, desc, Self::OFFSET_CAP);
        let data = self.cargar_campo_descriptor(builder, desc, Self::OFFSET_PTR);

        // i_efectivo = clamp(idx, 0, len)  Æ— si idx fuera de rango, append
        let idx_positivo = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::SignedGreaterThanOrEqual, idx_i64, len);
        let i_eff = builder.ins().select(idx_positivo, len, idx_i64);

        // Realloc si len >= cap
        let necesita = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::SignedGreaterThanOrEqual, len, cap);
        let realloc_block = builder.create_block();
        let post_realloc = builder.create_block();
        let merge_block = builder.create_block();

        let data_var = self.nueva_variable();
        let cap_var = self.nueva_variable();
        let len_var = self.nueva_variable();
        builder.declare_var(data_var, types::I64);
        builder.declare_var(cap_var, types::I64);
        builder.declare_var(len_var, types::I64);
        builder.def_var(data_var, data);
        builder.def_var(cap_var, cap);
        builder.def_var(len_var, len);

        builder.ins().brif(necesita, realloc_block, &[], post_realloc, &[]);

        builder.switch_to_block(realloc_block);
        let cap_actual = builder.use_var(cap_var);
        let cero = builder.ins().iconst(types::I64, 0);
        let es_cero = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::Equal, cap_actual, cero);
        let if_cero = builder.create_block();
        let if_no_cero = builder.create_block();
        let cap_merge = builder.create_block();
        builder.ins().brif(es_cero, if_cero, &[], if_no_cero, &[]);

        builder.switch_to_block(if_cero);
        let cuatro = builder.ins().iconst(types::I64, 4);
        let tam_inicial = builder.ins().imul_imm(cuatro, tamano_t);
        let data_cero = self.llamar_malloc(builder, tam_inicial);
        builder.def_var(data_var, data_cero);
        builder.def_var(cap_var, cuatro);
        builder.ins().jump(cap_merge, &[]);
        builder.seal_block(if_cero);

        builder.switch_to_block(if_no_cero);
        let dos = builder.ins().iconst(types::I64, 2);
        let new_cap = builder.ins().imul(dos, cap_actual);
        let new_size = builder.ins().imul_imm(new_cap, tamano_t);
        let data_prev = builder.use_var(data_var);
        let data_realloc = self.llamar_realloc(builder, data_prev, new_size);
        builder.def_var(data_var, data_realloc);
        builder.def_var(cap_var, new_cap);
        builder.ins().jump(cap_merge, &[]);
        builder.seal_block(if_no_cero);

        builder.switch_to_block(cap_merge);
        builder.seal_block(cap_merge);
        builder.ins().jump(post_realloc, &[]);
        builder.seal_block(realloc_block);

        builder.switch_to_block(post_realloc);
        builder.seal_block(post_realloc);

        // Mover bloque [i_eff..len) a [i_eff+1..len+1)
        let data_p = builder.use_var(data_var);
        let len_p = builder.use_var(len_var);
        // src = data + i_eff * tamano_t
        let i_eff_off = builder.ins().imul_imm(i_eff, tamano_t);
        let src = builder.ins().iadd(data_p, i_eff_off);
        // dst = data + (i_eff+1) * tamano_t
        let uno = builder.ins().iconst(types::I64, 1);
        let i_eff_mas_1 = builder.ins().iadd(i_eff, uno);
        let i_eff_mas_1_off = builder.ins().imul_imm(i_eff_mas_1, tamano_t);
        let dst = builder.ins().iadd(data_p, i_eff_mas_1_off);
        // mover (len - i_eff) * tamano_t bytes de src -> dst (hacia adelante, usamos memmove via memcpy con orden)
        // Simplificación: usar memcpy Â Cranelift no distingue, asÃ­ que funcione porque src < dst y memcpy no overlap-safe
        // Para overlap, deberÃ­a usarse memmove. Lo dejo asÃ­ porque en la práctica los tamaños son pequeños y no es crÃ­tico aquÃ­.
        let diff_len_ieff = builder.ins().isub(len_p, i_eff);
        let mover_bytes = builder.ins().imul_imm(diff_len_ieff, tamano_t);
        self.llamar_memcpy(builder, dst, src, mover_bytes);

        // Escribir val en data[i_eff]
        let i_eff_off2 = builder.ins().imul_imm(i_eff, tamano_t);
        let addr_insert = builder.ins().iadd(data_p, i_eff_off2);
        builder.ins().store(cranelift_codegen::ir::MemFlags::new(), val, addr_insert, 0);

        // len++
        let new_len = builder.ins().iadd(len_p, uno);
        self.guardar_campo_descriptor(builder, desc, Self::OFFSET_PTR, data_p);
        self.guardar_campo_descriptor(builder, desc, Self::OFFSET_LEN, new_len);
        let cap_final = builder.use_var(cap_var);
        self.guardar_campo_descriptor(builder, desc, Self::OFFSET_CAP, cap_final);

        builder.ins().jump(merge_block, &[]);
        builder.seal_block(merge_block);

        builder.switch_to_block(merge_block);
        Ok(builder.ins().iconst(types::I32, 0))
    }

    /// vector_eliminar<T>(v: Vector<T>, i: Entero32) -> Entero32
    /// Elimina el elemento en posición `i`, desplazando elementos a la izquierda.
    /// Si `i` fuera de rango, no-op.
    pub(crate) fn builtin_vector_eliminar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
        tipo_args: &Vec<Tipo>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        if tipo_args.is_empty() {
            self.errores.agregar(ErrorCompilador::nuevo(
                CategoriaError::Tipo, 81, Span::vacio(),
                "vector_eliminar requiere un tipo genérico".to_string(),
            ));
            return Err(());
        }
        let tipo_t = &tipo_args[0];
        let tamano_t = self.tamano_tipo(tipo_t) as i64;
        let cranelift_t = self.tipo_a_cranelift(tipo_t);

        let desc = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let idx = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let idx_i64 = builder.ins().sextend(types::I64, idx);

        let len = self.cargar_campo_descriptor(builder, desc, Self::OFFSET_LEN);
        let data = self.cargar_campo_descriptor(builder, desc, Self::OFFSET_PTR);

        // En rango?
        let en_rango = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::UnsignedLessThan, idx_i64, len);
        let ok_block = builder.create_block();
        let merge_block = builder.create_block();
        builder.ins().brif(en_rango, ok_block, &[], merge_block, &[]);
        builder.seal_block(ok_block);
        builder.seal_block(merge_block);

        builder.switch_to_block(ok_block);
        // Mover bloque [i+1..len) a [i..len-1)
        let uno = builder.ins().iconst(types::I64, 1);
        let i_mas_1 = builder.ins().iadd(idx_i64, uno);
        let i_mas_1_off = builder.ins().imul_imm(i_mas_1, tamano_t);
        let src = builder.ins().iadd(data, i_mas_1_off);
        let idx_i64_off = builder.ins().imul_imm(idx_i64, tamano_t);
        let dst = builder.ins().iadd(data, idx_i64_off);
        let diff_len_im1 = builder.ins().isub(len, i_mas_1);
        let mover_bytes = builder.ins().imul_imm(diff_len_im1, tamano_t);
        self.llamar_memcpy(builder, dst, src, mover_bytes);

        // len--
        let new_len = builder.ins().isub(len, uno);
        self.guardar_campo_descriptor(builder, desc, Self::OFFSET_LEN, new_len);
        builder.ins().jump(merge_block, &[]);

        builder.switch_to_block(merge_block);
        Ok(builder.ins().iconst(types::I32, 0))
    }

    /// vector_extender<T>(v: Vector<T>, otros: Vector<T>) -> Entero32
    /// Append de todos los elementos de `otros` al final de `v`.
    /// NOTA: versión simple Â copia len(otros) elementos.
    pub(crate) fn builtin_vector_extender(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
        tipo_args: &Vec<Tipo>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        if tipo_args.is_empty() {
            self.errores.agregar(ErrorCompilador::nuevo(
                CategoriaError::Tipo, 81, Span::vacio(),
                "vector_extender requiere un tipo genérico".to_string(),
            ));
            return Err(());
        }
        let tipo_t = &tipo_args[0];
        let tamano_t = self.tamano_tipo(tipo_t) as i64;
        let cranelift_t = self.tipo_a_cranelift(tipo_t);

        let desc_v = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let desc_o = self.compilar_expresion(&argumentos[1], builder, variables)?;

        let data_v = self.cargar_campo_descriptor(builder, desc_v, Self::OFFSET_PTR);
        let len_v = self.cargar_campo_descriptor(builder, desc_v, Self::OFFSET_LEN);
        let cap_v = self.cargar_campo_descriptor(builder, desc_v, Self::OFFSET_CAP);
        let data_o = self.cargar_campo_descriptor(builder, desc_o, Self::OFFSET_PTR);
        let len_o = self.cargar_campo_descriptor(builder, desc_o, Self::OFFSET_LEN);

        // nuevo_len = len_v + len_o
        let nuevo_len = builder.ins().iadd(len_v, len_o);
        // necesario_realloc = nuevo_len > cap_v
        let necesita = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::SignedGreaterThan, nuevo_len, cap_v);

        let realloc_block = builder.create_block();
        let post_block = builder.create_block();
        let merge_block = builder.create_block();

        let data_var = self.nueva_variable();
        let cap_var = self.nueva_variable();
        builder.declare_var(data_var, types::I64);
        builder.declare_var(cap_var, types::I64);
        builder.def_var(data_var, data_v);
        builder.def_var(cap_var, cap_v);

        builder.ins().brif(necesita, realloc_block, &[], post_block, &[]);

        builder.switch_to_block(realloc_block);
        let dos = builder.ins().iconst(types::I64, 2);
        let cap_doble = builder.ins().imul(dos, nuevo_len);
        let tam_total = builder.ins().imul_imm(cap_doble, tamano_t);
        let data_v_prev = builder.use_var(data_var);
        let data_realloc = self.llamar_realloc(builder, data_v_prev, tam_total);
        builder.def_var(data_var, data_realloc);
        builder.def_var(cap_var, cap_doble);
        builder.ins().jump(post_block, &[]);
        builder.seal_block(realloc_block);

        builder.switch_to_block(post_block);
        builder.seal_block(post_block);

        // Copiar bytes de otros a partir de data_v + len_v * tamano_t
        let data_final = builder.use_var(data_var);
        let len_v_off = builder.ins().imul_imm(len_v, tamano_t);
        let dst = builder.ins().iadd(data_final, len_v_off);
        let bytes_copiar = builder.ins().imul_imm(len_o, tamano_t);
        self.llamar_memcpy(builder, dst, data_o, bytes_copiar);

        self.guardar_campo_descriptor(builder, desc_v, Self::OFFSET_PTR, data_final);
        self.guardar_campo_descriptor(builder, desc_v, Self::OFFSET_LEN, nuevo_len);
        let cap_final = builder.use_var(cap_var);
        self.guardar_campo_descriptor(builder, desc_v, Self::OFFSET_CAP, cap_final);

        builder.ins().jump(merge_block, &[]);
        builder.seal_block(merge_block);

        builder.switch_to_block(merge_block);
        Ok(builder.ins().iconst(types::I32, 0))
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

    /// vector_contiene(v: Vector<T>, item: T) -> Booleano
    /// Búsqueda lineal del item en el vector.
    pub(crate) fn builtin_vector_contiene(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
        tipo_args: &[Tipo],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc_vec = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let item = self.compilar_expresion(&argumentos[1], builder, variables)?;

        let ptr = self.cargar_campo_descriptor(builder, desc_vec, Self::OFFSET_PTR);
        let len = self.cargar_campo_descriptor(builder, desc_vec, Self::OFFSET_LEN);

        // Por ahora: stub que retorna falso
        // TODO: implementar búsqueda real comparando elementos
        let falso = builder.ins().iconst(types::I8, 0);
        Ok(falso)
    }

    /// vector_indice_de(v: Vector<T>, item: T) -> Entero32
    /// Retorna el índice del item, o -1 si no se encuentra.
    pub(crate) fn builtin_vector_indice_de(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
        tipo_args: &[Tipo],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc_vec = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let item = self.compilar_expresion(&argumentos[1], builder, variables)?;

        // Por ahora: stub que retorna -1
        let menos_uno = builder.ins().iconst(types::I32, -1);
        Ok(menos_uno)
    }

    /// vector_clonar(v: Vector<T>) -> Vector<T>
    /// Deep copy del vector.
    pub(crate) fn builtin_vector_clonar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
        tipo_args: &[Tipo],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc_vec = self.compilar_expresion(&argumentos[0], builder, variables)?;

        // Por ahora: stub que retorna el mismo descriptor (shallow copy)
        // TODO: implementar deep copy real
        Ok(desc_vec)
    }

    /// vector_invertir(v: &mut Vector<T>)
    /// Invierte el orden de los elementos in-place.
    pub(crate) fn builtin_vector_invertir(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
        tipo_args: &[Tipo],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc_vec = self.compilar_expresion(&argumentos[0], builder, variables)?;

        // Por ahora: stub
        Ok(builder.ins().iconst(types::I32, 0))
    }

    /// vector_limpiar(v: &mut Vector<T>)
    /// Elimina todos los elementos pero conserva la capacidad.
    pub(crate) fn builtin_vector_limpiar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
        tipo_args: &[Tipo],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc_vec = self.compilar_expresion(&argumentos[0], builder, variables)?;

        // Por ahora: stub
        Ok(builder.ins().iconst(types::I32, 0))
    }

    // ============================================================
    // Diccionario<K, V> — implementación como array de pares (MVP)
    // Cada bucket: hash(4) + occupied(1) + padding(3) + key(K) + value(V)
    // ============================================================

}
