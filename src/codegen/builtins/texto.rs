use crate::codegen::*;

impl Codegen {
    pub(crate) fn builtin_texto_nuevo(
        &mut self,
        builder: &mut FunctionBuilder,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        // Texto vacío REAL: descriptor con ptr a un buffer de 1 byte ('\0'),
        // len=0, cap=1. printf("%s") con NULL imprimiría "(null)".
        let desc = self.descriptor_nuevo(builder);
        let uno = builder.ins().iconst(types::I64, 1);
        let data = self.llamar_malloc(builder, uno);
        let cero_byte = builder.ins().iconst(types::I8, 0);
        builder.ins().store(cranelift_codegen::ir::MemFlags::new(), cero_byte, data, 0);
        let flags = cranelift_codegen::ir::MemFlags::new();
        builder.ins().store(flags, data, desc, Self::OFFSET_PTR);
        builder.ins().store(flags, uno, desc, Self::OFFSET_CAP);
        Ok(desc)
    }

    pub(crate) fn builtin_texto_desde(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let src = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let len = self.llamar_strlen(builder, src);
        let uno = builder.ins().iconst(types::I64, 1);
        let cap = builder.ins().iadd(len, uno);

        let data = self.llamar_malloc(builder, cap);
        self.llamar_memcpy(builder, data, src, cap);

        let desc = self.descriptor_nuevo(builder);
        self.guardar_campo_descriptor(builder, desc, Self::OFFSET_PTR, data);
        self.guardar_campo_descriptor(builder, desc, Self::OFFSET_LEN, len);
        self.guardar_campo_descriptor(builder, desc, Self::OFFSET_CAP, cap);
        Ok(desc)
    }

    pub(crate) fn builtin_texto_agregar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let src = self.compilar_expresion(&argumentos[1], builder, variables)?;

        let data = self.cargar_campo_descriptor(builder, desc, Self::OFFSET_PTR);
        let len_t = self.cargar_campo_descriptor(builder, desc, Self::OFFSET_LEN);
        let cap = self.cargar_campo_descriptor(builder, desc, Self::OFFSET_CAP);
        let len_s = self.llamar_strlen(builder, src);
        let uno = builder.ins().iconst(types::I64, 1);
        let temp_len = builder.ins().iadd(len_t, len_s);
        let new_len = builder.ins().iadd(temp_len, uno);

        // Si no cabe, realloc
        let necesita_realloc = builder.ins().icmp(
            cranelift_codegen::ir::condcodes::IntCC::SignedLessThanOrEqual,
            cap,
            new_len,
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

        // then: realloc
        builder.switch_to_block(then_block);
        let dos = builder.ins().iconst(types::I64, 2);
        let new_cap = builder.ins().imul(dos, new_len);
        let data_var_val = builder.use_var(data_var);
        let data_then = self.llamar_realloc(builder, data_var_val, new_cap);
        builder.def_var(data_var, data_then);
        builder.def_var(cap_var, new_cap);
        builder.ins().jump(merge_block, &[]);
        builder.seal_block(then_block);

        // merge
        builder.switch_to_block(merge_block);
        let data_final = builder.use_var(data_var);
        let cap_final = builder.use_var(cap_var);
        builder.seal_block(merge_block);

        let offset = builder.ins().iadd(data_final, len_t);
        let copy_len = builder.ins().iadd(len_s, uno);
        self.llamar_memcpy(builder, offset, src, copy_len);

        let nueva_longitud = builder.ins().iadd(len_t, len_s);
        self.guardar_campo_descriptor(builder, desc, Self::OFFSET_PTR, data_final);
        self.guardar_campo_descriptor(builder, desc, Self::OFFSET_LEN, nueva_longitud);
        self.guardar_campo_descriptor(builder, desc, Self::OFFSET_CAP, cap_final);

        Ok(builder.ins().iconst(types::I32, 0))
    }

    pub(crate) fn builtin_texto_longitud(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let len = self.cargar_campo_descriptor(builder, desc, Self::OFFSET_LEN);
        Ok(builder.ins().ireduce(types::I32, len))
    }

