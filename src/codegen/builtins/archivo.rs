use crate::codegen::*;

impl Codegen {
    pub(crate) fn builtin_archivo_leer(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let tipo_ruta = self.inferir_tipo(&argumentos[0], variables);
        let ruta_val = self.compilar_expresion(&argumentos[0], builder, variables)?;
        // 3.4: Palabra→Texto unificado — Palabra es I64 directo, Texto es descriptor
        let (c_ruta, _buf_ruta) = if tipo_ruta == crate::ast::Tipo::Texto {
            let ptr = self.cargar_campo_descriptor(builder, ruta_val, Self::OFFSET_PTR);
            let len = self.cargar_campo_descriptor(builder, ruta_val, Self::OFFSET_LEN);
            let uno = builder.ins().iconst(types::I64, 1);
            let cap = builder.ins().iadd(len, uno);
            let buf = self.llamar_malloc(builder, cap);
            self.llamar_memcpy(builder, buf, ptr, len);
            let cero = builder.ins().iconst(types::I8, 0);
            let fin = builder.ins().iadd(buf, len);
            builder.ins().store(cranelift_codegen::ir::MemFlags::new(), cero, fin, 0);
            (buf, Some(buf))
        } else {
            (ruta_val, None)
        };

        // fopen(ruta, "rb")
        let modo = self.crear_string_literal(builder, "rb");
        let fopen_id = self.asegurar_funcion_c("fopen", &[types::I64, types::I64], Some(types::I64));
        let fopen_ref = self.module.declare_func_in_func(fopen_id, builder.func);
        let call_fopen = builder.ins().call(fopen_ref, &[c_ruta, modo]);
        let file = builder.inst_results(call_fopen)[0];

        // if file == NULL ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ descriptor vacÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â­o, else leer contenido
        let cero_64 = builder.ins().iconst(types::I64, 0);
        let es_nulo = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::Equal, file, cero_64);

        let bloque_nulo = builder.create_block();
        let bloque_ok = builder.create_block();
        let merge = builder.create_block();

        // Variable para el descriptor resultado
        let var_desc = self.nueva_variable();
        builder.declare_var(var_desc, types::I64);

        builder.ins().brif(es_nulo, bloque_nulo, &[], bloque_ok, &[]);

        // bloque_nulo: descriptor vacÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â­o
        builder.switch_to_block(bloque_nulo);
        let desc_vacio = self.descriptor_nuevo(builder);
        builder.def_var(var_desc, desc_vacio);
        builder.ins().jump(merge, &[]);
        builder.seal_block(bloque_nulo);

        // bloque_ok: leer archivo
        builder.switch_to_block(bloque_ok);

        // fseek(file, 0, SEEK_END)
        let seek_end = builder.ins().iconst(types::I32, 2);
        let cero_32 = builder.ins().iconst(types::I32, 0);
        let fseek_id = self.asegurar_funcion_c("fseek", &[types::I64, types::I32, types::I32], Some(types::I32));
        let fseek_ref = self.module.declare_func_in_func(fseek_id, builder.func);
        builder.ins().call(fseek_ref, &[file, cero_32, seek_end]);

        // ftell(file) ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ tamaÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â±o
        let ftell_id = self.asegurar_funcion_c("ftell", &[types::I64], Some(types::I64));
        let ftell_ref = self.module.declare_func_in_func(ftell_id, builder.func);
        let call_ftell = builder.ins().call(ftell_ref, &[file]);
        let tamano = builder.inst_results(call_ftell)[0];

        // fseek(file, 0, SEEK_SET)
        let seek_set = builder.ins().iconst(types::I32, 0);
        let cero_32b = builder.ins().iconst(types::I32, 0);
        builder.ins().call(fseek_ref, &[file, cero_32b, seek_set]);

        // malloc(tamano + 1)
        let uno = builder.ins().iconst(types::I64, 1);
        let cap = builder.ins().iadd(tamano, uno);
        let data = self.llamar_malloc(builder, cap);

        // fread(data, 1, tamano, file)
        let fread_id = self.asegurar_funcion_c("fread", &[types::I64, types::I64, types::I64, types::I64], Some(types::I64));
        let fread_ref = self.module.declare_func_in_func(fread_id, builder.func);
        builder.ins().call(fread_ref, &[data, uno, tamano, file]);

