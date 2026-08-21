//! Funciones — compilar_funcion, compilar_funcion_futuro, compilar_funcion_normal
//! State machine async (fut función), métodos, bitwise methods

use super::*;

impl Codegen {
    pub(crate) fn compilar_funcion(&mut self,
        func: &FuncionDecl,
    ) -> Result<(), ()> {
        // Si es FFI sin cuerpo, no compilar
        if func.es_insegura && func.cuerpo.sentencias.is_empty() {
            return Ok(());
        }

        // Fase 18E: fut funciÃ³n â†’ state machine transform
        if func.es_futuro {
            return self.compilar_funcion_futuro(func);
        }

        let mut ctx = self.module.make_context();
        let mut func_ctx = FunctionBuilderContext::new();

        // Crear firma para la funciÃ³n
        let mut sig = Signature::new(self.call_conv_default());
        // R9.0.1 — Retorno de struct → sret (puntero oculto como primer parámetro).
        let ret_es_struct = match &func.retorno {
            Some(ret) => self.tipo_es_struct(ret).is_some(),
            None => false,
        };
        if let Some(ref ret) = func.retorno {
            if !ret_es_struct {
                sig.returns.push(AbiParam::new(self.tipo_a_cranelift(ret)));
            }
        }
        if ret_es_struct {
            sig.params.push(AbiParam::new(types::I64));
        }
        for param in &func.parametros {
            if self.tipo_es_struct(&param.tipo).is_some() {
                sig.params.push(AbiParam::new(types::I64)); // por referencia
            } else {
                sig.params.push(AbiParam::new(self.tipo_a_cranelift(&param.tipo)));
            }
        }

        // Asignar firma al contexto
        ctx.func.signature = sig.clone();

        // Crear builder
        let mut builder = FunctionBuilder::new(
            &mut ctx.func,
            &mut func_ctx
        );

        let entry_block = builder.create_block();

        // Añadir parámetros al bloque de entrada ANTES de cualquier instrucción
        // R9.0.1: el sret ptr (si hay) es el primero; los params struct son I64 (ptr)
        let mut num_params_ir = 0usize;
        if ret_es_struct {
            builder.append_block_param(entry_block, types::I64);
            num_params_ir += 1;
        }
        for param in &func.parametros {
            if self.tipo_es_struct(&param.tipo).is_some() {
                builder.append_block_param(entry_block, types::I64);
            } else {
                builder.append_block_param(entry_block, self.tipo_a_cranelift(&param.tipo));
            }
            num_params_ir += 1;
        }

        builder.switch_to_block(entry_block);
        builder.seal_block(entry_block);

        // Variables locales: nombre â†’ (slot, tipo, artÃ­culo)
        let mut variables: HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)> = HashMap::new();

        // R6: Drop automÃ¡tico — estado limpio por funciÃ³n
        self.heap_vivas.clear();
        self.scope_marcas.clear();

        // R9.0.1 — guardar el sret ptr para `retornar struct`
        self.sret_destino = if ret_es_struct {
            Some(builder.block_params(entry_block)[0])
        } else {
            None
        };

        // 3.1b — instalar manejador de panics si es principal (auto-flush + stack trace)
        if func.nombre == "principal" {
            let func_id = self.asegurar_funcion_c("falcato_instalar_manejador_panico", &[], None);
            let func_ref = self.module.declare_func_in_func(func_id, builder.func);
            builder.ins().call(func_ref, &[]);
            // Memoria debug — lente graduable (nivel 0=off)
            if self.nivel_memoria > 0 {
                let mem_id = self.asegurar_funcion_c("falcato_memoria_init", &[types::I8], None);
                let mem_ref = self.module.declare_func_in_func(mem_id, builder.func);
                let nivel_val = builder.ins().iconst(types::I8, self.nivel_memoria as i64);
                builder.ins().call(mem_ref, &[nivel_val]);
            }
        }

