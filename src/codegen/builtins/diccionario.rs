use crate::codegen::*;

impl Codegen {
    pub(crate) fn diccionario_bucket_stride(&self, tipo_k: &Tipo, tipo_v: &Tipo) -> u32 {
        let key_size = self.tamano_tipo(tipo_k);
        let val_size = self.tamano_tipo(tipo_v);
        let raw = 8 + key_size + val_size;
        ((raw + 7) / 8) * 8
    }

    pub(crate) fn diccionario_guardar_valor(
        &self,
        builder: &mut FunctionBuilder,
        addr: cranelift_codegen::ir::Value,
        val: cranelift_codegen::ir::Value,
        tipo: &Tipo,
        flags: cranelift_codegen::ir::MemFlags,
    ) {
        // R9.0.2 — structs (incluso de 8/4/2/1 bytes) se guardan COPIANDO del ptr:
        // `val` es la dirección del slot del struct. Sin esto, un struct de 8 bytes
        // (2×Entero32) caía en el caso I64 y guardaba el PUNTERO en vez del struct.
        if self.tipo_es_struct(tipo).is_some() {
            let tam = self.tamano_tipo(tipo);
            for off in (0..tam).step_by(8) {
                let fv = builder.ins().load(types::I64, flags, val, off as i32);
                builder.ins().store(flags, fv, addr, off as i32);
            }
            return;
        }
        let tam = self.tamano_tipo(tipo);
        match tam {
            1 => { let v = builder.ins().ireduce(types::I8, val); builder.ins().store(flags, v, addr, 0); }
            4 => { let v = match builder.func.dfg.value_type(val) { types::I64 => builder.ins().ireduce(types::I32, val), _ => val }; builder.ins().store(flags, v, addr, 0); }
            8 => { let v = match builder.func.dfg.value_type(val) { types::I32 => builder.ins().uextend(types::I64, val), _ => val }; builder.ins().store(flags, v, addr, 0); }
            _ => {
                for off in (0..tam).step_by(8) {
                    let fv = builder.ins().load(types::I64, flags, val, off as i32);
                    builder.ins().store(flags, fv, addr, off as i32);
                }
            }
        }
    }

    pub(crate) fn diccionario_cargar_valor(
        &self,
        builder: &mut FunctionBuilder,
        addr: cranelift_codegen::ir::Value,
        tipo: &Tipo,
        flags: cranelift_codegen::ir::MemFlags,
    ) -> cranelift_codegen::ir::Value {
        // R9.0.2 — structs: devolver el PUNTERO al valor en el bucket (el caller
        // copia al slot o accede por campos). Incluye structs de 8 bytes (2×Entero32),
        // que antes caían en el caso I64 y devolvían el struct empaquetado como puntero.
        if self.tipo_es_struct(tipo).is_some() {
            return addr;
        }
        let tam = self.tamano_tipo(tipo);
        match tam {
            1 => {
                let loaded = builder.ins().load(types::I8, flags, addr, 0);
                builder.ins().uextend(types::I32, loaded)
            }
            4 => builder.ins().load(types::I32, flags, addr, 0),
            8 => builder.ins().load(types::I64, flags, addr, 0),
            _ => builder.ins().load(types::I64, flags, addr, 0),
        }
    }

    pub(crate) fn compilar_hash(
        &self,
        tipo: &Tipo,
        builder: &mut FunctionBuilder,
        val: cranelift_codegen::ir::Value,
    ) -> cranelift_codegen::ir::Value {
        match tipo {
            Tipo::Entero32 => {
                let prime = builder.ins().iconst(types::I32, 0x45D9F3B);
                builder.ins().imul(val, prime)
            }
            Tipo::Palabra | Tipo::Entero64 => {
                let lo = builder.ins().ireduce(types::I32, val);
                let shift_amt = builder.ins().iconst(types::I64, 32);
                let hi_shifted = builder.ins().ushr(val, shift_amt);
                let hi = builder.ins().ireduce(types::I32, hi_shifted);
                let mixed = builder.ins().bxor(lo, hi);
                let prime = builder.ins().iconst(types::I32, 0x45D9F3B);
                builder.ins().imul(mixed, prime)
            }
            _ => {
                if builder.func.dfg.value_type(val) == types::I64 {
                    builder.ins().ireduce(types::I32, val)
                } else { val }
            }
        }
    }