    pub(crate) fn builtin_texto_liberar(
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

    /// Fase 15C: texto_concatenar(a: Texto, b: Texto) -> Texto
    /// Crea un nuevo Texto con a + b (no modifica los originales).
    pub(crate) fn builtin_texto_concatenar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc_a = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let desc_b = self.compilar_expresion(&argumentos[1], builder, variables)?;

        let ptr_a = self.cargar_campo_descriptor(builder, desc_a, Self::OFFSET_PTR);
        let len_a = self.cargar_campo_descriptor(builder, desc_a, Self::OFFSET_LEN);
        let ptr_b = self.cargar_campo_descriptor(builder, desc_b, Self::OFFSET_PTR);
        let len_b = self.cargar_campo_descriptor(builder, desc_b, Self::OFFSET_LEN);

        // new_len = len_a + len_b
        let new_len = builder.ins().iadd(len_a, len_b);
        // cap = new_len + 1 (null terminator)
        let uno = builder.ins().iconst(types::I64, 1);
        let cap = builder.ins().iadd(new_len, uno);

        // malloc(cap)
        let data = self.llamar_malloc(builder, cap);

        // memcpy(data, ptr_a, len_a)
        self.llamar_memcpy(builder, data, ptr_a, len_a);

        // memcpy(data + len_a, ptr_b, len_b + 1) ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â incluye null terminator de b
        let dest_b = builder.ins().iadd(data, len_a);
        let copy_b_len = builder.ins().iadd(len_b, uno);
        self.llamar_memcpy(builder, dest_b, ptr_b, copy_b_len);

        // Crear descriptor
        let desc = self.descriptor_nuevo(builder);
        self.guardar_campo_descriptor(builder, desc, Self::OFFSET_PTR, data);
        self.guardar_campo_descriptor(builder, desc, Self::OFFSET_LEN, new_len);
        self.guardar_campo_descriptor(builder, desc, Self::OFFSET_CAP, cap);
        Ok(desc)
    }

    /// Fase 15C: texto_subtexto(t: Texto, inicio: Entero32, fin: Entero32) -> Texto
    /// Extrae bytes [inicio, fin) como nuevo Texto.
    pub(crate) fn builtin_texto_subtexto(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let inicio = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let fin = self.compilar_expresion(&argumentos[2], builder, variables)?;

        let ptr = self.cargar_campo_descriptor(builder, desc, Self::OFFSET_PTR);

        // sub_len = fin - inicio (como i64)
        let inicio_64 = builder.ins().sextend(types::I64, inicio);
        let fin_64 = builder.ins().sextend(types::I64, fin);
        let sub_len = builder.ins().isub(fin_64, inicio_64);

        // cap = sub_len + 1
        let uno = builder.ins().iconst(types::I64, 1);
        let cap = builder.ins().iadd(sub_len, uno);

        // malloc(cap)
        let data = self.llamar_malloc(builder, cap);

        // memcpy(data, ptr + inicio, sub_len)
        let src = builder.ins().iadd(ptr, inicio_64);
        self.llamar_memcpy(builder, data, src, sub_len);

        // data[sub_len] = 0 (null terminator)
        let null_pos = builder.ins().iadd(data, sub_len);
        let cero = builder.ins().iconst(types::I8, 0);
        builder.ins().store(cranelift_codegen::ir::MemFlags::new(), cero, null_pos, 0);

        // Crear descriptor
        let desc_nuevo = self.descriptor_nuevo(builder);
        self.guardar_campo_descriptor(builder, desc_nuevo, Self::OFFSET_PTR, data);
        self.guardar_campo_descriptor(builder, desc_nuevo, Self::OFFSET_LEN, sub_len);
        self.guardar_campo_descriptor(builder, desc_nuevo, Self::OFFSET_CAP, cap);
        Ok(desc_nuevo)
    }

    /// Fase 15C: texto_comparar(a: Texto, b: Texto) -> Entero32
    /// Compara byte a byte. Retorna 0 si iguales, <0 si a<b, >0 si a>b.
    pub(crate) fn builtin_texto_comparar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc_a = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let desc_b = self.compilar_expresion(&argumentos[1], builder, variables)?;

        let ptr_a = self.cargar_campo_descriptor(builder, desc_a, Self::OFFSET_PTR);
        let len_a = self.cargar_campo_descriptor(builder, desc_a, Self::OFFSET_LEN);
        let ptr_b = self.cargar_campo_descriptor(builder, desc_b, Self::OFFSET_PTR);
        let len_b = self.cargar_campo_descriptor(builder, desc_b, Self::OFFSET_LEN);