        // ParÃ¡metros como variables
        for (i, param) in func.parametros.iter().enumerate() {
            // +1 si hay sret (el primer block_param es el ptr oculto)
            let idx_ir = i + if ret_es_struct { 1 } else { 0 };
            let val = builder.block_params(entry_block)[idx_ir];
            let tamano = self.tamano_tipo(&param.tipo);
            let slot = builder.create_sized_stack_slot(
                cranelift_codegen::ir::StackSlotData::new(
                    cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                    tamano,
                    0,
                )
            );
            if self.tipo_es_struct(&param.tipo).is_some() {
                // R9.0.1 — param struct llega como puntero → copiar al slot local
                let slot_ptr = builder.ins().stack_addr(types::I64, slot, 0);
                self.copiar_mem(slot_ptr, val, tamano, &mut builder);
            } else {
                builder.ins().stack_store(val, slot, 0);
            }
            variables.insert(param.nombre.clone(), (slot, param.tipo.clone(), param.articulo));
            // R6: parÃ¡metro owned (el/los) de tipo heap → el callee es dueÃ±o → liberar al final
            if matches!(param.articulo, crate::ast::Articulo::El | crate::ast::Articulo::Los) {
                self.registrar_heap(&param.nombre, &param.tipo);
            }
        }

        // Compilar sentencias
        for sentencia in &func.cuerpo.sentencias {
            self.compilar_sentencia(
                sentencia,
                &mut builder,
                &mut variables,
                &func.span,
            )?;
        }

        // R6: Drop automÃ¡tico — liberar variables heap owned vivas al final de la funciÃ³n
        // (las movidas/liberadas/retornadas ya se quitaron de heap_vivas durante la compilaciÃ³n)
        self.liberar_scope(0, &mut builder, &variables)?;

        // Si no hay retorno explÃ­cito, aÃ±adir retorno void
        if func.retorno.is_none() {
            builder.ins().return_(&[]);
        }

        builder.finalize();

        // Definir funciÃ³n en el mÃ³dulo (usar func_id existente o declarar nueva)
        let func_id = match self.funciones.get(&func.nombre).copied() {
            Some(id) => id,
            None => {
                self.errores.agregar(ErrorCompilador::nuevo(
                    CategoriaError::Interno,
                    10,
                    func.span.clone(),
                    format!("FunciÃ³n '{}' no declarada previamente", func.nombre),
                ));
                return Err(());
            }
        };

        self.definir_funcion(func_id, &mut ctx, &func.nombre)
            .map_err(|e| {
                self.errores.agregar(ErrorCompilador::nuevo(
                    CategoriaError::Interno,
                    10,
                    func.span.clone(),
                    format!("Error definiendo función: {}", e),
                ));
            })?;

