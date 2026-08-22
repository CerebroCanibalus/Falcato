use crate::codegen::*;

impl Codegen {
    pub(crate) fn builtin_como_entero64(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        self.builtin_conversion_numerica(argumentos, Tipo::Entero64, builder, variables)
    }

    /// R7.5 Fase 2: como_entero32(valor: Entero64) -> Entero32
    /// Trunca Entero64 a Entero32 (para parseo tipado de args).
    pub(crate) fn builtin_como_entero32(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        self.builtin_conversion_numerica(argumentos, Tipo::Entero32, builder, variables)
    }

    /// R9.0.3 — Conversión numérica completa.
    /// `builtin_conversion_numerica(argumentos, destino, ...)` convierte el primer argumento
    /// al tipo destino, manejando todas las combinaciones:
    /// - entero → entero: sextend/ireduce según ancho
    /// - flotante → entero: fcvt_to_sint (trunca hacia cero, como cast en C)
    /// - entero → flotante: fcvt_from_sint
    /// - flotante → flotante: fpromote/fdemote
    /// VITAL para WAV 16/24-bit (muestras Flotante64 → Entero16/32) y DSP.
    pub(crate) fn builtin_conversion_numerica(
        &mut self,
        argumentos: &Vec<Expresion>,
        destino: Tipo,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        // F-013 fix: `5000000000 largo` → como_entero64(literal grande). El literal 5000000000 no cabe en I32,
        // pero compilar_expresion lo trunca a I32 antes de sextend. Detectar literal y emitir I64 directo.
        if let Expresion::Literal(Literal::Entero(n, _)) = &argumentos[0] {
            let ir_destino = self.tipo_a_cranelift(&destino);
            // Solo para destinos enteros — flotantes usan fcvt
            if !matches!(ir_destino, types::F32 | types::F64) {
                return Ok(builder.ins().iconst(ir_destino, *n as i64));
            }
        }
        // También `como_entero64(-5000000000)` → Unaria Negación de literal grande
        if let Expresion::Unaria(OperadorUnario::Negacion, inner, _) = &argumentos[0] {
            if let Expresion::Literal(Literal::Entero(n, _)) = inner.as_ref() {
                let ir_destino = self.tipo_a_cranelift(&destino);
                if !matches!(ir_destino, types::F32 | types::F64) {
                    return Ok(builder.ins().iconst(ir_destino, -(*n as i64)));
                }
            }
        }
        let val = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let tipo_fuente = self.resolver_alias(&self.inferir_tipo(&argumentos[0], variables));
        let ir_destino = self.tipo_a_cranelift(&destino);

        let es_fuente_flotante = matches!(tipo_fuente, Tipo::Flotante32 | Tipo::Flotante64);
        let es_destino_flotante = matches!(destino, Tipo::Flotante32 | Tipo::Flotante64);

        match (es_fuente_flotante, es_destino_flotante) {
            // flotante → entero: truncar hacia cero (fcvt_to_sint)
            // fcvt_to_sint solo soporta I32/I64 como destino.
            // Para I8/I16: convertir a I32 y luego ireduce.
            (true, false) => {
                let ancho_destino = ir_destino.bits();
                if ancho_destino <= 32 {
                    // Convertir a I32 y luego truncar si es I8/I16
                    let val_i32 = builder.ins().fcvt_to_sint(types::I32, val);
                    if ancho_destino == 32 {
                        Ok(val_i32)
                    } else {
                        Ok(builder.ins().ireduce(ir_destino, val_i32))
                    }
                } else {
                    // I64: fcvt_to_sint directo
                    Ok(builder.ins().fcvt_to_sint(ir_destino, val))
                }
            }
            // entero → flotante
            (false, true) => Ok(builder.ins().fcvt_from_sint(ir_destino, val)),
            // flotante → flotante: promover o degradar
            (true, true) => {
                if ir_destino == types::F64 && self.tipo_a_cranelift(&tipo_fuente) == types::F32 {
                    Ok(builder.ins().fpromote(types::F64, val))
                } else if ir_destino == types::F32 && self.tipo_a_cranelift(&tipo_fuente) == types::F64 {
                    Ok(builder.ins().fdemote(types::F32, val))
                } else {
                    Ok(val)
                }
            }
            // entero → entero: extender o truncar
            (false, false) => {
                let ir_fuente = self.tipo_a_cranelift(&tipo_fuente);
                let ancho_fuente = ir_fuente.bits();
                let ancho_destino = ir_destino.bits();
                if ancho_destino > ancho_fuente {
                    Ok(builder.ins().sextend(ir_destino, val))
                } else if ancho_destino < ancho_fuente {
                    Ok(builder.ins().ireduce(ir_destino, val))
                } else {
                    Ok(val)
                }
            }
        }
    }

    /// R7.5 Fase 2: conversión de Texto a número (texto_a_entero/natural/flotante/booleano).
    /// Extrae (ptr, len) del descriptor Texto y llama a la función C del runtime
    /// `falcato_texto_a_*` que parsea respetando `len` (sin asumir null terminator).
    fn builtin_texto_convertir(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
        nombre_c: &str,
        retorno: cranelift_codegen::ir::types::Type,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let desc = self.compilar_expresion(&argumentos[0], builder, variables)?;
        let ptr = self.cargar_campo_descriptor(builder, desc, Self::OFFSET_PTR);
        let len = self.cargar_campo_descriptor(builder, desc, Self::OFFSET_LEN);
        let func_id = self.asegurar_funcion_c(nombre_c, &[types::I64, types::I64], Some(retorno));
        let func_ref = self.module.declare_func_in_func(func_id, builder.func);
        let inst = builder.ins().call(func_ref, &[ptr, len]);
        Ok(builder.func.dfg.first_result(inst))
    }