        // min_len = min(len_a, len_b)
        let a_menor = builder.ins().icmp(
            cranelift_codegen::ir::condcodes::IntCC::SignedLessThan,
            len_a, len_b,
        );
        let min_len = builder.ins().select(a_menor, len_a, len_b);

        // Loop: for i in 0..min_len { if a[i] != b[i] return a[i] - b[i] }
        let header = builder.create_block();
        let body = builder.create_block();
        let next_block = builder.create_block();
        let done = builder.create_block();

        let var_i = self.nueva_variable();
        builder.declare_var(var_i, types::I64);
        let cero = builder.ins().iconst(types::I64, 0);
        builder.def_var(var_i, cero);
        builder.ins().jump(header, &[]);

        // header: if i < min_len goto body else goto done
        builder.switch_to_block(header);
        let i = builder.use_var(var_i);
        let cond = builder.ins().icmp(
            cranelift_codegen::ir::condcodes::IntCC::SignedLessThan,
            i, min_len,
        );
        builder.ins().brif(cond, body, &[], done, &[]);
        // NO sellar header aquÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â­ ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â falta el back-edge desde next_block

        // body: comparar bytes
        builder.switch_to_block(body);
        let i_body = builder.use_var(var_i);
        let addr_a = builder.ins().iadd(ptr_a, i_body);
        let addr_b = builder.ins().iadd(ptr_b, i_body);
        let byte_a = builder.ins().load(types::I8, cranelift_codegen::ir::MemFlags::new(), addr_a, 0);
        let byte_b = builder.ins().load(types::I8, cranelift_codegen::ir::MemFlags::new(), addr_b, 0);
        let byte_a_32 = builder.ins().uextend(types::I32, byte_a);
        let byte_b_32 = builder.ins().uextend(types::I32, byte_b);
        let iguales = builder.ins().icmp(
            cranelift_codegen::ir::condcodes::IntCC::Equal,
            byte_a_32, byte_b_32,
        );
        // si iguales ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ next_block (i++), si no ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ done (bytes difieren)
        builder.ins().brif(iguales, next_block, &[], done, &[]);
        builder.seal_block(body);

        // next_block: i++ y volver al header
        builder.switch_to_block(next_block);
        let i_next = builder.use_var(var_i);
        let uno = builder.ins().iconst(types::I64, 1);
        let i_mas = builder.ins().iadd(i_next, uno);
        builder.def_var(var_i, i_mas);
        builder.ins().jump(header, &[]);
        builder.seal_block(next_block);

        // AHORA sÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â­ sellar header (back-edge completo)
        builder.seal_block(header);

        // done: determinar resultado
        builder.switch_to_block(done);
        let i_final = builder.use_var(var_i);
        let salio_early = builder.ins().icmp(
            cranelift_codegen::ir::condcodes::IntCC::SignedLessThan,
            i_final, min_len,
        );
        // Si early exit: return byte_a[i] - byte_b[i]
        let addr_a_f = builder.ins().iadd(ptr_a, i_final);
        let addr_b_f = builder.ins().iadd(ptr_b, i_final);
        let ba = builder.ins().load(types::I8, cranelift_codegen::ir::MemFlags::new(), addr_a_f, 0);
        let bb = builder.ins().load(types::I8, cranelift_codegen::ir::MemFlags::new(), addr_b_f, 0);
        let ba_32 = builder.ins().uextend(types::I32, ba);
        let bb_32 = builder.ins().uextend(types::I32, bb);
        let diff = builder.ins().isub(ba_32, bb_32);
        // Si no early: return len_a - len_b (como i32)
        let len_a_32 = builder.ins().ireduce(types::I32, len_a);
        let len_b_32 = builder.ins().ireduce(types::I32, len_b);
        let len_diff = builder.ins().isub(len_a_32, len_b_32);
        let resultado = builder.ins().select(salio_early, diff, len_diff);
        builder.seal_block(done);