        Ok(())
    }

    // ============================================================
    // Fase 18E: State machine transform para fut funciÃ³n
    // ============================================================

    /// Compila una `fut funciÃ³n` como state machine poll-based.
    /// Genera:
    /// - `__init_NOMBRE(args...) -> i64`: malloc struct + init state=0 + store params
    /// - `__poll_NOMBRE(ptr: i64) -> i64`: switch on state, returns 0=Pending, 1=Ready
    /// - `NOMBRE(args...) -> T`: wrapper que hace init + poll loop (para uso sync)
    fn compilar_funcion_futuro(&mut self, func: &FuncionDecl) -> Result<(), ()> {
        use crate::futuros;

        let analisis = futuros::analizar_futuro(func);
        let tamano_struct = futuros::tamano_struct_futuro(&analisis);
        let off_deadline = futuros::offset_deadline(&analisis);

        // Si no hay puntos de suspensiÃ³n, compilar normal (no necesita state machine)
        if analisis.num_estados <= 1 {
            return self.compilar_funcion_normal(func);
        }

        // --- 1. Generar __init_NOMBRE ---
        self.generar_init_futuro(func, &analisis, tamano_struct)?;

        // --- 2. Generar __poll_NOMBRE ---
        self.generar_poll_futuro(func, &analisis, tamano_struct, off_deadline)?;

        // --- 3. Generar wrapper sync NOMBRE (init + poll loop con Sleep(1)) ---
        self.generar_wrapper_sync_futuro(func, &analisis)?;

        Ok(())
    }

    /// Genera `__init_NOMBRE(args...) -> i64`
    fn generar_init_futuro(
        &mut self,
        func: &FuncionDecl,
        analisis: &crate::futuros::AnalisisFuturo,
        tamano_struct: u32,
    ) -> Result<(), ()> {
        let nombre_init = format!("__init_{}", func.nombre);

        // Declarar la funciÃ³n en el mÃ³dulo
        let mut sig = Signature::new(self.call_conv_default());
        for param in &func.parametros {
            sig.params.push(AbiParam::new(self.tipo_a_cranelift(&param.tipo)));
        }
        sig.returns.push(AbiParam::new(types::I64)); // retorna ptr

        let func_id = self.module.declare_function(&nombre_init, Linkage::Local, &sig)
            .expect("declarar __init");
        self.funciones.insert(nombre_init.clone(), func_id);

        let mut ctx = self.module.make_context();
        let mut func_ctx = FunctionBuilderContext::new();
        ctx.func.signature = sig;

        let mut builder = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);
        let entry = builder.create_block();

        for param in &func.parametros {
            builder.append_block_param(entry, self.tipo_a_cranelift(&param.tipo));
        }

        builder.switch_to_block(entry);
        builder.seal_block(entry);

        // malloc(tamano_struct)
        let tam = builder.ins().iconst(types::I64, tamano_struct as i64);
        let ptr = self.llamar_malloc(&mut builder, tam);

        // state = 0 (offset 0, i32)
        let cero = builder.ins().iconst(types::I32, 0);
        builder.ins().store(cranelift_codegen::ir::MemFlags::new(), cero, ptr, 0);

        // deadline = 0 (offset off_deadline, i64)
        let cero64 = builder.ins().iconst(types::I64, 0);
        let off_dl = crate::futuros::offset_deadline(analisis);
        builder.ins().store(cranelift_codegen::ir::MemFlags::new(), cero64, ptr, off_dl as i32);

        // Store params en el struct
        let block_params = builder.block_params(entry).to_vec();
        for (i, param) in func.parametros.iter().enumerate() {
            if let Some(offset) = crate::futuros::offset_var(analisis, &param.nombre) {
                let val = block_params[i];
                builder.ins().store(cranelift_codegen::ir::MemFlags::new(), val, ptr, offset as i32);
            }
        }

        builder.ins().return_(&[ptr]);
        builder.finalize();

        self.definir_funcion(func_id, &mut ctx, &nombre_init).map_err(|e| {
            self.errores.agregar(ErrorCompilador::nuevo(
                CategoriaError::Interno, 10, func.span.clone(),
                format!("Error definiendo __init: {}", e),
            ));
        })?;

        Ok(())
    }

    /// Genera `__poll_NOMBRE(ptr: i64) -> i64`
    /// Returns 0 = Pending, 1 = Ready
    fn generar_poll_futuro(
        &mut self,
        func: &FuncionDecl,
        analisis: &crate::futuros::AnalisisFuturo,
        _tamano_struct: u32,
        off_deadline: u32,
    ) -> Result<(), ()> {
        let nombre_poll = format!("__poll_{}", func.nombre);

        let mut sig = Signature::new(self.call_conv_default());
        sig.params.push(AbiParam::new(types::I64)); // ptr
        sig.returns.push(AbiParam::new(types::I64)); // 0=Pending, 1=Ready

        let func_id = self.module.declare_function(&nombre_poll, Linkage::Local, &sig)
            .expect("declarar __poll");
        self.funciones.insert(nombre_poll.clone(), func_id);

        let mut ctx = self.module.make_context();
        let mut func_ctx = FunctionBuilderContext::new();
        ctx.func.signature = sig;

        let mut builder = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);
        let entry = builder.create_block();
        builder.append_block_param(entry, types::I64); // ptr
        builder.switch_to_block(entry);
        builder.seal_block(entry);

        let ptr = builder.block_params(entry)[0];

        // Load state (offset 0, i32)
        let state = builder.ins().load(types::I32, cranelift_codegen::ir::MemFlags::new(), ptr, 0);

        // Crear bloques para cada estado + bloque de retorno
        let num_estados = analisis.num_estados;
        let mut bloques_estado: Vec<cranelift_codegen::ir::Block> = Vec::new();
        for _ in 0..num_estados {
            bloques_estado.push(builder.create_block());
        }
        let bloque_ready = builder.create_block();

        // Switch on state: cadena de if/else con sellado inmediato
        // Cada bloque tiene exactly 1 predecesor â†’ sellar inmediato es seguro
        // NO sellar ningÃºn bloque dos veces.
        let cero_i32 = builder.ins().iconst(types::I32, 0);
        let es_cero = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::Equal, state, cero_i32);
        let bloque_check1 = builder.create_block();
        builder.ins().brif(es_cero, bloques_estado[0], &[], bloque_check1, &[]);

        // Checks para estados 1..N-1
        builder.switch_to_block(bloque_check1);
        builder.seal_block(bloque_check1); // 1 predecesor: entry
        for i in 1..num_estados {
            let val_i = builder.ins().iconst(types::I32, i as i64);
            let es_i = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::Equal, state, val_i);
            if i < num_estados - 1 {
                let siguiente_check = builder.create_block();
                builder.ins().brif(es_i, bloques_estado[i], &[], siguiente_check, &[]);
                builder.switch_to_block(siguiente_check);
                builder.seal_block(siguiente_check); // 1 predecesor: check anterior
            } else {
                builder.ins().brif(es_i, bloques_estado[i], &[], bloque_ready, &[]);
            }
        }

        // Compilar cada estado
        for (estado_idx, segmento) in analisis.segmentos.iter().enumerate() {
            builder.switch_to_block(bloques_estado[estado_idx]);
            builder.seal_block(bloques_estado[estado_idx]); // 1 predecesor: dispatch chain

            // Para estados > 0: verificar timer (deadline)
            if estado_idx > 0 {
                let now = {
                    let mut ctx = self.ctx();
                    let runtime = platform::current_runtime();
                    runtime.timestamp(&mut ctx, &mut builder)
                };
                let deadline = builder.ins().load(types::I64, cranelift_codegen::ir::MemFlags::new(), ptr, off_deadline as i32);
                let listo = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::UnsignedGreaterThanOrEqual, now, deadline);
                let bloque_continuar = builder.create_block();
                let bloque_pending = builder.create_block();
                builder.ins().brif(listo, bloque_continuar, &[], bloque_pending, &[]);

                // Pending: return 0 (1 predecesor)
                builder.switch_to_block(bloque_pending);
                builder.seal_block(bloque_pending);
                let ret_cero = builder.ins().iconst(types::I64, 0);
                builder.ins().return_(&[ret_cero]);

                // Continuar (1 predecesor)
                builder.switch_to_block(bloque_continuar);
                builder.seal_block(bloque_continuar);
            }

            // Cargar variables del struct a stack slots locales
            let mut variables: HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)> = HashMap::new();

            for var in &analisis.vars_struct {
                let offset = crate::futuros::offset_var(analisis, &var.nombre).unwrap_or(0);
                let tipo_cranelift = self.tipo_a_cranelift(&var.tipo);
                let tamano = self.tamano_tipo(&var.tipo);
                let slot = builder.create_sized_stack_slot(
                    cranelift_codegen::ir::StackSlotData::new(
                        cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                        tamano,
                        0,
                    )
                );
                let val = builder.ins().load(tipo_cranelift, cranelift_codegen::ir::MemFlags::new(), ptr, offset as i32);
                builder.ins().stack_store(val, slot, 0);
                variables.insert(var.nombre.clone(), (slot, var.tipo.clone(), Articulo::El));
            }

            // Compilar sentencias del segmento (filtrar Retornar â€” el poll maneja sus propios returns)
            for sentencia in segmento {
                if matches!(sentencia, Sentencia::Retornar(_, _)) {
                    continue;
                }
                self.compilar_sentencia(sentencia, &mut builder, &mut variables, &func.span)?;
            }

            // Si hay una suspensiÃ³n despuÃ©s de este estado:
            if estado_idx < analisis.suspensiones.len() {
                let susp = &analisis.suspensiones[estado_idx];

                // Extraer el valor de ms de dormir(ms)
                let ms_val = self.extraer_ms_de_suspension(&susp.expresion, &mut builder, &mut variables)?;

                // deadline = GetTickCount64() + ms
                let now = {
                    let mut ctx = self.ctx();
                    let runtime = platform::current_runtime();
                    runtime.timestamp(&mut ctx, &mut builder)
                };
                let deadline_val = builder.ins().iadd(now, ms_val);
                builder.ins().store(cranelift_codegen::ir::MemFlags::new(), deadline_val, ptr, off_deadline as i32);

                // Guardar variables locales de vuelta al struct
                for var in &analisis.vars_struct {
                    if let Some((slot, _, _)) = variables.get(&var.nombre) {
                        let slot = *slot;
                        let offset = crate::futuros::offset_var(analisis, &var.nombre).unwrap_or(0);
                        let tipo_cranelift = self.tipo_a_cranelift(&var.tipo);
                        let val = builder.ins().stack_load(tipo_cranelift, slot, 0);
                        builder.ins().store(cranelift_codegen::ir::MemFlags::new(), val, ptr, offset as i32);
                    }
                }

                // state = estado_idx + 1
                let nuevo_state = builder.ins().iconst(types::I32, (estado_idx + 1) as i64);
                builder.ins().store(cranelift_codegen::ir::MemFlags::new(), nuevo_state, ptr, 0);

                // return 0 (Pending)
                let ret_cero = builder.ins().iconst(types::I64, 0);
                builder.ins().return_(&[ret_cero]);
            } else {
                // Ãšltimo estado: return 1 (Ready)
                let ret_uno = builder.ins().iconst(types::I64, 1);
                builder.ins().return_(&[ret_uno]);
            }
        }

        // Bloque ready (default) â€” 1 predecesor: Ãºltimo check
        builder.switch_to_block(bloque_ready);
        builder.seal_block(bloque_ready);
        let ret_uno = builder.ins().iconst(types::I64, 1);
        builder.ins().return_(&[ret_uno]);

        builder.finalize();

        self.definir_funcion(func_id, &mut ctx, &func.nombre).map_err(|e| {
            self.errores.agregar(ErrorCompilador::nuevo(
                CategoriaError::Interno, 10, func.span.clone(),
                format!("Error definiendo __poll: {}", e),
            ));
        })?;

        Ok(())
    }

    /// Genera wrapper sync: `NOMBRE(args) -> T` que hace init + poll loop
    fn generar_wrapper_sync_futuro(
        &mut self,
        func: &FuncionDecl,
        _analisis: &crate::futuros::AnalisisFuturo,
    ) -> Result<(), ()> {
        let nombre_init = format!("__init_{}", func.nombre);
        let nombre_poll = format!("__poll_{}", func.nombre);

        let mut ctx = self.module.make_context();
        let mut func_ctx = FunctionBuilderContext::new();

        let mut sig = Signature::new(self.call_conv_default());
        for param in &func.parametros {
            sig.params.push(AbiParam::new(self.tipo_a_cranelift(&param.tipo)));
        }
        if let Some(ref ret) = func.retorno {
            sig.returns.push(AbiParam::new(self.tipo_a_cranelift(ret)));
        }
        ctx.func.signature = sig;

        let mut builder = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);
        let entry = builder.create_block();

        for param in &func.parametros {
            builder.append_block_param(entry, self.tipo_a_cranelift(&param.tipo));
        }

        builder.switch_to_block(entry);
        builder.seal_block(entry);

        let block_params = builder.block_params(entry).to_vec();

        // Llamar __init(args...)
        let init_id = *self.funciones.get(&nombre_init).unwrap();
        let init_ref = self.module.declare_func_in_func(init_id, builder.func);
        let init_call = builder.ins().call(init_ref, &block_params);
        let fut_ptr = builder.inst_results(init_call)[0];

        // Poll loop: while __poll(ptr) == 0 { Sleep(1); }
        let _poll_id = *self.funciones.get(&nombre_poll).unwrap();
        // Poll loop: while __poll(ptr) == 0 { Sleep(1); }
        let poll_id = *self.funciones.get(&nombre_poll).unwrap();

        let bloque_loop = builder.create_block();
        let bloque_check = builder.create_block();
        let bloque_done = builder.create_block();

        builder.ins().jump(bloque_check, &[]);

        // Check: result = __poll(ptr); if result == 0 goto loop else done
        builder.switch_to_block(bloque_check);
        let poll_ref = self.module.declare_func_in_func(poll_id, builder.func);
        let poll_call = builder.ins().call(poll_ref, &[fut_ptr]);
        let poll_result = builder.inst_results(poll_call)[0];
        let cero64 = builder.ins().iconst(types::I64, 0);
        let es_pending = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::Equal, poll_result, cero64);
        builder.ins().brif(es_pending, bloque_loop, &[], bloque_done, &[]);
        // NO sellar bloque_check aquÃ­ â€” tiene 2 predecesores (entry + loop back-edge)

        // Loop body: Sleep(1) + jump back to check
        builder.switch_to_block(bloque_loop);
        let uno32 = builder.ins().iconst(types::I32, 1);
        self.platform_call_void("sleep", &mut builder, &[uno32]);
        builder.ins().jump(bloque_check, &[]);
        builder.seal_block(bloque_loop); // 1 predecesor: check

        // Ahora sÃ­ sellar check (2 predecesores: entry + loop)
        builder.seal_block(bloque_check);

        // Done: free(ptr) + return
        builder.switch_to_block(bloque_done);
        builder.seal_block(bloque_done);
        self.llamar_free(&mut builder, fut_ptr);

        if func.retorno.is_some() {
            // TODO: cargar resultado del struct antes de free
            let dummy = builder.ins().iconst(types::I32, 0);
            builder.ins().return_(&[dummy]);
        } else {
            builder.ins().return_(&[]);
        }

        builder.finalize();

        let func_id = *self.funciones.get(&func.nombre).unwrap();
        self.definir_funcion(func_id, &mut ctx, &func.nombre).map_err(|e| {
            self.errores.agregar(ErrorCompilador::nuevo(
                CategoriaError::Interno, 10, func.span.clone(),
                format!("Error definiendo wrapper sync: {}", e),
            ));
        })?;

        Ok(())
    }

    /// Extrae el valor de ms de una expresiÃ³n de suspensiÃ³n (dormir(ms))
    // ============================================================
    // Fase 15A: MÃ©todos bitwise en enteros
    // ============================================================

    pub(crate) fn compilar_metodo(
        &mut self,
        receptor: &Expresion,
        nombre: &str,
        args: &[Expresion],
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        use crate::ast::Tipo;
        
        // Inferir tipo del receptor
        let tipo_receptor = self.inferir_tipo(receptor, variables);
        
        // Si es mÃ©todo de tipo (Texto, Vector), desugar a llamada built-in
        match &tipo_receptor {
            Tipo::Texto => {
                let builtin = match nombre {
                    "agregar" => Some("texto_agregar"),
                    "tam" => Some("texto_longitud"),
                    "liberar" => Some("texto_liberar"),
                    "obtener" => Some("texto_obtener_byte"),
                    "concatenar" => Some("texto_concatenar"),
                    "subtexto" => Some("texto_subtexto"),
                    "comparar" => Some("texto_comparar"),
                    _ => None,
                };
                if let Some(func) = builtin {
                    let mut argumentos = vec![receptor.clone()];
                    argumentos.extend(args.iter().cloned());
                    let llamada = Llamada {
                        funcion: func.to_string(),
                        tipo_args: vec![],
                        argumentos,
                        span: receptor.span().clone(),
                    };
                    return self.compilar_llamada(&llamada, builder, variables);
                }
                // Fallback a bitwise
                self.compilar_metodo_bitwise(receptor, nombre, args, builder, variables)
            }
            Tipo::Vector(_) => {
                let builtin = match nombre {
                    "agregar" => Some("vector_agregar"),
                    "tam" => Some("vector_longitud"),
                    "obtener" => Some("vector_obtener"),
                    "liberar" => Some("vector_liberar"),
                    _ => None,
                };
                if let Some(func) = builtin {
                    let mut argumentos = vec![receptor.clone()];
                    argumentos.extend(args.iter().cloned());
                    let llamada = Llamada {
                        funcion: func.to_string(),
                        tipo_args: vec![],
                        argumentos,
                        span: receptor.span().clone(),
                    };
                    return self.compilar_llamada(&llamada, builder, variables);
                }
                // Fallback a bitwise
                self.compilar_metodo_bitwise(receptor, nombre, args, builder, variables)
            }
            Tipo::Diccionario(_, _) => {
                let builtin = match nombre {
                    "insertar" => Some("diccionario_insertar"),
                    "obtener" => Some("diccionario_obtener"),
                    "existe" => Some("diccionario_existe"),
                    "eliminar" => Some("diccionario_eliminar"),
                    "tam" => Some("diccionario_longitud"),
                    "liberar" => Some("diccionario_liberar"),
                    _ => None,
                };
                if let Some(func) = builtin {
                    let mut argumentos = vec![receptor.clone()];
                    argumentos.extend(args.iter().cloned());
                    let llamada = Llamada {
                        funcion: func.to_string(),
                        tipo_args: vec![],
                        argumentos,
                        span: receptor.span().clone(),
                    };
                    return self.compilar_llamada(&llamada, builder, variables);
                }
                self.compilar_metodo_bitwise(receptor, nombre, args, builder, variables)
            }
            Tipo::Conjunto(_) => {
                let builtin = match nombre {
                    "insertar" => Some("conjunto_insertar"),
                    "contiene" => Some("conjunto_contiene"),
                    "eliminar" => Some("conjunto_eliminar"),
                    "tam" => Some("conjunto_longitud"),
                    "liberar" => Some("conjunto_liberar"),
                    _ => None,
                };
                if let Some(func) = builtin {
                    let mut argumentos = vec![receptor.clone()];
                    argumentos.extend(args.iter().cloned());
                    let llamada = Llamada {
                        funcion: func.to_string(),
                        tipo_args: vec![],
                        argumentos,
                        span: receptor.span().clone(),
                    };
                    return self.compilar_llamada(&llamada, builder, variables);
                }
                self.compilar_metodo_bitwise(receptor, nombre, args, builder, variables)
            }
            _ => {
                // Bitwise methods u otros
                self.compilar_metodo_bitwise(receptor, nombre, args, builder, variables)
            }
        }
    }

    fn compilar_metodo_bitwise(
        &mut self,
        receptor: &Expresion,
        nombre: &str,
        args: &[Expresion],
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        let x = self.compilar_expresion(receptor, builder, variables)?;
        let tipo_x = builder.func.dfg.value_type(x);

        match nombre {
            // x.poner_bit(n) â†’ x | (1 << n)
            "poner_bit" => {
                let n = self.compilar_expresion(&args[0], builder, variables)?;
                let uno = builder.ins().iconst(tipo_x, 1);
                let mask = builder.ins().ishl(uno, n);
                Ok(builder.ins().bor(x, mask))
            }
            // x.quitar_bit(n) â†’ x & ~(1 << n)
            "quitar_bit" => {
                let n = self.compilar_expresion(&args[0], builder, variables)?;
                let uno = builder.ins().iconst(tipo_x, 1);
                let mask = builder.ins().ishl(uno, n);
                let not_mask = builder.ins().bnot(mask);
                Ok(builder.ins().band(x, not_mask))
            }
            // x.alternar_bit(n) â†’ x ^ (1 << n)
            "alternar_bit" => {
                let n = self.compilar_expresion(&args[0], builder, variables)?;
                let uno = builder.ins().iconst(tipo_x, 1);
                let mask = builder.ins().ishl(uno, n);
                Ok(builder.ins().bxor(x, mask))
            }
            // x.extraer_bits(offset, cantidad) â†’ (x >> offset) & ((1 << cantidad) - 1)
            "extraer_bits" => {
                let offset = self.compilar_expresion(&args[0], builder, variables)?;
                let cantidad = self.compilar_expresion(&args[1], builder, variables)?;
                let shifted = builder.ins().ushr(x, offset);
                let uno = builder.ins().iconst(tipo_x, 1);
                let mask = builder.ins().ishl(uno, cantidad);
                let menos_uno = builder.ins().iconst(tipo_x, -1);
                let mask_menos1 = builder.ins().iadd(mask, menos_uno);
                Ok(builder.ins().band(shifted, mask_menos1))
            }
            // x.ceros_izquierda() â†’ clz
            "ceros_izquierda" => {
                Ok(builder.ins().clz(x))
            }
            // x.unos() â†’ popcount
            "unos" => {
                Ok(builder.ins().popcnt(x))
            }
            _ => {
                // No deberÃ­a llegar aquÃ­ (semantic lo filtra)
                Ok(x)
            }
        }
    }

    fn extraer_ms_de_suspension(
        &mut self,
        expr: &Expresion,
        builder: &mut FunctionBuilder,
        variables: &HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)>,
    ) -> Result<cranelift_codegen::ir::Value, ()> {
        // La expresiÃ³n es la llamada a dormir(ms)
        // Extraer solo el argumento ms (no compilar la llamada Sleep completa)
        if let Expresion::Llamada(llamada) = expr {
            if llamada.funcion == "dormir" && !llamada.argumentos.is_empty() {
                let ms_val = self.compilar_expresion(&llamada.argumentos[0], builder, variables)?;
                // Extender a i64 si es i32
                let ms_i64 = builder.ins().uextend(types::I64, ms_val);
                return Ok(ms_i64);
            }
        }
        // Fallback: compilar como expresiÃ³n y extender
        let val = self.compilar_expresion(expr, builder, variables)?;
        let val_i64 = builder.ins().uextend(types::I64, val);
        Ok(val_i64)
    }

    /// Compila una funciÃ³n normal (no-futuro) â€” extraÃ­do de compilar_funcion para reuso
    fn compilar_funcion_normal(&mut self, func: &FuncionDecl) -> Result<(), ()> {
        // Misma lÃ³gica que compilar_funcion original
        if func.es_insegura && func.cuerpo.sentencias.is_empty() {
            return Ok(());
        }

        let mut ctx = self.module.make_context();
        let mut func_ctx = FunctionBuilderContext::new();

        let mut sig = Signature::new(self.call_conv_default());
        if let Some(ref ret) = func.retorno {
            sig.returns.push(AbiParam::new(self.tipo_a_cranelift(ret)));
        }
        for param in &func.parametros {
            sig.params.push(AbiParam::new(self.tipo_a_cranelift(&param.tipo)));
        }
        ctx.func.signature = sig;

        let mut builder = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);
        let entry_block = builder.create_block();

        for param in &func.parametros {
            let tipo = self.tipo_a_cranelift(&param.tipo);
            builder.append_block_param(entry_block, tipo);
        }

        builder.switch_to_block(entry_block);
        builder.seal_block(entry_block);

        let mut variables: HashMap<String, (cranelift_codegen::ir::StackSlot, Tipo, crate::ast::Articulo)> = HashMap::new();

        for (i, param) in func.parametros.iter().enumerate() {
            let val = builder.block_params(entry_block)[i];
            let tamano = self.tamano_tipo(&param.tipo);
            let slot = builder.create_sized_stack_slot(
                cranelift_codegen::ir::StackSlotData::new(
                    cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                    tamano,
                    0,
                )
            );
            builder.ins().stack_store(val, slot, 0);
            variables.insert(param.nombre.clone(), (slot, param.tipo.clone(), param.articulo));
        }

        for sentencia in &func.cuerpo.sentencias {
            self.compilar_sentencia(sentencia, &mut builder, &mut variables, &func.span)?;
        }

        if func.retorno.is_none() {
            builder.ins().return_(&[]);
        }

        builder.finalize();

        let func_id = match self.funciones.get(&func.nombre).copied() {
            Some(id) => id,
            None => {
                self.errores.agregar(ErrorCompilador::nuevo(
                    CategoriaError::Interno, 10, func.span.clone(),
                    format!("FunciÃ³n '{}' no declarada previamente", func.nombre),
                ));
                return Err(());
            }
        };

        self.definir_funcion(func_id, &mut ctx, &func.nombre).map_err(|e| {
            self.errores.agregar(ErrorCompilador::nuevo(
                CategoriaError::Interno, 10, func.span.clone(),
                format!("Error definiendo función: {}", e),
            ));
        })?;

        Ok(())
    }
}