    /// R7.5 Fase 2: texto_a_entero(t: Texto) -> Entero64
    pub(crate) fn builtin_texto_a_entero(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        self.builtin_texto_convertir(builder, variables, argumentos, "falcato_texto_a_entero", types::I64)
    }

    /// R7.5 Fase 2: texto_a_natural(t: Texto) -> Entero64 (-1 si no es número)
    pub(crate) fn builtin_texto_a_natural(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        self.builtin_texto_convertir(builder, variables, argumentos, "falcato_texto_a_natural", types::I64)
    }

    /// R7.5 Fase 2: texto_a_flotante(t: Texto) -> Flotante64
    pub(crate) fn builtin_texto_a_flotante(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        self.builtin_texto_convertir(builder, variables, argumentos, "falcato_texto_a_flotante", types::F64)
    }

    /// R7.5 Fase 2: texto_a_booleano(t: Texto) -> Booleano (1/0)
    pub(crate) fn builtin_texto_a_booleano(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &Vec<Expresion>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        self.builtin_texto_convertir(builder, variables, argumentos, "falcato_texto_a_booleano", types::I64)
    }

    /// Fase 15D: archivo_leer(ruta: Palabra) -> Texto
    /// Lee un archivo completo. Retorna Texto vacÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â­o si no existe.

    // ============================================================
    // Conversión numérica (R7.8 FASE 3): número → texto
    // ============================================================

    /// entero_a_texto(n: Entero) -> Texto — convierte entero a texto decimal (polimórfico)
    pub(crate) fn builtin_entero_a_texto(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let n = self.compilar_expresion(&argumentos[0], builder, variables)?;
        // F-013: aceptar cualquier ancho (I8/I16/I32 → I64). Elegir sextend/uextend según signo.
        let n_type = builder.func.dfg.value_type(n);
        let n_i64 = if n_type != types::I64 {
            let tipo_src = self.inferir_tipo(&argumentos[0], variables);
            let es_signed = matches!(self.resolver_alias(&tipo_src), Tipo::Entero8 | Tipo::Entero16 | Tipo::Entero32 | Tipo::Entero64);
            if es_signed {
                builder.ins().sextend(types::I64, n)
            } else {
                builder.ins().uextend(types::I64, n)
            }
        } else {
            n
        };
        
        // Crear descriptor de salida
        let desc_out = self.descriptor_nuevo(builder);
        
        // falcato_entero_a_texto(n: i64, desc_out: i64) -> void
        let fn_id = self.asegurar_funcion_c("falcato_entero_a_texto", &[types::I64, types::I64], None);
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[n_i64, desc_out]);
        
        Ok(desc_out)
    }

    /// flotante_a_texto(f: Flotante64) -> Texto — convierte flotante a texto (%.17g)
    pub(crate) fn builtin_flotante_a_texto(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let f = self.compilar_expresion(&argumentos[0], builder, variables)?;
        
        // Crear descriptor de salida
        let desc_out = self.descriptor_nuevo(builder);
        
        // falcato_flotante_a_texto(f: f64, desc_out: i64) -> void
        // Nota: necesitamos convertir f64 a i64 (reinterpretación de bits) para pasar a la función C
        // Si f es F32 (corto), promover a F64 primero, luego bitcast
        let flags = cranelift_codegen::ir::MemFlags::new();
        let f_tipo = builder.func.dfg.value_type(f);
        let f_para_bitcast = if f_tipo == types::F32 {
            builder.ins().fpromote(types::F64, f)
        } else {
            f
        };
        let f_i64 = builder.ins().bitcast(types::I64, flags, f_para_bitcast);
        let fn_id = self.asegurar_funcion_c("falcato_flotante_a_texto", &[types::I64, types::I64], None);
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[f_i64, desc_out]);
        
        Ok(desc_out)
    }

    /// booleano_a_texto(b: Booleano) -> Texto — convierte booleano a "verdadero"/"falso"
    pub(crate) fn builtin_booleano_a_texto(
        &mut self,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
        argumentos: &[Expresion],
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let b = self.compilar_expresion(&argumentos[0], builder, variables)?;
        // Booleano es i8 en Falcato, necesitamos extenderlo a i32 para la función C
        let b_tipo = builder.func.dfg.value_type(b);
        let b_i32 = if b_tipo == types::I8 {
            builder.ins().uextend(types::I32, b)
        } else if b_tipo == types::I64 {
            builder.ins().ireduce(types::I32, b)
        } else {
            b
        };
        
        // Crear descriptor de salida
        let desc_out = self.descriptor_nuevo(builder);
        
        // falcato_booleano_a_texto(b: i32, desc_out: i64) -> void
        let fn_id = self.asegurar_funcion_c("falcato_booleano_a_texto", &[types::I32, types::I64], None);
        let fn_ref = self.module.declare_func_in_func(fn_id, builder.func);
        builder.ins().call(fn_ref, &[b_i32, desc_out]);
        
        Ok(desc_out)
    }

}