    pub(crate) fn compilar_comparar_claves(
        &self,
        _tipo: &Tipo,
        builder: &mut FunctionBuilder,
        a: cranelift_codegen::ir::Value,
        b: cranelift_codegen::ir::Value,
    ) -> cranelift_codegen::ir::Value {
        let cc = cranelift_codegen::ir::condcodes::IntCC::Equal;
        builder.ins().icmp(cc, a, b)
    }

    /// Retorna I32: bucket index si existe, -1 si no
    pub(crate) fn compilar_buscar_en_diccionario(
        &self,
        builder: &mut FunctionBuilder,
        buckets_ptr: cranelift_codegen::ir::Value,
        cap: cranelift_codegen::ir::Value,
        tipo_k: &Tipo,
        key_val: cranelift_codegen::ir::Value,
        hash_val: cranelift_codegen::ir::Value,
        stride: u32,
    ) -> cranelift_codegen::ir::Value {
        let flags = cranelift_codegen::ir::MemFlags::new();
        let one_i64 = builder.ins().iconst(types::I64, 1);
        let neg_one = builder.ins().iconst(types::I32, -1);
        let stride_val = builder.ins().iconst(types::I64, stride as i64);
        let four_i64 = builder.ins().iconst(types::I64, 4);
        let eight_i64 = builder.ins().iconst(types::I64, 8);

        // Compute initial index = hash % cap
        let cap_i32 = builder.ins().ireduce(types::I32, cap);
        let start_idx = builder.ins().urem(hash_val, cap_i32);
        let start_idx_i64 = builder.ins().uextend(types::I64, start_idx);

        let header_block = builder.create_block();
        builder.append_block_param(header_block, types::I64);
        let body_block = builder.create_block();
        let found_block = builder.create_block();
        let exit_block = builder.create_block();
        let merge_block = builder.create_block();
        builder.append_block_param(merge_block, types::I32);

        builder.ins().jump(header_block, &[start_idx_i64]);

        // Loop header: compare i < cap
        builder.switch_to_block(header_block);
        let i = builder.block_params(header_block)[0];
        let done = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::UnsignedGreaterThanOrEqual, i, cap);
        builder.ins().brif(done, exit_block, &[], body_block, &[]);
        // body_block tiene 1 predecesor (el brif del header) → sellar inmediato es seguro
        builder.seal_block(body_block);