        // data[tamano] = 0
        let null_pos = builder.ins().iadd(data, tamano);
        let cero_8 = builder.ins().iconst(types::I8, 0);
        builder.ins().store(cranelift_codegen::ir::MemFlags::new(), cero_8, null_pos, 0);

        // fclose(file)
        let fclose_id = self.asegurar_funcion_c("fclose", &[types::I64], Some(types::I32));
        let fclose_ref = self.module.declare_func_in_func(fclose_id, builder.func);
        builder.ins().call(fclose_ref, &[file]);

        // 3.4: liberar buffer C-string temporal si se alocó
        if let Some(buf) = _buf_ruta { self.llamar_free(builder, buf); }

        // Crear descriptor Texto
        let desc_ok = self.descriptor_nuevo(builder);
        self.guardar_campo_descriptor(builder, desc_ok, Self::OFFSET_PTR, data);
        self.guardar_campo_descriptor(builder, desc_ok, Self::OFFSET_LEN, tamano);
        self.guardar_campo_descriptor(builder, desc_ok, Self::OFFSET_CAP, cap);
        builder.def_var(var_desc, desc_ok);
        builder.ins().jump(merge, &[]);
        builder.seal_block(bloque_ok);

        // merge
        builder.switch_to_block(merge);
        let resultado = builder.use_var(var_desc);
        builder.seal_block(merge);