        Ok(resultado)
    }


    /// texto_igual(a: Texto, b: Texto) -> Booleano (I8: 1 true, 0 false)
    /// Compara byte a byte (auto-contenido, no reutiliza texto_comparar).
    /// Devuelve 1 si a == b, 0 si difieren.
    /// Implementación:
    ///   - si len_a != len_b Æ— falso
    ///   - si no, comparar byte por byte; cualquier diferencia Æ— falso
    ///   - si todos iguales Æ— verdadero
    pub(crate) fn builtin_texto_igual(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc_a = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let desc_b = self.compilar_expresion(&argumentos[1], builder, variables)?;

        let len_a = self.cargar_campo_descriptor(builder, desc_a, Self::OFFSET_LEN);
        let len_b = self.cargar_campo_descriptor(builder, desc_b, Self::OFFSET_LEN);

        // Si len_a != len_b Æ— falso (short-circuit)
        let len_eq = builder.ins().icmp(
            cranelift_codegen::ir::condcodes::IntCC::Equal,
            len_a, len_b,
        );
        let check_bytes = builder.create_block();
        let final_block = builder.create_block();
        let merge = builder.create_block();
        builder.append_block_param(merge, types::I8);
        builder.ins().brif(len_eq, check_bytes, &[], final_block, &[]);
        builder.seal_block(check_bytes);
        builder.seal_block(final_block);

        // Final: devolver 0 (falso)
        builder.switch_to_block(final_block);
        let cero = builder.ins().iconst(types::I8, 0);
        builder.ins().jump(merge, &[cero]);

        // Check bytes: loop byte por byte
        builder.switch_to_block(check_bytes);
        let ptr_a = self.cargar_campo_descriptor(builder, desc_a, Self::OFFSET_PTR);
        let ptr_b = self.cargar_campo_descriptor(builder, desc_b, Self::OFFSET_PTR);

        // min_len = min(len_a, len_b)
        let a_menor = builder.ins().icmp(
            cranelift_codegen::ir::condcodes::IntCC::SignedLessThan,
            len_a, len_b,
        );
        let min_len = builder.ins().select(a_menor, len_a, len_b);

        // Loop: for i in 0..min_len { if a[i] != b[i] Æ— falso }
        let header = builder.create_block();
        let body = builder.create_block();
        let next_block = builder.create_block();
        let iguales_block = builder.create_block();

        let var_i = self.nueva_variable();
        builder.declare_var(var_i, types::I64);
        let cero_i64 = builder.ins().iconst(types::I64, 0);
        builder.def_var(var_i, cero_i64);
        builder.ins().jump(header, &[]);

        builder.switch_to_block(header);
        let i = builder.use_var(var_i);
        let cond = builder.ins().icmp(
            cranelift_codegen::ir::condcodes::IntCC::SignedLessThan,
            i, min_len,
        );
        builder.ins().brif(cond, body, &[], iguales_block, &[]);
        // NO sellar header (back-edge desde next_block)

        builder.switch_to_block(body);
        let i_body = builder.use_var(var_i);
        let addr_a = builder.ins().iadd(ptr_a, i_body);
        let addr_b = builder.ins().iadd(ptr_b, i_body);
        let byte_a = builder.ins().load(types::I8, cranelift_codegen::ir::MemFlags::new(), addr_a, 0);
        let byte_b = builder.ins().load(types::I8, cranelift_codegen::ir::MemFlags::new(), addr_b, 0);
        let iguales = builder.ins().icmp(
            cranelift_codegen::ir::condcodes::IntCC::Equal,
            byte_a, byte_b,
        );
        builder.ins().brif(iguales, next_block, &[], final_block, &[]);
        builder.seal_block(body);

        builder.switch_to_block(next_block);
        let i_next = builder.use_var(var_i);
        let uno = builder.ins().iconst(types::I64, 1);
        let i_mas = builder.ins().iadd(i_next, uno);
        builder.def_var(var_i, i_mas);
        builder.ins().jump(header, &[]);
        builder.seal_block(next_block);

        // AHORA sellar header (back-edge completo)
        builder.seal_block(header);

        // Iguales: devolver 1 (verdadero)
        builder.switch_to_block(iguales_block);
        let uno_i8 = builder.ins().iconst(types::I8, 1);
        builder.ins().jump(merge, &[uno_i8]);
        builder.seal_block(iguales_block);

        builder.seal_block(merge);
        builder.switch_to_block(merge);
        Ok(builder.block_params(merge)[0])
    }

    /// texto_desigual(a: Texto, b: Texto) -> Booleano (I8: 1 true, 0 false)
    /// Inverso de `texto_igual`. Devuelve 1 si a != b, 0 si son iguales.
    /// Reutiliza texto_igual + negación Booleana (seguro porque texto_igual
    /// termina en merge con Booleano, no reutiliza control flow externo).
    pub(crate) fn builtin_texto_desigual(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let eq = self.builtin_texto_igual(builder, variables, argumentos)?;
        // eq es Booleano (I8: 1 true, 0 false). Negar.
        let uno = builder.ins().iconst(types::I8, 1);
        let ne = builder.ins().isub(uno, eq);
        Ok(ne)
    }


    /// Fase 15C: texto_obtener_byte(t: Texto, indice: Entero32) -> Entero8
    /// Retorna el byte en la posiciÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â³n dada.
    pub(crate) fn builtin_texto_obtener_byte(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let indice = self.compilar_expresion(&argumentos[1], builder, variables)?;

        let ptr = self.cargar_campo_descriptor(builder, desc, Self::OFFSET_PTR);
        let idx_64 = builder.ins().sextend(types::I64, indice);
        let addr = builder.ins().iadd(ptr, idx_64);
        let byte = builder.ins().load(types::I8, cranelift_codegen::ir::MemFlags::new(), addr, 0);
        Ok(byte)
    }

    /// Fase GUI-1: texto_a_puntero(texto: Palabra) -> Entero64
    /// Retorna la direcciÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â³n de memoria de un literal de cadena.
    /// ÃƒÆ’Ã†â€™Ãƒâ€¦Ã‚Â¡til para pasar punteros a string en structs FFI (ej: WNDCLASSEXA).
    pub(crate) fn builtin_texto_a_puntero(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let ptr = self.compilar_expresion(&argumentos[0], builder, variables)?;
        Ok(ptr)
    }

    /// Fase GUI-1: como_entero64(valor: Entero32) -> Entero64
    /// Extiende Entero32 a Entero64 con signo. Para pasar NULL (0) como puntero en FFI.

    // ============================================================
    // Texto dinámico (R7.8 FASE 2): operaciones eficientes sobre strings
    // ============================================================

    /// texto_agregar_texto(texto: &mut Texto, fragmento: Texto) — append con realloc eficiente
    pub(crate) fn builtin_texto_agregar_texto(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        // Primer argumento: referencia mutable a Texto
        // En Falcato, Texto es un descriptor en el HEAP (ptr a ptr, len, cap).
        // &mut base nos da la dirección del stack slot, que contiene el puntero al descriptor.
        // Necesitamos cargar ese puntero para pasarlo a la función del runtime.
        let desc_ptr = if let Expresion::Unaria(crate::ast::OperadorUnario::ReferenciaMut, inner, _) = &argumentos[0] {
            if let Expresion::Identificador(nombre, _) = inner.as_ref() {
                if let Some((slot, _tipo, _articulo)) = variables.get(nombre) {
                    // Cargar el puntero al descriptor desde el stack slot
                    builder.ins().stack_load(types::I64, *slot, 0)
                } else {
                    return Err(());
                }
            } else {
                return Err(());
            }
        } else {
            return Err(());
        };
        
        // Segundo argumento: Texto — también es un puntero al descriptor en el heap
        let frag_desc_ptr = if let Expresion::Identificador(nombre, _) = &argumentos[1] {
            if let Some((slot, _tipo, _articulo)) = variables.get(nombre) {
                // Cargar el puntero al descriptor desde el stack slot
                builder.ins().stack_load(types::I64, *slot, 0)
            } else {
                return Err(());
            }
        } else {
            self.compilar_expresion(&argumentos[1], builder, variables)?
        };
        
        // falcato_texto_agregar_texto(desc: i64, frag_desc: i64) -> void
        let fn_id = self.asegurar_funcion_c("falcato_texto_agregar_texto", &[types::I64, types::I64], None);
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[desc_ptr, frag_desc_ptr]);
        Ok(builder.ins().iconst(types::I32, 0))
    }

    /// texto_poner_byte(texto: &mut Texto, indice: Entero32, byte: Entero32) — mutación in-place
    pub(crate) fn builtin_texto_poner_byte(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        // Primer argumento: referencia mutable a Texto — cargar el puntero al descriptor
        let desc_ptr = if let Expresion::Unaria(crate::ast::OperadorUnario::ReferenciaMut, inner, _) = &argumentos[0] {
            if let Expresion::Identificador(nombre, _) = inner.as_ref() {
                if let Some((slot, _tipo, _articulo)) = variables.get(nombre) {
                    builder.ins().stack_load(types::I64, *slot, 0)
                } else {
                    return Err(());
                }
            } else {
                return Err(());
            }
        } else {
            return Err(());
        };
        
        let indice = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let indice_i32 = if builder.func.dfg.value_type(indice) == types::I64 {
            builder.ins().ireduce(types::I32, indice)
        } else {
            indice
        };
        let byte = self.compilar_expresion(&argumentos[2], builder, variables)?;
        let byte_i32 = if builder.func.dfg.value_type(byte) == types::I64 {
            builder.ins().ireduce(types::I32, byte)
        } else {
            byte
        };
        
        // falcato_texto_poner_byte(desc: i64, i: i32, b: i32) -> void
        let fn_id = self.asegurar_funcion_c("falcato_texto_poner_byte", &[types::I64, types::I32, types::I32], None);
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[desc_ptr, indice_i32, byte_i32]);
        Ok(builder.ins().iconst(types::I32, 0))
    }

    /// texto_puntero(texto: Texto) -> Entero64 — ptr interno del Texto
    pub(crate) fn builtin_texto_puntero(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc = self.compilar_expresion(&argumentos[0], builder, variables)?;
        // falcato_texto_puntero(desc: i64) -> i64
        let fn_id = self.asegurar_funcion_c("falcato_texto_puntero", &[types::I64], Some(types::I64));
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        let call = builder.ins().call(fn_ref, &[desc]);
        Ok(builder.inst_results(call)[0])
    }

    /// texto_desde_bytes(ptr: Entero64, longitud: Entero32) -> Texto — construir desde buffer crudo
    pub(crate) fn builtin_texto_desde_bytes(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let ptr = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let longitud = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let longitud_i32 = if builder.func.dfg.value_type(longitud) == types::I64 {
            builder.ins().ireduce(types::I32, longitud)
        } else {
            longitud
        };
        
        // Crear descriptor de salida en stack
        let desc_out = self.descriptor_nuevo(builder);
        
        // falcato_texto_desde_bytes(ptr: i64, n: i32, desc_out: i64) -> void
        let fn_id = self.asegurar_funcion_c("falcato_texto_desde_bytes", &[types::I64, types::I32, types::I64], None);
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[ptr, longitud_i32, desc_out]);
        
        Ok(desc_out)
    }

    // === libEst builtins ===

    /// texto_contiene(texto: Texto, sub: Texto) -> Booleano
    /// Busca si `sub` aparece dentro de `texto`.
    pub(crate) fn builtin_texto_contiene(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc_texto = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let desc_sub = self.compilar_expresion(&argumentos[1], builder, variables)?;

        // falcato_texto_contiene(desc_texto: i64, desc_sub: i64) -> i32
        let fn_id = self.asegurar_funcion_c("falcato_texto_contiene", &[types::I64, types::I64], Some(types::I32));
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        let call = builder.ins().call(fn_ref, &[desc_texto, desc_sub]);
        Ok(builder.inst_results(call)[0])
    }

    /// texto_reemplazar(texto: Texto, de: Texto, a: Texto) -> Texto
    /// Reemplaza todas las ocurrencias de `de` por `a`.
    pub(crate) fn builtin_texto_reemplazar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc_texto = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let desc_de = self.compilar_expresion(&argumentos[1], builder, variables)?;
        let desc_a = self.compilar_expresion(&argumentos[2], builder, variables)?;

        let desc_out = self.descriptor_nuevo(builder);

        // falcato_texto_reemplazar(desc_texto, desc_de, desc_a, desc_out) -> void
        let fn_id = self.asegurar_funcion_c("falcato_texto_reemplazar", &[types::I64, types::I64, types::I64, types::I64], None);
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[desc_texto, desc_de, desc_a, desc_out]);

        Ok(desc_out)
    }

    /// texto_mayusculas(texto: Texto) -> Texto
    /// Convierte a mayúsculas (ASCII).
    pub(crate) fn builtin_texto_mayusculas(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc_texto = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let desc_out = self.descriptor_nuevo(builder);

        // falcato_texto_mayusculas(desc_texto, desc_out) -> void
        let fn_id = self.asegurar_funcion_c("falcato_texto_mayusculas", &[types::I64, types::I64], None);
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[desc_texto, desc_out]);

        Ok(desc_out)
    }

    /// texto_minusculas(texto: Texto) -> Texto
    /// Convierte a minúsculas (ASCII).
    pub(crate) fn builtin_texto_minusculas(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc_texto = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let desc_out = self.descriptor_nuevo(builder);

        // falcato_texto_minusculas(desc_texto, desc_out) -> void
        let fn_id = self.asegurar_funcion_c("falcato_texto_minusculas", &[types::I64, types::I64], None);
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[desc_texto, desc_out]);

        Ok(desc_out)
    }

    /// texto_recortar(texto: Texto) -> Texto
    /// Recorta espacios en blanco al inicio y final.
    pub(crate) fn builtin_texto_recortar(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc_texto = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let desc_out = self.descriptor_nuevo(builder);

        // falcato_texto_recortar(desc_texto, desc_out) -> void
        let fn_id = self.asegurar_funcion_c("falcato_texto_recortar", &[types::I64, types::I64], None);
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[desc_texto, desc_out]);

        Ok(desc_out)
    }

    /// texto_empieza_con(texto: Texto, prefijo: Texto) -> Booleano
    pub(crate) fn builtin_texto_empieza_con(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc_texto = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let desc_prefijo = self.compilar_expresion(&argumentos[1], builder, variables)?;

        let fn_id = self.asegurar_funcion_c("falcato_texto_empieza_con", &[types::I64, types::I64], Some(types::I32));
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        let call = builder.ins().call(fn_ref, &[desc_texto, desc_prefijo]);
        Ok(builder.inst_results(call)[0])
    }

    /// texto_termina_con(texto: Texto, sufijo: Texto) -> Booleano
    pub(crate) fn builtin_texto_termina_con(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc_texto = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let desc_sufijo = self.compilar_expresion(&argumentos[1], builder, variables)?;

        let fn_id = self.asegurar_funcion_c("falcato_texto_termina_con", &[types::I64, types::I64], Some(types::I32));
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        let call = builder.ins().call(fn_ref, &[desc_texto, desc_sufijo]);
        Ok(builder.inst_results(call)[0])
    }

    /// texto_codificar_base64(texto: Texto) -> Texto
    pub(crate) fn builtin_texto_codificar_base64(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc_texto = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let desc_out = self.descriptor_nuevo(builder);

        let fn_id = self.asegurar_funcion_c("falcato_texto_codificar_base64", &[types::I64, types::I64], None);
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[desc_texto, desc_out]);

        Ok(desc_out)
    }

    /// texto_decodificar_base64(texto: Texto) -> Texto
    pub(crate) fn builtin_texto_decodificar_base64(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc_texto = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let desc_out = self.descriptor_nuevo(builder);

        let fn_id = self.asegurar_funcion_c("falcato_texto_decodificar_base64", &[types::I64, types::I64], None);
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[desc_texto, desc_out]);

        Ok(desc_out)
    }

    /// texto_a_bytes(texto: Texto) -> Vector<Entero8>
    pub(crate) fn builtin_texto_a_bytes(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc_texto = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let desc_out = self.descriptor_nuevo(builder);

        let fn_id = self.asegurar_funcion_c("falcato_texto_a_bytes", &[types::I64, types::I64], None);
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[desc_texto, desc_out]);

        Ok(desc_out)
    }

    /// texto_dividir(texto: Texto, sep: Texto) -> Vector<Texto>
    /// Divide el texto por el separador.
    pub(crate) fn builtin_texto_dividir(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc_texto = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let desc_sep = self.compilar_expresion(&argumentos[1], builder, variables)?;

        // Crear descriptor de Vector vacío
        let desc_vector = self.descriptor_nuevo(builder);
        let cero = builder.ins().iconst(types::I64, 0);
        self.guardar_campo_descriptor(builder, desc_vector, Self::OFFSET_PTR, cero);
        self.guardar_campo_descriptor(builder, desc_vector, Self::OFFSET_LEN, cero);
        self.guardar_campo_descriptor(builder, desc_vector, Self::OFFSET_CAP, cero);

        // falcato_texto_dividir(desc_texto, desc_sep, desc_vector) -> void
        let fn_id = self.asegurar_funcion_c(
            "falcato_texto_dividir",
            &[types::I64, types::I64, types::I64],
            None,
        );
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[desc_texto, desc_sep, desc_vector]);

        Ok(desc_vector)
    }

}