        // Body: check if bucket is occupied and key matches
        builder.switch_to_block(body_block);
        let offset = builder.ins().imul(i, stride_val);
        let bucket_addr = builder.ins().iadd(buckets_ptr, offset);
        let occupied_addr = builder.ins().iadd(bucket_addr, four_i64);
        let occupied_i8 = builder.ins().load(types::I8, flags, occupied_addr, 0);
        let occupied_i32 = builder.ins().uextend(types::I32, occupied_i8);
        let uno = builder.ins().iconst(types::I32, 1);
        let is_occupied = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::Equal, occupied_i32, uno);
        
        let check_block = builder.create_block();
        let advance_block = builder.create_block();
        builder.ins().brif(is_occupied, check_block, &[], advance_block, &[]);
        builder.seal_block(check_block);
        
        // Occupied: check key match
        builder.switch_to_block(check_block);
        let key_addr = builder.ins().iadd(bucket_addr, eight_i64);
        let stored_key = self.diccionario_cargar_valor(builder, key_addr, tipo_k, flags);
        let keys_match = self.compilar_comparar_claves(tipo_k, builder, stored_key, key_val);
        builder.ins().brif(keys_match, found_block, &[], advance_block, &[]);
        builder.seal_block(advance_block);

        // Advance: i++
        builder.switch_to_block(advance_block);
        let next_i = builder.ins().iadd(i, one_i64);
        let wrapped = builder.ins().urem(next_i, cap);
        // Check if wrapped back to start ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ full circle, exit
        let full_circle = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::Equal, wrapped, start_idx_i64);
        builder.ins().brif(full_circle, exit_block, &[], header_block, &[wrapped]);
        // NOTE: header_block sealed AFTER this brif (in back-edge)

        // Seal header after back-edge
        builder.seal_block(header_block);

        // Found
        builder.seal_block(found_block);
        builder.switch_to_block(found_block);
        let found_idx = builder.ins().ireduce(types::I32, i);
        builder.ins().jump(merge_block, &[found_idx]);

        // Exit (not found)
        builder.seal_block(exit_block);
        builder.switch_to_block(exit_block);
        builder.ins().jump(merge_block, &[neg_one]);

        builder.seal_block(merge_block);
        builder.switch_to_block(merge_block);
        builder.block_params(merge_block)[0]
    }

    pub(crate) fn builtin_diccionario_nuevo(
        &mut self,
        builder: &mut FunctionBuilder,
        tipo_args: &Vec<Tipo>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        // Diccionario necesita buckets REALES desde el inicio: descriptor_nuevo
        // deja cap=0 y compilar_buscar hace `hash % cap` → división por cero.
        // Bug R7.6: cap inicial 16 buckets (stride = 8 + K + V, alineado a 8).
        let tipo_k = &tipo_args[0];
        let tipo_v = &tipo_args[1];
        let stride = self.diccionario_bucket_stride(tipo_k, tipo_v);
        let cap_inicial: i64 = 16;
        let tam_buckets = builder.ins().iconst(types::I64, stride as i64 * cap_inicial);
        let buckets = self.llamar_malloc(builder, tam_buckets);
        let desc = self.descriptor_nuevo(builder);
        let flags = cranelift_codegen::ir::MemFlags::new();
        builder.ins().store(flags, buckets, desc, Self::OFFSET_PTR);
        let cap_val = builder.ins().iconst(types::I64, cap_inicial);
        builder.ins().store(flags, cap_val, desc, Self::OFFSET_CAP);
        Ok(desc)
    }

    pub(crate) fn builtin_diccionario_insertar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
        tipo_args: &Vec<Tipo>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let tipo_k = &tipo_args[0];
        let tipo_v = &tipo_args[1];
        let dict_ptr = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let key_val = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let val_val = self.compilar_expresion(&argumentos[2], builder, variables)?;
        let flags = cranelift_codegen::ir::MemFlags::new();
        let stride = self.diccionario_bucket_stride(tipo_k, tipo_v);
        let buckets_ptr = self.cargar_campo_descriptor(builder, dict_ptr, Self::OFFSET_PTR);
        let cap = self.cargar_campo_descriptor(builder, dict_ptr, Self::OFFSET_CAP);
        let hash_insert = self.compilar_hash(tipo_k, builder, key_val);

        let existing_idx = self.compilar_buscar_en_diccionario(builder, buckets_ptr, cap, tipo_k, key_val, hash_insert, stride);
        
        // Constantes compartidas por AMBOS bloques (found/not_found) — definirlas
        // AQUÍ (bloque dominante) o el verifier falla: "uses value from non-dominating
        // inst" (SSA dominance). Bug R7.6: estaban dentro de found_block.
        let stride_i64 = builder.ins().iconst(types::I64, stride as i64);
        let val_offset_amt = (8 + self.tamano_tipo(tipo_k)) as i64;
        let val_offset_val = builder.ins().iconst(types::I64, val_offset_amt);

        let found_block = builder.create_block();
        let not_found_block = builder.create_block();
        let merge_block = builder.create_block();
        builder.append_block_param(merge_block, types::I64);
        let neg_one = builder.ins().iconst(types::I32, -1);
        let cmp = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::NotEqual, existing_idx, neg_one);
        builder.ins().brif(cmp, found_block, &[], not_found_block, &[]);
        builder.seal_block(found_block);
        builder.seal_block(not_found_block);

        // Found: overwrite value at existing_idx
        builder.switch_to_block(found_block);
        let idx_i64 = builder.ins().uextend(types::I64, existing_idx);
        let offset_bytes = builder.ins().imul(idx_i64, stride_i64);
        let bucket_addr = builder.ins().iadd(buckets_ptr, offset_bytes);
        let val_addr = builder.ins().iadd(bucket_addr, val_offset_val);
        self.diccionario_guardar_valor(builder, val_addr, val_val, tipo_v, flags);
        builder.ins().jump(merge_block, &[dict_ptr]);

        // Not found: insert into first empty slot (at len position)
        builder.switch_to_block(not_found_block);
        let len = self.cargar_campo_descriptor(builder, dict_ptr, Self::OFFSET_LEN);
        let cap = self.cargar_campo_descriptor(builder, dict_ptr, Self::OFFSET_CAP);
        // Resize (R7.6): si len >= cap → realloc a 2*cap. realloc preserva el
        // contenido; la búsqueda escanea todos los buckets (probing completo),
        // así que las claves existentes siguen siendo encontrables.
        let necesita_resize = builder.ins().icmp(
            cranelift_codegen::ir::condcodes::IntCC::UnsignedGreaterThanOrEqual,
            len,
            cap,
        );
        let resize_block = builder.create_block();
        let no_resize_block = builder.create_block();
        let merge_resize = builder.create_block();
        builder.ins().brif(necesita_resize, resize_block, &[], no_resize_block, &[]);
        builder.seal_block(resize_block);
        builder.seal_block(no_resize_block);

        builder.switch_to_block(resize_block);
        let dos = builder.ins().iconst(types::I64, 2);
        let new_cap = builder.ins().imul(dos, cap);
        let new_size = builder.ins().imul_imm(new_cap, stride as i64);
        let buckets_nuevos = self.llamar_realloc(builder, buckets_ptr, new_size);
        let flags_resize = cranelift_codegen::ir::MemFlags::new();
        builder.ins().store(flags_resize, buckets_nuevos, dict_ptr, Self::OFFSET_PTR);
        builder.ins().store(flags_resize, new_cap, dict_ptr, Self::OFFSET_CAP);
        builder.ins().jump(merge_resize, &[]);

        builder.switch_to_block(no_resize_block);
        builder.ins().jump(merge_resize, &[]);
        builder.seal_block(merge_resize);
        builder.switch_to_block(merge_resize);

        // Re-cargar tras el posible resize (memoria = fuente de verdad)
        let buckets_final = self.cargar_campo_descriptor(builder, dict_ptr, Self::OFFSET_PTR);
        let len_offset = builder.ins().imul(len, stride_i64);
        let empty_addr = builder.ins().iadd(buckets_final, len_offset);
        let hash_val = self.compilar_hash(tipo_k, builder, key_val);
        builder.ins().store(flags, hash_val, empty_addr, 0);
        let uno_i8 = builder.ins().iconst(types::I8, 1);
        builder.ins().store(flags, uno_i8, empty_addr, 4);
        let key_offset = builder.ins().iconst(types::I64, 8);
        let key_addr = builder.ins().iadd(empty_addr, key_offset);
        self.diccionario_guardar_valor(builder, key_addr, key_val, tipo_k, flags);
        let val_addr2 = builder.ins().iadd(empty_addr, val_offset_val);
        self.diccionario_guardar_valor(builder, val_addr2, val_val, tipo_v, flags);
        let one_i64 = builder.ins().iconst(types::I64, 1);
        let real_new_len = builder.ins().iadd(len, one_i64);
        builder.ins().store(flags, real_new_len, dict_ptr, Self::OFFSET_LEN);
        builder.ins().jump(merge_block, &[dict_ptr]);

        builder.seal_block(merge_block);
        builder.switch_to_block(merge_block);
        Ok(builder.block_params(merge_block)[0])
    }

    pub(crate) fn builtin_diccionario_obtener(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
        tipo_args: &Vec<Tipo>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let tipo_k = &tipo_args[0];
        let tipo_v = &tipo_args[1];
        let dict_ptr = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let key_val = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let flags = cranelift_codegen::ir::MemFlags::new();
        let stride = self.diccionario_bucket_stride(tipo_k, tipo_v);
        let buckets_ptr = self.cargar_campo_descriptor(builder, dict_ptr, Self::OFFSET_PTR);
        let cap = self.cargar_campo_descriptor(builder, dict_ptr, Self::OFFSET_CAP);
        let hash_val = self.compilar_hash(tipo_k, builder, key_val);

        let idx = self.compilar_buscar_en_diccionario(builder, buckets_ptr, cap, tipo_k, key_val, hash_val, stride);
        let stride_i64 = builder.ins().iconst(types::I64, stride as i64);
        let idx_i64 = builder.ins().uextend(types::I64, idx);
        let offset_bytes = builder.ins().imul(idx_i64, stride_i64);
        let bucket_addr = builder.ins().iadd(buckets_ptr, offset_bytes);
        let val_offset_amt = (8 + self.tamano_tipo(tipo_k)) as i64;
        let val_offset_val = builder.ins().iconst(types::I64, val_offset_amt);
        let val_addr = builder.ins().iadd(bucket_addr, val_offset_val);
        Ok(self.diccionario_cargar_valor(builder, val_addr, tipo_v, flags))
    }

    pub(crate) fn builtin_diccionario_existe(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
        tipo_args: &Vec<Tipo>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let tipo_k = &tipo_args[0];
        let dict_ptr = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let key_val = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let stride = self.diccionario_bucket_stride(tipo_k, &Tipo::Booleano);
        let buckets_ptr = self.cargar_campo_descriptor(builder, dict_ptr, Self::OFFSET_PTR);
        let cap = self.cargar_campo_descriptor(builder, dict_ptr, Self::OFFSET_CAP);
        let hash_val = self.compilar_hash(tipo_k, builder, key_val);

        let idx = self.compilar_buscar_en_diccionario(builder, buckets_ptr, cap, tipo_k, key_val, hash_val, stride);
        let found = builder.ins().icmp_imm(cranelift_codegen::ir::condcodes::IntCC::SignedGreaterThanOrEqual, idx, 0);
        let uno = builder.ins().iconst(types::I32, 1);
        let cero = builder.ins().iconst(types::I32, 0);
        Ok(builder.ins().select(found, uno, cero))
    }

    pub(crate) fn builtin_diccionario_eliminar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
        tipo_args: &Vec<Tipo>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let tipo_k = &tipo_args[0];
        let dict_ptr = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let key_val = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let flags = cranelift_codegen::ir::MemFlags::new();
        let stride = self.diccionario_bucket_stride(tipo_k, &Tipo::Booleano);
        let buckets_ptr = self.cargar_campo_descriptor(builder, dict_ptr, Self::OFFSET_PTR);
        let cap = self.cargar_campo_descriptor(builder, dict_ptr, Self::OFFSET_CAP);
        let hash_val = self.compilar_hash(tipo_k, builder, key_val);

        let idx = self.compilar_buscar_en_diccionario(builder, buckets_ptr, cap, tipo_k, key_val, hash_val, stride);
        let found_block = builder.create_block();
        let not_found_block = builder.create_block();
        let merge_block = builder.create_block();
        builder.append_block_param(merge_block, types::I32);
        let neg_one = builder.ins().iconst(types::I32, -1);
        let found = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::NotEqual, idx, neg_one);
        builder.ins().brif(found, found_block, &[], not_found_block, &[]);
        builder.seal_block(found_block);
        builder.seal_block(not_found_block);

        builder.switch_to_block(found_block);
        let stride_i64 = builder.ins().iconst(types::I64, stride as i64);
        let idx_i64 = builder.ins().uextend(types::I64, idx);
        let offset_bytes = builder.ins().imul(idx_i64, stride_i64);
        let bucket_addr = builder.ins().iadd(buckets_ptr, offset_bytes);
        let zero_i8 = builder.ins().iconst(types::I8, 0);
        builder.ins().store(flags, zero_i8, bucket_addr, 4);
        let len = self.cargar_campo_descriptor(builder, dict_ptr, Self::OFFSET_LEN);
        let uno_i64 = builder.ins().iconst(types::I64, 1);
        let new_len = builder.ins().isub(len, uno_i64);
        builder.ins().store(flags, new_len, dict_ptr, Self::OFFSET_LEN);
        let uno_ret = builder.ins().iconst(types::I32, 1);
        builder.ins().jump(merge_block, &[uno_ret]);

        builder.switch_to_block(not_found_block);
        let cero_ret = builder.ins().iconst(types::I32, 0);
        builder.ins().jump(merge_block, &[cero_ret]);

        builder.seal_block(merge_block);
        builder.switch_to_block(merge_block);
        Ok(builder.block_params(merge_block)[0])
    }

    pub(crate) fn builtin_diccionario_longitud(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let dict_ptr = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let len = self.cargar_campo_descriptor(builder, dict_ptr, Self::OFFSET_LEN);
        Ok(builder.ins().ireduce(types::I32, len))
    }

    pub(crate) fn builtin_diccionario_liberar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
        _tipo_args: &Vec<Tipo>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let dict_ptr = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let data = self.cargar_campo_descriptor(builder, dict_ptr, Self::OFFSET_PTR);
        self.llamar_free(builder, data);
        self.llamar_free(builder, dict_ptr);
        Ok(builder.ins().iconst(types::I32, 0))
    }

    // Conjunto<T> ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â wrapper de Diccionario<T, Booleano>
    pub(crate) fn builtin_conjunto_nuevo(
        &mut self,
        builder: &mut FunctionBuilder,
        _tipo_args: &Vec<Tipo>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        Ok(self.descriptor_nuevo(builder))
    }

    pub(crate) fn builtin_conjunto_insertar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
        tipo_args: &Vec<Tipo>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let mut dict_args = vec![argumentos[0].clone(), argumentos[1].clone()];
        dict_args.push(Expresion::Literal(crate::ast::Literal::Entero(1, crate::span::Span::vacio())));
        let dict_tipos = vec![tipo_args[0].clone(), Tipo::Booleano];
        self.builtin_diccionario_insertar(builder, variables, &dict_args, &dict_tipos)
    }

    pub(crate) fn builtin_conjunto_contiene(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
        tipo_args: &Vec<Tipo>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let dict_tipos = vec![tipo_args[0].clone(), Tipo::Booleano];
        self.builtin_diccionario_existe(builder, variables, argumentos, &dict_tipos)
    }

    pub(crate) fn builtin_conjunto_eliminar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
        tipo_args: &Vec<Tipo>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let dict_tipos = vec![tipo_args[0].clone(), Tipo::Booleano];
        self.builtin_diccionario_eliminar(builder, variables, argumentos, &dict_tipos)
    }

    pub(crate) fn builtin_conjunto_longitud(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        self.builtin_diccionario_longitud(builder, variables, argumentos)
    }

    pub(crate) fn builtin_conjunto_liberar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
        _tipo_args: &Vec<Tipo>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        self.builtin_diccionario_liberar(builder, variables, argumentos, _tipo_args)
    }

    /// diccionario_claves(d: Diccionario<K,V>) -> Vector<Texto>
    /// Extrae las claves del diccionario como vector de textos.
    pub(crate) fn builtin_diccionario_claves(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
        tipo_args: &[Tipo],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc_dict = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let desc_out = self.descriptor_nuevo(builder);

        // Tamaños de tipos
        let key_size = if tipo_args.len() > 0 { self.tamano_tipo(&tipo_args[0]) as i32 } else { 8 };
        let val_size = if tipo_args.len() > 1 { self.tamano_tipo(&tipo_args[1]) as i32 } else { 8 };

        let key_size_val = builder.ins().iconst(types::I32, key_size as i64);
        let val_size_val = builder.ins().iconst(types::I32, val_size as i64);

        let fn_id = self.asegurar_funcion_c(
            "falcato_diccionario_claves",
            &[types::I64, types::I64, types::I32, types::I32],
            None,
        );
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[desc_dict, desc_out, key_size_val, val_size_val]);

        Ok(desc_out)
    }

    /// diccionario_valores(d: Diccionario<K,V>) -> Vector<Texto>
    /// Extrae los valores del diccionario como vector de textos serializados.
    pub(crate) fn builtin_diccionario_valores(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
        tipo_args: &[Tipo],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc_dict = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let desc_out = self.descriptor_nuevo(builder);

        let key_size = if tipo_args.len() > 0 { self.tamano_tipo(&tipo_args[0]) as i32 } else { 8 };
        let val_size = if tipo_args.len() > 1 { self.tamano_tipo(&tipo_args[1]) as i32 } else { 8 };

        let key_size_val = builder.ins().iconst(types::I32, key_size as i64);
        let val_size_val = builder.ins().iconst(types::I32, val_size as i64);

        let fn_id = self.asegurar_funcion_c(
            "falcato_diccionario_valores",
            &[types::I64, types::I64, types::I32, types::I32],
            None,
        );
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[desc_dict, desc_out, key_size_val, val_size_val]);

        Ok(desc_out)
    }

    /// diccionario_limpiar(d: &mut Diccionario<K,V>)
    /// Vacía el diccionario sin deallocar.
    pub(crate) fn builtin_diccionario_limpiar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
        tipo_args: &[Tipo],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc_dict = self.compilar_expresion(&argumentos[0], builder, variables)?;

        let key_size = if tipo_args.len() > 0 { self.tamano_tipo(&tipo_args[0]) as i32 } else { 8 };
        let val_size = if tipo_args.len() > 1 { self.tamano_tipo(&tipo_args[1]) as i32 } else { 8 };

        let key_size_val = builder.ins().iconst(types::I32, key_size as i64);
        let val_size_val = builder.ins().iconst(types::I32, val_size as i64);

        let fn_id = self.asegurar_funcion_c(
            "falcato_diccionario_limpiar",
            &[types::I64, types::I32, types::I32],
            None,
        );
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[desc_dict, key_size_val, val_size_val]);

        Ok(builder.ins().iconst(types::I32, 0))
    }

    /// conjunto_elementos(c: Conjunto<T>) -> Vector<Texto>
    /// Extrae los elementos del conjunto como vector de textos.
    pub(crate) fn builtin_conjunto_elementos(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
        tipo_args: &[Tipo],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc_set = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let desc_out = self.descriptor_nuevo(builder);

        let key_size = if tipo_args.len() > 0 { self.tamano_tipo(&tipo_args[0]) as i32 } else { 8 };

        let key_size_val = builder.ins().iconst(types::I32, key_size as i64);

        let fn_id = self.asegurar_funcion_c(
            "falcato_conjunto_elementos",
            &[types::I64, types::I64, types::I32],
            None,
        );
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[desc_set, desc_out, key_size_val]);

        Ok(desc_out)
    }

    // ============================================================
    // Procesos (R7.1): lanzar comandos y capturar salida
    // ============================================================


}