        Ok(resultado)
    }

    /// Fase 15D: archivo_escribir(ruta: Palabra, contenido: Texto) -> Entero32
    /// Escribe contenido a archivo. Retorna 0 si OK, -1 si error.
    pub(crate) fn builtin_archivo_escribir(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let tipo_ruta = self.inferir_tipo(&argumentos[0], variables);
        let ruta_val = self.compilar_expresion(&argumentos[0], builder, variables)?;
        // 3.4: Palabra→Texto unificado — Palabra es I64 directo, Texto es descriptor
        let (c_ruta, _buf_ruta) = if tipo_ruta == crate::ast::Tipo::Texto {
            let ptr = self.cargar_campo_descriptor(builder, ruta_val, Self::OFFSET_PTR);
            let len = self.cargar_campo_descriptor(builder, ruta_val, Self::OFFSET_LEN);
            let uno = builder.ins().iconst(types::I64, 1);
            let cap = builder.ins().iadd(len, uno);
            let buf = self.llamar_malloc(builder, cap);
            self.llamar_memcpy(builder, buf, ptr, len);
            let cero = builder.ins().iconst(types::I8, 0);
            let fin = builder.ins().iadd(buf, len);
            builder.ins().store(cranelift_codegen::ir::MemFlags::new(), cero, fin, 0);
            (buf, Some(buf))
        } else {
            (ruta_val, None)
        };
        let desc = self.compilar_expresion(&argumentos[1], builder, variables)?;

        let ptr = self.cargar_campo_descriptor(builder, desc, Self::OFFSET_PTR);
        let len = self.cargar_campo_descriptor(builder, desc, Self::OFFSET_LEN);

        // fopen(ruta, "wb")
        let modo = self.crear_string_literal(builder, "wb");
        let fopen_id = self.asegurar_funcion_c("fopen", &[types::I64, types::I64], Some(types::I64));
        let fopen_ref = self.module.declare_func_in_func(fopen_id, builder.func);
        let call_fopen = builder.ins().call(fopen_ref, &[c_ruta, modo]);
        let file = builder.inst_results(call_fopen)[0];

        // if file == NULL ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ retornar -1
        let cero_64 = builder.ins().iconst(types::I64, 0);
        let es_nulo = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::Equal, file, cero_64);
        let menos_uno = builder.ins().iconst(types::I32, -1);
        let cero_32 = builder.ins().iconst(types::I32, 0);

        let bloque_error = builder.create_block();
        let bloque_ok = builder.create_block();
        let merge = builder.create_block();
        builder.ins().brif(es_nulo, bloque_error, &[], bloque_ok, &[]);

        // bloque_error: retornar -1
        builder.switch_to_block(bloque_error);
        builder.ins().jump(merge, &[]);
        builder.seal_block(bloque_error);

        // bloque_ok: fwrite(ptr, 1, len, file) + fclose
        builder.switch_to_block(bloque_ok);
        let uno = builder.ins().iconst(types::I64, 1);
        let fwrite_id = self.asegurar_funcion_c("fwrite", &[types::I64, types::I64, types::I64, types::I64], Some(types::I64));
        let fwrite_ref = self.module.declare_func_in_func(fwrite_id, builder.func);
        builder.ins().call(fwrite_ref, &[ptr, uno, len, file]);

        let fclose_id = self.asegurar_funcion_c("fclose", &[types::I64], Some(types::I32));
        let fclose_ref = self.module.declare_func_in_func(fclose_id, builder.func);
        builder.ins().call(fclose_ref, &[file]);
        builder.ins().jump(merge, &[]);
        builder.seal_block(bloque_ok);

        // merge: select(es_nulo, -1, 0)
        builder.switch_to_block(merge);
        // 3.4: liberar buffer C-string temporal si se alocó (en merge, no en bloque lleno)
        if let Some(buf) = _buf_ruta { self.llamar_free(builder, buf); }
        let resultado = builder.ins().select(es_nulo, menos_uno, cero_32);
        builder.seal_block(merge);

        Ok(resultado)
    }

    /// Fase 15D: archivo_existe(ruta: Palabra) -> Booleano
    /// Verifica si un archivo existe. Retorna I8 (0 o 1).
    pub(crate) fn builtin_archivo_existe(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let tipo_ruta = self.inferir_tipo(&argumentos[0], variables);
        let ruta_val = self.compilar_expresion(&argumentos[0], builder, variables)?;
        // 3.4: Palabra→Texto unificado — Palabra es I64 directo, Texto es descriptor
        let (c_ruta, _buf_ruta) = if tipo_ruta == crate::ast::Tipo::Texto {
            let ptr = self.cargar_campo_descriptor(builder, ruta_val, Self::OFFSET_PTR);
            let len = self.cargar_campo_descriptor(builder, ruta_val, Self::OFFSET_LEN);
            let uno = builder.ins().iconst(types::I64, 1);
            let cap = builder.ins().iadd(len, uno);
            let buf = self.llamar_malloc(builder, cap);
            self.llamar_memcpy(builder, buf, ptr, len);
            let cero = builder.ins().iconst(types::I8, 0);
            let fin = builder.ins().iadd(buf, len);
            builder.ins().store(cranelift_codegen::ir::MemFlags::new(), cero, fin, 0);
            (buf, Some(buf))
        } else {
            (ruta_val, None)
        };

        // fopen(ruta, "rb")
        let modo = self.crear_string_literal(builder, "rb");
        let fopen_id = self.asegurar_funcion_c("fopen", &[types::I64, types::I64], Some(types::I64));
        let fopen_ref = self.module.declare_func_in_func(fopen_id, builder.func);
        let call_fopen = builder.ins().call(fopen_ref, &[c_ruta, modo]);
        let file = builder.inst_results(call_fopen)[0];

        // if file != NULL ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ fclose + retornar 1, else retornar 0
        let cero_64 = builder.ins().iconst(types::I64, 0);
        let no_nulo = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::NotEqual, file, cero_64);

        let bloque_existe = builder.create_block();
        let bloque_no = builder.create_block();
        let merge = builder.create_block();
        builder.ins().brif(no_nulo, bloque_existe, &[], bloque_no, &[]);

        // bloque_existe: fclose(file), resultado = 1
        builder.switch_to_block(bloque_existe);
        let fclose_id = self.asegurar_funcion_c("fclose", &[types::I64], Some(types::I32));
        let fclose_ref = self.module.declare_func_in_func(fclose_id, builder.func);
        builder.ins().call(fclose_ref, &[file]);
        builder.ins().jump(merge, &[]);
        builder.seal_block(bloque_existe);

        // bloque_no: resultado = 0
        builder.switch_to_block(bloque_no);
        builder.ins().jump(merge, &[]);
        builder.seal_block(bloque_no);

        // merge: select(no_nulo, 1, 0) como I8
        builder.switch_to_block(merge);
        // 3.4: liberar buffer C-string temporal si se alocó (en merge)
        if let Some(buf) = _buf_ruta { self.llamar_free(builder, buf); }
        let uno_8 = builder.ins().iconst(types::I8, 1);
        let cero_8 = builder.ins().iconst(types::I8, 0);
        let resultado = builder.ins().select(no_nulo, uno_8, cero_8);
        builder.seal_block(merge);

        Ok(resultado)
    }

    // ============================================================
    // Fase 15E: MatemÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¡ticas
    // ============================================================


    // ============================================================
    // Archivos avanzados + entorno (R7.8 FASE 4)
    // ============================================================

    /// archivo_agregar(ruta: Texto, texto: Texto) — append a archivo
    pub(crate) fn builtin_archivo_agregar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let ruta = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let texto = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let fn_id = self.asegurar_funcion_c("falcato_archivo_agregar", &[types::I64, types::I64], None);
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[ruta, texto]);
        Ok(builder.ins().iconst(types::I32, 0))
    }

    /// archivo_borrar(ruta: Texto) — eliminar archivo
    pub(crate) fn builtin_archivo_borrar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let ruta = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let fn_id = self.asegurar_funcion_c("falcato_archivo_borrar", &[types::I64], None);
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[ruta]);
        Ok(builder.ins().iconst(types::I32, 0))
    }

    /// archivo_renombrar(vieja: Texto, nueva: Texto) — mover/renombrar
    pub(crate) fn builtin_archivo_renombrar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let vieja = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let nueva = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let fn_id = self.asegurar_funcion_c("falcato_archivo_renombrar", &[types::I64, types::I64], None);
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[vieja, nueva]);
        Ok(builder.ins().iconst(types::I32, 0))
    }

    /// archivo_escribir_bytes(ruta: Texto, datos: Entero64, n: Entero32) — escribir bytes crudos
    pub(crate) fn builtin_archivo_escribir_bytes(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let ruta = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let datos = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let n = self.compilar_expresion(&argumentos[2], builder, variables)?;
        let n_i32 = if builder.func.dfg.value_type(n) == types::I64 {
            builder.ins().ireduce(types::I32, n)
        } else {
            n
        };
        let fn_id = self.asegurar_funcion_c("falcato_archivo_escribir_bytes", &[types::I64, types::I64, types::I32], None);
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[ruta, datos, n_i32]);
        Ok(builder.ins().iconst(types::I32, 0))
    }

    /// archivo_listar(dir: Texto) -> Vector<Texto> — listar directorio
    pub(crate) fn builtin_archivo_listar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let tipo_dir = self.inferir_tipo(&argumentos[0], variables);
        let dir_val = self.compilar_expresion(&argumentos[0], builder, variables)?;
        // F-016/F-019 — Palabra literal → Texto: convertir Palabra (ptr C) a Texto descriptor
        let mut data_a_liberar: Option<cranelift_codegen::ir::Value> = None;
        let dir = if tipo_dir == crate::ast::Tipo::Texto {
            dir_val
        } else {
            // Palabra (I64 ptr a string null-terminada) → Texto descriptor
            let len = self.llamar_strlen(builder, dir_val);
            let uno = builder.ins().iconst(types::I64, 1);
            let cap = builder.ins().iadd(len, uno);
            let data = self.llamar_malloc(builder, cap);
            self.llamar_memcpy(builder, data, dir_val, cap);
            // data ya tiene null-terminator por memcpy de cap (incluye \0)
            let desc = self.descriptor_nuevo(builder);
            self.guardar_campo_descriptor(builder, desc, Self::OFFSET_PTR, data);
            self.guardar_campo_descriptor(builder, desc, Self::OFFSET_LEN, len);
            self.guardar_campo_descriptor(builder, desc, Self::OFFSET_CAP, cap);
            data_a_liberar = Some(data);
            desc
        };
        let desc_out = self.descriptor_nuevo(builder);
        let fn_id = self.asegurar_funcion_c("falcato_archivo_listar", &[types::I64, types::I64], None);
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[dir, desc_out]);
        // Liberar buffer temporal de la ruta literal (no el descriptor de salida)
        if let Some(data) = data_a_liberar {
            self.llamar_free(builder, data);
        }
        Ok(desc_out)
    }

}
